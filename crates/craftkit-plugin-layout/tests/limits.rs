//! The **limits of the layout battery**: the numbers it will not go past.
//!
//! A limit is a number the battery keeps. The numbers that come in are
//! from outside — a command line, a script, an agent — so they are
//! checked before they are used, and a limit that no test names is a
//! limit nothing proves holds.

use craftkit_plugin_layout::{MAX_CHAIN, MAX_ITEMS, MAX_NAME, MAX_TARGET, MAX_WEIGHT};
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_layout::CRAFTKIT_PLUGIN_ANCHOR;

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

/// A document with a spread, a page and an item in it: the ids of the
/// document and of the item.
fn a_document_with_an_item(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    session
        .execute("layout.page.add", &json!({ "spread": spread, "name": "the page" }))
        .expect("the page is added");
    let outcome = session
        .execute("layout.item.add", &json!({ "spread": spread, "name": "the item" }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    (document, item)
}

/// A group of **more items** than `MAX_ITEMS` allows is refused.
///
/// The items come from outside, and a list of a thousand and one ids is
/// refused rather than half-grouped.
#[test]
fn a_group_with_too_many_items_is_refused() {
    let mut session = session();
    let items: Vec<u64> = (0..=MAX_ITEMS as u64).collect();
    let error = session
        .execute("layout.item.group", &json!({ "items": items }))
        .expect_err("there are too many items");
    let message = error.to_string();
    assert!(message.contains("the most there may be"), "{message}");
    assert!(message.contains(&MAX_ITEMS.to_string()), "the refusal names the bound: {message}");
}

/// A stroke **weight** over `MAX_WEIGHT` is refused.
///
/// A weight is a number from zero to the limit, and a number outside it
/// is not a weight.
#[test]
fn a_stroke_weight_over_the_limit_is_refused() {
    let mut session = session();
    let (_, item) = a_document_with_an_item(&mut session);
    let error = session
        .execute("layout.item.stroke", &json!({ "item": item, "weight": MAX_WEIGHT + 1.0 }))
        .expect_err("that weight is too heavy");
    let message = error.to_string();
    assert!(message.contains("is not a weight"), "{message}");
    assert!(message.contains(&MAX_WEIGHT.to_string()), "the refusal names the bound: {message}");
}

/// A bookmark **name** over `MAX_NAME` is refused.
#[test]
fn a_bookmark_name_over_the_limit_is_refused() {
    let mut session = session();
    let (document, _) = a_document_with_an_item(&mut session);
    let name = "n".repeat(MAX_NAME + 1);
    let error = session
        .execute("layout.bookmark.add", &json!({ "document": document, "name": name }))
        .expect_err("that name is too long");
    let message = error.to_string();
    assert!(message.contains("longer than"), "{message}");
    assert!(message.contains(&MAX_NAME.to_string()), "the refusal names the bound: {message}");
}

/// A link **target** over `MAX_TARGET` is refused.
#[test]
fn a_link_target_over_the_limit_is_refused() {
    let mut session = session();
    let (_, item) = a_document_with_an_item(&mut session);
    let target = format!("https://example.com/{}", "a".repeat(MAX_TARGET));
    let error = session
        .execute("layout.item.link", &json!({ "item": item, "target": target }))
        .expect_err("that target is too long");
    let message = error.to_string();
    assert!(message.contains("at most"), "{message}");
    assert!(message.contains(&MAX_TARGET.to_string()), "the refusal names the bound: {message}");
}

/// A **document**, and its id.
fn a_document(session: &mut Session) -> u64 {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    outcome.data["id"].as_u64().expect("the document has an id")
}

/// A **style** in a document, based on that base.
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

/// A **chain of styles** stops at `MAX_CHAIN`.
///
/// The chain of a style is walked from the style upwards, and the walk
/// is bounded: a chain of forty styles is cut at the limit, and the
/// bases beyond it are not inherited. The tree reports the furthest
/// base the walk reached as the style's **base**, and that base is the
/// last one within the limit.
#[test]
fn a_chain_of_styles_stops_at_the_limit() {
    let mut session = session();
    let document = a_document(&mut session);
    // Forty styles, each one based on the one before it.
    let mut styles = Vec::new();
    for index in 0..40 {
        let base = styles.last().copied();
        let style = a_style(&mut session, document, &format!("the style {index}"), base);
        styles.push(style);
    }
    let deepest = styles[0];
    let last = styles[39];
    // The deepest style is filled, and its fill is **not** inherited:
    // the walk was cut before it.
    session
        .execute("layout.style.fill", &json!({ "style": deepest, "colour": "#00ff00" }))
        .expect("the deepest style is filled");
    let said = said_of(&mut session, last);
    assert!(said["fill"].is_null(), "the fill beyond the limit is not inherited: {said:?}");
    // The base the tree reports is the last one the walk reached: the
    // walk went up as many bases as the cap allows, and no more.
    let reached = styles[styles.len() - 1 - MAX_CHAIN];
    assert_eq!(said["base"], json!(reached), "the chain stops at the limit: {said:?}");
    assert_ne!(said["base"], json!(deepest), "and the deepest base is not in it: {said:?}");
}

/// A **chain of styles within the limit** is walked to its end.
///
/// This is the other half of the test above: a cap that cut every chain
/// would not be a cap, it is a wall.
#[test]
fn a_chain_within_the_limit_is_walked_to_its_end() {
    let mut session = session();
    let document = a_document(&mut session);
    let mut styles = Vec::new();
    for index in 0..5 {
        let base = styles.last().copied();
        let style = a_style(&mut session, document, &format!("the style {index}"), base);
        styles.push(style);
    }
    let deepest = styles[0];
    let last = styles[4];
    session
        .execute("layout.style.fill", &json!({ "style": deepest, "colour": "#00ff00" }))
        .expect("the deepest style is filled");
    let said = said_of(&mut session, last);
    assert_eq!(said["fill"]["colour"], json!([0.0, 1.0, 0.0, 1.0]), "the base's fill is under it: {said:?}");
    assert_eq!(said["base"], json!(deepest), "and the base is the deepest: {said:?}");
}
