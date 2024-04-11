use crate::{
    ops::{self, MathOp},
    timings::Timings,
};

use super::Eval;

pub struct AstInterpreter;

impl Eval for AstInterpreter {
    fn eval(&mut self, ops: &MathOp) -> Option<(f64, Timings)> {
        let timings = Timings::start();
        Some((
            match ops {
                ops::MathOp::Add { lhs, rhs } => self.eval(lhs)?.0 + self.eval(rhs)?.0,
                ops::MathOp::Sub { lhs, rhs } => self.eval(lhs)?.0 - self.eval(rhs)?.0,
                ops::MathOp::Mul { lhs, rhs } => self.eval(lhs)?.0 * self.eval(rhs)?.0,
                ops::MathOp::Div { lhs, rhs } => self.eval(lhs)?.0 / self.eval(rhs)?.0,
                ops::MathOp::Exp { lhs, rhs } => self.eval(lhs)?.0.powf(self.eval(rhs)?.0),
                ops::MathOp::Num(x) => *x,
                ops::MathOp::Neg(x) => -self.eval(x)?.0,
            },
            timings,
        ))
    }

    fn new(verbose: bool) -> Self {
        let _ = verbose;
        Self
    }
}
