//! The **item** of a spread: its opacity, its hiddenness, its lock and
//! its scale.

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

/// An **item** on a spread, with a shape.
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

/// The **facts** of an item, as the plan says them.
fn facts_of(session: &Session, item: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == item).expect("the item is in the plan");
    entry.value.clone()
}

/// **The opacity is a percentage and it is kept as the fraction.**
#[test]
fn the_opacity_is_a_percentage() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.opacity", &json!({ "item": item, "opacity": 40.0 }))
        .expect("the opacity is set");
    assert_eq!(facts_of(&session, item)["opacity"], json!(0.4));
}

/// **An opacity outside the percentage is refused.**
#[test]
fn an_opacity_outside_the_percentage_is_refused() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let failure = session
        .execute("layout.item.opacity", &json!({ "item": item, "opacity": 140.0 }))
        .expect_err("the opacity is refused");
    let message = failure.to_string();
    assert!(message.contains("a percentage from zero to a hundred"), "the refusal says why: {message}");
}

/// **An item may be hidden and shown again.**
#[test]
fn an_item_may_be_hidden_and_shown() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session.execute("layout.item.hide", &json!({ "item": item })).expect("the item is hidden");
    assert_eq!(facts_of(&session, item)["hidden"], json!(true));
    session.execute("layout.item.show", &json!({ "item": item })).expect("the item is shown");
    assert_eq!(facts_of(&session, item)["hidden"], json!(false));
}

/// **A locked item may not be edited.**
#[test]
fn a_locked_item_may_not_be_edited() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session.execute("layout.item.lock", &json!({ "item": item })).expect("the item is locked");
    assert_eq!(facts_of(&session, item)["locked"], json!(true));
    let failure = session
        .execute("layout.item.opacity", &json!({ "item": item, "opacity": 40.0 }))
        .expect_err("the edit is refused");
    let message = failure.to_string();
    assert!(message.contains("a locked item may not be edited"), "the refusal says why: {message}");
}

/// **An unlocked item may be edited again.**
#[test]
fn an_unlocked_item_may_be_edited() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session.execute("layout.item.lock", &json!({ "item": item })).expect("the item is locked");
    session.execute("layout.item.unlock", &json!({ "item": item })).expect("the item is unlocked");
    assert_eq!(facts_of(&session, item)["locked"], json!(false));
    session
        .execute("layout.item.opacity", &json!({ "item": item, "opacity": 40.0 }))
        .expect("the opacity is set");
}

/// **A locked item refuses the story and the layer too.**
#[test]
fn a_locked_item_refuses_the_story() {
    let mut session = session();
    let (document, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let outcome = session
        .execute(
            "layout.story.add",
            &json!({ "document": document, "name": "the story", "text": "the words" }),
        )
        .expect("the story is added");
    let story = outcome.data["id"].as_u64().expect("the story has an id");
    session.execute("layout.item.lock", &json!({ "item": item })).expect("the item is locked");
    let failure = session
        .execute("layout.item.story", &json!({ "item": item, "story": story }))
        .expect_err("the story is refused");
    let message = failure.to_string();
    assert!(message.contains("a locked item may not be edited"), "the refusal says why: {message}");
}

/// **The scale is a factor on the size of the shape.**
#[test]
fn the_scale_is_a_factor_on_the_size() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    session
        .execute("layout.item.scale", &json!({ "item": item, "factor": 2.0 }))
        .expect("the item is scaled");
    let facts = facts_of(&session, item);
    assert_eq!(facts["rect"]["width"], json!(200.0));
    assert_eq!(facts["rect"]["height"], json!(100.0));
    assert_eq!(facts["rect"]["x"], json!(0.0), "the place of the item does not move");
}

/// **A factor that is not a number above zero is refused.**
#[test]
fn a_factor_that_is_not_a_number_is_refused() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let item = an_item(&mut session, spread);
    let failure = session
        .execute("layout.item.scale", &json!({ "item": item, "factor": 0.0 }))
        .expect_err("the factor is refused");
    let message = failure.to_string();
    assert!(message.contains("a factor is a number above zero"), "the refusal says why: {message}");
}

/// **An item with no shape has nothing to scale.**
#[test]
fn an_item_with_no_shape_has_nothing_to_scale() {
    let mut session = session();
    let (_, spread) = a_spread(&mut session);
    let outcome = session
        .execute("layout.item.add", &json!({ "spread": spread, "name": "the frame" }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let failure = session
        .execute("layout.item.scale", &json!({ "item": item, "factor": 2.0 }))
        .expect_err("the scale is refused");
    let message = failure.to_string();
    assert!(message.contains("has no shape to scale"), "the refusal says why: {message}");
}

/// **A node that is not an item is refused.**
#[test]
fn a_node_that_is_not_an_item_is_refused() {
    let mut session = session();
    let (document, _) = a_spread(&mut session);
    let failure = session
        .execute("layout.item.opacity", &json!({ "item": document, "opacity": 40.0 }))
        .expect_err("the item is refused");
    let message = failure.to_string();
    assert!(message.contains("and not an item"), "the refusal says why: {message}");
}
