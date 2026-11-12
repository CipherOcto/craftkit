//! The **turn** of an item.

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
fn a_spread(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    (document, spread)
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

/// An item that is turned **is** turned.
#[test]
fn an_item_may_be_turned() {
    let mut session = session();
    let (_document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.rotate", &json!({ "item": item, "degrees": 30.0 }))
        .expect("the item is turned");
    let facts = facts_of(&mut session, item);
    assert_eq!(facts["rotation"], json!(30.0), "the turn is there");
}

/// An item that is **not** turned is turned by no degrees.
#[test]
fn an_item_that_is_not_turned_is_turned_by_no_degrees() {
    let mut session = session();
    let (_document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let facts = facts_of(&mut session, item);
    assert_eq!(facts["rotation"], json!(0.0), "there is no turn");
}

/// The **radians** of a turn are derived.
#[test]
fn the_radians_of_a_turn_are_derived() {
    let mut session = session();
    let (_document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.rotate", &json!({ "item": item, "degrees": 180.0 }))
        .expect("the item is turned");
    let facts = facts_of(&mut session, item);
    let radians = facts["radians"].as_f64().expect("the radians are a number");
    assert!((radians - std::f64::consts::PI).abs() < 1e-12, "a half turn is pi radians");
}

/// The **turned box** of an item is derived.
#[test]
fn the_turned_box_of_an_item_is_derived() {
    let mut session = session();
    let (_document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.rotate", &json!({ "item": item, "degrees": 90.0 }))
        .expect("the item is turned");
    let facts = facts_of(&mut session, item);
    let width = facts["turned"]["width"].as_f64().expect("the box has a width");
    let height = facts["turned"]["height"].as_f64().expect("the box has a height");
    assert!((width - 50.0).abs() < 1e-9, "a quarter turn swaps the width and the height");
    assert!((height - 100.0).abs() < 1e-9, "a quarter turn swaps the width and the height");
}

/// The **place** of an item does not move when it is turned.
#[test]
fn the_place_of_an_item_does_not_move_when_it_is_turned() {
    let mut session = session();
    let (_document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let before = facts_of(&mut session, item);
    session
        .execute("layout.item.rotate", &json!({ "item": item, "degrees": 45.0 }))
        .expect("the item is turned");
    let after = facts_of(&mut session, item);
    assert_eq!(before["rect"], after["rect"], "the shape is where it was");
}

/// A turn of **ninety** makes a **square** bigger on the page.
#[test]
fn a_turn_of_ninety_makes_a_square_bigger() {
    let mut session = session();
    let (_document, spread) = a_spread(&mut session);
    let outcome = session
        .execute(
            "layout.item.add",
            &json!({
                "spread": spread,
                "name": "the square",
                "kind": "layout.item.graphic",
                "shape": "rectangle",
                "rect": { "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 },
            }),
        )
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    session
        .execute("layout.item.rotate", &json!({ "item": item, "degrees": 45.0 }))
        .expect("the item is turned");
    let facts = facts_of(&mut session, item);
    let width = facts["turned"]["width"].as_f64().expect("the box has a width");
    let diagonal = 10.0 * 2f64.sqrt();
    assert!((width - diagonal).abs() < 1e-9, "a square turned by forty-five is as wide as its diagonal");
    let x = facts["turned"]["x"].as_f64().expect("the box has a place");
    assert!((x - (5.0 - diagonal / 2.0)).abs() < 1e-9, "and it sits about its middle");
}

/// A turn that is **not a number** is refused.
#[test]
fn a_turn_that_is_not_a_number_is_refused() {
    let mut session = session();
    let (_document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let refused = session
        .execute("layout.item.rotate", &json!({ "item": item, "degrees": "sideways" }))
        .expect_err("the turn is refused");
    assert!(refused.to_string().contains("number"), "the refusal says: {}", refused);
}

/// A **locked** item may not be turned.
#[test]
fn a_locked_item_may_not_be_turned() {
    let mut session = session();
    let (_document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session.execute("layout.item.lock", &json!({ "item": item })).expect("the item is locked");
    let refused = session
        .execute("layout.item.rotate", &json!({ "item": item, "degrees": 30.0 }))
        .expect_err("the turn is refused");
    assert!(refused.to_string().contains("locked"), "the refusal says why");
}
