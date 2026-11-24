//! The **kinds** of the kit itself: what a layer means.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem of the kit itself.
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

/// What the plan says about a node.
fn said_of(session: &Session, node: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == node).expect("the node is in the plan");
    item.value.clone()
}

/// A **layer** is a kind, and it means its name.
///
/// A layer draws nothing and is hit by nothing: a layer is not a
/// thing, it is where things are.
#[test]
fn a_layer_means_its_name() {
    let mut session = session();
    let outcome = session.execute("core.layer.add", &json!({ "name": "Walls" })).expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    let node = session.document().node(layer).expect("the layer is there");
    assert_eq!(node.kind, "core.layer", "the node is a layer");
    let said = said_of(&session, layer);
    assert_eq!(said["name"], json!("Walls"), "the layer says its name");
}
