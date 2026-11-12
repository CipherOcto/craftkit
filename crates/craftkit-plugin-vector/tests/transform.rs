//! The **transform** of a path: where the path is put.
//!
//! A path is drawn once and placed somewhere: the six numbers of a
//! matrix say where. The path in the payload is the path as it was
//! drawn, and the transform says where that path is.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_vector::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the vector battery.
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

/// A **rectangle** of ten by ten, and its id.
fn a_rectangle(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .last()
        .map(|node| node.id)
        .expect("the rectangle is there")
}

/// The **node** of a path, as the document holds it.
fn node_of(session: &Session, id: u64) -> craftkit_sdk::serde_json::Value {
    session.document().node(id).expect("the path is there").data.clone()
}

/// The **box** of a path: where it is.
fn box_of(session: &Session, id: u64) -> craftkit_sdk::Rect {
    session.document().node(id).expect("the path is there").rect.expect("the path has a box")
}

/// A path may be **placed** with a transform, and the transform is
/// kept in the payload.
#[test]
fn a_path_may_be_placed() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.transform", &json!({ "path": id, "matrix": [2.0, 0.0, 0.0, 2.0, 10.0, 5.0] }))
        .expect("the path is placed");
    let node = node_of(&session, id);
    assert_eq!(node["transform"], json!([2.0, 0.0, 0.0, 2.0, 10.0, 5.0]), "the transform is kept");
}

/// The path is **drawn** where the transform says, and its box moves
/// with it.
#[test]
fn a_placed_path_is_where_the_transform_says() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let before = box_of(&session, id);
    session
        .execute("vector.path.transform", &json!({ "path": id, "matrix": [1.0, 0.0, 0.0, 1.0, 100.0, 50.0] }))
        .expect("the path is placed");
    let after = box_of(&session, id);
    assert_eq!(after.x, before.x + 100.0, "the box moves across");
    assert_eq!(after.y, before.y + 50.0, "the box moves up");
}

/// A transform that is **not a transform** is refused by name, and so
/// is one that collapses the plane.
#[test]
fn a_transform_that_cannot_be_used_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let refused = session.execute("vector.path.transform", &json!({ "path": id, "matrix": [1.0, 0.0, 0.0] }));
    let message = match refused {
        Ok(_) => panic!("a transform of the wrong size is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("six"), "the refusal says why: {message}");
    let refused = session
        .execute("vector.path.transform", &json!({ "path": id, "matrix": [0.0, 0.0, 0.0, 0.0, 0.0, 0.0] }));
    assert!(refused.is_err(), "a transform that cannot be undone is refused");
}
