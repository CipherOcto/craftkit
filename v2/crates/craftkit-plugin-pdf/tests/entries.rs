//! The **key** of an object's dictionary.
//!
//! The dictionary is the payload, and a payload that cannot be changed
//! is a payload that was wrong the moment it was written.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_pdf::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the PDF battery.
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

/// An **object** with the dictionary it was given.
fn an_object(session: &mut Session, entries: craftkit_sdk::serde_json::Value) -> u64 {
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": entries }))
        .expect("the object is added");
    outcome.data["id"].as_u64().expect("the object has an id")
}

/// The **entries** of a node's dictionary, as the pairs they are.
fn entries_of(session: &Session, id: u64) -> Vec<(String, craftkit_sdk::serde_json::Value)> {
    let node = session.document().node(id).expect("the node is there");
    node.data["entries"]
        .as_array()
        .expect("the dictionary is there")
        .iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            let value = pair.get(1)?.clone();
            Some((key, value))
        })
        .collect()
}

/// A key is **added** to an object's dictionary.
#[test]
fn a_key_is_added() {
    let mut session = session();
    let object = an_object(&mut session, json!([["Type", "Page"]]));
    session
        .execute("pdf.object.set", &json!({ "object": object, "key": "MediaBox", "value": [0, 0, 595, 842] }))
        .expect("the key is set");
    let entries = entries_of(&session, object);
    assert_eq!(entries.len(), 2, "the dictionary has two keys: {entries:?}");
    assert_eq!(entries[0].0, "Type", "the key that was there keeps its place");
    assert_eq!(entries[1].0, "MediaBox", "and the new one is at the end");
}

/// A key that is **there** is changed, and it keeps its place.
#[test]
fn a_key_is_changed() {
    let mut session = session();
    let object = an_object(&mut session, json!([["Type", "Page"], ["Rotate", 0]]));
    session
        .execute("pdf.object.set", &json!({ "object": object, "key": "Rotate", "value": 90 }))
        .expect("the key is set");
    let entries = entries_of(&session, object);
    assert_eq!(entries.len(), 2, "the dictionary still has two keys: {entries:?}");
    assert_eq!(entries[1].0, "Rotate", "the key keeps its place");
    assert_eq!(entries[1].1, json!(90), "and it has the new value");
}

/// A key is **taken away**.
#[test]
fn a_key_is_taken_away() {
    let mut session = session();
    let object = an_object(&mut session, json!([["Type", "Page"], ["Rotate", 90]]));
    session
        .execute("pdf.object.remove", &json!({ "object": object, "key": "Rotate" }))
        .expect("the key is taken away");
    let entries = entries_of(&session, object);
    assert_eq!(entries.len(), 1, "the dictionary has one key: {entries:?}");
    assert_eq!(entries[0].0, "Type", "and it is the other one");
}

/// A key that is **not there** is refused: a key that was never there
/// is not a key that was taken away.
#[test]
fn a_key_that_is_not_there_is_refused() {
    let mut session = session();
    let object = an_object(&mut session, json!([["Type", "Page"]]));
    let error = session
        .execute("pdf.object.remove", &json!({ "object": object, "key": "Rotate" }))
        .expect_err("the removal is refused");
    assert!(error.to_string().contains("no key `Rotate`"), "the error says why: {error}");
    assert_eq!(entries_of(&session, object).len(), 1, "and the dictionary is as it was");
}

/// A node that is **not an object** is refused.
#[test]
fn a_node_that_is_not_an_object_is_refused() {
    let mut session = session();
    let object = an_object(&mut session, json!([]));
    let error = session
        .execute("pdf.object.set", &json!({ "object": object + 1_000_000, "key": "Rotate", "value": 90 }))
        .expect_err("the setting is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}

/// An **empty** key is refused.
#[test]
fn an_empty_key_is_refused() {
    let mut session = session();
    let object = an_object(&mut session, json!([]));
    let error = session
        .execute("pdf.object.set", &json!({ "object": object, "key": "  ", "value": 90 }))
        .expect_err("the key is refused");
    assert!(error.to_string().contains("a key is text"), "the error says why: {error}");
}
