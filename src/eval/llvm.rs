use inkwell::{
    builder::Builder,
    context::Context,
    execution_engine::{ExecutionEngine, JitFunction},
    intrinsics::Intrinsic,
    module::Module,
    targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine},
    values::FloatValue,
    OptimizationLevel,
};

use crate::ops::MathOp;

use super::MathEval;

pub struct LlvmJit {
    pub verbose: bool,
    pub compile_ms: f64,
    pub run_ms: f64,
}

impl LlvmJit {
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            compile_ms: 0f64,
            run_ms: 0f64,
        }
    }
}

type EvalFunc = unsafe extern "C" fn() -> f64;

struct CodeGen<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    execution_engine: ExecutionEngine<'ctx>,
}

impl<'ctx> CodeGen<'ctx> {
    fn compile(&self, ops: &MathOp, verbose: bool) -> Option<JitFunction<EvalFunc>> {
        let f64_type = self.context.f64_type();
        let fn_type = f64_type.fn_type(&[], false);
        let function = self.module.add_function("eval", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);
        self.builder
            .build_return(Some(&self.build_block(ops)))
            .expect("Failed to build return");
        if verbose {
            println!("--- LLVM IR ---");
            function.print_to_stderr();
        }
        unsafe { self.execution_engine.get_function("eval").ok() }
    }
    fn build_block(&self, ops: &MathOp) -> FloatValue {
        match ops {
            MathOp::Num(x) => self.context.f64_type().const_float(*x),
            MathOp::Neg(x) => self
                .builder
                .build_float_neg(self.build_block(x), "float neg")
                .expect("Failed to negate float"),
            MathOp::Add { lhs, rhs } => self
                .builder
                .build_float_add(self.build_block(lhs), self.build_block(rhs), "float add")
                .expect("Failed to add floats"),
            MathOp::Sub { lhs, rhs } => self
                .builder
                .build_float_sub(self.build_block(lhs), self.build_block(rhs), "float sub")
                .expect("Failed to sub floats"),
            MathOp::Mul { lhs, rhs } => self
                .builder
                .build_float_mul(self.build_block(lhs), self.build_block(rhs), "float mul")
                .expect("Failed to mul floats"),
            MathOp::Div { lhs, rhs } => self
                .builder
                .build_float_div(self.build_block(lhs), self.build_block(rhs), "float div")
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
                        &[self.build_block(lhs).into(), self.build_block(rhs).into()],
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

impl MathEval for LlvmJit {
    fn eval(&mut self, ops: &crate::ops::MathOp) -> Option<f64> {
        let compile_start = std::time::Instant::now();
        if self.verbose {
            println!("--- AST ---\n{ops:?}");
        }

        let config = InitializationConfig {
            asm_printer: true,
            ..Default::default()
        };

        Target::initialize_native(&config).expect("failed to initialize target");
        let context = Context::create();
        let module = context.create_module("jit");
        let execution_engine = module
            .create_jit_execution_engine(inkwell::OptimizationLevel::Aggressive)
            .expect("Failed to create execution engine");

        let codegen = CodeGen {
            context: &context,
            module,
            builder: context.create_builder(),
            execution_engine,
        };
        let eval = codegen
            .compile(ops, self.verbose)
            .expect("Failed to JIT compile");

        if self.verbose {
            println!("--- Assembly ---\n{}", codegen.get_assembly());
            println!("-- Result --");
        }
        let compile_end = std::time::Instant::now();
        let ret = Some(unsafe { eval.call() });
        let runtime_end = std::time::Instant::now();
        self.compile_ms = compile_end.duration_since(compile_start).as_secs_f64() * 1000.0;
        self.run_ms = runtime_end.duration_since(compile_end).as_secs_f64() * 1000.0;

        ret
    }
}
