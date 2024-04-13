use std::{rc::Rc, sync::RwLock};

use crate::{
    ops::MathOp,
    parser::{Function, ParseOutput},
    timings::Timings,
};

use super::{Eval, EvalResponse};

pub struct AstInterpreter {
    functions: Vec<Function>,
    arg_frames: Rc<RwLock<Vec<Vec<f64>>>>,
}

impl AstInterpreter {
    fn inner_eval(&self, ops: &MathOp, func: &Function) -> Option<f64> {
        Some(match ops {
            MathOp::Add { lhs, rhs } => self.inner_eval(lhs, func)? + self.inner_eval(rhs, func)?,
            MathOp::Sub { lhs, rhs } => self.inner_eval(lhs, func)? - self.inner_eval(rhs, func)?,
            MathOp::Mul { lhs, rhs } => self.inner_eval(lhs, func)? * self.inner_eval(rhs, func)?,
            MathOp::Div { lhs, rhs } => self.inner_eval(lhs, func)? / self.inner_eval(rhs, func)?,
            MathOp::Exp { lhs, rhs } => self
                .inner_eval(lhs, func)?
                .powf(self.inner_eval(rhs, func)?),
            MathOp::Num(x) => *x,
            MathOp::Neg(x) => -self.inner_eval(x, func)?,
            MathOp::Call { name, args } => {
                // Fill function frame with evaluated arguments
                let mut frame = Vec::new();
                for arg in args {
                    let val = self.inner_eval(arg, func)?;
                    frame.push(val);
                }

                // Push function frame and call function
                self.arg_frames.write().unwrap().push(frame);
                let func = self
                    .functions
                    .iter()
                    .find(|x| x.name == *name)
                    .expect("Could not find function");
                let val = self.inner_eval(&func.body, func)?;

                // Pop function frame after function call
                self.arg_frames.write().unwrap().pop();
                val
            }
            MathOp::Arg(n) => {
                if let Some((index, _)) = func.args.iter().enumerate().find(|x| x.1 == n) {
                    *self
                        .arg_frames
                        .read()
                        .unwrap()
                        .last()
                        .expect("Could not find function frame")
                        .get(index)
                        .expect("Could not find argument")
                } else {
                    panic!("Argument specified in function body was not passed in function call")
                }
            }
        })
    }
}

impl Eval for AstInterpreter {
    fn new(verbose: bool) -> Self {
        let _ = verbose;

        Self {
            functions: vec![],
            arg_frames: Rc::new(RwLock::new(vec![])),
        }
    }

    fn eval(&mut self, ops: ParseOutput) -> Option<(super::EvalResponse, Timings)> {
        let timings = Timings::start();
        match ops {
            ParseOutput::Body(ops) => Some((
                EvalResponse::Value(self.inner_eval(
                    &ops,
                    &Function {
                        name: "".to_string(),
                        args: vec![],
                        body: ops.clone(),
                    },
                )?),
                timings,
            )),
            ParseOutput::Functions(funcs) => {
                for func in funcs {
                    if let Some(item) = self.functions.iter_mut().find(|x| x.name == func.name) {
                        *item = func;
                    } else {
                        self.functions.push(func);
                    }
                }
                Some((EvalResponse::Ok, timings))
            }
        }
    }
}
