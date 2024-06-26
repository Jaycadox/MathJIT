use std::collections::HashMap;

use inkwell::{
    attributes::Attribute,
    builder::Builder,
    context::Context,
    execution_engine::ExecutionEngine,
    intrinsics::Intrinsic,
    memory_buffer::MemoryBuffer,
    module::Module,
    passes::PassBuilderOptions,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    values::{FloatValue, FunctionValue},
    OptimizationLevel,
};

use crate::{
    ops::MathOp,
    parser::{Function, ParseOutput},
    timings::Timings,
};

use super::{
    intrinsic::{self, BuiltinFunction},
    Eval, Response,
};

pub struct Jit {
    pub verbose: bool,
    pub compile_ms: f64,
    pub run_ms: f64,
    context: Context,
    functions: Vec<Function>,
    cached_module: Option<Vec<u8>>,
}

type EvalFunc = unsafe extern "C" fn() -> f64;

pub struct CodeGen<'a> {
    pub context: &'a Context,
    pub module: Module<'a>,
    pub builder: Builder<'a>,
    execution_engine: ExecutionEngine<'a>,
    intrinsics: HashMap<&'static str, Box<dyn BuiltinFunction>>,
    pub functions: &'a [Function],
}

pub struct FunctionGen<'a, 'b> {
    pub cg: &'b CodeGen<'a>,
    pub func: &'b Function,
    pub llvm_func: FunctionValue<'a>,
}

enum FunctionKind<'a> {
    Normal(FunctionValue<'a>),
    Intrinsic(Box<dyn BuiltinFunction>),
}

impl<'a> CodeGen<'a> {
    fn compile(&self, ops: &Function, _verbose: bool) {
        let f64_type = self.context.f64_type();
        let fn_type = f64_type.fn_type(&vec![f64_type.into(); ops.args.len()][..], false);
        let function = self.module.add_function(&ops.name, fn_type, None);

        let nofree = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("nofree"), 0);
        let nocallback = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("nocallback"), 0);
        let nounwind = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("nounwind"), 0);
        let speculatable = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("speculatable"), 0);
        let willreturn = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("willreturn"), 0);
        let alwaysinline = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("alwaysinline"), 0);
        let hot = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("hot"), 0);
        let inlinehint = self
            .context
            .create_enum_attribute(Attribute::get_named_enum_kind_id("inlinehint"), 0);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, nofree);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, nocallback);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, nounwind);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, speculatable);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, willreturn);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, alwaysinline);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, hot);
        function.add_attribute(inkwell::attributes::AttributeLoc::Function, inlinehint);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);

        let gen = FunctionGen {
            cg: self,
            func: ops,
            llvm_func: function,
        };

        self.builder
            .build_return(Some(&self.build_block(&ops.body, &gen)))
            .expect("Failed to build return");
    }

    pub fn build_block(&self, ops: &MathOp, gen: &FunctionGen<'a, '_>) -> FloatValue<'a> {
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
                let lhs = *lhs.clone();
                let rhs = *rhs.clone();
                self.call_llvm_intrinsic(gen, "llvm.pow.f64", &[lhs, rhs])
            }
            MathOp::Call { name, args } => match self.get_function(name) {
                FunctionKind::Intrinsic(func) => func.gen_jit(gen, args),
                FunctionKind::Normal(cfunc) => {
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
            },
            MathOp::Arg(n) => {
                if let Some((index, _)) = gen.func.args.iter().enumerate().find(|x| x.1 == n) {
                    let arg = gen
                        .llvm_func
                        .get_nth_param(u32::try_from(index).unwrap())
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

    fn get_function(&self, name: &str) -> FunctionKind<'a> {
        if let Some(func) = self.module.get_function(name) {
            return FunctionKind::Normal(func);
        } else if let Some(func) = self.intrinsics.get(name) {
            return FunctionKind::Intrinsic(func.replicate());
        }
        panic!("could not find function {name}")
    }

    pub fn call_llvm_intrinsic(
        &self,
        gen: &FunctionGen<'a, '_>,
        name: &str,
        args: &[MathOp],
    ) -> FloatValue<'a> {
        let pow_intrinsic =
            Intrinsic::find(name).unwrap_or_else(|| panic!("Failed to find {name} intrinsic"));
        let pow_fn = pow_intrinsic
            .get_declaration(
                &self.module,
                &vec![self.context.f64_type().into(); args.len()],
            )
            .unwrap_or_else(|| panic!("Failed to get {name} declaration"));
        let call_args = args
            .iter()
            .map(|x| self.build_block(x, gen).into())
            .collect::<Vec<_>>();
        let call = self
            .builder
            .build_call(pow_fn, &call_args, "call")
            .expect("Failed to call");
        let ret = call
            .try_as_basic_value()
            .left()
            .expect("Could not find left value")
            .into_float_value();
        ret
    }
}

impl Jit {
    fn compile_function(&self, codegen: &CodeGen, func: &Function, timings: &mut Timings) {
        codegen.compile(func, self.verbose);
        timings.lap(&format!("Codegen({})", func.name));
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
            intrinsics: intrinsic::standard_intrinsics(),
            functions: &self.functions,
        };
        codegen
    }
}

impl Eval for Jit {
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

    fn eval(&mut self, ops: ParseOutput) -> Option<(Response, Timings)> {
        self.functions.retain(|x| x.name != "_repl");
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
                changed_functions.push(item.name.clone());
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
            .for_each(|x| self.compile_function(&codegen, x, &mut timings));

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
        let passes: &[&str] = &[
            "instcombine",
            "lcssa",
            "jump-threading",
            "loop-reduce",
            "loop-rotate",
            "loop-simplify",
            "loop-unroll",
            "sroa",
            "sccp",
            "sink",
            "reassociate",
            "gvn",
            "simplifycfg",
            "mem2reg",
        ];
        let pass_cfg = PassBuilderOptions::create();
        pass_cfg.set_loop_interleaving(true);
        pass_cfg.set_loop_slp_vectorization(true);
        pass_cfg.set_loop_unrolling(true);
        pass_cfg.set_loop_vectorization(true);
        pass_cfg.set_merge_functions(true);

        codegen
            .module
            .run_passes(&passes.join(","), &machine, pass_cfg)
            .unwrap();

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
            return Some((Response::Value(val), timings));
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

        Some((Response::Ok, timings))
    }
}
