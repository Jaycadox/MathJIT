#[derive(Debug, Clone)]
pub enum MathOp {
    Add { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Sub { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Mul { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Div { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Exp { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Neg(Box<MathOp>),
    Num(f64),
}
