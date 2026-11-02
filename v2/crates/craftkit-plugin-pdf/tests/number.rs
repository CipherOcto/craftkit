//! The **number** of an object.
//!
//! An object is named by its number in the file, and every reference to
//! it is that number: so a number may not be taken twice, and a number
//! that is changed takes its references with it.

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

/// An **object** with the number it has in the file.
fn an_object(session: &mut Session, number: u64, entries: craftkit_sdk::serde_json::Value) -> u64 {
    let outcome = session
        .execute("pdf.object.add", &json!({ "object": number, "entries": entries }))
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

/// The **number** an object has.
fn number_of(session: &Session, id: u64) -> u64 {
    let node = session.document().node(id).expect("the node is there");
    node.data["object"].as_u64().expect("the object has a number")
}

/// A number that is **taken** is refused: two objects with one number
/// are a file that cannot say which one it means.
#[test]
fn a_number_that_is_taken_is_refused() {
    let mut session = session();
    let first = an_object(&mut session, 1, json!([["Type", "Page"]]));
    let error = session
        .execute("pdf.object.add", &json!({ "object": 1, "entries": [] }))
        .expect_err("the second object is refused");
    assert!(error.to_string().contains("is the number of the object"), "the error says why: {error}");
    assert_eq!(number_of(&session, first), 1, "and the object that has it keeps it");
}

/// The number of an object is **changed**, and the references to it
/// follow.
#[test]
fn the_number_is_changed_and_the_references_follow() {
    let mut session = session();
    let page = an_object(&mut session, 1, json!([["Type", "Page"]]));
    let tree = an_object(&mut session, 2, json!([["Type", "Pages"]]));
    session
        .execute("pdf.ref.add", &json!({ "from": tree, "to": page, "key": "Parent" }))
        .expect("the reference is made");
    session
        .execute("pdf.object.number", &json!({ "object": page, "number": 7 }))
        .expect("the number is changed");
    assert_eq!(number_of(&session, page), 7, "the object has the new number");
    let entries = entries_of(&session, tree);
    let parent = entries
        .iter()
        .find(|(key, _)| key == "Parent")
        .map(|(_, value)| value.clone())
        .expect("the key is still there");
    assert_eq!(parent, json!({ "ref": [7, 0] }), "and the reference names the new number");
}

/// A number that is **another object's** is refused.
#[test]
fn a_number_that_is_another_objects_is_refused() {
    let mut session = session();
    let first = an_object(&mut session, 1, json!([["Type", "Page"]]));
    let second = an_object(&mut session, 2, json!([["Type", "Page"]]));
    let error = session
        .execute("pdf.object.number", &json!({ "object": first, "number": 2 }))
        .expect_err("the number is refused");
    assert!(error.to_string().contains("is the number of the object"), "the error says why: {error}");
    assert_eq!(number_of(&session, first), 1, "and the object keeps its number");
    assert_eq!(number_of(&session, second), 2, "and so does the other one");
}

/// A number the object **already has** is refused: nothing changed.
#[test]
fn a_number_it_already_has_is_refused() {
    let mut session = session();
    let object = an_object(&mut session, 3, json!([["Type", "Page"]]));
    let error = session
        .execute("pdf.object.number", &json!({ "object": object, "number": 3 }))
        .expect_err("the number is refused");
    assert!(error.to_string().contains("already has the number"), "the error says why: {error}");
}

/// A number that is **not a whole number** is refused.
#[test]
fn a_number_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let object = an_object(&mut session, 1, json!([["Type", "Page"]]));
    let error = session
        .execute("pdf.object.number", &json!({ "object": object, "number": 1.5 }))
        .expect_err("the number is refused");
    assert!(error.to_string().contains("a whole number"), "the error says why: {error}");
    assert_eq!(number_of(&session, object), 1, "and the object keeps its number");
}

/// A node that is **not an object** is refused.
#[test]
fn a_node_that_is_not_an_object_is_refused() {
    let mut session = session();
    let outcome =
        session.execute("core.layer.add", &json!({ "name": "a layer" })).expect("the layer is added");
    let node = outcome.data["id"].as_u64().expect("the layer has an id");
    let error = session
        .execute("pdf.object.number", &json!({ "object": node, "number": 4 }))
        .expect_err("the number is refused");
    assert!(error.to_string().contains("not an object"), "the error says why: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("pdf.object.number", &json!({ "object": 1_000_000, "number": 4 }))
        .expect_err("the number is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
