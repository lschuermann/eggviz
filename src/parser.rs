use crate::program::{ArityChecker, Function, Term, Variable};
use std::fmt::{Display, Formatter, Pointer};
use std::iter::Peekable;
use std::str::Chars;

const GENERIC_IDENTIFIER: &str = "p";

#[derive(Debug)]
pub struct ParseError {
    msg: String,
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.msg.fmt(f)
    }
}

impl ParseError {
    pub fn new(msg: &str) -> ParseError {
        ParseError {
            msg: String::from(msg),
        }
    }

    pub fn new_owned(msg: String) -> ParseError {
        ParseError { msg }
    }
}

pub struct Parser<'a> {
    input: Peekable<Chars<'a>>,
}

impl Parser<'_> {
    pub fn parse(
        input: &str,
        disallow_generics: bool,
        arity_checker: &mut ArityChecker,
    ) -> Result<Term, ParseError> {
        let mut tok = Parser {
            input: input.chars().peekable(),
        };
        match Self::parse_term(&mut tok, disallow_generics, arity_checker, false)? {
            None => Err(ParseError::new("Empty expression.")),
            Some(t) => {
                if let Token::None = tok.consume() {
                    Ok(t)
                } else {
                    Err(ParseError::new("Unexpected token at end of expression."))
                }
            }
        }
    }

    fn parse_term(
        tok: &mut Parser,
        disallow_generics: bool,
        arity_checker: &mut ArityChecker,
        internal: bool,
    ) -> Result<Option<Term>, ParseError> {
        match tok.consume() {
            Token::LParen => {
                let (f, args) = Self::parse_function(tok, disallow_generics, arity_checker)?;
                Ok(Some(Term::Invocation(f, args)))
            }
            Token::GenericVariable(v) => {
                if disallow_generics {
                    Err(ParseError::new_owned(
                        format!("Unexpected generic variable in program. Variables beginning with '{}' are reserved for generic variables in rewrite rules.", GENERIC_IDENTIFIER.to_string())
                    ))
                } else {
                    Ok(Some(Term::Singleton(Variable::Generic(v))))
                }
            }
            Token::ConcreteVariable(v) => Ok(Some(Term::Singleton(Variable::Concrete(v)))),
            Token::RParen => {
                if internal {
                    Ok(None)
                } else {
                    Err(ParseError::new(
                        "Expected function term or variable name. Got ')'.",
                    ))
                }
            }
            Token::None => {
                if internal {
                    Err(ParseError::new("Unmatched '(' token."))
                } else {
                    Ok(None)
                }
            }
        }
    }

    fn parse_function(
        tok: &mut Parser,
        disallow_generics: bool,
        arity_checker: &mut ArityChecker,
    ) -> Result<(Function, Vec<Term>), ParseError> {
        let name = match tok.consume() {
            Token::LParen => Err(ParseError::new("Cannot have two '(' tokens in a row.")),
            Token::GenericVariable(name) => {
                // It's ok for functions to have generic names. They're functions.
                // The distinction only matters for normal variables.
                Ok(name)
            }
            Token::ConcreteVariable(name) => Ok(name),
            Token::RParen => Err(ParseError::new("Empty function body.")),
            Token::None => Err(ParseError::new("Unexpected end of input.")),
        }?;
        let mut arguments = Vec::new();
        loop {
            match Self::parse_term(tok, disallow_generics, arity_checker, true)? {
                None => {
                    let arity = arguments.len();
                    arity_checker.check_new_arity(&name, arity)?;
                    return Ok((Function::new(name, arity), arguments));
                }
                Some(arg) => {
                    arguments.push(arg);
                }
            }
        }
    }

    fn consume(&mut self) -> Token {
        let mut token = String::new();
        let make_variable_token = |tok: String| {
            if tok.starts_with(GENERIC_IDENTIFIER) {
                Token::GenericVariable(tok)
            } else {
                Token::ConcreteVariable(tok)
            }
        };
        loop {
            match self.input.peek() {
                None => {
                    return if token.is_empty() {
                        Token::None
                    } else {
                        make_variable_token(token)
                    }
                }
                Some(&c) => {
                    if c.is_whitespace() {
                        self.input.next();
                        if token.is_empty() {
                            continue;
                        } else {
                            return make_variable_token(token);
                        }
                    }
                    if c == '(' {
                        return if token.is_empty() {
                            self.input.next();
                            Token::LParen
                        } else {
                            make_variable_token(token)
                        };
                    } else if c == ')' {
                        return if token.is_empty() {
                            self.input.next();
                            Token::RParen
                        } else {
                            make_variable_token(token)
                        };
                    } else {
                        token.push(c);
                        self.input.next();
                    }
                }
            };
        }
    }
}

pub enum Token {
    LParen,
    GenericVariable(String),
    ConcreteVariable(String),
    RParen,
    None,
}
