//! A **locked** node is in the document and cannot be changed.
//!
//! Locking a **layer** locks everything on it, because the layer is
//! the thing that is locked. It is not the same as hiding: a hidden
//! node cannot be seen and a locked node cannot be touched.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, Ecosystem};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with one rectangle in it.
fn a_rectangle() -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    session
        .execute("geometry.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
}

/// A locked node cannot be **moved**.
#[test]
fn a_locked_node_cannot_be_moved() {
    let mut session = a_rectangle();
    session.execute("node.lock", &json!({ "node": 1 })).expect("the node is locked");

    let refused = session
        .execute("node.move", &json!({ "node": 1, "x": 5.0, "y": 5.0 }))
        .expect_err("the move is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");

    let session_view = session.document();
    let rect = session_view.node(1).and_then(|node| node.rect).expect("the node has a place");
    assert_eq!(rect.x, 0.0, "the node did not move");
}

/// Nor can it be **resized**, and nor can it be **deleted**.
#[test]
fn a_locked_node_cannot_be_resized_or_deleted() {
    let mut session = a_rectangle();
    session.execute("node.lock", &json!({ "node": 1 })).expect("the node is locked");

    let refused = session
        .execute("node.resize", &json!({ "node": 1, "w": 5.0, "h": 5.0 }))
        .expect_err("the resize is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");

    let refused = session.execute("node.delete", &json!({ "node": 1 })).expect_err("the delete is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
    assert!(session.document().node(1).is_some(), "the node is still there");
}

/// Unlocking gives it back.
#[test]
fn unlocking_gives_it_back() {
    let mut session = a_rectangle();
    session.execute("node.lock", &json!({ "node": 1 })).expect("the node is locked");
    session.execute("node.unlock", &json!({ "node": 1 })).expect("the node is unlocked");
    session.execute("node.move", &json!({ "node": 1, "x": 5.0, "y": 5.0 })).expect("the node moves");
    let session_view = session.document();
    let rect = session_view.node(1).and_then(|node| node.rect).expect("the node has a place");
    assert_eq!(rect.x, 5.0, "the node moved");
}

/// A locked **layer** locks what is on it.
///
/// The layer is the thing that is locked, and what is on it is behind
/// it.
#[test]
fn a_locked_layer_locks_what_is_on_it() {
    let mut session = ecosystem().session();
    let layer = session
        .execute("core.layer.add", &json!({ "name": "Walls" }))
        .expect("the layer is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    let shape = session
        .execute("geometry.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    session.execute("core.layer.on", &json!({ "node": shape, "layer": layer })).expect("it is on the layer");

    session.execute("node.lock", &json!({ "node": layer })).expect("the layer is locked");
    let refused = session
        .execute("node.move", &json!({ "node": shape, "x": 5.0, "y": 5.0 }))
        .expect_err("the move is refused");
    assert!(refused.to_string().contains("locked"), "the error says why: {refused}");
}

/// The flag is the **tenth** version's field, and the version is
/// derived from the document.
#[test]
fn the_flag_is_the_tenth_versions_field() {
    let ecosystem = ecosystem();
    let mut document = Document::new("locked");
    let id = document.add_node("rect", "geometry.rect", None);
    document.node_mut(id).expect("the node is there").locked = true;

    let (written, version) = craftkit_sdk::format::write_document_oldest(
        &document,
        &ecosystem.formats,
        &ecosystem.migrations,
        "craftkit",
    )
    .expect("the document is written");
    assert_eq!(version, 10, "a locked node needs the tenth version");
    assert_eq!(written["nodes"][0]["locked"], json!(true), "and the flag is in the file");

    // And the ninth version does not carry it: a file of a version must
    // not have a field that version cannot explain.
    let ninth = ecosystem
        .formats
        .all()
        .iter()
        .find(|format| format.version == 9)
        .expect("the ninth version is there");
    let written = craftkit_sdk::format::write_document(&document, ninth);
    assert!(written["nodes"][0].get("locked").is_none(), "the ninth version does not know the flag");
}
