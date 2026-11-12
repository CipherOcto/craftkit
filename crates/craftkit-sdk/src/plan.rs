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
use crate::{EngineError, Outcome, Session};

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

/// Something that went wrong while the plan was made.
///
/// A kind and an evaluator are **contributions**: they are code that
/// craftkit does not own, and a contribution that panics must not take
/// the program with it. So the panic is caught, the contribution is
/// skipped, and the failure is **in the plan**: the plan says what it
/// could not say, and why.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanFailure {
    /// Which contribution panicked: a kind's name, or an evaluator's.
    pub who: String,
    /// What it said when it panicked.
    pub message: String,
}

/// What a document means.
///
/// The items are in the order the nodes are in the document, so a plan is
/// stable: the same document and the same kinds give the same plan.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    /// The items.
    pub items: Vec<PlanItem>,
    /// The contributions that panicked, and what they said.
    ///
    /// A plan with a failure in it is a plan that is **short**: the
    /// contribution that panicked said nothing, and the plan says so
    /// rather than pretending the document has no meaning.
    pub failures: Vec<PlanFailure>,
}

impl Plan {
    /// An empty plan.
    pub fn new() -> Self {
        Plan { items: Vec::new(), failures: Vec::new() }
    }

    /// The plan, as it is written.
    pub fn to_json(&self) -> Json {
        serde_json::json!({
            "failures": self
                .failures
                .iter()
                .map(|failure| serde_json::json!({
                    "who": failure.who,
                    "message": failure.message,
                }))
                .collect::<Vec<_>>(),
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
    for node in document.walk() {
        let Some(kind) = kinds.get(&node.kind) else { continue };
        let Some(evaluate) = kind.evaluate else { continue };
        // The kind is a contribution: code this tree does not own. A
        // contribution that panics must not take the program with it, so
        // the panic is caught here and the plan says what went wrong.
        let guarded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| evaluate(node, document)));
        match guarded {
            Ok(Some(value)) => {
                plan.items.push(PlanItem { node: node.id, kind: node.kind.clone(), value });
            }
            Ok(None) => {}
            Err(panic) => {
                plan.failures
                    .push(PlanFailure { who: node.kind.clone(), message: crate::guard::message_of(&panic) });
            }
        }
    }
    plan
}

/// The plan of a document, as the kinds say it.
///
/// A kind that panics says nothing here either: the panic is caught and
/// the answer is `None`.
pub fn evaluate_node(node: &Node, document: &Document, kind: &crate::document::NodeKindSpec) -> Option<Json> {
    let evaluate = kind.evaluate?;
    let guarded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| evaluate(node, document)));
    guarded.ok().flatten()
}

/// What the document means, as a **command**.
///
/// The plan is what every front end wants to show, and until now a front
/// end had to reach for the evaluator to get it: the studio, the bridge
/// and the terminal door all called `session.evaluate()` themselves. That
/// is a second path, and a second path is where two answers drift apart.
///
/// So the plan is a **command**. It is a query — it changes nothing, so
/// it is not an undo step — and every door reaches it the same way, and
/// the answer is the same wherever it is asked for.
#[derive(craftkit_derive::Command)]
#[command(id = "doc.plan", area = "doc", icon = "list", menu("File"), no_journal)]
pub struct DocPlan {}

impl DocPlan {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let plan = session.evaluate();
        Ok(Outcome::new(format!("{} item(s)", plan.items.len())).with_data(plan.to_json()))
    }
}
