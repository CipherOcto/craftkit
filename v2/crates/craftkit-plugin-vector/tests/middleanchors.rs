//! The **point in the middle of every side** of a path.
//!
//! The tree's rule: an anchor is added at the middle of each side, so
//! that the path may be edited in finer pieces — and the sides keep
//! their shape.

use craftkit_sdk::serde_json::Value as Json;
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

/// The **sides** of the first contour of the path.
fn sides_of(session: &Session, id: u64) -> Vec<Json> {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"][0]["segments"].as_array().expect("the path has sides").clone()
}

/// Where a side ends.
fn end_of(side: &Json) -> (f64, f64) {
    let end = side
        .get("line")
        .and_then(|line| line.get("to"))
        .or_else(|| side.get("cubic").and_then(|cubic| cubic.get("to")))
        .expect("the side has an end");
    (
        end.get("x").and_then(|x| x.as_f64()).expect("the end is across"),
        end.get("y").and_then(|y| y.as_f64()).expect("the end is down"),
    )
}

/// A **closed path**: a rectangle, and its id.
fn a_rectangle(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "vector.path")
        .map(|node| node.id)
        .expect("the rectangle is there")
}

/// Every side of a path gets a point in its **middle**.
#[test]
fn every_side_gets_a_point_in_its_middle() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let before = sides_of(&session, id);
    assert_eq!(before.len(), 3, "the rectangle has three sides and the edge that closes it");
    session.execute("vector.path.anchors.middle", &json!({ "path": id })).expect("the points are added");
    let after = sides_of(&session, id);
    assert_eq!(after.len(), 8, "four sides, each in two: {after:?}");
}

/// The point is in the **middle** of the side.
#[test]
fn the_point_is_in_the_middle() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session.execute("vector.path.anchors.middle", &json!({ "path": id })).expect("the points are added");
    let sides = sides_of(&session, id);
    assert_eq!(end_of(&sides[0]), (5.0, 0.0), "the first half of the first side");
    assert_eq!(end_of(&sides[1]), (10.0, 0.0), "the second half of the first side");
}

/// The shape does **not** change: the path is in the same place.
#[test]
fn the_shape_does_not_change() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let before = session.document().node(id).and_then(|node| node.rect);
    session.execute("vector.path.anchors.middle", &json!({ "path": id })).expect("the points are added");
    let after = session.document().node(id).and_then(|node| node.rect);
    assert_eq!(before, after, "the path is in the same place: {before:?} and {after:?}");
}

/// An **open path** gets a point in every side too.
#[test]
fn an_open_path_gets_a_point_in_every_side() {
    let mut session = session();
    session
        .execute(
            "vector.path.create",
            &json!({
                "anchors": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 10.0, "y": 0.0 },
                    { "x": 10.0, "y": 10.0 }
                ]
            }),
        )
        .expect("the path is added");
    let id = session
        .document()
        .walk()
        .find(|node| node.kind == "vector.path")
        .map(|node| node.id)
        .expect("the path is there");
    session.execute("vector.path.anchors.middle", &json!({ "path": id })).expect("the points are added");
    let sides = sides_of(&session, id);
    assert_eq!(sides.len(), 4, "two sides, each in two: {sides:?}");
    assert_eq!(end_of(&sides[1]), (10.0, 0.0), "the first half of the first side");
}
