//! The **line cut**: a shape split along a line.
//!
//! The tree's rules: the line goes on for ever, through two points
//! that must differ; each side of the line keeps the part of the shape
//! that is on it; a line that misses the shape changes nothing; and a
//! hole travels with the shape it is in.

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

/// The paths in the document, in the order they are in it.
fn paths(session: &Session) -> Vec<u64> {
    session.document().walk().filter(|node| node.kind == "vector.path").map(|node| node.id).collect()
}

/// The **contours** of the path, as they are in the document.
fn contours(session: &Session, id: u64) -> Vec<Json> {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"].as_array().expect("the path has contours").clone()
}

/// The **ids** the outcome says it made.
fn ids_of(outcome: &craftkit_sdk::Outcome) -> Vec<u64> {
    outcome
        .data
        .get("ids")
        .and_then(|ids| ids.as_array())
        .map(|ids| ids.iter().filter_map(|id| id.as_u64()).collect())
        .unwrap_or_default()
}

/// A **rectangle** twenty by twenty, and its id.
fn a_square(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 20.0, "height": 20.0 }))
        .expect("the rectangle is added");
    paths(session)[0]
}

/// A shape is **cut in two** by a line that crosses it.
#[test]
fn a_shape_is_cut_in_two_by_a_line() {
    let mut session = session();
    let id = a_square(&mut session);
    session
        .execute("vector.path.cut.line", &json!({ "path": id, "from": [10.0, -5.0], "to": [10.0, 25.0] }))
        .expect("the shape is cut");
    assert_eq!(paths(&session).len(), 2, "the square is two paths now");
}

/// The line goes on **for ever**: two points close together are a line
/// and not a cut of a piece of it.
#[test]
fn the_line_goes_on_for_ever() {
    let mut session = session();
    let id = a_square(&mut session);
    session
        .execute("vector.path.cut.line", &json!({ "path": id, "from": [10.0, 0.0], "to": [10.0, 1.0] }))
        .expect("the shape is cut");
    assert_eq!(paths(&session).len(), 2, "the square is two paths now");
    let ids = paths(&session);
    let left = contours(&session, ids[0]);
    let right = contours(&session, ids[1]);
    assert_eq!(left.len(), 1, "the first piece is one shape");
    assert_eq!(right.len(), 1, "the second piece is one shape");
}

/// The pieces are **nodes of their own**, and the first keeps the node
/// that was cut.
#[test]
fn the_pieces_are_nodes_of_their_own() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session
        .execute("vector.path.cut.line", &json!({ "path": id, "from": [10.0, -5.0], "to": [10.0, 25.0] }))
        .expect("the shape is cut");
    let ids = ids_of(&outcome);
    assert_eq!(ids.len(), 2, "the cut says what it made: {ids:?}");
    assert_eq!(ids[0], id, "the first piece keeps the node");
    assert_ne!(ids[1], id, "the second piece is a new node");
}

/// A line that **misses** the shape leaves it as it was.
#[test]
fn a_line_that_misses_changes_nothing() {
    let mut session = session();
    let id = a_square(&mut session);
    let before = contours(&session, id);
    let outcome = session
        .execute("vector.path.cut.line", &json!({ "path": id, "from": [50.0, -5.0], "to": [50.0, 25.0] }))
        .expect("the cut runs");
    assert!(!outcome.changed, "nothing was cut: {outcome:?}");
    assert_eq!(contours(&session, id), before, "the shape is as it was");
    assert_eq!(paths(&session).len(), 1, "there is still one path");
}

/// Two points that are the **same** are refused: there is no line
/// through them.
#[test]
fn two_points_that_are_the_same_are_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome =
        session.execute("vector.path.cut.line", &json!({ "path": id, "from": [5.0, 5.0], "to": [5.0, 5.0] }));
    assert!(outcome.is_err(), "the line with two equal points is refused");
}

/// A point that is **not a point** is refused.
#[test]
fn a_point_that_is_not_a_point_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome =
        session.execute("vector.path.cut.line", &json!({ "path": id, "from": "there", "to": [5.0, 5.0] }));
    assert!(outcome.is_err(), "the point that is not a point is refused");
}

/// An **open** path is refused: the line cuts closed shapes.
#[test]
fn an_open_path_is_refused() {
    let mut session = session();
    session
        .execute(
            "vector.path.create",
            &json!({ "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 20.0, "y": 0.0 }] }),
        )
        .expect("the path is added");
    let id = paths(&session)[0];
    let outcome = session
        .execute("vector.path.cut.line", &json!({ "path": id, "from": [10.0, -5.0], "to": [10.0, 5.0] }));
    assert!(outcome.is_err(), "the open path is refused");
}

/// A **hole** travels with the shape it is in: the piece that keeps the
/// hole is one path and not two.
#[test]
fn a_hole_travels_with_its_shape() {
    let mut session = session();
    a_square(&mut session);
    let outer = paths(&session)[0];
    session
        .execute("vector.rectangle", &json!({ "x": 8.0, "y": 8.0, "width": 4.0, "height": 4.0 }))
        .expect("the hole is added");
    let inner = paths(&session)[1];
    session
        .execute("vector.path.subtract", &json!({ "first": outer, "second": inner }))
        .expect("the hole is taken out");
    assert_eq!(contours(&session, outer).len(), 2, "the ring has an outer contour and a hole");
    session
        .execute("vector.path.cut.line", &json!({ "path": outer, "from": [3.0, -5.0], "to": [3.0, 25.0] }))
        .expect("the ring is cut");
    assert_eq!(paths(&session).len(), 2, "the ring is two paths now");
    let ids = paths(&session);
    let kept = contours(&session, ids[1]);
    assert_eq!(kept.len(), 2, "the piece that keeps the hole keeps it whole: {kept:?}");
}
