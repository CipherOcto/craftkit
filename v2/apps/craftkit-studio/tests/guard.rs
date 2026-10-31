//! The **guard**: a contribution that panics does not take the program
//! with it.
//!
//! A kind and an evaluator are code this tree does not own. The command
//! path is already guarded — a command that panics leaves the document
//! as it was — and this is the other path: the **evaluation**.

use craftkit_derive::{Evaluator, NodeKind};
use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{Document, Node};

/// A kind whose evaluation **panics** on purpose.
#[derive(NodeKind)]
#[node_kind(kind = "test.panic.kind", label = "Panic", layer = "core", evaluates)]
pub struct PanicKind;

impl PanicKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// Panics: this is the point of the kind.
    fn evaluate(_node: &Node, _document: &Document) -> Option<Json> {
        panic!("the kind panicked on purpose")
    }
}

/// An evaluator that **panics** on purpose.
#[derive(Evaluator)]
#[evaluator(id = "test.panic.evaluator", label = "Panic", layer = "core")]
pub struct PanicEvaluator;

impl PanicEvaluator {
    /// Panics: this is the point of the evaluator.
    fn evaluate(_document: &Document) -> craftkit_sdk::Plan {
        panic!("the evaluator panicked on purpose")
    }
}

/// The ecosystem, with the panicking contributions in it.
fn ecosystem() -> craftkit_sdk::Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A kind that panics says nothing, and the plan says **why**.
#[test]
fn a_kind_that_panics_is_skipped_and_the_plan_says_why() {
    let ecosystem = ecosystem();
    assert!(ecosystem.kinds.get("test.panic.kind").is_some(), "the kind is in the ecosystem");
    let mut session = ecosystem.session();
    session
        .execute(
            "node.add",
            &craftkit_sdk::serde_json::json!({ "name": "landmine", "kind": "test.panic.kind" }),
        )
        .expect("the node is added");
    let plan = session.evaluate();
    assert!(plan.item(1).is_none(), "the kind said nothing: {plan:?}");
    let failures: Vec<&craftkit_sdk::plan::PlanFailure> =
        plan.failures.iter().filter(|failure| failure.who == "test.panic.kind").collect();
    assert_eq!(failures.len(), 1, "and the plan says why: {plan:?}");
    assert!(failures[0].message.contains("panicked on purpose"), "{:?}", failures[0]);
}

/// And the program carries on: the session is still a session.
#[test]
fn the_program_carries_on_after_a_kind_panics() {
    let mut session = ecosystem().session();
    session
        .execute(
            "node.add",
            &craftkit_sdk::serde_json::json!({ "name": "landmine", "kind": "test.panic.kind" }),
        )
        .expect("the node is added");
    let _ = session.evaluate();
    // The session still works.
    session
        .execute(
            "node.add",
            &craftkit_sdk::serde_json::json!({ "name": "ordinary", "kind": "geometry.rect" }),
        )
        .expect("the node is added");
    let plan = session.evaluate();
    assert!(plan.item(2).is_some(), "the ordinary node is in the plan: {plan:?}");
}

/// An evaluator that panics is skipped too, and the plan says why.
#[test]
fn an_evaluator_that_panics_is_skipped_and_the_plan_says_why() {
    let ecosystem = ecosystem();
    assert!(ecosystem.evaluators.get("test.panic.evaluator").is_some(), "the evaluator is in the ecosystem");
    let session = ecosystem.session();
    let plan = session.evaluate();
    let failures: Vec<&craftkit_sdk::plan::PlanFailure> =
        plan.failures.iter().filter(|failure| failure.who == "test.panic.evaluator").collect();
    assert_eq!(failures.len(), 1, "the plan says why: {plan:?}");
    assert!(failures[0].message.contains("panicked on purpose"), "{:?}", failures[0]);
}

/// The failures are in the plan's JSON as well, so a front end can show
/// them.
#[test]
fn the_failures_are_in_the_plans_json() {
    let mut session = ecosystem().session();
    session
        .execute(
            "node.add",
            &craftkit_sdk::serde_json::json!({ "name": "landmine", "kind": "test.panic.kind" }),
        )
        .expect("the node is added");
    let plan = session.evaluate();
    let written = plan.to_json();
    let failures = written["failures"].as_array().expect("there are failures");
    let mine: Vec<&craftkit_sdk::serde_json::Value> = failures
        .iter()
        .filter(|failure| failure["who"] == craftkit_sdk::serde_json::json!("test.panic.kind"))
        .collect();
    assert_eq!(mine.len(), 1, "{written}");
    assert!(mine[0]["message"].as_str().unwrap_or_default().contains("panicked on purpose"), "{written}");
}
