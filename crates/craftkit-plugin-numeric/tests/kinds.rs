//! The **kinds** of the numeric battery: what an angle and a box mean.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_numeric::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the numeric battery.
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

/// An **angle** is a kind, and it means its radians, its degrees and
/// the turn it makes.
#[test]
fn an_angle_means_its_radians_its_degrees_and_its_turn() {
    let mut session = session();
    let outcome =
        session.execute("numeric.angle.add", &json!({ "value": "90deg" })).expect("the angle is added");
    let angle = outcome.data["id"].as_u64().expect("the angle has an id");
    let node = session.document().node(angle).expect("the angle is there");
    assert_eq!(node.kind, "numeric.angle", "the node is an angle");
    let said = said_of(&session, angle);
    let radians = said["radians"].as_f64().expect("the angle says its radians");
    assert!((radians - std::f64::consts::FRAC_PI_2).abs() < 1e-9, "a quarter turn is a half pi: {radians}");
    let degrees = said["degrees"].as_f64().expect("the angle says its degrees");
    assert!((degrees - 90.0).abs() < 1e-9, "and it is ninety degrees: {degrees}");
}

/// A **box** is a kind, and it means its two lengths and the area they
/// make.
#[test]
fn a_box_means_its_two_lengths_and_the_area_they_make() {
    let mut session = session();
    let outcome = session
        .execute("numeric.box.add", &json!({ "width": "20mm", "height": "10mm" }))
        .expect("the box is added");
    let box_id = outcome.data["id"].as_u64().expect("the box has an id");
    let node = session.document().node(box_id).expect("the box is there");
    assert_eq!(node.kind, "numeric.box", "the node is a box");
    let said = said_of(&session, box_id);
    assert_eq!(said["width_mm"], json!(20.0), "the box says how wide it is");
    assert_eq!(said["height_mm"], json!(10.0), "and how high");
    assert_eq!(said["area_mm2"], json!(200.0), "and the area they make");
}
