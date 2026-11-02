//! **Taking away** an object of the file.
//!
//! A file is only a file when the objects that are left still refer to
//! things that are there, so the references go with the object.

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

/// An object is **gone** when it is taken away.
#[test]
fn an_object_is_taken_away() {
    let mut session = session();
    let object = an_object(&mut session, 1, json!([["Type", "Page"]]));
    session.execute("pdf.object.remove", &json!({ "object": object })).expect("the object is taken away");
    assert!(session.document().node(object).is_none(), "the node is gone: {}", object);
}

/// The **references** to it go with it: a key that named it is gone.
#[test]
fn the_references_to_it_go_with_it() {
    let mut session = session();
    let page = an_object(&mut session, 1, json!([["Type", "Page"]]));
    let tree = an_object(&mut session, 2, json!([["Type", "Pages"]]));
    session
        .execute("pdf.ref.add", &json!({ "from": tree, "to": page, "key": "Parent" }))
        .expect("the reference is made");
    let entries = entries_of(&session, tree);
    assert_eq!(entries.len(), 2, "the reference is in the dictionary: {entries:?}");
    session.execute("pdf.object.remove", &json!({ "object": page })).expect("the page is taken away");
    let entries = entries_of(&session, tree);
    assert_eq!(entries.len(), 1, "the key that named it is gone: {entries:?}");
    assert_eq!(entries[0].0, "Type", "and the other key is left");
}

/// A key that holds a **list** of references loses the one element that
/// named the object, and keeps the rest.
#[test]
fn a_list_loses_the_one_element_that_named_it() {
    let mut session = session();
    let first = an_object(&mut session, 1, json!([["Type", "Page"]]));
    let second = an_object(&mut session, 2, json!([["Type", "Page"]]));
    let tree = an_object(&mut session, 3, json!([["Type", "Pages"]]));
    session
        .execute("pdf.ref.add", &json!({ "from": tree, "to": first, "key": "Kids", "list": true }))
        .expect("the reference is made");
    session
        .execute("pdf.ref.add", &json!({ "from": tree, "to": second, "key": "Kids", "list": true }))
        .expect("the reference is made");
    session.execute("pdf.object.remove", &json!({ "object": first })).expect("the page is taken away");
    let entries = entries_of(&session, tree);
    assert_eq!(entries.len(), 2, "the key is still there: {entries:?}");
    let kids = entries[1].1.as_array().expect("the list is there");
    assert_eq!(kids.len(), 1, "the list has the one that is left: {kids:?}");
    assert_eq!(kids[0], json!({ "ref": [2, 0] }), "and it is the other one");
}

/// The **root** of the file cannot be taken away: a file without a root
/// is not a file.
#[test]
fn the_root_of_the_file_is_refused() {
    let mut session = session();
    let root = an_object(&mut session, 1, json!([["Type", "Catalog"]]));
    let error =
        session.execute("pdf.object.remove", &json!({ "object": root })).expect_err("the removal is refused");
    assert!(error.to_string().contains("the root of the file"), "the error says why: {error}");
    assert!(session.document().node(root).is_some(), "and the root is still there");
}

/// A node that is **not an object** is refused.
#[test]
fn a_node_that_is_not_an_object_is_refused() {
    let mut session = session();
    let outcome =
        session.execute("core.layer.add", &json!({ "name": "a layer" })).expect("the layer is added");
    let node = outcome.data["id"].as_u64().expect("the layer has an id");
    let error =
        session.execute("pdf.object.remove", &json!({ "object": node })).expect_err("the removal is refused");
    assert!(error.to_string().contains("not an object"), "the error says why: {error}");
    assert!(session.document().node(node).is_some(), "and the node is still there");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("pdf.object.remove", &json!({ "object": 1_000_000 }))
        .expect_err("the removal is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
