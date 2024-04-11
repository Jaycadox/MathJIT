use crate::ops::MathOp;

pub mod ast_interpret;
pub mod llvm;

pub trait MathEval {
    fn eval(&mut self, ops: &MathOp) -> Option<f64>;
}
