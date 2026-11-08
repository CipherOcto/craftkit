//! The **group** of items.
//!
//! The group holds the items, and it stands where the item at the back
//! stood.

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

/// A **spread** with **four** items on it.
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
    for name in ["first", "second", "third", "fourth"] {
        let outcome = session
            .execute("layout.item.add", &json!({ "spread": spread, "name": name }))
            .expect("the item is added");
        items.push(outcome.data["id"].as_u64().expect("the item has an id"));
    }
    (spread, items)
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

/// The **items** a group holds.
fn items_of(session: &Session, group: u64) -> Vec<u64> {
    let node = session.document().node(group).expect("the group is there");
    node.children.iter().map(|child| child.id).collect()
}

/// Two items are grouped: the group holds them, in the order they
/// were.
#[test]
fn two_items_are_grouped() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    let outcome = session
        .execute("layout.item.group", &json!({ "items": [items[1], items[2]] }))
        .expect("the items are grouped");
    let group = outcome.data["group"].as_u64().expect("the group has an id");
    assert_eq!(items_of(&session, group), vec![items[1], items[2]], "the group holds them");
    let order = order_of(&mut session, spread);
    assert_eq!(order.len(), 3, "the spread holds three items now: {order:?}");
    assert!(order.contains(&group), "and the group is one of them: {order:?}");
    assert!(!order.contains(&items[1]), "and the items are not: {order:?}");
}

/// The group stands where the item at the **back** stood.
#[test]
fn the_group_stands_where_the_item_at_the_back_stood() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    let outcome = session
        .execute("layout.item.group", &json!({ "items": [items[1], items[2]] }))
        .expect("the items are grouped");
    let group = outcome.data["group"].as_u64().expect("the group has an id");
    let order = order_of(&mut session, spread);
    assert_eq!(order, vec![items[0], group, items[3]], "the group is where the second item stood");
}

/// Ungrouping puts the items back where the group stood, in the order
/// they were in.
#[test]
fn ungrouping_puts_the_items_back() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    let outcome = session
        .execute("layout.item.group", &json!({ "items": [items[1], items[2]] }))
        .expect("the items are grouped");
    let group = outcome.data["group"].as_u64().expect("the group has an id");
    session.execute("layout.item.ungroup", &json!({ "group": group })).expect("the group is ungrouped");
    let order = order_of(&mut session, spread);
    assert_eq!(order, items, "the items are back where they were: {order:?}");
}

/// **One item** is not a group, and that is refused.
#[test]
fn one_item_is_not_a_group() {
    let mut session = session();
    let (_spread, items) = a_spread(&mut session);
    let outcome = session.execute("layout.item.group", &json!({ "items": [items[0]] }));
    assert!(outcome.is_err(), "the group is refused: {outcome:?}");
}

/// Items that are **not at the same level** may not be grouped: the
/// tree's own rule.
#[test]
fn items_that_are_not_at_the_same_level_are_refused() {
    let mut session = session();
    let (spread, items) = a_spread(&mut session);
    session
        .execute("layout.item.group", &json!({ "items": [items[1], items[2]] }))
        .expect("the items are grouped");
    let outcome = session
        .execute("layout.item.group", &json!({ "items": [items[0], items[1]] }))
        .expect_err("the items are refused");
    assert!(
        outcome.to_string().contains("the same spread and at the same level"),
        "the refusal says why: {outcome}"
    );
    let _ = spread;
}

/// A **locked** item may not be grouped: an edit of a locked item is
/// refused.
#[test]
fn a_locked_item_is_refused() {
    let mut session = session();
    let (_spread, items) = a_spread(&mut session);
    session.execute("layout.item.lock", &json!({ "item": items[1] })).expect("the item is locked");
    let outcome = session.execute("layout.item.group", &json!({ "items": [items[0], items[1]] }));
    assert!(outcome.is_err(), "the group is refused: {outcome:?}");
}

/// The items of a group are drawn in the order they stand in the
/// group: a group is a container of items like a spread is.
#[test]
fn the_items_of_a_group_may_be_reordered() {
    let mut session = session();
    let (_spread, items) = a_spread(&mut session);
    let outcome = session
        .execute("layout.item.group", &json!({ "items": [items[1], items[2]] }))
        .expect("the items are grouped");
    let group = outcome.data["group"].as_u64().expect("the group has an id");
    session
        .execute("layout.item.front", &json!({ "item": items[1] }))
        .expect("the item is brought to the front");
    assert_eq!(items_of(&session, group), vec![items[2], items[1]], "the order of the group changed");
}
