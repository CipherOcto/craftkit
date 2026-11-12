//! Evaluators: contributions that say what a document means.
//!
//! # Two kinds of evaluator
//!
//! There are two ways for a program to say what a document means, and both
//! are **contributions**:
//!
//! - a **node kind** evaluates **its own nodes**: it knows what a
//!   rectangle is, and the core does not;
//! - an **evaluator** evaluates the **whole document**: it may look at
//!   every node, count them, find the ones that overlap, and say something
//!   about the whole.
//!
//! The second is the seam this file is about, because it is the one the
//! first cannot reach: a kind can only speak about its own nodes, and an
//! evaluator can speak about the document.
//!
//! # Why it is a contribution
//!
//! An evaluator is a value, so it is in the **registry** with everything
//! else: it has an id, it has a layer, and the ecosystem checks that the
//! layer is there. A plugin that adds an evaluator adds it to the same
//! list, and the plan grows without the core changing.
//!
//! # The plan is the sum
//!
//! A plan is what the node kinds say **and** what the evaluators say, in
//! one value. The core never merges them into something else: it asks, and
//! it holds the answers.

use crate::contribution::Contribution;
use crate::document::Document;
use crate::plan::Plan;

/// An evaluator: something that says what a whole document means.
#[derive(Clone, Copy)]
pub struct EvaluatorSpec {
    /// The evaluator's id: `<area>.<thing>`, dotted and lower case.
    pub id: &'static str,
    /// What it is called.
    pub label: &'static str,
    /// The layer that owns it.
    pub layer: &'static str,
    /// One line about what it does.
    pub doc: &'static str,
    /// The evaluation itself.
    ///
    /// It takes the document and gives back a plan. The items it returns
    /// are **its own**: it may use node ids that are not in the document,
    /// because an evaluator may be about something that is not a node at
    /// all.
    pub evaluate: fn(&Document) -> Plan,
}

impl std::fmt::Debug for EvaluatorSpec {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("EvaluatorSpec").field("id", &self.id).finish()
    }
}

impl Contribution for EvaluatorSpec {
    fn id(&self) -> &'static str {
        self.id
    }

    fn capability(&self) -> &'static str {
        "evaluator"
    }

    fn layer(&self) -> &'static str {
        self.layer
    }
}

// The link collects evaluators here.
inventory::collect!(crate::contribution::Registration<EvaluatorSpec>);

/// The evaluator that counts the document's nodes.
///
/// It is the smallest thing an evaluator can be, and it is here so that the
/// seam is **used**: a registry of evaluators that nothing puts anything in
/// is a list nobody can learn from.
pub static NODE_COUNT: EvaluatorSpec = EvaluatorSpec {
    id: "core.nodecount",
    label: "Node count",
    layer: "core",
    doc: "Count the document's nodes.",
    evaluate: |document| {
        let mut plan = Plan::new();
        plan.items.push(crate::plan::PlanItem {
            node: 0,
            kind: "core.count".to_string(),
            value: serde_json::json!({ "nodes": document.nodes.len() }),
        });
        plan
    },
};

inventory::submit! {
    crate::contribution::Registration { item: &NODE_COUNT }
}
