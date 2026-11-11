//! The **moving** of many points at once.
//!
//! The tree's rules: the points are moved by a delta, every point
//! named must be there, and the handles move with the point they belong
//! to.

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
fn sides(session: &Session, id: u64) -> Vec<Json> {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"][0]["segments"].as_array().expect("the path has sides").clone()
}

/// The **handles** of a side, when it is a curve.
fn handles(side: &Json) -> Option<((f64, f64), (f64, f64))> {
    let cubic = side.get("cubic")?;
    let read = |key: &str| {
        let point = cubic.get(key)?;
        Some((point.get("x")?.as_f64()?, point.get("y")?.as_f64()?))
    };
    Some((read("c1")?, read("c2")?))
}

/// A **path** of three points, and its id.
fn a_path(session: &mut Session) -> u64 {
    session
        .execute(
            "vector.path.create",
            &json!({
                "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }]
            }),
        )
        .expect("the path is added");
    paths(session)[0]
}

/// The points are moved by the **delta**.
#[test]
fn the_points_are_moved_by_the_delta() {
    let mut session = session();
    let id = a_path(&mut session);
    session
        .execute("vector.path.move.points", &json!({ "path": id, "points": [1], "dx": 5.0, "dy": -3.0 }))
        .expect("the points are moved");
    let sides = sides(&session, id);
    let end = sides[0].get("line").and_then(|line| line.get("to")).expect("the side has an end");
    let x = end.get("x").and_then(|x| x.as_f64()).expect("the end is across");
    let y = end.get("y").and_then(|y| y.as_f64()).expect("the end is down");
    assert!((x - 15.0).abs() < 1e-6, "the point moved across: {x}");
    assert!((y + 3.0).abs() < 1e-6, "the point moved down: {y}");
}

/// **Many** points move in one go.
#[test]
fn many_points_move_in_one_go() {
    let mut session = session();
    let id = a_path(&mut session);
    let outcome = session
        .execute("vector.path.move.points", &json!({ "path": id, "points": [0, 2], "dx": 1.0, "dy": 1.0 }))
        .expect("the points are moved");
    let moved: Vec<u64> = outcome
        .data
        .get("points")
        .and_then(|list| list.as_array())
        .map(|list| list.iter().filter_map(|point| point.as_u64()).collect())
        .unwrap_or_default();
    assert_eq!(moved, vec![0, 2], "both points are moved: {moved:?}");
    let node = session.document().node(id).expect("the path is there");
    let start = node.data["path"]["subpaths"][0]["start"].as_object().expect("the contour starts");
    let x = start.get("x").and_then(|x| x.as_f64()).expect("the start is across");
    let y = start.get("y").and_then(|y| y.as_f64()).expect("the start is down");
    assert!((x - 1.0).abs() < 1e-6, "the first point moved: {x}");
    assert!((y - 1.0).abs() < 1e-6, "the first point moved: {y}");
}

/// The **handles** move with the point they belong to.
#[test]
fn the_handles_move_with_the_point() {
    let mut session = session();
    let id = a_path(&mut session);
    session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1], "to": "smooth" }))
        .expect("the point is converted");
    let before = handles(&sides(&session, id)[0]).expect("the side is a curve");
    session
        .execute("vector.path.move.points", &json!({ "path": id, "points": [1], "dx": 5.0, "dy": 0.0 }))
        .expect("the points are moved");
    let after = handles(&sides(&session, id)[0]).expect("the side is a curve");
    assert!((after.1.0 - (before.1.0 + 5.0)).abs() < 1e-6, "the handle moved with it: {after:?}");
    assert!((after.1.1 - before.1.1).abs() < 1e-6, "the handle kept its height: {after:?}");
}

/// A point that is **not there** is refused.
#[test]
fn a_point_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_path(&mut session);
    let before = sides(&session, id);
    let outcome = session
        .execute("vector.path.move.points", &json!({ "path": id, "points": [9], "dx": 1.0, "dy": 0.0 }));
    assert!(outcome.is_err(), "the point that is not there is refused");
    assert_eq!(sides(&session, id), before, "the path is as it was");
}

/// A move that is **not a number** is refused.
#[test]
fn a_move_that_is_not_a_number_is_refused() {
    let mut session = session();
    let id = a_path(&mut session);
    let outcome = session
        .execute("vector.path.move.points", &json!({ "path": id, "points": [1], "dx": "there", "dy": 0.0 }));
    assert!(outcome.is_err(), "the move that is not a number is refused");
}

/// The points that are **not a list** are refused.
#[test]
fn points_that_are_not_a_list_are_refused() {
    let mut session = session();
    let id = a_path(&mut session);
    let outcome = session
        .execute("vector.path.move.points", &json!({ "path": id, "points": "here", "dx": 1.0, "dy": 0.0 }));
    assert!(outcome.is_err(), "the points that are not a list are refused");
}
