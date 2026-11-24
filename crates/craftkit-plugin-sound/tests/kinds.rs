//! The **kinds** of the sound battery: what a ruler means.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the sound battery.
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

/// A **ruler** is a kind, and it means the scale it counts by and how
/// many numbers it carries.
#[test]
fn a_ruler_means_its_scale_and_its_numbers() {
    let mut session = session();
    let outcome =
        session.execute("sound.ruler.add", &json!({ "name": "the ruler" })).expect("the ruler is added");
    let ruler = outcome.data["id"].as_u64().expect("the ruler has an id");
    let node = session.document().node(ruler).expect("the ruler is there");
    assert_eq!(node.kind, "sound.ruler", "the node is a ruler");
    let said = said_of(&session, ruler);
    assert_eq!(said["scale"], json!("bars"), "the ruler says what it counts by");
    assert_eq!(said["numbers"], json!(0), "and how many numbers it carries");
}
