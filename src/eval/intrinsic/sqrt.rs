use inkwell::values::FloatValue;

use crate::{
    eval::{ast_interpret::AstInterpreter, llvm::FunctionGen},
    ops::MathOp,
};

use super::IntrinsicFunction;

#[derive(Default)]
pub(super) struct Sqrt;
impl IntrinsicFunction for Sqrt {
    fn eval_interpreter(&self, _: &AstInterpreter, args: Vec<f64>) -> f64 {
        if args.len() != 1 {
            panic!("too many arguments passed into sqrt function");
        }

        args[0].sqrt()
    }

    fn gen_jit<'b>(&self, fg: &FunctionGen<'b, '_>, args: &[MathOp]) -> FloatValue<'b> {
        if args.len() != 1 {
            panic!("too many arguments passed into sqrt function");
        }
        fg.cg.call_llvm_intrinsic(fg, "llvm.sqrt.f64", &args[..1])
    }

    fn replicate(&self) -> Box<dyn IntrinsicFunction> {
        Box::new(Self)
    }
}
