//! The **kinds** of the cad battery: what a drawing means.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_cad::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the cad battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
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

/// A **drawing** is a kind, and it means the layers and the blocks it
/// holds.
#[test]
fn a_drawing_means_its_layers_and_its_blocks() {
    let mut session = session();
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the plan" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let node = session.document().node(drawing).expect("the drawing is there");
    assert_eq!(node.kind, "cad.drawing", "the node is a drawing");
    let said = said_of(&session, drawing);
    assert_eq!(said["layers"], json!(0), "the drawing says how many layers it holds");
    assert_eq!(said["blocks"], json!(0), "and how many blocks");
}
