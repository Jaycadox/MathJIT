use std::fmt::Display;

use crate::eval::intrinsic;
use crate::ops;
use crate::tokenizer;
use crate::util;
use anyhow::Context;
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct Parser {
    tokens: Vec<tokenizer::MathToken>,
    original_tokens: Vec<tokenizer::MathToken>,
    original_string: String,
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub args: Vec<char>,
    pub body: ops::MathOp,
}

#[derive(Debug)]
pub enum ParseOutput {
    Body(ops::MathOp),
    Functions(Vec<Function>),
}

impl Parser {
    pub fn new(input: &str) -> Result<Self> {
        let tokens = tokenizer::MathToken::try_new(input.to_string())?;
        Ok(Self {
            tokens: tokens.clone(),
            original_tokens: tokens,
            original_string: input.to_string(),
        })
    }

    pub fn original_tokens(&self) -> &[tokenizer::MathToken] {
        &self.original_tokens
    }

    fn from_tokens(input: &str, tokens: Vec<tokenizer::MathToken>) -> Self {
        Self {
            tokens: tokens.clone(),
            original_tokens: tokens,
            original_string: input.to_string(),
        }
    }

    fn peek(&self) -> Option<&tokenizer::MathToken> {
        self.tokens.first()
    }

    fn pop(&mut self) -> Option<tokenizer::MathToken> {
        if self.tokens.is_empty() {
            return None;
        }
        Some(self.tokens.remove(0))
    }

    fn parse_primary_func_call(&mut self) -> Result<Option<ops::MathOp>> {
        let mut name_buf = String::new();
        let mut args = vec![];
        while let Some(tokenizer::MathToken::Id(_, chr)) = self.peek() {
            name_buf.push(*chr);
            self.pop();
        }

        let Some(tokenizer::MathToken::Open(start)) = self.peek() else {
            return Ok(None);
        };

        let start = *start;
        let end;
        self.pop();
        loop {
            match self.peek() {
                Some(tokenizer::MathToken::Close(pos)) => {
                    end = *pos;
                    break;
                }
                _ => {
                    let arg = self.parse_expr()?;
                    args.push(arg);
                    if let Some(tokenizer::MathToken::Delim(_)) = self.peek() {
                        self.pop();
                    }
                }
            }
        }
        self.pop();

        // Attempt to perform typechecking given a function proto and the standard intrinsics, note that this is probably not the best place to be doing this.

        let standard_intrinsics = intrinsic::standard_intrinsics();
        if let Some(intrin) = standard_intrinsics.get(&name_buf[..]) {
            if intrin.proto().arg_count as usize != args.len() {
                let error = util::error_message(&self.original_string, start, end);
                return Err(anyhow!(
                    "incorrect argument count for '{name_buf}' call, {} provided, {} expected {error}",
                    args.len(),
                    intrin.proto().arg_count
                ));
            }
        }

        Ok(Some(ops::MathOp::Call {
            name: name_buf,
            args,
        }))
    }

    fn parse_primary(&mut self) -> Result<ops::MathOp> {
        if let Some(tokenizer::MathToken::Sub(_)) = self.peek() {
            self.pop();
            return Ok(ops::MathOp::Neg(Box::new(self.parse_inner_func()?)));
        }
        if let Some(tokenizer::MathToken::Open(start)) = self.peek() {
            let start = *start;
            let mut end = 0;
            let _ = self.pop();
            let mut tok_list = vec![];
            let mut depth = 1;
            while let Some(tok) = self.pop() {
                if let tokenizer::MathToken::Close(endpos) = tok {
                    end = endpos;
                    depth -= 1;
                    if depth == 0 {
                        if let Some(tokenizer::MathToken::Close(_)) = self.peek() {
                            return Err(anyhow!("brackets not balanced"));
                        }
                        break;
                    }
                } else if let tokenizer::MathToken::Open(_) = tok {
                    depth += 1;
                }
                tok_list.push(tok);
            }
            if depth != 0 {
                let error = util::error_message(&self.original_string, start, start);
                return Err(anyhow!("brackets not balanced{error}"));
            }
            let mut parser = Self::from_tokens(&self.original_string, tok_list);
            return parser.parse_inner_func().with_context(|| {
                let error = util::error_message(&self.original_string, start, end);
                anyhow!("while evaluating brackets{error}")
            });
        } else if let Some(tokenizer::MathToken::Num(_, _)) = self.peek() {
            let bb = self.pop();
            if let Some(tokenizer::MathToken::Num(_, x)) = bb {
                if let Some(tokenizer::MathToken::Open(_)) = self.peek() {
                    let expr = self.parse_primary()?;
                    return Ok(ops::MathOp::Mul {
                        lhs: Box::new(ops::MathOp::Num(x)),
                        rhs: Box::new(expr),
                    });
                }
                return Ok(ops::MathOp::Num(x));
            }
            panic!("Should never happen {bb:?}");
        } else if let Some(tokenizer::MathToken::Id(_, name)) = self.peek() {
            let name = *name;
            let before = self.tokens.clone();

            if let Some(call) = self.parse_primary_func_call()? {
                return Ok(call);
            }
            self.tokens = before;
            self.pop();
            return Ok(ops::MathOp::Arg(name));
        }
        let pos = self.peek().map_or(
            self.original_string.len() - 1,
            tokenizer::MathToken::position,
        );
        let error = util::error_message(&self.original_string, pos, pos);
        Err(anyhow!("expected number or open bracket{error}"))
    }

    fn parse_exp(&mut self) -> Result<ops::MathOp> {
        let mut lhs = self.parse_primary()?;
        loop {
            match self.peek() {
                Some(tokenizer::MathToken::Exp(_)) => {
                    let _ = self.pop();
                    let rhs = self.parse_primary()?;
                    lhs = ops::MathOp::Exp {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                _ => {
                    return Ok(lhs);
                }
            }
        }
    }

    fn parse_term(&mut self) -> Result<ops::MathOp> {
        if let Some(tokenizer::MathToken::Sub(_)) = self.peek() {
            self.pop();
            return Ok(ops::MathOp::Neg(Box::new(self.parse_term()?)));
        }
        let mut lhs = self.parse_exp()?;
        loop {
            match self.peek() {
                Some(tokenizer::MathToken::Mul(_)) => {
                    let _ = self.pop();
                    let rhs = self.parse_exp()?;
                    lhs = ops::MathOp::Mul {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                Some(tokenizer::MathToken::Div(_)) => {
                    let _ = self.pop();
                    let rhs = self.parse_exp()?;
                    lhs = ops::MathOp::Div {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                _ => {
                    return Ok(lhs);
                }
            }
        }
    }

    fn parse_expr(&mut self) -> Result<ops::MathOp> {
        if let Some(tokenizer::MathToken::Sub(_)) = self.peek() {
            self.pop();
            return Ok(ops::MathOp::Neg(Box::new(self.parse_expr()?)));
        }

        let mut lhs = self.parse_term()?;
        loop {
            match self.peek() {
                Some(tokenizer::MathToken::Add(_)) => {
                    let _ = self.pop();
                    let rhs = self.parse_term()?;
                    lhs = ops::MathOp::Add {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                Some(tokenizer::MathToken::Sub(_)) => {
                    let _ = self.pop();
                    let rhs = self.parse_term()?;
                    lhs = ops::MathOp::Sub {
                        lhs: Box::new(lhs),
                        rhs: Box::new(rhs),
                    };
                }
                _ => {
                    return Ok(lhs);
                }
            }
        }
    }

    fn parse_inner_func(&mut self) -> Result<ops::MathOp> {
        if self.tokens.is_empty() {
            return Err(anyhow!("no input provided"));
        }

        let out = self.parse_expr();
        out
    }

    fn parse_expression_chain_single(&mut self) -> Result<ParseOutput> {
        let save = self.tokens.clone();
        if let Some(func) = self.parse_full_func()? {
            return Ok(func);
        }
        self.tokens = save;

        Ok(ParseOutput::Body(self.parse_inner_func()?))
    }

    pub fn parse(&mut self) -> Result<Vec<ParseOutput>> {
        let first = self.parse_expression_chain_single()?;

        let mut exprs = vec![first];
        while matches!(self.peek(), Some(tokenizer::MathToken::Chain(_))) {
            self.pop();
            exprs.push(self.parse_expression_chain_single()?);
        }

        Ok(exprs)
    }

    fn parse_full_func(&mut self) -> Result<Option<ParseOutput>> {
        if let Some(tokenizer::MathToken::Id(_, name)) = self.peek() {
            let name = name.to_string();
            self.pop();
            if let Some(tokenizer::MathToken::Open(_)) = self.peek() {
                let mut args = vec![];
                self.pop();
                while let Some(tokenizer::MathToken::Id(_, arg_name)) = self.peek() {
                    args.push(*arg_name);
                    self.pop();
                    match self.peek() {
                        Some(tokenizer::MathToken::Delim(_)) => {
                            self.pop();
                        }
                        Some(tokenizer::MathToken::Close(_)) => {
                            break;
                        }
                        _ => {
                            return Ok(None);
                        }
                    }
                }

                if let Some(tokenizer::MathToken::Close(_)) = self.peek() {
                    self.pop();
                    if let Some(tokenizer::MathToken::Eq(_)) = self.peek() {
                        self.pop();
                        let inner_func = self.parse_inner_func()?;
                        let func = Function {
                            name,
                            args,
                            body: inner_func,
                        };
                        return Ok(Some(ParseOutput::Functions(vec![func])));
                    }
                }
            }
        }
        Ok(None)
    }
}

impl Display for Parser {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out_buf = String::new();
        for tok in &self.original_tokens {
            out_buf.push_str(&match tok {
                tokenizer::MathToken::Add(_) => " + ".to_string(),
                tokenizer::MathToken::Sub(_) => " - ".to_string(),
                tokenizer::MathToken::Div(_) => " / ".to_string(),
                tokenizer::MathToken::Mul(_) => " * ".to_string(),
                tokenizer::MathToken::Exp(_) => " ^ ".to_string(),
                tokenizer::MathToken::Open(_) => "(".to_string(),
                tokenizer::MathToken::Close(_) => ")".to_string(),
                tokenizer::MathToken::Id(_, x) => x.to_string(),
                tokenizer::MathToken::Delim(_) => ", ".to_string(),
                tokenizer::MathToken::Eq(_) => " = ".to_string(),
                tokenizer::MathToken::Num(_, x) => format!("{x}"),
                tokenizer::MathToken::Chain(_) => " & ".to_string(),
            });
        }

        write!(f, "{}", out_buf.trim())
    }
}
