//! The document can be **emptied**, and nothing is left dangling.
//!
//! A node that is deleted takes its **edges** with it: an edge to a
//! node that is not there is an edge to nothing. And the undo brings
//! it back, edges and all.

use craftkit_sdk::Ecosystem;
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with two nodes and an edge between them.
fn two_with_an_edge() -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    session
        .execute("node.add", &json!({ "name": "one", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
        .execute("node.add", &json!({ "name": "two", "kind": "geometry.rect" }))
        .expect("the node is added");
    session
        .execute("relation.add", &json!({ "from": 1, "to": 2, "kind": "test.link", "payload": { "a": 1 } }))
        .expect("the edge is added");
    session
}

/// The edge goes with the node.
///
/// An edge to a node that is not there is an edge to nothing, and a
/// document that keeps them says a clip plays a source that is gone.
#[test]
fn the_edge_goes_with_the_node() {
    let mut session = two_with_an_edge();
    assert_eq!(session.document().relations.len(), 1, "there is an edge");

    session.execute("node.delete", &json!({ "node": 1 })).expect("the node is deleted");

    assert!(session.document().node(1).is_none(), "the node is gone");
    assert_eq!(session.document().relations.len(), 0, "and the edge is gone too");
}

/// And the undo brings it **back**, edges and all.
///
/// The history keeps the document as it was, and an edge is part of the
/// document.
#[test]
fn the_undo_brings_the_edge_back() {
    let mut session = two_with_an_edge();
    session.execute("node.delete", &json!({ "node": 1 })).expect("the node is deleted");
    session.undo().expect("the deletion is undone");

    assert!(session.document().node(1).is_some(), "the node is back");
    assert_eq!(session.document().relations.len(), 1, "and so is the edge");
}

/// The **selection** is deleted in one step.
///
/// A person who deletes twenty nodes and undoes it expects the twenty
/// nodes to come back, and not twenty undos.
#[test]
fn the_selection_is_deleted_in_one_step() {
    let mut session = two_with_an_edge();
    session.execute("select.all", &json!({})).expect("everything is selected");
    let before = session.history().len();

    session.execute("select.delete", &json!({})).expect("the selection is deleted");
    assert_eq!(session.history().len(), before + 1, "one command is one step");
    assert_eq!(session.document().nodes.len(), 0, "the document is empty");
    assert_eq!(session.document().relations.len(), 0, "and there is no edge to nothing");

    session.undo().expect("the deletion is undone");
    assert_eq!(session.document().nodes.len(), 2, "the nodes came back");
    assert_eq!(session.document().relations.len(), 1, "and so did the edge");
}

/// Nothing selected is **refused**.
#[test]
fn nothing_selected_is_refused() {
    let mut session = ecosystem().session();
    let refused = session.execute("select.delete", &json!({})).expect_err("the command refuses");
    assert!(refused.to_string().contains("nothing is selected"), "the error says why: {refused}");
}

/// A node that is not there is **refused**.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = ecosystem().session();
    let refused = session.execute("node.delete", &json!({ "node": 9 })).expect_err("the command refuses");
    assert!(refused.to_string().contains("no node"), "the error says why: {refused}");
}

/// An edge to a node that is **gone** is not made by the kit.
///
/// The relation is a piece of the document, and the document is what
/// the commands write.
#[test]
fn an_edge_to_a_node_that_is_gone_is_refused() {
    let mut session = ecosystem().session();
    session
        .execute("node.add", &json!({ "name": "one", "kind": "geometry.rect" }))
        .expect("the node is added");
    let refused = session
        .execute("relation.add", &json!({ "from": 1, "to": 9, "kind": "test.link" }))
        .expect_err("the edge is refused");
    assert!(refused.to_string().contains("no node"), "the error says why: {refused}");
}
