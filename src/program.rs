use crate::parser::{ParseError, Parser};
use std::collections::hash_map::Entry;
use std::collections::HashMap;

pub struct Program(Term);

impl Program {
    pub fn from_str(input: &str, arity_checker: &mut ArityChecker) -> Result<Program, ParseError> {
        Ok(Program(Parser::parse(input, true, arity_checker)?))
    }

    pub fn to_egg(&self) -> String {
        self.0.to_egg()
    }
}

pub struct RewriteRule {
    left: Term,
    right: Term,
}

impl RewriteRule {
    pub fn from_str(
        l: &str,
        r: &str,
        arity_checker: &mut ArityChecker,
    ) -> Result<RewriteRule, ParseError> {
        Ok(RewriteRule {
            left: Parser::parse(l, false, arity_checker)?,
            right: Parser::parse(r, false, arity_checker)?,
        })
    }

    pub fn left_to_egg(&self) -> String {
        self.left.to_egg()
    }

    pub fn right_to_egg(&self) -> String {
        self.right.to_egg()
    }
}

pub struct Function {
    name: String,
    arity: usize,
}

impl Function {
    pub fn new(name: String, arity: usize) -> Function {
        Function { name, arity }
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

pub struct ArityChecker {
    map: HashMap<String, usize>,
}

impl ArityChecker {
    pub fn new() -> ArityChecker {
        ArityChecker {
            map: HashMap::new(),
        }
    }

    pub fn check_new_arity(&mut self, name: &str, new_arity: usize) -> Result<(), ParseError> {
        match self.map.entry(name.to_string()) {
            Entry::Occupied(e) => {
                let existing_arity = *e.get();
                if existing_arity != new_arity {
                    Err(ParseError::new_owned(
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
