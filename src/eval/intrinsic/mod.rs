use std::collections::HashMap;

use inkwell::values::FloatValue;

use crate::ops::MathOp;

use super::{ast_interpret::AstInterpreter, llvm::FunctionGen};

pub trait IntrinsicFunction {
    fn eval_interpreter(&self, ast: &AstInterpreter, args: Vec<f64>) -> f64;
    fn gen_jit<'b>(&self, fg: &FunctionGen<'b, '_>, args: &[MathOp]) -> FloatValue<'b>;
    fn replicate(&self) -> Box<dyn IntrinsicFunction>;
}

mod sqrt;
mod sum;
mod trig;
pub fn standard_intrinsics() -> HashMap<&'static str, Box<dyn IntrinsicFunction>> {
    let mut funcs = HashMap::<&'static str, Box<dyn IntrinsicFunction>>::new();
    funcs.insert("sqrt", Box::new(sqrt::Sqrt));
    funcs.insert("pi", Box::new(trig::Pi));
    funcs.insert("sin", Box::new(trig::Sin));
    funcs.insert("cos", Box::new(trig::Cos));
    funcs.insert("sum", Box::new(sum::Sum));

    funcs
}
