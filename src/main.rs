mod eval;
mod ops;
mod parser;
mod timings;
mod tokenizer;
mod util;

use anyhow::anyhow;
use eval::Eval;
use parser::ParseOutput;
use rustyline::DefaultEditor;
use std::{fmt::Display, str::FromStr};
use timings::Timings;

use crate::eval::{ast_interpret::AstInterpreter, llvm::LlvmJit};
use clap::Parser;

#[derive(clap::Parser, Debug)]
#[command(
    version = "1.0",
    author = "jayphen",
    about = "MathJIT -- Just-In-Time mathematical evaluator"
)]
struct Args {
    math_expr: Option<String>,
    #[clap(short, long, default_value_t = Mode::Interpret)]
    mode: Mode,
    #[clap(short, long)]
    verbose: bool,
    #[clap(short, long)]
    timings: bool,
}

#[derive(Debug, Clone, Copy)]
enum Mode {
    Interpret,
    Jit,
}

impl Display for Mode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Mode::Interpret => "Interpreter",
                Mode::Jit => "JIT",
            }
        )
    }
}

impl FromStr for Mode {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "jit" | "j" | "JIT" => Ok(Mode::Jit),
            "interpret" | "i" | "interpreter" | "Interpreter" => Ok(Mode::Interpret),
            _ => Err(anyhow!("invalid selection, wanted 'jit' or 'interpret'")),
        }
    }
}

fn into_ops(math_expr: &str, verbose: bool) -> Option<(ParseOutput, Timings)> {
    let mut timings = Timings::start();
    let mut parser = match parser::MathParser::new(math_expr) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Tokenizer error:");
            for cause in e.chain() {
                eprintln!("{cause}");
            }
            return None;
        }
    };

    if verbose {
        println!("--- Tokenized --");
        println!("{:?}", parser.tokens());
    }

    timings.lap("Tokenizer");

    let ops = match parser.parse() {
        Ok(x) => x,
        Err(e) => {
            eprintln!("Parser error:");
            for cause in e.chain() {
                eprintln!("{cause}");
            }
            return None;
        }
    };

    if verbose {
        println!("--- AST --");
        println!("{:?}", ops);
    }

    timings.lap("Parser");
    Some((ops, timings))
}

enum ReplMode {
    Single(String),
    Loop,
}

fn main() {
    let args = Args::parse();
    let repl_mode = if let Some(expr) = &args.math_expr {
        ReplMode::Single(expr.to_string())
    } else {
        ReplMode::Loop
    };

    match args.mode {
        Mode::Interpret => {
            start_repl_loop::<AstInterpreter>(args, repl_mode);
        }
        Mode::Jit => {
            start_repl_loop::<LlvmJit>(args, repl_mode);
        }
    }
}

fn start_repl_loop<T: Eval>(args: Args, repl_mode: ReplMode) {
    if let ReplMode::Loop = repl_mode {
        println!("MathJIT ({} mode)", args.mode);
    }

    let mut rl = DefaultEditor::new().unwrap();

    let mut repl = T::new(args.verbose);
    loop {
        let input = match repl_mode {
            ReplMode::Single(ref inp) => inp.to_string(),
            ReplMode::Loop => {
                let readline = rl.readline("> ");
                match readline {
                    Ok(line) => {
                        let _ = rl.add_history_entry(line.clone());
                        line.to_string()
                    }
                    _ => std::process::exit(0),
                }
            }
        };

        if let Some(val) = run_repl_expr::<T>(&mut repl, input.trim(), args.timings, args.verbose) {
            println!("{val}");
        }

        if let ReplMode::Single(_) = repl_mode {
            break;
        }
    }
}

fn run_repl_expr<T: Eval>(
    env: &mut T,
    math_expr: &str,
    do_timings: bool,
    verbose: bool,
) -> Option<f64> {
    let mut full_timings = Timings::start();
    let (ops, timings) = into_ops(math_expr, verbose)?;
    full_timings.append(timings, "Init");

    let (value, timings) = env.eval(ops).unwrap();
    full_timings.append(timings, "Eval");
    if do_timings {
        println!("{}", full_timings.report());
    }
    match value {
        eval::EvalResponse::Ok => {
            println!("Ok");
            None
        }
        eval::EvalResponse::Value(value) => Some(value),
    }
}
