use inkwell::values::FloatValue;

use crate::{
    eval::{ast_interpret::AstInterpreter, llvm::FunctionGen},
    ops::MathOp,
};

use super::{BuiltinFunction, BuiltinProto};

#[derive(Default)]
pub(super) struct Sqrt;
impl BuiltinFunction for Sqrt {
    fn eval_interpreter(&self, _: &AstInterpreter, args: Vec<f64>) -> f64 {
        args[0].sqrt()
    }

    fn gen_jit<'b>(&self, fg: &FunctionGen<'b, '_>, args: &[MathOp]) -> FloatValue<'b> {
        fg.cg.call_llvm_intrinsic(fg, "llvm.sqrt.f64", &args[..1])
    }

    fn replicate(&self) -> Box<dyn BuiltinFunction> {
        Box::new(Self)
    }

    fn proto(&self) -> BuiltinProto {
        BuiltinProto { arg_count: 1 }
    }
}
