//! The **style** of an item: a named set of paint facts that items may
//! use.

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

/// A **style** in a document.
fn a_style(session: &mut Session, document: u64, name: &str, based_on: Option<u64>) -> u64 {
    let mut params = json!({ "document": document, "name": name });
    if let Some(base) = based_on {
        params["based_on"] = json!(base);
    }
    let outcome = session.execute("layout.style.add", &params).expect("the style is added");
    outcome.data["style"].as_u64().expect("the style has an id")
}

/// What the plan says of a node.
fn said_of(session: &mut Session, node: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == node).expect("the node is in the plan");
    entry.value.clone()
}

/// **A style may be added, and it is a style of its own kind.**
#[test]
fn a_style_may_be_added() {
    let mut session = session();
    let (document, _) = a_spread(&mut session);
    let style = a_style(&mut session, document, "the style", None);
    let said = said_of(&mut session, style);
    assert!(said.get("fill").is_some(), "the style says its paint: {said}");
}

/// **The style's fill may be set.**
#[test]
fn the_fill_of_a_style_may_be_set() {
    let mut session = session();
    let (document, _) = a_spread(&mut session);
    let style = a_style(&mut session, document, "the style", None);
    session
        .execute("layout.style.fill", &json!({ "style": style, "colour": "#ff0000" }))
        .expect("the style is filled");
    assert_eq!(said_of(&mut session, style)["fill"]["colour"], json!([1.0, 0.0, 0.0, 1.0]));
}

/// **A style has the fill of its base under it.**
#[test]
fn a_style_has_the_fill_of_its_base_under_it() {
    let mut session = session();
    let (document, _) = a_spread(&mut session);
    let base = a_style(&mut session, document, "the base", None);
    let style = a_style(&mut session, document, "the style", Some(base));
    session
        .execute("layout.style.fill", &json!({ "style": base, "colour": "#00ff00" }))
        .expect("the base is filled");
    assert_eq!(said_of(&mut session, style)["fill"]["colour"], json!([0.0, 1.0, 0.0, 1.0]));
    session
        .execute("layout.style.fill", &json!({ "style": style, "colour": "#0000ff" }))
        .expect("the style is filled");
    assert_eq!(
        said_of(&mut session, style)["fill"]["colour"],
        json!([0.0, 0.0, 1.0, 1.0]),
        "and the nearer style wins"
    );
}

/// **An item on a style is painted with the style's paint.**
#[test]
fn an_item_on_a_style_is_painted_with_its_paint() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let style = a_style(&mut session, document, "the style", None);
    session
        .execute("layout.style.fill", &json!({ "style": style, "colour": "#ff0000" }))
        .expect("the style is filled");
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.style", &json!({ "item": item, "style": style }))
        .expect("the item is on the style");
    let said = said_of(&mut session, item);
    assert_eq!(said["fill"]["colour"], json!([1.0, 0.0, 0.0, 1.0]), "the paint is derived");
    assert_eq!(said["style"], json!("the style"));
}

/// **An item's own paint overrides its style's.**
#[test]
fn an_items_own_paint_overrides_its_styles() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let style = a_style(&mut session, document, "the style", None);
    session
        .execute("layout.style.fill", &json!({ "style": style, "colour": "#ff0000" }))
        .expect("the style is filled");
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.style", &json!({ "item": item, "style": style }))
        .expect("the item is on the style");
    session
        .execute("layout.item.fill", &json!({ "item": item, "colour": "#0000ff" }))
        .expect("the item is filled");
    let said = said_of(&mut session, item);
    assert_eq!(said["fill"]["colour"], json!([0.0, 0.0, 1.0, 1.0]), "the item's own paint wins");
    assert_eq!(said["own_fill"]["colour"], json!([0.0, 0.0, 1.0, 1.0]));
}

/// **An item may be taken off its style.**
#[test]
fn an_item_may_be_taken_off_its_style() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let style = a_style(&mut session, document, "the style", None);
    session
        .execute("layout.style.fill", &json!({ "style": style, "colour": "#ff0000" }))
        .expect("the style is filled");
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.style", &json!({ "item": item, "style": style }))
        .expect("the item is on the style");
    session.execute("layout.item.style", &json!({ "item": item })).expect("the item is on no style");
    let said = said_of(&mut session, item);
    assert!(said["fill"].is_null(), "the item is not painted");
    assert!(said["style"].is_null());
}

/// **A node that is not a style is refused.**
#[test]
fn a_node_that_is_not_a_style_is_refused() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let outcome = session.execute("layout.style.fill", &json!({ "style": item, "colour": "#ff0000" }));
    assert!(outcome.is_err(), "an item is not a style");
    let outcome = session
        .execute("layout.style.add", &json!({ "document": document, "name": "the style", "based_on": item }));
    assert!(outcome.is_err(), "a style may not be based on an item");
}

/// **A locked item may not be put on a style.**
#[test]
fn a_locked_item_may_not_be_put_on_a_style() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let style = a_style(&mut session, document, "the style", None);
    let item = an_item(&mut session, spread);
    session.execute("layout.item.lock", &json!({ "item": item })).expect("the item is locked");
    let outcome = session.execute("layout.item.style", &json!({ "item": item, "style": style }));
    assert!(outcome.is_err(), "a locked item may not be edited");
}
