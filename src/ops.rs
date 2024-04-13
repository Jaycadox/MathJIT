#[derive(Debug, Clone)]
pub enum MathOp {
    Add { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Sub { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Mul { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Div { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Exp { lhs: Box<MathOp>, rhs: Box<MathOp> },
    Call { name: String, args: Vec<MathOp> },
    Neg(Box<MathOp>),
    Arg(char),
    Num(f64),
}
