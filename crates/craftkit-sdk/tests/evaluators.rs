//! The **evaluators** of the kit itself: what the whole document means.
//!
//! An evaluator is a gesture about the **whole document**. The kit has
//! one of its own — the node count — and it is a rule the kit publishes
//! as much as a command or a kind is, so a test in the kit's own crate is
//! what proves it.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem of the kit.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the kit's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// The **node count** evaluator is there, and it is the core layer's.
#[test]
fn the_node_count_evaluator_is_there() {
    let ecosystem = ecosystem();
    let evaluator = ecosystem.evaluators.get("core.nodecount").expect("the evaluator is there");
    assert_eq!(evaluator.label, "Node count", "the evaluator is called Node count");
    assert_eq!(evaluator.layer, "core", "and the core layer owns it");
}

/// It counts the **nodes** of the document.
#[test]
fn the_node_count_evaluator_counts_the_nodes() {
    let mut session = session();
    session.execute("core.layer.add", &json!({ "name": "the layer" })).expect("the layer is added");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "core.count").expect("the plan has the count");
    let nodes = item.value["nodes"].as_u64().expect("the count is a number");
    let held = session.document().nodes.len() as u64;
    assert_eq!(nodes, held, "the count is the number of nodes the document holds");
}
