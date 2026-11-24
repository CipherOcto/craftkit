//! The **evaluators** of the fx battery: what the whole document means.
//!
//! An evaluator is a gesture about the **whole document**: it may look at
//! every node and say something about all of them. It is a rule the kit
//! publishes as much as a command or a kind is, and an evaluator that is
//! only proved in another battery's tests is a rule that does not travel
//! with the thing it is about.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_fx::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// A comp with **one layer** in it, and the layer's id.
fn a_layer(session: &mut Session) -> u64 {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    outcome.data["id"].as_u64().expect("the layer has an id")
}

/// The **properties** evaluator is there, and it is the fx layer's.
#[test]
fn the_properties_evaluator_is_there() {
    let ecosystem = ecosystem();
    let evaluator = ecosystem.evaluators.get("fx.props").expect("the evaluator is there");
    assert_eq!(evaluator.label, "Properties", "the evaluator is called Properties");
    assert_eq!(evaluator.layer, "fx", "and the fx layer owns it");
}

/// The evaluator counts the **properties** of every layer.
#[test]
fn the_properties_evaluator_counts_the_properties_of_a_layer() {
    let mut session = session();
    let layer = a_layer(&mut session);
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "position", "value": { "keys": [] } }))
        .expect("the property is set");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.kind == "fx.props").expect("the plan has the properties");
    assert_eq!(item.node, layer, "the item is about the layer");
    assert_eq!(item.value["properties"], json!(1), "the layer has one property: {item:?}");
}
