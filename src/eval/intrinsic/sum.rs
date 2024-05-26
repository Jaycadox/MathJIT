use inkwell::values::FloatValue;

use crate::{
    eval::{ast_interpret::AstInterpreter, llvm::FunctionGen},
    ops::MathOp,
};

use super::BuiltinFunction;

#[derive(Default)]
pub(super) struct Sum;
impl BuiltinFunction for Sum {
    fn eval_interpreter(&self, ast: &AstInterpreter, args: Vec<f64>) -> f64 {
        assert!(
            args.len() != 3,
            "too many arguments passed into Sum function"
        );

        let (start, stop, step) = (args[0], args[1], args[2]);
        let Some(func) = ast.functions.last() else {
            panic!("could not find last function for sum function");
        };

        assert!(
            func.args.len() != 1,
            "last function takes too many argument"
        );

        let mut sum = 0.0;
        let mut i = start;
        loop {
            sum += ast.eval_func(&func.body, func, &[i]).unwrap();
            i += step;
            if i > stop {
                break;
            }
        }
        sum
    }

    fn gen_jit<'b>(&self, fg: &FunctionGen<'b, '_>, args: &[MathOp]) -> FloatValue<'b> {
        assert!(
            args.len() != 3,
            "too many arguments passed into Sum function"
        );
        let (start, stop, step) = (
            fg.cg.build_block(args.first().as_ref().unwrap(), fg),
            fg.cg.build_block(args.get(1).as_ref().unwrap(), fg),
            fg.cg.build_block(args.get(2).as_ref().unwrap(), fg),
        );
        let Some(func) = fg
            .cg
            .functions
            .iter()
            .filter(|x| x.name != "_repl")
            .last()
            .and_then(|x| fg.cg.module.get_function(&x.name))
        else {
            panic!("could not find last function for sum function");
        };

        assert!(
            func.count_params() != 1,
            "last function {} has an incorrect number of arguments {}",
            func.get_name().to_string_lossy(),
            func.count_params()
        );

        let counter = fg
            .cg
            .builder
            .build_alloca(fg.cg.context.f64_type(), "counter")
            .unwrap();
        let sum = fg
            .cg
            .builder
            .build_alloca(fg.cg.context.f64_type(), "sum")
            .unwrap();

        fg.cg.builder.build_store(counter, start).unwrap();
        fg.cg
            .builder
            .build_store(sum, fg.cg.context.f64_type().const_zero())
            .unwrap();

        let loop_blk = fg.cg.context.append_basic_block(fg.llvm_func, "loop");
        fg.cg.builder.build_unconditional_branch(loop_blk).unwrap();
        fg.cg.builder.position_at_end(loop_blk);

        let fn_call = fg
            .cg
            .builder
            .build_call(
                func,
                &[fg.cg
                    .builder
                    .build_load(fg.cg.context.f64_type(), counter, "load counter")
                    .unwrap()
                    .into_float_value()
                    .into()],
                "func call",
            )
            .expect("Failed to call");

        let ret = fn_call
            .try_as_basic_value()
            .left()
            .expect("Could not find left value");
        let new_sum = fg
            .cg
            .builder
            .build_float_add::<FloatValue>(
                ret.into_float_value(),
                fg.cg
                    .builder
                    .build_load(fg.cg.context.f64_type(), sum, "load sum")
                    .unwrap()
                    .into_float_value(),
                "add sum",
            )
            .unwrap();

        fg.cg.builder.build_store(sum, new_sum).unwrap();

        let new_counter = fg
            .cg
            .builder
            .build_float_add::<FloatValue>(
                fg.cg
                    .builder
                    .build_load(fg.cg.context.f64_type(), counter, "load counter")
                    .unwrap()
                    .into_float_value(),
                step,
                "add counter",
            )
            .unwrap();

        fg.cg.builder.build_store(counter, new_counter).unwrap();
        let cmp = fg
            .cg
            .builder
            .build_float_compare(inkwell::FloatPredicate::OLE, new_counter, stop, "check")
            .unwrap();
        let loop_exit_blk = fg.cg.context.append_basic_block(fg.llvm_func, "exit");
        fg.cg
            .builder
            .build_conditional_branch(cmp, loop_blk, loop_exit_blk)
            .unwrap();
        fg.cg.builder.position_at_end(loop_exit_blk);
        new_sum
    }

    fn replicate(&self) -> Box<dyn BuiltinFunction> {
        Box::new(Self)
    }
}
