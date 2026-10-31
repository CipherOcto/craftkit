//! A node has a **place**, and the place has a size.
//!
//! A move and a resize are two commands and not one: a move that
//! changed the size as well would be two commands wearing one name,
//! and the undo of it would take back more than the person asked for.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with one rectangle in it.
fn a_rectangle() -> Session {
    let mut session = ecosystem().session();
    session
        .execute("geometry.rectangle", &json!({ "x": 10.0, "y": 20.0, "width": 30.0, "height": 40.0 }))
        .expect("the rectangle is added");
    session
}

/// A resize changes the size and keeps the place.
#[test]
fn a_resize_changes_the_size_and_keeps_the_place() {
    let mut session = a_rectangle();
    session.execute("node.resize", &json!({ "node": 1, "w": 5.0, "h": 6.0 })).expect("the node is resized");
    let node = session.document().node(1).expect("the node is there");
    let rect = node.rect.expect("the node has a place");
    assert_eq!(rect.x, 10.0, "the node did not move across");
    assert_eq!(rect.y, 20.0, "and it did not move up");
    assert_eq!(rect.w, 5.0, "the width changed");
    assert_eq!(rect.h, 6.0, "and the height");
}

/// A node with **no place** has no size either.
///
/// The command refuses it rather than inventing one: a clip in a sound
/// document is at a time and not at a place.
#[test]
fn a_node_with_no_place_cannot_be_resized() {
    let mut session = ecosystem().session();
    session
        .execute("node.add", &json!({ "name": "a note", "kind": "core.note" }))
        .expect("the node is added");
    let error = session
        .execute("node.resize", &json!({ "node": 1, "w": 5.0, "h": 6.0 }))
        .expect_err("the resize is refused");
    assert!(error.to_string().contains("no size"), "the error says why: {error}");
}

/// A node with no area cannot be drawn and cannot be picked.
///
/// The command refuses such a size rather than making a node that is
/// nothing.
#[test]
fn a_size_of_nothing_is_refused() {
    let mut session = a_rectangle();
    let error = session
        .execute("node.resize", &json!({ "node": 1, "w": 0.0, "h": 6.0 }))
        .expect_err("the resize is refused");
    assert!(error.to_string().contains("no area"), "the error says why: {error}");
    let rect = session.document().node(1).and_then(|node| node.rect);
    assert_eq!(rect.map(|rect| rect.w), Some(30.0), "the node kept its size");
}
