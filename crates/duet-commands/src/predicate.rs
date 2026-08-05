use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Context predicate AST for conditional command enablement and keybinding context resolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextPredicate {
    True,
    False,
    Flag(String),
    Eq(String, String),
    Not(Box<ContextPredicate>),
    And(Vec<ContextPredicate>),
    Or(Vec<ContextPredicate>),
}

/// Evaluation context representing current active shell UI state.
#[derive(Debug, Clone, Default)]
pub struct CommandContext {
    pub flags: HashSet<String>,
    pub vars: HashMap<String, String>,
}

impl CommandContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_flag(mut self, flag: impl Into<String>) -> Self {
        self.flags.insert(flag.into());
        self
    }

    pub fn with_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    pub fn eval(&self, pred: &ContextPredicate) -> bool {
        match pred {
            ContextPredicate::True => true,
            ContextPredicate::False => false,
            ContextPredicate::Flag(flag) => self.flags.contains(flag),
            ContextPredicate::Eq(var, val) => self.vars.get(var) == Some(val),
            ContextPredicate::Not(inner) => !self.eval(inner),
            ContextPredicate::And(preds) => preds.iter().all(|p| self.eval(p)),
            ContextPredicate::Or(preds) => preds.iter().any(|p| self.eval(p)),
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PredicateParseError {
    #[error("Unexpected token: {0}")]
    UnexpectedToken(String),
    #[error("Unexpected end of input")]
    UnexpectedEof,
    #[error("Unmatched parenthesis")]
    UnmatchedParen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Ident(String),
    StrLiteral(String),
    EqEq,
    AndAnd,
    OrOr,
    Bang,
    LParen,
    RParen,
}

fn tokenize(input: &str) -> Result<Vec<Token>, PredicateParseError> {
    let mut chars = input.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(&c) = chars.peek() {
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                chars.next();
            }
            '(' => {
                chars.next();
                tokens.push(Token::LParen);
            }
            ')' => {
                chars.next();
                tokens.push(Token::RParen);
            }
            '!' => {
                chars.next();
                tokens.push(Token::Bang);
            }
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') {
                    chars.next();
                    tokens.push(Token::EqEq);
                } else {
                    return Err(PredicateParseError::UnexpectedToken("=".to_string()));
                }
            }
            '&' => {
                chars.next();
                if chars.peek() == Some(&'&') {
                    chars.next();
                    tokens.push(Token::AndAnd);
                } else {
                    return Err(PredicateParseError::UnexpectedToken("&".to_string()));
                }
            }
            '|' => {
                chars.next();
                if chars.peek() == Some(&'|') {
                    chars.next();
                    tokens.push(Token::OrOr);
                } else {
                    return Err(PredicateParseError::UnexpectedToken("|".to_string()));
                }
            }
            '\'' | '"' => {
                let quote = c;
                chars.next();
                let mut s = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc == quote {
                        chars.next();
                        break;
                    }
                    s.push(nc);
                    chars.next();
                }
                tokens.push(Token::StrLiteral(s));
            }
            _ if c.is_alphanumeric() || c == '_' || c == '-' || c == '.' => {
                let mut s = String::new();
                while let Some(&nc) = chars.peek() {
                    if nc.is_alphanumeric() || nc == '_' || nc == '-' || nc == '.' {
                        s.push(nc);
                        chars.next();
                    } else {
                        break;
                    }
                }
                tokens.push(Token::Ident(s));
            }
            _ => {
                return Err(PredicateParseError::UnexpectedToken(c.to_string()));
            }
        }
    }

    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        if self.pos < self.tokens.len() {
            let tok = self.tokens[self.pos].clone();
            self.pos += 1;
            Some(tok)
        } else {
            None
        }
    }

    fn parse_expr(&mut self) -> Result<ContextPredicate, PredicateParseError> {
        let mut terms = vec![self.parse_term()?];
        while let Some(Token::OrOr) = self.peek() {
            self.next();
            terms.push(self.parse_term()?);
        }
        if terms.len() == 1 {
            Ok(terms.remove(0))
        } else {
            Ok(ContextPredicate::Or(terms))
        }
    }

    fn parse_term(&mut self) -> Result<ContextPredicate, PredicateParseError> {
        let mut factors = vec![self.parse_factor()?];
        while let Some(Token::AndAnd) = self.peek() {
            self.next();
            factors.push(self.parse_factor()?);
        }
        if factors.len() == 1 {
            Ok(factors.remove(0))
        } else {
            Ok(ContextPredicate::And(factors))
        }
    }

    fn parse_factor(&mut self) -> Result<ContextPredicate, PredicateParseError> {
        match self.peek() {
            Some(Token::Bang) => {
                self.next();
                let inner = self.parse_factor()?;
                Ok(ContextPredicate::Not(Box::new(inner)))
            }
            Some(Token::LParen) => {
                self.next();
                let expr = self.parse_expr()?;
                match self.next() {
                    Some(Token::RParen) => Ok(expr),
                    _ => Err(PredicateParseError::UnmatchedParen),
                }
            }
            Some(Token::Ident(name)) => {
                let name = name.clone();
                self.next();
                if let Some(Token::EqEq) = self.peek() {
                    self.next();
                    match self.next() {
                        Some(Token::Ident(val)) | Some(Token::StrLiteral(val)) => {
                            Ok(ContextPredicate::Eq(name, val))
                        }
                        _ => Err(PredicateParseError::UnexpectedEof),
                    }
                } else if name == "true" {
                    Ok(ContextPredicate::True)
                } else if name == "false" {
                    Ok(ContextPredicate::False)
                } else {
                    Ok(ContextPredicate::Flag(name))
                }
            }
            Some(tok) => Err(PredicateParseError::UnexpectedToken(format!("{:?}", tok))),
            None => Err(PredicateParseError::UnexpectedEof),
        }
    }
}

impl ContextPredicate {
    pub fn parse(expr: &str) -> Result<Self, PredicateParseError> {
        let trimmed = expr.trim();
        if trimmed.is_empty() {
            return Ok(ContextPredicate::True);
        }
        let tokens = tokenize(trimmed)?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_expr()?;
        if parser.pos < parser.tokens.len() {
            return Err(PredicateParseError::UnexpectedToken(format!(
                "{:?}",
                parser.tokens[parser.pos]
            )));
        }
        Ok(ast)
    }
}
