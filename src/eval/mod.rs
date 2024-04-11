use crate::{ops::MathOp, timings::Timings};

pub mod ast_interpret;
pub mod llvm;

pub trait Eval {
    fn new(verbose: bool) -> Self;
    fn eval(&mut self, ops: &MathOp) -> Option<(f64, Timings)>;
}
