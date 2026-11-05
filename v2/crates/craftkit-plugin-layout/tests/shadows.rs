//! The **shadow** of an item.

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

/// **An item may have a shadow.**
#[test]
fn an_item_may_have_a_shadow() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute(
            "layout.item.shadow",
            &json!({ "item": item, "angle": 120.0, "distance": 7.0, "blur": 4.0, "colour": "#000000" }),
        )
        .expect("the item has a shadow");
    let facts = facts_of(&mut session, item);
    assert_eq!(facts["shadow"]["angle"], json!(120.0), "the angle is there");
    assert_eq!(facts["shadow"]["distance"], json!(7.0), "and the distance");
    assert_eq!(facts["shadow"]["blur"], json!(4.0), "and the blur");
}

/// The **offset** of a shadow is derived.
#[test]
fn the_offset_of_a_shadow_is_derived() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.shadow", &json!({ "item": item, "angle": 90.0, "distance": 10.0 }))
        .expect("the item has a shadow");
    let facts = facts_of(&mut session, item);
    let offset = facts["shadow"]["offset"].as_object().expect("the offset is there");
    assert!((offset["x"].as_f64().expect("a number") - 0.0).abs() < 1e-9, "a quarter turn is no room across");
    assert!((offset["y"].as_f64().expect("a number") + 10.0).abs() < 1e-9, "and ten up");
}

/// The **box** of a shadow is derived.
#[test]
fn the_box_of_a_shadow_is_derived() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.shadow", &json!({ "item": item, "angle": 0.0, "distance": 5.0, "blur": 2.0 }))
        .expect("the item has a shadow");
    let facts = facts_of(&mut session, item);
    let box_of = facts["shadow"]["box"].as_object().expect("the box is there");
    assert_eq!(box_of["x"], json!(3.0), "the box starts after the item by the distance, less the blur");
    assert_eq!(box_of["width"], json!(104.0), "and is wider by the blur at both ends");
}

/// An item with **no shadow** has no shadow facts.
#[test]
fn an_item_with_no_shadow_has_no_shadow_facts() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let facts = facts_of(&mut session, item);
    assert!(facts["shadow"].is_null(), "there is no shadow");
}

/// A **distance** that is negative is refused.
#[test]
fn a_distance_that_is_negative_is_refused() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let refused = session
        .execute("layout.item.shadow", &json!({ "item": item, "angle": 120.0, "distance": -5.0 }))
        .expect_err("the shadow is refused");
    assert!(refused.to_string().contains("negative"), "the refusal says why");
}

/// A **colour** that is not a colour is refused.
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let refused = session
        .execute(
            "layout.item.shadow",
            &json!({ "item": item, "angle": 120.0, "distance": 5.0, "colour": "black" }),
        )
        .expect_err("the shadow is refused");
    assert!(refused.to_string().contains("#rrggbb"), "the refusal says what a colour is");
}

/// An **opacity** outside one is refused.
#[test]
fn an_opacity_outside_one_is_refused() {
    let mut session = session();
    let spread = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let refused = session
        .execute(
            "layout.item.shadow",
            &json!({ "item": item, "angle": 120.0, "distance": 5.0, "opacity": 1.5 }),
        )
        .expect_err("the shadow is refused");
    assert!(refused.to_string().contains("opacity"), "the refusal says why");
}
