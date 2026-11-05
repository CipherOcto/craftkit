//! The **order** the items of a spread are drawn in.

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

/// A **spread** with **three** items on it.
fn a_spread(session: &mut Session) -> (u64, Vec<u64>) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let mut items = Vec::new();
    for name in ["first", "second", "third"] {
        let outcome = session
            .execute("layout.item.add", &json!({ "spread": spread, "name": name }))
            .expect("the item is added");
        items.push(outcome.data["id"].as_u64().expect("the item has an id"));
    }
    (spread, items)
}

/// The **children** of a spread, in the order they stand in.
fn places_of(session: &Session, spread: u64) -> Vec<u64> {
    let node = session.document().node(spread).expect("the spread is there");
    node.children.iter().map(|child| child.id).collect()
}

/// The **items** of a spread, in the order they are drawn.
fn order_of(session: &mut Session, spread: u64) -> Vec<u64> {
    let outcome =
        session.execute("layout.item.list", &json!({ "spread": spread })).expect("the items are read");
    outcome.data["items"]
        .as_array()
        .expect("the items are a list")
        .clone()
        .into_iter()
        .map(|item| item.as_u64().unwrap_or_default())
        .collect()
}

/// **The items are listed in the order they are drawn.**
#[test]
fn the_items_are_listed_in_the_order_they_are_drawn() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    assert_eq!(order_of(&mut session, spread), items);
}

/// **An item is brought to the front.**
#[test]
fn an_item_is_brought_to_the_front() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    session
        .execute("layout.item.front", &json!({ "item": items[0] }))
        .expect("the item is brought to the front");
    let order = order_of(&mut session, spread);
    assert_eq!(order, vec![items[1], items[2], items[0]]);
}

/// **An item is sent to the back.**
#[test]
fn an_item_is_sent_to_the_back() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    session.execute("layout.item.back", &json!({ "item": items[2] })).expect("the item is sent to the back");
    let order = order_of(&mut session, spread);
    assert_eq!(order, vec![items[2], items[0], items[1]]);
}

/// **An item moves one step at a time.**
#[test]
fn an_item_moves_one_step_at_a_time() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    session.execute("layout.item.forward", &json!({ "item": items[0] })).expect("the item moves forward");
    assert_eq!(order_of(&mut session, spread), vec![items[1], items[0], items[2]]);
    session.execute("layout.item.backward", &json!({ "item": items[0] })).expect("the item moves backward");
    assert_eq!(order_of(&mut session, spread), vec![items[0], items[1], items[2]]);
}

/// **An item that is already at the front does not move.**
#[test]
fn an_item_that_is_already_at_the_front_does_not_move() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    let outcome = session
        .execute("layout.item.front", &json!({ "item": items[2] }))
        .expect("the item is told to go to the front");
    assert_eq!(outcome.data["moved"], json!(false));
    assert_eq!(order_of(&mut session, spread), items);
}

/// **A locked item may not be moved.**
#[test]
fn a_locked_item_may_not_be_moved() {
    let mut session = session();
    let (_, items) = a_spread(&mut session);
    session.execute("layout.item.lock", &json!({ "item": items[0] })).expect("the item is locked");
    let failure =
        session.execute("layout.item.front", &json!({ "item": items[0] })).expect_err("the move is refused");
    let message = failure.to_string();
    assert!(message.contains("a locked item may not be edited"), "the refusal says why: {message}");
}

/// **The pages of a spread keep their places.**
#[test]
fn the_pages_keep_their_places() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    let outcome = session
        .execute("layout.page.add", &json!({ "spread": spread, "name": "the page" }))
        .expect("the page is added");
    let page = outcome.data["id"].as_u64().expect("the page has an id");
    let places_before = places_of(&session, spread);
    let page_at = places_before.iter().position(|id| *id == page).expect("the page is in the spread");
    session
        .execute("layout.item.front", &json!({ "item": items[0] }))
        .expect("the item is brought to the front");
    let places_after = places_of(&session, spread);
    let page_after = places_after.iter().position(|id| *id == page).expect("the page is in the spread");
    assert_eq!(page_at, page_after, "the page keeps its place: {places_before:?} {places_after:?}");
}

/// **A node that is not a spread is refused.**
#[test]
fn a_node_that_is_not_a_spread_is_refused() {
    let mut session = session();
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let failure = session
        .execute("layout.item.list", &json!({ "spread": document }))
        .expect_err("the spread is refused");
    let message = failure.to_string();
    assert!(message.contains("and not a spread"), "the refusal says why: {message}");
}
