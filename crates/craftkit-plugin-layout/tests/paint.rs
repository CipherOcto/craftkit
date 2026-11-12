//! The **paint** of an item: its fill and its stroke.

use craftkit_sdk::serde_json::json;
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

/// The **paint** of an item, as the plan reports it.
fn paint_of(session: &mut Session, document: u64, item: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == item).expect("the item is in the plan");
    let _ = document;
    entry.value.clone()
}

/// **An item may be filled with a colour.**
#[test]
fn an_item_may_be_filled() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.fill", &json!({ "item": item, "colour": "#ff0000" }))
        .expect("the item is filled");
    let paint = paint_of(&mut session, document, item);
    assert_eq!(paint["fill"]["colour"], json!([1.0, 0.0, 0.0, 1.0]));
}

/// **An item with no fill is not painted.**
#[test]
fn an_item_with_no_fill_is_not_painted() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    assert!(paint_of(&mut session, document, item)["fill"].is_null(), "an item is not painted to begin with");
    session
        .execute("layout.item.fill", &json!({ "item": item, "colour": "#00ff00" }))
        .expect("the item is filled");
    session
        .execute("layout.item.fill", &json!({ "item": item, "colour": "" }))
        .expect("the fill is taken away");
    assert!(paint_of(&mut session, document, item)["fill"].is_null());
}

/// **An item may be stroked, and the stroke weighs one point unless it
/// is told otherwise.**
#[test]
fn an_item_may_be_stroked() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.stroke", &json!({ "item": item, "colour": "#000000" }))
        .expect("the item is stroked");
    let paint = paint_of(&mut session, document, item);
    assert_eq!(paint["stroke"]["weight"], json!(1.0), "the tree's own one point");
    session
        .execute("layout.item.stroke", &json!({ "item": item, "weight": 4.0 }))
        .expect("the weight is set");
    assert_eq!(paint_of(&mut session, document, item)["stroke"]["weight"], json!(4.0));
}

/// **A stroke may be taken away.**
#[test]
fn a_stroke_may_be_taken_away() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.stroke", &json!({ "item": item, "colour": "#000000" }))
        .expect("the item is stroked");
    session
        .execute("layout.item.stroke", &json!({ "item": item, "colour": "" }))
        .expect("the stroke is taken away");
    assert!(paint_of(&mut session, document, item)["stroke"].is_null());
}

/// **A colour that is not a colour is refused.**
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    for colour in ["red", "#ff00", "#gggggg", "ff0000"] {
        let outcome = session.execute("layout.item.fill", &json!({ "item": item, "colour": colour }));
        assert!(outcome.is_err(), "the colour `{colour}` is refused");
    }
}

/// **A weight that is not a weight is refused.**
#[test]
fn a_weight_that_is_not_a_weight_is_refused() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    for weight in [-1.0, 10_000.0] {
        let outcome = session
            .execute("layout.item.stroke", &json!({ "item": item, "colour": "#000000", "weight": weight }));
        assert!(outcome.is_err(), "the weight {weight} is refused");
    }
}

/// **A locked item may not be painted.**
#[test]
fn a_locked_item_may_not_be_painted() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session.execute("layout.item.lock", &json!({ "item": item })).expect("the item is locked");
    let outcome = session.execute("layout.item.fill", &json!({ "item": item, "colour": "#ff0000" }));
    assert!(outcome.is_err(), "a locked item may not be edited");
}
