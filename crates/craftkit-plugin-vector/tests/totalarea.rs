//! The **total area** of every path in the document.
//!
//! The area is derived: the document holds the paths, and the total is
//! what they add up to. Nothing stores it, and nothing has to keep it
//! true.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_vector::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the vector battery.
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

/// The **total area** the plan reports.
fn total_of(session: &Session) -> f64 {
    let plan = session.evaluate();
    let item =
        plan.items.iter().find(|item| item.kind == "vector.totalarea").expect("the plan has the total area");
    item.value["area"].as_f64().expect("the area is a number")
}

/// A rectangle of ten by ten is a hundred square units.
#[test]
fn the_area_of_a_rectangle_is_its_area() {
    let mut session = session();
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    // The area is a float, and a float is a float: the answer is close
    // to a hundred and not exactly a hundred.
    assert!((total_of(&session) - 100.0).abs() < 1e-9, "ten by ten is a hundred: {}", total_of(&session));
}

/// A document with no path has no area.
#[test]
fn a_document_with_no_path_has_no_area() {
    let session = session();
    assert_eq!(total_of(&session), 0.0, "there is nothing to add up");
}

/// And the areas of **every** path are added up.
#[test]
fn the_areas_of_every_path_are_added_up() {
    let mut session = session();
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
        .execute("vector.rectangle", &json!({ "x": 20.0, "y": 0.0, "width": 5.0, "height": 5.0 }))
        .expect("the other rectangle is added");
    assert!((total_of(&session) - 125.0).abs() < 1e-9, "a hundred and twenty five: {}", total_of(&session));
}
