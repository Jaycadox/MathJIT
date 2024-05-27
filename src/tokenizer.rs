use anyhow::{anyhow, Result};

use crate::util;

#[derive(Debug, Clone)]
pub enum MathToken {
    Add(usize),
    Sub(usize),
    Div(usize),
    Mul(usize),
    Open(usize),
    Close(usize),
    Exp(usize),
    Num(usize, f64),
    Id(usize, char),
    Delim(usize),
    Eq(usize),
    Chain(usize),
}

impl MathToken {
    pub fn try_new(mut input: String) -> Result<Vec<MathToken>> {
        let mut tokens = vec![];
        let original_size = input.len();
        let original_input = input.clone();
        while !input.is_empty() {
            let mut current = input.chars().next().unwrap();
            let current_idx = original_size - input.len();
            if current == ' ' {
                input.remove(0);
                continue;
            }

            if current == '(' && matches!(tokens.last(), Some(MathToken::Num(_, _))) {
                tokens.push(MathToken::Mul(current_idx));
            }

            if let Some(trivial) = match current {
                '+' => Some(MathToken::Add(current_idx)),
                '-' => Some(MathToken::Sub(current_idx)),
                '*' => Some(MathToken::Mul(current_idx)),
                '/' => Some(MathToken::Div(current_idx)),
                '^' => Some(MathToken::Exp(current_idx)),
                '(' => Some(MathToken::Open(current_idx)),
                ')' => Some(MathToken::Close(current_idx)),
                ',' => Some(MathToken::Delim(current_idx)),
                '=' => Some(MathToken::Eq(current_idx)),
                '&' => Some(MathToken::Chain(current_idx)),
                'A'..='Z' | 'a'..='z' => Some(MathToken::Id(current_idx, current)),
                _ => None,
            } {
                input.remove(0);
                tokens.push(trivial);
                continue;
            }

            let mut num_buf = String::new();
            while !input.is_empty() && (current.is_numeric() || current == '.') {
                num_buf.push(input.remove(0));
                if !input.is_empty() {
                    current = input.chars().next().unwrap();
                }
            }
            if let Ok(num) = num_buf.parse() {
                tokens.push(MathToken::Num(current_idx, num));
                continue;
            }
            let error = util::error_message(&original_input, current_idx, current_idx);
            return Err(anyhow!("unexpected token: '{}'", current).context(error));
        }
        Ok(tokens)
    }
    pub fn position(&self) -> usize {
        *match self {
            MathToken::Add(x)
            | MathToken::Sub(x)
            | MathToken::Mul(x)
            | MathToken::Div(x)
            | MathToken::Exp(x)
            | MathToken::Open(x)
            | MathToken::Close(x)
            | MathToken::Id(x, _)
            | MathToken::Delim(x)
            | MathToken::Eq(x)
            | MathToken::Num(x, _)
            | MathToken::Chain(x) => x,
        }
    }
}
