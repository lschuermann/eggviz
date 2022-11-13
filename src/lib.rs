mod language;
mod parser;
mod program;
mod scheduler;

use js_sys::{Array, JsString};
use wasm_bindgen::prelude::*;
use web_sys::console;

use crate::language::{CustomLanguage, Runtime};
use crate::parser::ParseError;
use crate::scheduler::IntermediateState;
use egg::*;
use wasm_bindgen::JsObject;

#[wasm_bindgen]
pub fn start_graph(program: &str, rewrite_rules: Box<[JsValue]>) -> Result<Runtime, String> {
    //console::log_1(&JsValue::from_str(&format!("rwrs.len = {}", rewrite_rules.len())));
    let mut iter = rewrite_rules.iter();
    if rewrite_rules.len() % 2 != 0 {
        panic!("Odd number of terms passed for rewrite rules. This is a bug.")
    }
    let cap = rewrite_rules.len() / 2;
    let mut rules = Vec::with_capacity(cap);
    for _ in 0..cap {
        let err = "Non-string passed in rewrite rules array. This is a bug.";
        let left = iter.next().unwrap().as_string().expect(err);
        let right = iter.next().unwrap().as_string().expect(err);
        rules.push((left, right))
    }
    Runtime::new(program, &rules).map_err(|e| e.to_string())
}

#[wasm_bindgen]
pub fn run_runtime(r: &mut Runtime) {
    r.run()
}

#[wasm_bindgen]
pub struct IntermediateWrapper(IntermediateState);

#[wasm_bindgen]
pub fn can_poll_change(r: &mut Runtime) -> bool {
    r.can_poll()
}

#[wasm_bindgen]
pub fn poll_change(r: &mut Runtime) -> Result<IntermediateWrapper, String> {
    match r.next_change() {
        None => Err(String::from("No more changes to read.")),
        Some(c) => Ok(IntermediateWrapper(c)),
    }
}

#[wasm_bindgen(start)]
pub fn startup() -> Result<(), JsValue> {
    // This provides better error messages.
    console_error_panic_hook::set_once();

    console::log_1(&JsValue::from_str("eggviz WASM module initialized!"));

    Ok(())
}
