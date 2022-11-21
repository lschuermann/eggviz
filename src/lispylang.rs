use std::collections::{hash_map::Entry, HashMap};
use std::iter::Peekable;
use std::str::Chars;

use crate::EggvizLanguage;
use crate::EggvizProgram;
use crate::EggvizProgramParseError;
use crate::EggvizRewriteRule;
use crate::EggvizRewriteRuleLabel;

const GENERIC_IDENTIFIER: &str = "p";

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct Language {
    function: String,
    children: Vec<egg::Id>,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl egg::Language for Language {
    fn matches(&self, other: &Self) -> bool {
        let function_eq = self.function.eq(&other.function);
        if function_eq {
            assert!(self.children.len() == other.children().len());
        }
        function_eq
    }

    fn children(&self) -> &[egg::Id] {
        self.children.as_slice()
    }

    fn children_mut(&mut self) -> &mut [egg::Id] {
        self.children.as_mut_slice()
    }
}

impl egg::FromOp for Language {
    type Error = EggvizProgramParseError;

    fn from_op(op: &str, children: Vec<egg::Id>) -> Result<Self, Self::Error> {
        // This is infallible because there are no invalid symbols in our language,
        // and generic variables should have caused an error before we get here.
        Ok(Language {
            function: op.to_string(),
            children,
        })
    }
}

impl EggvizLanguage for Language {
    fn get_function_name(&self) -> &str {
        &self.function
    }
}

pub struct Program {
    root: Term,
}

impl EggvizProgram for Program {
    type Language = Language;
    type RewriteRule = RewriteRule;
    type ParseState = ArityChecker;

    fn parse_str(input: &str) -> Result<(Self, ArityChecker), EggvizProgramParseError> {
        let mut arity_checker = ArityChecker::new();
        let root = Parser::parse(input, true, &mut arity_checker)?;

        Ok((Program { root }, arity_checker))
    }

    fn parse_rewrite_rule(
        &self,
        arity_checker: &mut ArityChecker,
        _label: &EggvizRewriteRuleLabel,
        left: &str,
        right: &str,
    ) -> Result<Self::RewriteRule, EggvizProgramParseError> {
        // TODO: use label!
        RewriteRule::from_str(left, right, arity_checker)
    }

    fn to_egg(&self) -> String {
        self.root.to_egg()
    }
}

pub struct RewriteRule {
    left: Term,
    right: Term,
}

impl RewriteRule {
    fn from_str(
        l: &str,
        r: &str,
        arity_checker: &mut ArityChecker,
    ) -> Result<RewriteRule, EggvizProgramParseError> {
        Ok(RewriteRule {
            left: Parser::parse(l, false, arity_checker)?,
            right: Parser::parse(r, false, arity_checker)?,
        })
    }
}

impl EggvizRewriteRule for RewriteRule {
    fn left_to_egg(&self) -> String {
        self.left.to_egg()
    }

    fn right_to_egg(&self) -> String {
        self.right.to_egg()
    }
}

pub struct Function {
    name: String,
    _arity: usize,
}

impl Function {
    pub fn new(name: String, arity: usize) -> Function {
        Function {
            name,
            _arity: arity,
        }
    }
}

pub enum Term {
    Singleton(Variable),
    Invocation(Function, Vec<Term>),
}

impl Term {
    pub fn to_egg(&self) -> String {
        match self {
            Term::Singleton(var) => var.to_egg(),
            Term::Invocation(f, args) => {
                let mut s = String::from("(");
                s.push_str(&f.name);
                args.iter().for_each(|a| {
                    s.push_str(" ");
                    s.push_str(&a.to_egg())
                });
                s.push(')');
                s
            }
        }
    }
}

pub enum Variable {
    Concrete(String),
    Generic(String),
}

impl Variable {
    pub fn to_egg(&self) -> String {
        match self {
            Variable::Concrete(s) => s.to_string(),
            Variable::Generic(s) => format!("?{}", s),
        }
    }
}

#[derive(Clone)]
pub struct ArityChecker {
    map: HashMap<String, usize>,
}

impl ArityChecker {
    pub fn new() -> ArityChecker {
        ArityChecker {
            map: HashMap::new(),
        }
    }

    pub fn check_new_arity(
        &mut self,
        name: &str,
        new_arity: usize,
    ) -> Result<(), EggvizProgramParseError> {
        match self.map.entry(name.to_string()) {
            Entry::Occupied(e) => {
                let existing_arity = *e.get();
                if existing_arity != new_arity {
                    Err(EggvizProgramParseError::context_less_owned(
                            format!("Cannot instantiate function with symbol '{}' with an arity of {}, because it was already declared with an arity of {}.", name, new_arity, existing_arity)))
                } else {
                    Ok(())
                }
            }
            Entry::Vacant(v) => {
                v.insert(new_arity);
                Ok(())
            }
        }
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
    ) -> Result<Term, EggvizProgramParseError> {
        let mut tok = Parser {
            input: input.chars().peekable(),
        };
        match Self::parse_term(&mut tok, disallow_generics, arity_checker, false)? {
            None => Err(EggvizProgramParseError::context_less("Empty expression.")),
            Some(t) => {
                if let Token::None = tok.consume() {
                    Ok(t)
                } else {
                    Err(EggvizProgramParseError::context_less(
                        "Unexpected token at end of expression.",
                    ))
                }
            }
        }
    }

    fn parse_term(
        tok: &mut Parser,
        disallow_generics: bool,
        arity_checker: &mut ArityChecker,
        internal: bool,
    ) -> Result<Option<Term>, EggvizProgramParseError> {
        match tok.consume() {
            Token::LParen => {
                let (f, args) = Self::parse_function(tok, disallow_generics, arity_checker)?;
                Ok(Some(Term::Invocation(f, args)))
            }
            Token::GenericVariable(v) => {
                if disallow_generics {
                    Err(EggvizProgramParseError::context_less_owned(
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
                    Err(EggvizProgramParseError::context_less(
                        "Expected function term or variable name. Got ')'.",
                    ))
                }
            }
            Token::None => {
                if internal {
                    Err(EggvizProgramParseError::context_less(
                        "Unmatched '(' token.",
                    ))
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
    ) -> Result<(Function, Vec<Term>), EggvizProgramParseError> {
        let name = match tok.consume() {
            Token::LParen => Err(EggvizProgramParseError::context_less(
                "Cannot have two '(' tokens in a row.",
            )),
            Token::GenericVariable(name) => {
                // It's ok for functions to have generic names. They're functions.
                // The distinction only matters for normal variables.
                Ok(name)
            }
            Token::ConcreteVariable(name) => Ok(name),
            Token::RParen => Err(EggvizProgramParseError::context_less(
                "Empty function body.",
            )),
            Token::None => Err(EggvizProgramParseError::context_less(
                "Unexpected end of input.",
            )),
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
