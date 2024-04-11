use std::fmt::Display;

use crate::ops;
use crate::tokenizer;
use crate::util;
use anyhow::Context;
use anyhow::{anyhow, Result};

#[derive(Debug)]
pub struct MathParser {
    tokens: Vec<tokenizer::MathToken>,
    original_tokens: Vec<tokenizer::MathToken>,
    original_string: String,
}

impl MathParser {
    pub fn new(input: &str) -> Result<Self> {
        let tokens = tokenizer::MathToken::try_new(input.to_string())?;
        Ok(Self {
            tokens: tokens.clone(),
            original_tokens: tokens,
            original_string: input.to_string(),
        })
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

    fn parse_primary(&mut self) -> Result<ops::MathOp> {
        if let Some(tokenizer::MathToken::Sub(_)) = self.peek() {
            self.pop();
            return Ok(ops::MathOp::Neg(Box::new(self.parse()?)));
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
            return parser.parse().with_context(|| {
                let error = util::error_message(&self.original_string, start, end);
                return anyhow!("while evaluating brackets{error}");
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
            } else {
                panic!("Should never happen {bb:?}");
            }
        }
        let pos = self
            .peek()
            .map(|x| x.position())
            .unwrap_or(self.original_string.len() - 1);
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

    pub fn parse(&mut self) -> Result<ops::MathOp> {
        if self.tokens.is_empty() {
            return Err(anyhow!("no input provided"));
        }

        let out = self.parse_expr();
        if !self.tokens.is_empty() {
            let idx = self.tokens.remove(0).position();
            let msg = util::error_message(&self.original_string, idx, idx);
            return Err(anyhow!("unexpected sequence{msg}"));
        }
        return out;
    }
}

impl Display for MathParser {
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
                tokenizer::MathToken::Num(_, x) => format!("{}", x),
            });
        }

        write!(f, "{}", out_buf.trim())
    }
}
