//! The **radius** of an item.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_layout::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the layout battery.
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

/// A **document** with a **spread** in it.
fn a_spread(session: &mut Session) -> u64 {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    outcome.data["id"].as_u64().expect("the spread has an id")
}

/// An **item** on a spread.
fn an_item(session: &mut Session, spread: u64) -> u64 {
    let outcome = session
        .execute(
            "layout.item.add",
            &json!({
                "spread": spread,
                "name": "the shape",
                "kind": "layout.item.graphic",
                "shape": "rectangle",
                "rect": { "x": 0.0, "y": 0.0, "width": 100.0, "height": 50.0 },
            }),
        )
        .expect("the item is added");
    outcome.data["id"].as_u64().expect("the item has an id")
}

/// The **facts** of an item, as the plan reports them.
fn facts_of(session: &mut Session, item: u64) -> Json {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == item).expect("the item is in the plan");
    entry.value.clone()
}

/// **An item may be rounded.**
#[test]
fn an_item_may_be_rounded() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.radius", &json!({ "item": item, "radius": 5.0 }))
        .expect("the item is rounded");
    let facts = facts_of(&mut session, item);
    assert_eq!(facts["radius"], json!(5.0), "the radius is there");
}

/// The **corners** of an item are derived.
#[test]
fn the_corners_of_an_item_are_derived() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.radius", &json!({ "item": item, "radius": 5.0 }))
        .expect("the item is rounded");
    let facts = facts_of(&mut session, item);
    let corners = facts["corners"].as_array().expect("the corners are a list");
    assert_eq!(corners.len(), 4, "there are four corners");
    assert_eq!(corners[0]["x"], json!(5.0), "the first is inset by the radius");
    assert_eq!(corners[0]["y"], json!(5.0), "and by the radius too");
    assert_eq!(corners[2]["x"], json!(95.0), "and the third is inset at the other end");
}

/// An item with **no radius** has no corners.
#[test]
fn an_item_with_no_radius_has_no_corners() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let facts = facts_of(&mut session, item);
    assert!(facts["radius"].is_null(), "there is no radius");
    assert!(facts["corners"].is_null(), "and so there are no corners");
}

/// A radius that is **negative** is refused.
#[test]
fn a_radius_that_is_negative_is_refused() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let refused = session
        .execute("layout.item.radius", &json!({ "item": item, "radius": -1.0 }))
        .expect_err("the radius is refused");
    assert!(refused.to_string().contains("negative"), "the refusal says why");
}

/// A radius beyond **half the shorter side** is refused.
#[test]
fn a_radius_beyond_half_the_shorter_side_is_refused() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let refused = session
        .execute("layout.item.radius", &json!({ "item": item, "radius": 30.0 }))
        .expect_err("the radius is refused");
    assert!(refused.to_string().contains("half"), "the refusal says why");
}

/// An item that is **not a rectangle** may not have a radius.
#[test]
fn an_item_that_is_not_a_rectangle_may_not_have_a_radius() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let outcome = session
        .execute(
            "layout.item.add",
            &json!({
                "spread": spread,
                "name": "the oval",
                "kind": "layout.item.graphic",
                "shape": "ellipse",
                "rect": { "x": 0.0, "y": 0.0, "width": 100.0, "height": 50.0 },
            }),
        )
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let refused = session
        .execute("layout.item.radius", &json!({ "item": item, "radius": 5.0 }))
        .expect_err("the radius is refused");
    assert!(refused.to_string().contains("rectangle"), "the refusal says why");
}
