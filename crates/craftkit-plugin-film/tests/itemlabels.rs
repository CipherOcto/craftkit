//! The **label** of a media item.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
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

/// A **bin** and a **media item** in it, and their ids.
fn an_item(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    (bin, item)
}

/// The label of an item, as the document holds it.
fn label_of(session: &Session, item: u64) -> Option<String> {
    session
        .document()
        .node(item)
        .expect("the item is there")
        .data
        .get("label")
        .and_then(|label| label.as_str())
        .map(|label| label.to_string())
}

/// An item may be **labelled**.
#[test]
fn an_item_may_be_labelled() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let outcome = session
        .execute("film.item.label", &json!({ "item": item, "label": "Violet" }))
        .expect("the item is labelled");
    assert_eq!(outcome.data["label"].as_str(), Some("Violet"), "the label is said");
    assert_eq!(label_of(&session, item).as_deref(), Some("Violet"), "and the item carries it");
}

/// A label that is **not one of the tree's** is refused.
#[test]
fn a_label_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let error = session
        .execute("film.item.label", &json!({ "item": item, "label": "Chartreuse" }))
        .expect_err("the label is refused");
    let message = error.to_string();
    assert!(message.contains("there is no label"), "it says why: {message}");
    assert_eq!(label_of(&session, item), None, "and nothing was written");
}

/// The label may be **taken away**.
#[test]
fn the_label_may_be_taken_away() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    session
        .execute("film.item.label", &json!({ "item": item, "label": "Iris" }))
        .expect("the item is labelled");
    let outcome =
        session.execute("film.item.label.delete", &json!({ "item": item })).expect("the label is taken away");
    assert_eq!(outcome.data["label"].as_str(), Some("Iris"), "the label that was there is said");
    assert_eq!(label_of(&session, item), None, "and the item carries none");
}

/// The **group** of an item is derived from the labels.
#[test]
fn the_group_of_an_item_is_derived() {
    let mut session = session();
    let (bin, first) = an_item(&mut session);
    let outcome = session
        .execute(
            "film.item.add",
            &json!({ "name": "the other media", "bin": bin, "item": 2, "duration": 100 }),
        )
        .expect("the item is added");
    let second = outcome.data["id"].as_u64().expect("the item has an id");
    session
        .execute("film.item.label", &json!({ "item": first, "label": "Teal" }))
        .expect("the item is labelled");
    session
        .execute("film.item.label", &json!({ "item": second, "label": "Teal" }))
        .expect("the item is labelled");
    let outcome =
        session.execute("film.item.label.group", &json!({ "item": first })).expect("the group is read");
    let items = outcome.data["items"].as_array().expect("the items are a list");
    assert_eq!(items.len(), 2, "the group holds both");
    assert!(items.iter().any(|item| item.as_u64() == Some(first)), "and the item itself");
}

/// A node that is **not an item** is refused.
#[test]
fn a_node_that_is_not_an_item_is_refused() {
    let mut session = session();
    let (bin, _item) = an_item(&mut session);
    let error = session
        .execute("film.item.label", &json!({ "item": bin, "label": "Violet" }))
        .expect_err("the label is refused");
    let message = error.to_string();
    assert!(message.contains("not an item"), "it says why: {message}");
    let error = session
        .execute("film.item.label", &json!({ "item": 9999, "label": "Violet" }))
        .expect_err("the label is refused");
    let message = error.to_string();
    assert!(message.contains("there is no item"), "and there is nothing: {message}");
}

/// The label is **saved** with the document.
#[test]
fn the_label_is_saved() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    session
        .execute("film.item.label", &json!({ "item": item, "label": "Rose" }))
        .expect("the item is labelled");
    let text = session.document().to_json().to_string();
    assert!(text.contains("Rose"), "the label is in the document");
}
