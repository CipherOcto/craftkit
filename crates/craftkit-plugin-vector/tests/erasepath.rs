//! The **erasing** of the parts of a path under a drag.
//!
//! The tree's rule: the parts of a path within half the width of the
//! drag are taken away, and what is left is the stretches that
//! survive — each a stretch of the path as it was.

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

/// Where a side starts.
fn start_of(session: &Session, id: u64) -> (f64, f64) {
    let node = session.document().node(id).expect("the path is there");
    let start = node.data["path"]["subpaths"][0]["start"].as_object().expect("the contour starts");
    (
        start.get("x").and_then(|x| x.as_f64()).expect("the start is across"),
        start.get("y").and_then(|y| y.as_f64()).expect("the start is down"),
    )
}

/// An **open path** of one straight side from the origin to twenty
/// across, and its id.
fn a_line(session: &mut Session) -> u64 {
    session
        .execute(
            "vector.path.create",
            &json!({ "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 20.0, "y": 0.0 }] }),
        )
        .expect("the path is added");
    paths(session)[0]
}

/// The parts of a path **under a drag** are taken away.
#[test]
fn the_parts_under_the_drag_are_taken_away() {
    let mut session = session();
    let id = a_line(&mut session);
    session
        .execute(
            "vector.path.erase",
            &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 5.0]], "width": 4.0 }),
        )
        .expect("the path is erased");
    assert_eq!(paths(&session).len(), 2, "the path is two paths now");
    let sides = sides_of(&session, id);
    assert_eq!(sides.len(), 1, "the first piece is one side");
    let end = end_of(&sides[0]);
    assert!((end.0 - 8.0).abs() < 1e-6, "the piece ends where the eraser began: {end:?}");
}

/// The pieces are **paths of their own**.
#[test]
fn the_pieces_are_paths_of_their_own() {
    let mut session = session();
    let id = a_line(&mut session);
    let outcome = session
        .execute(
            "vector.path.erase",
            &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 5.0]], "width": 4.0 }),
        )
        .expect("the path is erased");
    let ids: Vec<u64> = outcome
        .data
        .get("ids")
        .and_then(|ids| ids.as_array())
        .map(|ids| ids.iter().filter_map(|id| id.as_u64()).collect())
        .unwrap_or_default();
    assert_eq!(ids.len(), 2, "the eraser says what it made: {ids:?}");
    assert_eq!(ids[0], id, "the first piece keeps the node");
    let other = ids[1];
    let start = start_of(&session, other);
    assert!((start.0 - 12.0).abs() < 1e-6, "the second piece begins where the eraser ended: {start:?}");
}

/// A path with **nothing under the drag** is left as it was.
#[test]
fn a_path_with_nothing_under_the_drag_is_left_alone() {
    let mut session = session();
    let id = a_line(&mut session);
    let before = sides_of(&session, id);
    let outcome = session
        .execute(
            "vector.path.erase",
            &json!({ "path": id, "points": [[10.0, 20.0], [10.0, 30.0]], "width": 4.0 }),
        )
        .expect("the eraser runs");
    assert!(!outcome.changed, "nothing was erased: {outcome:?}");
    assert_eq!(sides_of(&session, id), before, "the path is as it was");
}

/// A **closed** path is erased too, and what is left are the stretches
/// that survive.
#[test]
fn a_closed_path_is_erased_too() {
    let mut session = session();
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 20.0, "height": 20.0 }))
        .expect("the rectangle is added");
    let id = paths(&session)[0];
    session
        .execute(
            "vector.path.erase",
            &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 25.0]], "width": 4.0 }),
        )
        .expect("the path is erased");
    assert_eq!(paths(&session).len(), 2, "the rectangle is two pieces now");
}

/// A **width** that is not a positive number is refused.
#[test]
fn a_width_that_is_not_positive_is_refused() {
    let mut session = session();
    let id = a_line(&mut session);
    let outcome = session.execute(
        "vector.path.erase",
        &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 5.0]], "width": 0.0 }),
    );
    assert!(outcome.is_err(), "the width that is not positive is refused");
}

/// A drag with **no points** is refused.
#[test]
fn a_drag_with_no_points_is_refused() {
    let mut session = session();
    let id = a_line(&mut session);
    let outcome = session.execute("vector.path.erase", &json!({ "path": id, "points": [] }));
    assert!(outcome.is_err(), "the drag with no points is refused");
}
