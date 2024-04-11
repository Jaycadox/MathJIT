mod eval;
mod ops;
mod parser;
mod tokenizer;
mod util;

use anyhow::anyhow;
use std::{fmt::Display, io::Write, str::FromStr};

use crate::eval::{ast_interpret::AstInterpreter, llvm::LlvmJit, MathEval};
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
fn main() {
    let args = Args::parse();
    if let Some(expr) = args.math_expr {
        if let Some(val) = run_repl_expr(&expr, args.mode, args.verbose, args.timings) {
            println!("{val}");
        }
        return;
    }

    println!("MathJIT ({} mode)", args.mode);
    loop {
        print!("> ");
        let _ = std::io::stdout().flush();
        let mut buf = String::new();
        std::io::stdin()
            .read_line(&mut buf)
            .expect("Failed to read line");
        if let Some(val) = run_repl_expr(buf.trim(), args.mode, args.verbose, args.timings) {
            println!("{val}");
        }
    }
}
fn run_repl_expr(math_expr: &str, mode: Mode, verbose: bool, timings: bool) -> Option<f64> {
    let tokenize_start = std::time::Instant::now();
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
    let tokenize_end = std::time::Instant::now();

    let parse_start = tokenize_end;
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
    let parse_end = std::time::Instant::now();

    let eval_start = parse_end;
    let mut jit_timings = None;
    let value = match mode {
        Mode::Interpret => AstInterpreter.eval(&ops).unwrap(),
        Mode::Jit => {
            let mut llvm = LlvmJit::new(verbose);
            let val = llvm.eval(&ops).unwrap();
            jit_timings = Some((llvm.compile_ms, llvm.run_ms));
            val
        }
    };

    if timings {
        let eval_end = std::time::Instant::now();
        let total_time = eval_end.duration_since(tokenize_start).as_secs_f64() * 1000.0;

        let tokenize_time = tokenize_end.duration_since(tokenize_start).as_secs_f64() * 1000.0;
        println!(
            "Tokenization took {tokenize_time:.4} milliseconds ({:.4}%)",
            tokenize_time * 100.0 / total_time
        );

        let parse_time = parse_end.duration_since(parse_start).as_secs_f64() * 1000.0;
        println!(
            "Parsing took      {parse_time:.4} milliseconds ({:.4}%)",
            parse_time * 100.0 / total_time
        );

        let eval_time = eval_end.duration_since(eval_start).as_secs_f64() * 1000.0;
        println!(
            "Evaluation took   {eval_time:.4} milliseconds ({:.4}%)",
            eval_time * 100.0 / total_time
        );
        if let Some((compile, run)) = jit_timings {
            println!(
                "\tCompilation took {compile:.4} milliseconds ({:.4}% of Eval)",
                compile * 100.0 / eval_time
            );
            println!(
                "\tRuntime took     {run:.4} milliseconds ({:.4}% of Eval)",
                run * 100.0 / eval_time
            );
        }
    }
    Some(value)
}
