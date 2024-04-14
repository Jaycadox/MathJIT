use crate::{parser::ParseOutput, timings::Timings};

pub mod ast_interpret;
pub mod intrinsic;
pub mod llvm;

pub enum EvalResponse {
    Value(f64),
    Ok,
}

pub trait Eval {
    fn new(verbose: bool) -> Self;
    fn eval(&mut self, ops: ParseOutput) -> Option<(EvalResponse, Timings)>;
}
