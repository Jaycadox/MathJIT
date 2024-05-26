use crate::{
    ops::MathOp,
    parser::{Function, ParseOutput},
    timings::Timings,
};

use super::{
    intrinsic::{self},
    Eval, Response,
};

pub struct AstInterpreter {
    pub functions: Vec<Function>,
}

impl AstInterpreter {
    pub fn eval_func(&self, ops: &MathOp, func: &Function, current_args: &[f64]) -> Option<f64> {
        Some(match ops {
            MathOp::Add { lhs, rhs } => {
                self.eval_func(lhs, func, current_args)?
                    + self.eval_func(rhs, func, current_args)?
            }
            MathOp::Sub { lhs, rhs } => {
                self.eval_func(lhs, func, current_args)?
                    - self.eval_func(rhs, func, current_args)?
            }
            MathOp::Mul { lhs, rhs } => {
                self.eval_func(lhs, func, current_args)?
                    * self.eval_func(rhs, func, current_args)?
            }
            MathOp::Div { lhs, rhs } => {
                self.eval_func(lhs, func, current_args)?
                    / self.eval_func(rhs, func, current_args)?
            }
            MathOp::Exp { lhs, rhs } => self
                .eval_func(lhs, func, current_args)?
                .powf(self.eval_func(rhs, func, current_args)?),
            MathOp::Num(x) => *x,
            MathOp::Neg(x) => -self.eval_func(x, func, current_args)?,
            MathOp::Call { name, args } => {
                let Some(func) = self.functions.iter().find(|x| x.name == *name) else {
                    if let Some(ifunc) = intrinsic::standard_intrinsics().get(&name[..]) {
                        return Some(
                            ifunc.eval_interpreter(
                                self,
                                args.iter()
                                    .map(|x| self.eval_func(x, func, current_args))
                                    .collect::<Option<Vec<_>>>()?,
                            ),
                        );
                    }
                    panic!("Could not find function")
                };

                self.eval_func(
                    &func.body,
                    func,
                    &args
                        .iter()
                        .map(|x| self.eval_func(x, func, current_args))
                        .collect::<Option<Vec<_>>>()?,
                )?
            }
            MathOp::Arg(n) => {
                if let Some((index, _)) = func.args.iter().enumerate().find(|x| x.1 == n) {
                    *current_args.get(index).expect("Could not find argument")
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

        Self { functions: vec![] }
    }

    fn eval(&mut self, ops: ParseOutput) -> Option<(super::Response, Timings)> {
        let timings = Timings::start();
        match ops {
            ParseOutput::Body(ops) => Some((
                Response::Value(self.eval_func(
                    &ops,
                    &Function {
                        name: String::new(),
                        args: vec![],
                        body: ops.clone(),
                    },
                    &[],
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
                Some((Response::Ok, timings))
            }
        }
    }
}
