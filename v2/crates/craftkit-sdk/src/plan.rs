//! The plan: what a document means, once it has been evaluated.
//!
//! # What a plan is
//!
//! A document is a list of nodes, and a node says what **kind** of thing it
//! is. What that kind *means* — how wide it is, how much it weighs, whether
//! it is inside another thing — is the kind's business, not the core's.
//!
//! So the core does not evaluate anything by itself. It asks each node's
//! kind, and what comes back is a **plan**: the document, said in the
//! language of the kinds that are in it.
//!
//! # Why it matters
//!
//! A tree that evaluates in the core has a `match` on the kind in the
//! core, and every plugin that adds a kind has to edit that match. Here the
//! match does not exist: a kind that is linked in brings its own
//! evaluation with it, and a document that holds a node of that kind is
//! evaluated by it.
//!
//! A plan is a **value**, not a drawing: what to do with it — to render it,
//! to measure it, to check it against a rule — is the front end's business.

use serde_json::Value as Json;

use crate::document::{Document, Node};

/// One thing the plan says about one node.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanItem {
    /// The node it is about.
    pub node: u64,
    /// The kind that said it.
    pub kind: String,
    /// What the kind said.
    pub value: Json,
}

/// What a document means.
///
/// The items are in the order the nodes are in the document, so a plan is
/// stable: the same document and the same kinds give the same plan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    /// The items.
    pub items: Vec<PlanItem>,
}

impl Plan {
    /// An empty plan.
    pub fn new() -> Self {
        Plan { items: Vec::new() }
    }

    /// The plan, as it is written.
    pub fn to_json(&self) -> Json {
        serde_json::json!({
            "items": self
                .items
                .iter()
                .map(|item| serde_json::json!({
                    "node": item.node,
                    "kind": item.kind,
                    "value": item.value,
                }))
                .collect::<Vec<_>>(),
        })
    }

    /// The item about a node, if the plan says anything about it.
    pub fn item(&self, node: u64) -> Option<&PlanItem> {
        self.items.iter().find(|item| item.node == node)
    }
}

/// The plan of a document, as the kinds say it.
///
/// This is the whole of the evaluator: it walks the document, and for each
/// node it asks the node's **kind** what the node means. A kind that has
/// nothing to say says nothing, and the node is simply not in the plan.
///
/// A node whose kind is not linked in is **not** in the plan either, and the
/// core does not know or care which kinds those are: it asked the registry
/// and the registry did not have an answer.
pub fn evaluate(
    document: &Document,
    kinds: &crate::contribution::Registry<crate::document::NodeKindSpec>,
) -> Plan {
    let mut plan = Plan::new();
    for node in document.nodes() {
        let Some(kind) = kinds.get(&node.kind) else { continue };
        let Some(evaluate) = kind.evaluate else { continue };
        let Some(value) = evaluate(node, document) else { continue };
        plan.items.push(PlanItem { node: node.id, kind: node.kind.clone(), value });
    }
    plan
}

/// The plan of a document, as the kinds say it.
pub fn evaluate_node(node: &Node, document: &Document, kind: &crate::document::NodeKindSpec) -> Option<Json> {
    let evaluate = kind.evaluate?;
    evaluate(node, document)
}
