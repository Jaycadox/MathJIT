use crate::ops::MathOp;

pub mod ast_interpret;

pub trait MathEval {
    fn eval(&mut self, ops: &MathOp) -> Option<f64>;
}
