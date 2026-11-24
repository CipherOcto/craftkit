//! The **evaluators** of the geometry battery: what the whole document
//! means.
//!
//! An evaluator is a gesture about the **whole document**: it may look at
//! every node and say something about all of them. It is a rule the kit
//! publishes as much as a command or a kind is, and an evaluator that is
//! only proved in another battery's tests is a rule that does not travel
//! with the thing it is about.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands linked
/// into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_shapes::CRAFTKIT_PLUGIN_ANCHOR;

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

/// The **total area** evaluator is there, and it is the geometry layer's.
#[test]
fn the_total_area_evaluator_is_there() {
    let ecosystem = ecosystem();
    let evaluator = ecosystem.evaluators.get("geometry.totalarea").expect("the evaluator is there");
    assert_eq!(evaluator.label, "Total area", "the evaluator is called Total area");
    assert_eq!(evaluator.layer, "geometry", "and the geometry layer owns it");
}

/// The evaluator adds up the area of **every** rectangle.
#[test]
fn the_total_area_evaluator_adds_up_every_rectangle() {
    let mut session = session();
    session
        .execute("geometry.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
        .execute("geometry.rectangle", &json!({ "x": 20.0, "y": 0.0, "width": 5.0, "height": 5.0 }))
        .expect("the other rectangle is added");
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "geometry.totalarea").expect("the plan has the area");
    let area = item.value["area"].as_f64().expect("the area is a number");
    assert!((area - 125.0).abs() < 1e-9, "a hundred and twenty five: {area}");
}
