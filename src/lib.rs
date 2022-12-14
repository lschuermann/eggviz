use std::cell::RefCell;
use std::collections::{hash_map::DefaultHasher, HashMap, HashSet, LinkedList};
use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;
use std::rc::Rc;
use std::str::FromStr;

use wasm_bindgen::prelude::*;
use web_sys::console;

mod lispylang;

/// Label identifying each rewrite rule defined over a program.
///
/// The purpose of this type is to support both explicitly named, and implicitly
/// numbered (indexed) rewrite rules in a single namespace.
///
/// Each [`EggvizRewriteRuleLabel`] has a canonical string representation
/// obtainable through its
/// [`ToString`](#impl-ToString-for-EggvizRewriteRuleLabel) implementation. An
/// equivalent [`EggvizRewriteRuleLabel`] can be reconstructed through the
/// [`FromStr`](#impl-FromStr-for-EggvizRewriteRuleLabel) implementation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EggvizRewriteRuleLabel {
    Supplied(String),
    Indexed(usize),
}

impl Display for EggvizRewriteRuleLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            EggvizRewriteRuleLabel::Supplied(label) => write!(f, "rwr:{}", label),
            EggvizRewriteRuleLabel::Indexed(idx) => write!(f, "rwr#{}", idx),
        }
    }
}

impl FromStr for EggvizRewriteRuleLabel {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some(label) = s.strip_prefix("rwr:") {
            Ok(EggvizRewriteRuleLabel::Supplied(label.to_string()))
        } else if let Some(idx_str) = s.strip_prefix("rwr#") {
            Ok(EggvizRewriteRuleLabel::Indexed(
                idx_str.parse::<usize>().map_err(|_| ())?,
            ))
        } else {
            Err(())
        }
    }
}

/// Parsing context (program or rewrite rule byte offset) for annotating error
/// messages.
#[derive(Clone, Debug)]
pub enum EggvizProgramParseContext {
    Program {
        /// Byte offset in the passed string.
        offset: usize,
    },
    RewriteRule {
        /// Rewrite rules are indexed as a pair of left & right rules. An even
        /// number refers to the left, an odd to the right rule.
        label: EggvizRewriteRuleLabel,

        /// Byte offset in the passed string.
        offset: usize,
    },
}

/// Program or rewrite rule parse error.
#[derive(Clone, Debug)]
pub struct EggvizProgramParseError {
    pub msg: String,
    pub context: Option<EggvizProgramParseContext>,
}

impl Display for EggvizProgramParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        self.msg.fmt(f)
    }
}

impl EggvizProgramParseError {
    pub fn context_less_owned(msg: String) -> Self {
        EggvizProgramParseError { msg, context: None }
    }

    pub fn context_less(msg: impl AsRef<str>) -> Self {
        Self::context_less_owned(msg.as_ref().to_string())
    }
}

/// Interface to a high-level representation of a rewrite rule related to a
/// program.
pub trait EggvizRewriteRule {
    /// Obtain a string-representation of the left-hand side of the rewrite
    /// rule, to be parsed into an [`egg::Pattern`].
    fn left_to_egg(&self) -> String;

    /// Obtain a string-representation of the right-hand side of the rewrite
    /// rule, to be parsed into an [`egg::Pattern`].
    fn right_to_egg(&self) -> String;
}

pub trait EggvizLanguage: egg::Language + egg::FromOp + Hash + Send + Sync + 'static {
    /// Get the function or constant name represented by this language node:
    fn get_function_name(&self) -> &str;
}

/// Pluggable eggviz program interface. Extends an [`egg::Language`] by
/// factories for parsing the program and associated rewrite rules.
pub trait EggvizProgram {
    /// [`egg::Language`] type associated with this type.
    ///
    /// The program is expected to generate an egg-compatible string
    /// representation of the program with [`EggvizProgram::to_egg`].
    /// [`EggvizProgram::Language`] will then be used to operate on the terms
    /// produced by this method and parsed by egg.
    type Language: EggvizLanguage;

    /// High-level rewrite rule representation for the [`EggvizProgram`].
    ///
    /// This type will be used to query string-representation `egg` terms for
    /// both the left- and right-hand side of a rewrite rule.
    type RewriteRule: EggvizRewriteRule;

    /// State maintained throughout parsing a program and its associated rewrite
    /// rules.
    ///
    /// In `eggviz`, require rules are not universally applicable for a
    /// language, but are rather specific to a program. Implementors of this
    /// trait may want to validate, for instance, that function arities do not
    /// conflict between the program and any rewrite rules. This associated type
    /// is output by the initial (factory) method to parse a program
    /// ([`EggvizProgram::parse_str`]) and will be passed back into each invocation of
    /// [`EggvizProgram::parse_rewrite_rule`].
    ///
    /// Implementors can use it to keep track of arbitrary state. The
    /// [`EggvizRuntime`] guarantees that a given [`Self::ParseState`] reference
    /// will be provided to all invocations of
    /// [`EggvizProgram::parse_rewrite_rule`]. However, [`EggvizRuntime`] may
    /// make abritrary clones of this type and, when re-parsing rewrite rules
    /// but not the full program, pass an earlier version of the state into
    /// subsequent calls to [`EggvizProgram::parse_rewrite_rule`].
    type ParseState: Clone;

    /// Try to parse an [`EggvizProgram`] from a string.
    ///
    /// The string should contain all information necessary to construct a
    /// program of the given [`egg::Language`]. This information is further
    /// accessible when constructing rewrite rules.
    fn parse_str(input: &str) -> Result<(Self, Self::ParseState), EggvizProgramParseError>
    where
        Self: Sized;

    /// Given a parsed [`EggvizProgram`], try to parse and interpret rewrite
    /// rules.
    ///
    /// This method is further responsible for ensuring that the rewrite rules
    /// are valid given the parsed program. This means that in eggviz, rewrite
    /// rules are not universally applicable for a language, but are rather
    /// specific to a program. For instance, the method implementation should
    /// ensure that a rewrite rule containing functions respects the arity (and
    /// potentially types) of these functions as declared in the program.
    fn parse_rewrite_rule(
        &self,
        parse_state: &mut Self::ParseState,
        label: &EggvizRewriteRuleLabel,
        left: &str,
        right: &str,
    ) -> Result<Self::RewriteRule, EggvizProgramParseError>;

    /// Dump the program as a recursive expression, to be parsed into a
    /// [`egg::RecExpr`].
    ///
    ///
    fn to_egg(&self) -> String;
}

pub struct EggvizSingleStepSchedulerState(Rc<RefCell<EggvizSingleStepSchedulerInnerState>>);

pub struct EggvizSingleStepSchedulerInnerState {
    target_iteration: usize,
    rewrite_target: Option<EggvizRewriteRuleLabel>,
    applied_rules: LinkedList<EggvizRewriteRuleLabel>,
}

impl EggvizSingleStepSchedulerState {
    pub fn rewrite<'a, L: egg::Language + 'a, IterData: egg::IterationData<L, ()>>(
        &self,
        runner: &mut egg::Runner<L, (), IterData>,
        rewrite_rules: impl IntoIterator<Item = &'a egg::Rewrite<L, ()>>,
        iters: NonZeroUsize,
        rule: Option<EggvizRewriteRuleLabel>,
    ) -> LinkedList<EggvizRewriteRuleLabel> {
        // Set the rule to apply in the state, and apply it only in the next

        // iteration (enforced for the first iteration by wrapping from
        // usize::MAX to 0):
        {
            let mut state = self.0.borrow_mut();
            state.rewrite_target = rule;
            let latest_iteration = runner.iterations.len().wrapping_sub(1);
            state.target_iteration = latest_iteration.wrapping_add(iters.get());
        }

        // Get ownership of the runner by swapping it out with a default value.
        // TODO: we should seek to optimize this probably?
        let owned_runner = std::mem::replace(runner, egg::Runner::new(()));

        // Set an instance based on our state as the runner's scheduler. We
        // don't have a way to retain the previous scheduler (currently), so
        // this is a destructive operation:
        let mut owned_runner = owned_runner.with_scheduler(EggvizSingleStepScheduler(
            EggvizSingleStepSchedulerState(Rc::clone(&self.0)),
        ));

        // Now, reset any previous stop reason, otherwise egg with panic:
        owned_runner.stop_reason = None;

        // Actually perform the rewrites.
        let mut owned_runner = owned_runner.run(rewrite_rules);

        // Put the runner back, we are done.
        std::mem::swap(&mut owned_runner, runner);

        {
            let mut state = self.0.borrow_mut();

            // Swap the list of applied rules out to return and clear it:
            let mut applied_rules = LinkedList::new();
            std::mem::swap(&mut applied_rules, &mut state.applied_rules);

            applied_rules
        }
    }

    pub fn rewrite_rule<'a, L: egg::Language + 'a, IterData: egg::IterationData<L, ()>>(
        &self,
        runner: &mut egg::Runner<L, (), IterData>,
        rewrite_rules: impl IntoIterator<Item = &'a egg::Rewrite<L, ()>>,
        rule: EggvizRewriteRuleLabel,
    ) -> bool {
        let applied_rules = self.rewrite(
            runner,
            rewrite_rules,
            NonZeroUsize::new(1).unwrap(),
            Some(rule.clone()),
        );

        let rule_count = applied_rules
            .into_iter()
            .map(|applied_rule| {
                assert!(applied_rule == rule);
            })
            .count();

        assert!(rule_count < 2);
        rule_count > 0
    }
}

/// Single-step scheduler for egg, to be used in tandem with a runtime
/// supporting this scheduler.
///
/// This scheduler is called as part of an inner loop hard-coded into `egg`. It
/// is designed to coordinate with some other module having control about the
/// invocation of this inner loop (specifically, having control over the
/// [`egg::Runner`]). By sharing a common state object
/// ([`EggvizSingleStepSchedulerState`]), the single-step scheduler can cause
/// egg to only apply a single rewrite rule per invocation of
/// [`egg::Runner::run`].
pub struct EggvizSingleStepScheduler(EggvizSingleStepSchedulerState);

impl EggvizSingleStepScheduler {
    pub fn initial_state() -> EggvizSingleStepSchedulerState {
        EggvizSingleStepSchedulerState(Rc::new(RefCell::new(EggvizSingleStepSchedulerInnerState {
            target_iteration: 0,
            rewrite_target: None,
            applied_rules: LinkedList::new(),
        })))
    }
}

impl<L: egg::Language, N: egg::Analysis<L>> egg::RewriteScheduler<L, N>
    for EggvizSingleStepScheduler
{
    fn search_rewrite<'a>(
        &mut self,
        iteration: usize,
        egraph: &egg::EGraph<L, N>,
        rewrite: &'a egg::Rewrite<L, N>,
    ) -> Vec<egg::SearchMatches<'a, L>> {
        // Get a mutable reference to the current state. The state must not be
        // borrowed anywhere while egg is running!
        let state = self.0 .0.borrow();

        if iteration > state.target_iteration {
            Vec::new()
        } else if state
            .rewrite_target
            .as_ref()
            .map(|target| {
                EggvizRewriteRuleLabel::from_str(rewrite.name.as_str()).unwrap() == *target
            })
            // If we don't have a target rewrite rule to search for,
            // apply all rules unconditionally:
            .unwrap_or(true)
        {
            rewrite.search(egraph)
        } else {
            Vec::new()
        }
    }

    fn apply_rewrite(
        &mut self,
        iteration: usize,
        egraph: &mut egg::EGraph<L, N>,
        rewrite: &egg::Rewrite<L, N>,
        matches: Vec<egg::SearchMatches<L>>,
    ) -> usize {
        // Convert the passed egg rewrite rule into an
        // `EggvizRewriteRuleLabel` for comparing against target rules
        // and tracking applied rules.
        let rewrite_label = EggvizRewriteRuleLabel::from_str(rewrite.name.as_str()).unwrap();

        // Get a mutable reference to the current state. The state must not be
        // borrowed anywhere while egg is running!
        let mut state = self.0 .0.borrow_mut();

        if iteration > state.target_iteration {
            0
        } else if matches.len() > 0
            && state
                .rewrite_target
                .as_ref()
                .map(|target| rewrite_label == *target)
                // If we don't have a target rewrite rule to search for,
                // apply all rules unconditionally:
                .unwrap_or(true)
        {
            let (prev_nodes, prev_classes) = (
                egraph.total_number_of_nodes(),
                egraph.classes().filter(|eclass| !eclass.is_empty()).count(),
            );

            // Apply the rewrite rules to the graph:
            rewrite.apply(egraph, &matches);

            let (new_nodes, new_classes) = (
                egraph.total_number_of_nodes(),
                egraph.classes().filter(|eclass| !eclass.is_empty()).count(),
            );

            // Add the rule (and how many nodes it affected) into the
            // list of applied rules:
            if prev_nodes != new_nodes || prev_classes != new_classes {
                state.applied_rules.push_back(rewrite_label);
            }

            matches.len()
        } else {
            0
        }
    }
}

#[derive(Clone, Debug)]
pub enum EggvizRuntimeError {
    ParseError(EggvizProgramParseError),
    DuplicateRewriteRuleLabel(EggvizRewriteRuleLabel),
    InternalError(String),
}

impl Display for EggvizRuntimeError {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            EggvizRuntimeError::ParseError(e) => format!("Parsing Error: {}", e).fmt(f),
            EggvizRuntimeError::DuplicateRewriteRuleLabel(l) => {
                format!("Error: Rewrite rule `{}` was duplicated.", l).fmt(f)
            }
            EggvizRuntimeError::InternalError(e) => format!("Internal Error: {}", e).fmt(f),
        }
    }
}

pub struct EggvizRuntime<P: EggvizProgram> {
    rewrite_rules: Vec<egg::Rewrite<<P as EggvizProgram>::Language, ()>>,
    sched_state: EggvizSingleStepSchedulerState,
    runner: egg::Runner<P::Language, ()>,
}

impl<P: EggvizProgram> EggvizRuntime<P> {
    pub fn new(
        program_str: impl AsRef<str>,
        rewrite_rules_str: impl IntoIterator<
            Item = (Option<impl AsRef<str>>, impl AsRef<str>, impl AsRef<str>),
        >,
    ) -> Result<Self, EggvizRuntimeError> {
        // Try to construct a program from the given string, returning any parse
        // errors to the caller. Further ensure that the context doesn't lie
        // about the parsing error type.
        let (program, mut parse_state) =
            P::parse_str(program_str.as_ref()).map_err(|e| match e.context {
                Some(EggvizProgramParseContext::RewriteRule { .. }) => {
                    EggvizRuntimeError::InternalError(format!(
                        "Invalid parse error context for parsing program: {:?}",
                        e
                    ))
                }
                _ => EggvizRuntimeError::ParseError(e),
            })?;

        // With the program constructed, now try to parse the passed rewrite
        // rules in the program's context. This also performs a sanity check
        // that the assigned labels are unique. We require this for our
        // single-step scheduler:
        let mut rewrite_rule_labels: HashSet<EggvizRewriteRuleLabel> = HashSet::new();
        let rewrite_rules = rewrite_rules_str
            .into_iter()
            .enumerate()
            .map(|(idx, (opt_str_label, left, right))| {
                if let Some(str_label) = opt_str_label {
                    (
                        EggvizRewriteRuleLabel::Supplied(str_label.as_ref().to_string()),
                        left,
                        right,
                    )
                } else {
                    (EggvizRewriteRuleLabel::Indexed(idx), left, right)
                }
            })
            .map(|(rwr_label, left, right)| {
                if !rewrite_rule_labels.insert(rwr_label.clone()) {
                    Err(EggvizRuntimeError::DuplicateRewriteRuleLabel(rwr_label))
                } else {
                    Ok((rwr_label, left, right))
                }
            })
            .map(|res| {
                res.and_then(|(rwr_label, left, right)| {
                    let rewrite_rule = program
                        .parse_rewrite_rule(
                            &mut parse_state,
                            &rwr_label,
                            left.as_ref(),
                            right.as_ref(),
                        )
                        .map_err(|e| match e.context {
                            Some(EggvizProgramParseContext::RewriteRule { ref label, .. })
                                if *label != rwr_label =>
                            {
                                EggvizRuntimeError::InternalError(format!(
                                "Invalid parse error context for parsing rewrite rule {:?}: {:?}",
                                &rwr_label, e
                            ))
                            }
                            _ => EggvizRuntimeError::ParseError(e),
                        })?;

                    Ok((rwr_label, rewrite_rule))
                })
            })
            .map(|res: Result<_, EggvizRuntimeError>| {
                res.and_then(|(rwr_label, rewrite_rule)| {
                    egg::Rewrite::new(
                        rwr_label.to_string(),
                        egg::Pattern::from_str(&rewrite_rule.left_to_egg()).map_err(|e| {
                            EggvizRuntimeError::InternalError(format!(
                                "Egg reported an error while trying to parse the \
                                 generated left-hand rewrite rule expression for \
                                 rule {:?}: {:?}",
                                rwr_label, e
                            ))
                        })?,
                        egg::Pattern::from_str(&rewrite_rule.right_to_egg()).map_err(|e| {
                            EggvizRuntimeError::InternalError(format!(
                                "Egg reported an error while trying to parse the \
                                 generated right-hand rewrite rule expression for \
                                 rule {:?}: {:?}",
                                rwr_label, e
                            ))
                        })?,
                    )
                    .map_err(|e| {
                        EggvizRuntimeError::InternalError(format!(
                            "Egg reported an error while constructing a rewrite \
                             at {:?}: {:?}",
                            rwr_label, e,
                        ))
                    })
                })
            })
            .collect::<Result<Vec<egg::Rewrite<_, _>>, EggvizRuntimeError>>()?;

        // Now, convert the program into an egg expression, annotated with the
        // type of the language we're using:
        let expr: egg::RecExpr<P::Language> = program.to_egg().parse().map_err(|e| {
            EggvizRuntimeError::InternalError(format!(
                "Egg reported error while trying to parse the generated program expression: {:?}",
                e
            ))
        })?;

        // Finally, we create an instance of our single-step scheduler. This
        // scheduler allows our runtime to retain control over egg's behavior
        // while we kick off control to egg's Runner. We share state with this
        // scheduler to provide instructions and collect responses.
        let sched_state = EggvizSingleStepScheduler::initial_state();

        // Piece it all together in an instance of egg's Runner (the scheduler
        // is set later implicitly by `rewrite_rule()`:
        let runner = egg::Runner::default()
            .with_expr(&expr)
            // required, the default timeout is 5sec which would make an
            // instance of EggvizRuntime unusable after that
            .with_time_limit(std::time::Duration::MAX);

        Ok(EggvizRuntime {
            rewrite_rules,
            sched_state,
            runner,
        })
    }

    pub fn rewrite_rule(&mut self, rule: EggvizRewriteRuleLabel) -> bool {
        self.sched_state
            .rewrite_rule(&mut self.runner, self.rewrite_rules.iter(), rule)
    }

    pub fn rewrite_auto(&mut self) -> LinkedList<EggvizRewriteRuleLabel> {
        self.sched_state.rewrite(
            &mut self.runner,
            &self.rewrite_rules,
            NonZeroUsize::new(1).unwrap(),
            None,
        )
    }

    pub fn dump_graph(&self) -> String {
        // TODO: this should be changed to actually return a usable graph
        // representation. For now, just print the graph:
        format!("{:?}", self.runner.egraph.dump())
    }

    pub fn current_graph(&self) -> HashMap<String, HashMap<u64, (String, Vec<String>)>> {
        self.runner
            .egraph
            .classes()
            .map(|eclass| {
                (
                    eclass.id.to_string(),
                    eclass
                        .nodes
                        .iter()
                        .map(|enode| {
                            let mut hasher = DefaultHasher::new();
                            enode.hash(&mut hasher);
                            (
                                hasher.finish(),
                                (
                                    enode.get_function_name().to_string(),
                                    egg::Language::children(enode)
                                        .iter()
                                        .map(|id| id.to_string())
                                        .collect(),
                                ),
                            )
                        })
                        .collect(),
                )
            })
            .collect()
    }
}

// ----- Expose an Eggviz struct parametrized over the lispylang to JS -----
#[wasm_bindgen]
pub struct LispylangEggvizRuntime {
    inner: EggvizRuntime<lispylang::Program>,
}

#[wasm_bindgen]
impl LispylangEggvizRuntime {
    pub fn new(
        program_str: &str,
        rewrite_rules_js: Box<[js_sys::JsString]>,
    ) -> Result<LispylangEggvizRuntime, String> {
        Ok(LispylangEggvizRuntime {
            inner: EggvizRuntime::new(
                program_str,
                rewrite_rules_js
                    .iter()
                    .step_by(2)
                    .zip(rewrite_rules_js.iter().skip(1).step_by(2))
                    .map(|(left, right)| {
                        (
                            None::<&str>,
                            <js_sys::JsString as ToString>::to_string(left),
                            <js_sys::JsString as ToString>::to_string(right),
                        )
                    }),
            )
            .map_err(|e| format!("{}", e))?,
        })
    }

    pub fn rewrite_rule(&mut self, rule_label: &str) -> Result<bool, String> {
        let parsed_label = EggvizRewriteRuleLabel::from_str(rule_label)
            .map_err(|_| format!("Unable to parse rule label \"{}\"", rule_label))?;
        Ok(self.inner.rewrite_rule(parsed_label))
    }

    pub fn rewrite_auto(&mut self) -> Result<js_sys::Array, String> {
        Ok(self
            .inner
            .rewrite_auto()
            .into_iter()
            .map(|rewrite_rule| js_sys::JsString::from(rewrite_rule.to_string()))
            .collect())
    }

    pub fn dump_graph(&self) -> String {
        self.inner.dump_graph()
    }

    pub fn current_graph(&self) -> js_sys::Map {
        let graph = self.inner.current_graph();

        let eclasses_map = js_sys::Map::new();
        for (eclass_id, enodes) in graph.into_iter() {
            let enode_map = js_sys::Map::new();
            for (enode_hash, (enode_label, children)) in enodes.into_iter() {
                let enode_children: js_sys::Array =
                    children.into_iter().map(js_sys::JsString::from).collect();

                let enode = js_sys::Map::new();
                enode.set(
                    &js_sys::JsString::from("label"),
                    &js_sys::JsString::from(enode_label),
                );
                enode.set(&js_sys::JsString::from("children"), &enode_children);

                enode_map.set(&js_sys::Number::from(enode_hash as u32), &enode);
            }
            eclasses_map.set(&js_sys::JsString::from(eclass_id), &enode_map);
        }
        eclasses_map
    }
}

#[wasm_bindgen(start)]
pub fn startup() -> Result<(), JsValue> {
    // This provides better error messages.
    console_error_panic_hook::set_once();

    console::log_1(&JsValue::from_str("eggviz WASM module initialized!"));

    Ok(())
}
