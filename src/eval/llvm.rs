use inkwell::{
    builder::Builder,
    context::Context,
    execution_engine::ExecutionEngine,
    intrinsics::Intrinsic,
    memory_buffer::MemoryBuffer,
    module::Module,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    values::{FloatValue, FunctionValue},
    OptimizationLevel,
};

use crate::{
    ops::MathOp,
    parser::{Function, ParseOutput},
    timings::Timings,
};

use super::{Eval, EvalResponse};

pub struct LlvmJit {
    pub verbose: bool,
    pub compile_ms: f64,
    pub run_ms: f64,
    context: Context,
    functions: Vec<Function>,
    cached_module: Option<Vec<u8>>,
}

type EvalFunc = unsafe extern "C" fn() -> f64;

pub struct CodeGen<'a> {
    context: &'a Context,
    module: Module<'a>,
    builder: Builder<'a>,
    execution_engine: ExecutionEngine<'a>,
}

pub struct FunctionGen<'a, 'b> {
    _cg: &'b CodeGen<'a>,
    func: &'b Function,
    llvm_func: FunctionValue<'a>,
}

impl<'a> CodeGen<'a> {
    fn compile(&self, ops: &Function, _verbose: bool) -> Option<()> {
        let f64_type = self.context.f64_type();
        let fn_type = f64_type.fn_type(&vec![f64_type.into(); ops.args.len()][..], false);
        let function = self.module.add_function(&ops.name, fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        let gen = FunctionGen {
            _cg: self,
            func: ops,
            llvm_func: function,
        };

        self.builder
            .build_return(Some(&self.build_block(&ops.body, &gen)))
            .expect("Failed to build return");
        Some(())
    }

    fn build_block(&self, ops: &MathOp, gen: &FunctionGen<'a, '_>) -> FloatValue<'a> {
        match ops {
            MathOp::Num(x) => self.context.f64_type().const_float(*x),
            MathOp::Neg(x) => self
                .builder
                .build_float_neg(self.build_block(x, gen), "float neg")
                .expect("Failed to negate float"),
            MathOp::Add { lhs, rhs } => self
                .builder
                .build_float_add(
                    self.build_block(lhs, gen),
                    self.build_block(rhs, gen),
                    "float add",
                )
                .expect("Failed to add floats"),
            MathOp::Sub { lhs, rhs } => self
                .builder
                .build_float_sub(
                    self.build_block(lhs, gen),
                    self.build_block(rhs, gen),
                    "float sub",
                )
                .expect("Failed to sub floats"),
            MathOp::Mul { lhs, rhs } => self
                .builder
                .build_float_mul(
                    self.build_block(lhs, gen),
                    self.build_block(rhs, gen),
                    "float mul",
                )
                .expect("Failed to mul floats"),
            MathOp::Div { lhs, rhs } => self
                .builder
                .build_float_div(
                    self.build_block(lhs, gen),
                    self.build_block(rhs, gen),
                    "float div",
                )
                .expect("Failed to div floats"),
            MathOp::Exp { lhs, rhs } => {
                let pow_intrinsic =
                    Intrinsic::find("llvm.pow.f64").expect("Failed to find llvm.pow.f64 intrinsic");
                let pow_fn = pow_intrinsic
                    .get_declaration(
                        &self.module,
                        &[
                            self.context.f64_type().into(),
                            self.context.f64_type().into(),
                        ],
                    )
                    .expect("Failed to get llvm.pow.f64 declaration");
                let call = self
                    .builder
                    .build_call(
                        pow_fn,
                        &[
                            self.build_block(lhs, gen).into(),
                            self.build_block(rhs, gen).into(),
                        ],
                        "powf call",
                    )
                    .expect("Failed to call powf");
                let ret = call
                    .try_as_basic_value()
                    .left()
                    .expect("Could not find left value")
                    .into_float_value();
                ret
            }
            MathOp::Call { name, args } => {
                let cfunc = self
                    .module
                    .get_function(name)
                    .expect("Could not find function");
                let fn_args = args
                    .iter()
                    .map(|x| self.build_block(x, gen).into())
                    .collect::<Vec<_>>();
                let fn_call = self
                    .builder
                    .build_call(cfunc, &fn_args[..], "func call")
                    .expect("Failed to call");
                let ret = fn_call
                    .try_as_basic_value()
                    .left()
                    .expect("Could not find left value")
                    .into_float_value();
                ret
            }
            MathOp::Arg(n) => {
                if let Some((index, _)) = gen.func.args.iter().enumerate().find(|x| x.1 == n) {
                    let arg = gen
                        .llvm_func
                        .get_nth_param(index as u32)
                        .expect("Could not get paramter")
                        .into_float_value();
                    return arg;
                }
                panic!("could not find argument")
            }
        }
    }
    fn get_assembly(&self) -> String {
        let triple = TargetMachine::get_default_triple();
        let cpu = TargetMachine::get_host_cpu_name().to_string();
        let features = TargetMachine::get_host_cpu_features().to_string();

        let target = Target::from_triple(&triple).unwrap();
        let machine = target
            .create_target_machine(
                &triple,
                &cpu,
                &features,
                OptimizationLevel::Aggressive,
                RelocMode::Default,
                CodeModel::JITDefault,
            )
            .unwrap();
        let mem_buf = machine
            .write_to_memory_buffer(&self.module, inkwell::targets::FileType::Assembly)
            .expect("Failed to get memory buffer");
        let asm = String::from_utf8_lossy(mem_buf.as_slice());
        asm.to_string()
    }
}

impl LlvmJit {
    fn compile_function(
        &self,
        codegen: &CodeGen,
        func: &Function,
        timings: &mut Timings,
    ) -> Option<()> {
        codegen
            .compile(func, self.verbose)
            .expect("Failed to JIT compile");
        timings.lap(&format!("Codegen({})", func.name));

        Some(())
    }

    fn create_codegen(&self, cached_module: &Option<Vec<u8>>) -> CodeGen {
        let module = if let Some(cached_module) = cached_module.as_ref() {
            Module::parse_bitcode_from_buffer(
                &MemoryBuffer::create_from_memory_range(cached_module, "Cached module"),
                &self.context,
            )
            .unwrap()
        } else {
            self.context.create_module("jit")
        };

        let execution_engine = module
            .create_jit_execution_engine(inkwell::OptimizationLevel::Aggressive)
            .expect("Failed to create execution engine");

        let codegen = CodeGen {
            context: &self.context,
            module,
            builder: self.context.create_builder(),
            execution_engine,
        };
        codegen
    }
}

impl Eval for LlvmJit {
    fn new(verbose: bool) -> Self {
        let config = InitializationConfig {
            asm_printer: true,
            ..Default::default()
        };

        Target::initialize_native(&config).expect("failed to initialize target");
        let context = Context::create();
        Self {
            verbose,
            compile_ms: 0f64,
            run_ms: 0f64,
            context,
            functions: Vec::new(),
            cached_module: None,
        }
    }

    fn eval(&mut self, ops: ParseOutput) -> Option<(EvalResponse, Timings)> {
        if self.verbose {
            println!("--- AST ---\n{ops:?}");
        }

        self.functions.retain(|x| !x.name.starts_with("__repl__"));
        let (functions, exec_last) = match ops {
            ParseOutput::Body(ops) => (
                vec![Function {
                    name: "_repl".to_string(),
                    args: vec![],
                    body: ops,
                }],
                true,
            ),
            ParseOutput::Functions(funcs) => (funcs, false),
        };

        let mut changed_functions = vec![];

        for func in functions {
            if let Some(item) = self.functions.iter_mut().find(|x| x.name == func.name) {
                *item = func;
                changed_functions.push(item.name.to_owned());
            } else {
                self.functions.push(func);
            }
        }

        let mut timings = Timings::start();
        let codegen = self.create_codegen(&self.cached_module);
        timings.lap("CreateCodegen");

        self.functions
            .iter()
            .filter(|x| {
                changed_functions.contains(&x.name)
                    || codegen.module.get_function(&x.name).is_none()
            })
            .map(|x| self.compile_function(&codegen, x, &mut timings))
            .collect::<Option<Vec<()>>>()?;

        if self.verbose {
            println!("--- LLVM IR ---");
            codegen.module.print_to_stderr();
            println!("--- Assembly ---\n{}", codegen.get_assembly());
        }

        if exec_last {
            let last = &self.functions.last().unwrap().name;
            let func = unsafe {
                codegen
                    .execution_engine
                    .get_function::<EvalFunc>(last)
                    .unwrap()
                    .as_raw()
            };
            timings.lap("LLVMCompile");
            let val = unsafe { func() };
            timings.lap("Exec");
            return Some((EvalResponse::Value(val), timings));
        }

        let cached = codegen.module.write_bitcode_to_memory().as_slice().to_vec();
        drop(codegen);

        if changed_functions.is_empty() {
            self.cached_module = Some(cached);
        } else {
            // We skip caching the module so LLVM can rebuild the entire IR with the new version of the func
            // Ideally, LLVM would provide a: module.remove_function(...)
            // Perhaps we could map changed functions with a seperate name, and call the new name? (LLVM might provide this through Function.name())
            // ^ but that might increase compile times for proper evaluations due to unneeded IR, though not caching also increases comp times
            self.cached_module = None;
        }

        Some((EvalResponse::Ok, timings))
    }
}
