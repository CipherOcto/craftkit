//! The **finish** of a path's line: how its corners meet and how its
//! ends are finished.
//!
//! A line has a way of turning and a way of stopping, and the tree's
//! stroke carries both. v2's stroke carried a colour and a width and
//! nothing else, so every line in the tree had the same corners and
//! the same ends.

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

/// The paint of a node, as the document holds it.
fn paint_of(session: &Session, id: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(id).expect("the node is there");
    node.data.clone()
}

/// The **corners** of a path's line may be set.
#[test]
fn the_corners_of_a_line_may_be_set() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.stroke", &json!({ "path": id, "colour": "#000000", "width": 2.0 }))
        .expect("the path has a line");
    session
        .execute("vector.path.join", &json!({ "path": id, "join": "round" }))
        .expect("the corners are set");
    let paint = paint_of(&session, id);
    let join = paint.get("stroke").and_then(|stroke| stroke.get("join")).and_then(|join| join.as_str());
    assert_eq!(join, Some("round"), "the line's corners are round");
}

/// The **ends** of a path's line may be set.
#[test]
fn the_ends_of_a_line_may_be_set() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.stroke", &json!({ "path": id, "colour": "#000000", "width": 2.0 }))
        .expect("the path has a line");
    session.execute("vector.path.cap", &json!({ "path": id, "cap": "square" })).expect("the ends are set");
    let paint = paint_of(&session, id);
    let cap = paint.get("stroke").and_then(|stroke| stroke.get("cap")).and_then(|cap| cap.as_str());
    assert_eq!(cap, Some("square"), "the line's ends are square");
}

/// A name that is **not one of the three** is refused by name, and so
/// is a path that has no line at all.
#[test]
fn a_finish_that_is_not_one_of_the_three_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.stroke", &json!({ "path": id, "colour": "#000000", "width": 2.0 }))
        .expect("the path has a line");
    let refused = session.execute("vector.path.join", &json!({ "path": id, "join": "sharp" }));
    let message = match refused {
        Ok(_) => panic!("a join that is not one of the three is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("sharp"), "the refusal says why: {message}");
    let refused = session.execute("vector.path.cap", &json!({ "path": id, "cap": "flat" }));
    assert!(refused.is_err(), "a cap that is not one of the three is refused");
    // A path with no line has no corners and no ends.
    let bare = a_rectangle(&mut session);
    let refused = session.execute("vector.path.join", &json!({ "path": bare, "join": "round" }));
    assert!(refused.is_err(), "a path with no line has no corners to set");
}
