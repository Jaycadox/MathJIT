mod eval;
mod ops;
mod parser;
mod tokenizer;
mod util;

use crate::eval::{ast_interpret::AstInterpreter, MathEval};

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    let math_expr = args.get(1).expect("Math argument provided");
    let mut parser = match parser::MathParser::new(&math_expr) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Tokenizer error:");
            for cause in e.chain() {
                eprintln!("{cause}");
            }
            std::process::exit(1);
        }
    };

    let ops = match parser.parse() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Parser error:");
            for cause in e.chain() {
                eprintln!("{cause}");
            }
            std::process::exit(1);
        }
    };

    println!("{:?}", ops);
    println!("{}", AstInterpreter.eval(&ops).unwrap());
}
