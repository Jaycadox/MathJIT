# MathJIT
Mathematical expression evaluator with an interpreter, and a Just-in-Time compiler back-end using LLVM. Currently only supporting 64-bit floating point scalars.

## Usage
MathJIT can be invoked with `--help` on the command line to display a help message.

By default, if no mathematical expression is provided via the CLI, the application will enter a REPL mode.

The available modes are: `interpreter` and `jit`. Note that the JIT is not always faster in comparison to the interpreter, due to the time it takes for expressions to compile with LLVM, though the expression run-times are almost always shorter.

To view timing information, split into sections, use the `--timings` flag.

To view verbose logs, such as the tokenized output, the AST (and the LLVM IR, and final assembly with the JIT back-end), use the `--verbose` flag.

### Regular evaluations
MathJIT supports regular mathematical expressions, such as `1 + 1`

### User defined functions
MathJIT supports user defined functions, such as `f(x) = ((4 * x^3 - 3 * x^2 + 2 * x) * sin(x) + (5 * x^4 - 2 * x^3 + 7 * x^2) * cos(x)) / ((3 * x^2 - 2 * x + 1) * sin(x) + (2 * x^3 + x^2 - 5 * x) * cos(x))`

Which can be invoked via `f(10)`.

### Intrinsic functions
`sqrt(number)`, `sin(numer)`, `cos(number)`, `pi()`, `sum(min, max, step)` (this will return the summation of your previously defined function, given it has one parameter. Between min and max, and with a step size of step)

### When should I use the JIT back-end?
Generally, it should be used for computationally expensive functions, which take more than a couple milliseconds.

## Building

``` sh
git clone https://github.com/jaycadox/mathjit --depth 1
cd mathjit
cargo build -r
```


