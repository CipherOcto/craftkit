//! The **name** of a thing in the project.

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

/// A **bin** with an **item** in it, and their ids.
fn an_item(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    (bin, item)
}

/// The **name** a node has.
fn name_of(session: &Session, node: u64) -> String {
    session.document().node(node).expect("the node is there").name.clone()
}

/// A media item may be **named**.
#[test]
fn a_media_item_may_be_named() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let outcome = session
        .execute("film.item.rename", &json!({ "item": item, "name": "the shot" }))
        .expect("the item is named");
    assert_eq!(outcome.data["name"].as_str(), Some("the shot"), "the name is said");
    assert_eq!(name_of(&session, item), "the shot", "and the item carries it");
}

/// A bin may be **named**.
#[test]
fn a_bin_may_be_named() {
    let mut session = session();
    let (bin, _item) = an_item(&mut session);
    session
        .execute("film.bin.rename", &json!({ "bin": bin, "name": "the good bin" }))
        .expect("the bin is named");
    assert_eq!(name_of(&session, bin), "the good bin", "and the bin carries it");
}

/// The name of an item is said by the **project**.
#[test]
fn the_name_of_an_item_is_said_by_the_project() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    session
        .execute("film.item.rename", &json!({ "item": item, "name": "the shot" }))
        .expect("the item is named");
    let outcome = session.execute("film.project", &json!({})).expect("the project is read");
    assert_eq!(outcome.data["items"][0]["name"].as_str(), Some("the shot"), "the name is the new one");
}

/// A name that is **nothing** is refused.
#[test]
fn a_name_that_is_nothing_is_refused() {
    let mut session = session();
    let (_bin, item) = an_item(&mut session);
    let error = session
        .execute("film.item.rename", &json!({ "item": item, "name": "  " }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("an item has a name"), "it says why: {error}");
    assert_eq!(name_of(&session, item), "the media", "and the name is the old one");
}

/// A node that is **not an item** is refused.
#[test]
fn a_node_that_is_not_an_item_is_refused() {
    let mut session = session();
    let (bin, _item) = an_item(&mut session);
    let error = session
        .execute("film.item.rename", &json!({ "item": bin, "name": "the shot" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("not an item"), "it says why: {error}");
    let error = session
        .execute("film.bin.rename", &json!({ "bin": 9999, "name": "the bin" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("there is no node"), "and there is none: {error}");
}
