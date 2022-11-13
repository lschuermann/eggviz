use crate::parser::ParseError;
use crate::program::{ArityChecker, Program, RewriteRule};
use crate::scheduler::{CollectingScheduler, IntermediateState};
use egg::{
    AstSize, EGraph, Extractor, FromOp, Id, Language, Pattern, RecExpr, Rewrite, Runner,
    SearchMatches,
};
use std::cell::RefCell;
use std::collections::LinkedList;
use std::rc::Rc;
use std::str::FromStr;
use std::sync::mpsc::Receiver;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use web_sys::console;

#[wasm_bindgen]
pub struct Runtime {
    runner: Runner<CustomLanguage, ()>,
    buf: Rc<RefCell<LinkedList<IntermediateState>>>,
}

impl Runtime {
    pub fn new(program: &str, rewrite_rules: &[(String, String)]) -> Result<Runtime, ParseError> {
        let mut arity_checker = ArityChecker::new();
        let program = Program::from_str(program, &mut arity_checker)?;
        let mut counter = 0usize;
        let mut parsed_rw_rules = Vec::with_capacity(rewrite_rules.len());
        console::log_1(&JsValue::from_str(&format!(
            "Number of rewrite rules: {}",
            rewrite_rules.len()
        )));
        for (left, right) in rewrite_rules {
            let rule = RewriteRule::from_str(left, right, &mut arity_checker)?;
            counter += 1;
            let l = match Pattern::from_str(&rule.left_to_egg()) {
                Err(e) => {
                    return Err(ParseError::new_owned(format!(
                        "Egg failed to parse rewrite rule (lhs) because of the following error: {}",
                        e
                    )));
                }
                Ok(l) => l,
            };
            let r = match Pattern::from_str(&rule.right_to_egg()) {
                Err(e) => {
                    return Err(ParseError::new_owned(format!(
                        "Egg failed to parse rewrite rule (rhs) because of the following error: {}",
                        e
                    )));
                }
                Ok(r) => r,
            };
            match Rewrite::new(format!("rule_{}", counter), l, r) {
                Ok(r) => {
                    parsed_rw_rules.push(r);
                }
                Err(e) => {
                    return Err(ParseError::new_owned(format!(
                        "Egg failed to parse rewrite rule because of the following error: {}",
                        e
                    )));
                }
            }
        }
        // Adapted from the egg docs

        // parse the expression, the type annotation tells it which Language to use
        let expr: RecExpr<CustomLanguage> = program.to_egg().parse().unwrap();

        // simplify the expression using a Runner, which creates an e-graph with
        // the given expression and runs the given rules over it

        let buf = Rc::new(RefCell::new(LinkedList::new()));
        let scheduler = CollectingScheduler::new(Rc::clone(&buf));
        let runner = Runner::default()
            .with_expr(&expr)
            .with_scheduler(scheduler)
            .run(&parsed_rw_rules);

        Ok(Runtime { runner, buf })
    }

    pub fn run(&mut self) {
        // the Runner knows which e-class the expression given with `with_expr` is in
        let root = self.runner.roots[0];
        // use an Extractor to pick the best element of the root eclass
        let extractor = Extractor::new(&self.runner.egraph, AstSize);
        let (best_cost, best) = extractor.find_best(root);
    }

    pub fn next_change(&mut self) -> Option<IntermediateState> {
        self.buf.as_ref().borrow_mut().pop_front()
    }

    pub fn can_poll(&self) -> bool {
        !self.buf.borrow().is_empty()
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct CustomLanguage {
    function: String,
    children: Vec<Id>,
}

impl Language for CustomLanguage {
    fn matches(&self, other: &Self) -> bool {
        self.function.eq(&other.function) && self.children.eq(other.children())
    }

    fn children(&self) -> &[Id] {
        self.children.as_slice()
    }

    fn children_mut(&mut self) -> &mut [Id] {
        self.children.as_mut_slice()
    }
}

impl FromOp for CustomLanguage {
    type Error = ParseError;

    fn from_op(op: &str, children: Vec<Id>) -> Result<Self, Self::Error> {
        // This is infallible because there are no invalid symbols in our language,
        // and generic variables should have caused an error before we get here.
        Ok(CustomLanguage {
            function: op.to_string(),
            children,
        })
    }
}
