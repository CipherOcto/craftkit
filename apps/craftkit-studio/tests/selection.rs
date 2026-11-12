//! The **selection** moves as one.
//!
//! A person who picks twenty nodes and moves them expects one undo,
//! and not twenty. So the command that moves the selection takes the
//! selection — the session's state — and moves every node in it that
//! has a place, in one step.

use craftkit_sdk::Ecosystem;
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with two rectangles in it, both selected.
fn two_selected() -> craftkit_sdk::Session {
    let mut session = ecosystem().session();
    for name in ["one", "two"] {
        session
            .execute(
                "geometry.rectangle",
                &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0, "name": name }),
            )
            .expect("the rectangle is added");
    }
    session.execute("select.all", &json!({})).expect("everything is selected");
    session
}

/// The whole selection **moves**.
#[test]
fn the_selection_moves() {
    let mut session = two_selected();
    session.execute("select.move", &json!({ "x": 5.0, "y": -3.0 })).expect("the selection moves");

    for id in [1, 2] {
        let rect = session.document().node(id).and_then(|node| node.rect).expect("the node has a place");
        assert_eq!(rect.x, 5.0, "the node moved across");
        assert_eq!(rect.y, -3.0, "and up");
    }
}

/// And it is **one** undo step.
///
/// A person who moves twenty nodes and undoes it expects the twenty
/// nodes to come back, and not twenty undos.
#[test]
fn the_selection_moves_in_one_step() {
    let mut session = two_selected();
    let before = session.history().len();
    session.execute("select.move", &json!({ "x": 5.0, "y": 0.0 })).expect("the selection moves");
    assert_eq!(session.history().len(), before + 1, "one command is one step");

    session.undo().expect("the step is undone");
    for id in [1, 2] {
        let rect = session.document().node(id).and_then(|node| node.rect).expect("the node has a place");
        assert_eq!(rect.x, 0.0, "the node came back");
    }
}

/// Nothing selected is **refused**.
///
/// A command that did nothing and said it had moved something would be
/// a command a person cannot trust.
#[test]
fn nothing_selected_is_refused() {
    let mut session = ecosystem().session();
    let refused =
        session.execute("select.move", &json!({ "x": 5.0, "y": 0.0 })).expect_err("the command refuses");
    assert!(refused.to_string().contains("nothing is selected"), "the error says why: {refused}");
}

/// A node with **no place** is left alone.
///
/// A clip in a sound document is at a time and not at a place, and the
/// command does not invent one.
#[test]
fn a_node_with_no_place_is_left_alone() {
    let mut session = ecosystem().session();
    session
        .execute("node.add", &json!({ "name": "a note", "kind": "core.note" }))
        .expect("the node is added");
    session.execute("select.all", &json!({})).expect("everything is selected");

    let outcome =
        session.execute("select.move", &json!({ "x": 5.0, "y": 0.0 })).expect("the selection moves");
    assert_eq!(outcome.data.get("without_place").and_then(|count| count.as_u64()), Some(1));
    assert_eq!(outcome.data.get("moved").and_then(|count| count.as_u64()), Some(0));
    assert!(session.document().node(1).and_then(|node| node.rect).is_none(), "the node still has no place");
}
