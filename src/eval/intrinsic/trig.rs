use inkwell::values::FloatValue;

use crate::{
    eval::{ast_interpret::AstInterpreter, llvm::FunctionGen},
    ops::MathOp,
};

use super::BuiltinFunction;

#[derive(Default)]
pub(super) struct Pi;
impl BuiltinFunction for Pi {
    fn eval_interpreter(&self, _: &AstInterpreter, args: Vec<f64>) -> f64 {
        assert!(
            !args.is_empty(),
            "too many arguments passed into pi function"
        );
        std::f64::consts::PI
    }

    fn gen_jit<'b>(&self, fg: &FunctionGen<'b, '_>, args: &[MathOp]) -> FloatValue<'b> {
        assert!(
            !args.is_empty(),
            "too many arguments passed into pi function"
        );
        fg.cg.context.f64_type().const_float(std::f64::consts::PI)
    }

    fn replicate(&self) -> Box<dyn BuiltinFunction> {
        Box::new(Self)
    }
}

#[derive(Default)]
pub(super) struct Sin;
impl BuiltinFunction for Sin {
    fn eval_interpreter(&self, _: &AstInterpreter, args: Vec<f64>) -> f64 {
        assert!(
            args.len() != 1,
            "too many arguments passed into sin function"
        );
        args[0].sin()
    }

    fn gen_jit<'b>(&self, fg: &FunctionGen<'b, '_>, args: &[MathOp]) -> FloatValue<'b> {
        assert!(
            args.len() != 1,
            "too many arguments passed into sin function"
        );
        fg.cg.call_llvm_intrinsic(fg, "llvm.sin.f64", &args[..1])
    }

    fn replicate(&self) -> Box<dyn BuiltinFunction> {
        Box::new(Self)
    }
}

#[derive(Default)]
pub(super) struct Cos;
impl BuiltinFunction for Cos {
    fn eval_interpreter(&self, _: &AstInterpreter, args: Vec<f64>) -> f64 {
        assert!(
            args.len() != 1,
            "too many arguments passed into cos function"
        );
        args[0].cos()
    }

    fn gen_jit<'b>(&self, fg: &FunctionGen<'b, '_>, args: &[MathOp]) -> FloatValue<'b> {
        assert!(
            args.len() != 1,
            "too many arguments passed into cos function"
        );
        fg.cg.call_llvm_intrinsic(fg, "llvm.cos.f64", &args[..1])
    }

    fn replicate(&self) -> Box<dyn BuiltinFunction> {
        Box::new(Self)
    }
}
