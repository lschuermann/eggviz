use crate::CustomLanguage;
use egg::{rewrite, Analysis, EGraph, Language, Rewrite, RewriteScheduler, SearchMatches, Subst};
use std::borrow::BorrowMut;
use std::cell::RefCell;
use std::collections::LinkedList;
use std::marker::PhantomData;
use std::ops::Index;
use std::rc::Rc;
use std::slice;
use std::slice::SliceIndex;
use std::str::FromStr;
use std::sync::mpsc::{channel, Receiver, Sender};
use wasm_bindgen::JsValue;
use web_sys::console;

pub type IntermediateState = (usize, usize, Vec<Subst>);

pub struct CollectingScheduler {
    buf: Rc<RefCell<LinkedList<IntermediateState>>>,
}

impl CollectingScheduler {
    pub fn new(buf: Rc<RefCell<LinkedList<IntermediateState>>>) -> CollectingScheduler {
        CollectingScheduler { buf }
    }
}

impl<L: Language, N: Analysis<L>> RewriteScheduler<L, N> for CollectingScheduler {
    fn apply_rewrite(
        &mut self,
        iteration: usize,
        egraph: &mut EGraph<L, N>,
        rewrite: &Rewrite<L, N>,
        matches: Vec<SearchMatches<L>>,
    ) -> usize {
        console::log_1(&JsValue::from_str("Applied a rewrite!"));
        matches.iter().for_each(|m| {
            // Wait for the user to advance
            let err = "Could not parse rewrite rule number.";
            self.buf.as_ref().borrow_mut().push_back((
                iteration,
                usize::from_str(rewrite.name.as_str().split("_").nth(1).expect(err)).expect(err),
                m.substs.clone(),
            ));
            rewrite.apply(egraph, slice::from_ref(&m));
        });
        matches.len()
    }
}
