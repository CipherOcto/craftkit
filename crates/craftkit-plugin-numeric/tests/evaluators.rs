//! The **evaluators** of the numeric battery: what the whole document
//! means.
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
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_numeric::CRAFTKIT_PLUGIN_ANCHOR;

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

/// The **measures** evaluator is there, and it is the numeric layer's.
#[test]
fn the_measures_evaluator_is_there() {
    let ecosystem = ecosystem();
    let evaluator = ecosystem.evaluators.get("numeric.measures").expect("the evaluator is there");
    assert_eq!(evaluator.label, "Measures", "the evaluator is called Measures");
    assert_eq!(evaluator.layer, "numeric", "and the numeric layer owns it");
}

/// The evaluator gives the **measures** of every numeric node: an angle
/// of ninety degrees is ninety degrees, and the radians it is made of.
#[test]
fn the_measures_evaluator_gives_the_measures_of_an_angle() {
    let mut session = session();
    let outcome =
        session.execute("numeric.angle.add", &json!({ "value": "90deg" })).expect("the angle is added");
    let angle = outcome.data["id"].as_u64().expect("the angle has an id");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "numeric.measures").expect("the plan has the measures");
    assert_eq!(item.node, angle, "the item is about the angle");
    assert_eq!(item.value["degrees"], json!(90.0), "ninety degrees: {item:?}");
}
