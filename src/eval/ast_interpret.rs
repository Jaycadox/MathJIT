use crate::ops;

pub struct AstInterpreter;

impl super::MathEval for AstInterpreter {
    fn eval(&mut self, ops: &ops::MathOp) -> Option<f64> {
        match ops {
            ops::MathOp::Add { lhs, rhs } => Some(self.eval(lhs)? + self.eval(rhs)?),
            ops::MathOp::Sub { lhs, rhs } => Some(self.eval(lhs)? - self.eval(rhs)?),
            ops::MathOp::Mul { lhs, rhs } => Some(self.eval(lhs)? * self.eval(rhs)?),
            ops::MathOp::Div { lhs, rhs } => Some(self.eval(lhs)? / self.eval(rhs)?),
            ops::MathOp::Exp { lhs, rhs } => Some(self.eval(lhs)?.powf(self.eval(rhs)?)),
            ops::MathOp::Num(x) => Some(*x),
            ops::MathOp::Neg(x) => Some(-self.eval(x)?),
        }
    }
}
