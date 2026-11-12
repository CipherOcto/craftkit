//! The **average** of the points of a path.
//!
//! The tree's average puts the selected points of a path on one line,
//! in one column, or all in one place. Here the points are given, one
//! per line, because this tree has no selection.

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
        .find(|node| node.kind == "vector.path")
        .map(|node| node.id)
        .expect("the rectangle is there")
}

/// The path of a node, as the document holds it.
fn path_of(session: &Session, id: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(id).expect("the node is there");
    node.data.get("path").cloned().expect("the node has a path")
}

/// The **point** of a contour, as the document holds it.
fn point_at(session: &Session, id: u64, contour: usize, point: usize) -> (f64, f64) {
    let subpath = path_of(session, id)
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(contour))
        .cloned()
        .expect("the contour is there");
    let end = if point == 0 {
        subpath.get("start").cloned().expect("the contour has a start")
    } else {
        let segment = subpath
            .get("segments")
            .and_then(|segments| segments.get(point - 1))
            .cloned()
            .expect("the segment is there");
        segment
            .get("line")
            .and_then(|line| line.get("to"))
            .or_else(|| segment.get("cubic").and_then(|cubic| cubic.get("to")))
            .cloned()
            .expect("the segment ends at a point")
    };
    (
        end.get("x").and_then(|x| x.as_f64()).expect("the point has a width"),
        end.get("y").and_then(|y| y.as_f64()).expect("the point has a height"),
    )
}

/// The points of a rectangle: the corner, the one after it, and the
/// one across.
const THREE: &str = "0:0\n0:1\n0:2";

/// The **both** axis puts the points all in one place: the mean of
/// them.
#[test]
fn both_puts_the_points_in_one_place() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.average", &json!({ "path": id, "points": THREE, "axis": "both" }))
        .expect("the points are averaged");
    let (x, y) = point_at(&session, id, 0, 0);
    assert!((x - 20.0 / 3.0).abs() < 1e-9, "the point is at the centre: {x}");
    assert!((y - 10.0 / 3.0).abs() < 1e-9, "the point is at the centre: {y}");
    let (x, y) = point_at(&session, id, 0, 2);
    assert!((x - 20.0 / 3.0).abs() < 1e-9, "every point is at the centre: {x}");
    assert!((y - 10.0 / 3.0).abs() < 1e-9, "every point is at the centre: {y}");
}

/// The **horizontal** axis puts the points on one line: the mean of
/// their heights.
#[test]
fn horizontal_puts_the_points_on_one_line() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.average", &json!({ "path": id, "points": "0:0\n0:2", "axis": "horizontal" }))
        .expect("the points are averaged");
    let (x, y) = point_at(&session, id, 0, 0);
    assert_eq!(x, 0.0, "the width of the point is its own");
    assert_eq!(y, 5.0, "and its height is the mean of the heights");
    let (x, y) = point_at(&session, id, 0, 2);
    assert_eq!(x, 10.0, "the width of the point is its own");
    assert_eq!(y, 5.0, "and its height is the mean of the heights");
}

/// The **vertical** axis puts the points in one column: the mean of
/// their widths.
#[test]
fn vertical_puts_the_points_in_one_column() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.average", &json!({ "path": id, "points": "0:0\n0:2", "axis": "vertical" }))
        .expect("the points are averaged");
    let (x, y) = point_at(&session, id, 0, 0);
    assert_eq!(x, 5.0, "the width of the point is the mean of the widths");
    assert_eq!(y, 0.0, "and its height is its own");
}

/// The axis that is left out is **both**, as the tree's is.
#[test]
fn the_axis_that_is_left_out_is_both() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.average", &json!({ "path": id, "points": "0:0\n0:1\n0:2" }))
        .expect("the points are averaged");
    let (x, y) = point_at(&session, id, 0, 1);
    assert!((x - 20.0 / 3.0).abs() < 1e-9, "the point is at the centre: {x}");
    assert!((y - 10.0 / 3.0).abs() < 1e-9, "the point is at the centre: {y}");
}

/// The **handles** move with the point they belong to.
#[test]
fn the_handles_move_with_the_point() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.handle", &json!({ "path": id, "point": 2, "c1": { "x": 8.0, "y": 10.0 } }))
        .expect("the handle is set");
    let before = path_of(&session, id);
    let handle = before
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(0))
        .and_then(|subpath| subpath.get("segments"))
        .and_then(|segments| segments.get(2))
        .and_then(|segment| segment.get("cubic"))
        .and_then(|cubic| cubic.get("c1"))
        .cloned()
        .expect("the segment is a curve");
    assert_eq!(handle.get("y").and_then(|y| y.as_f64()), Some(10.0), "the handle is where it was put");
    session
        .execute("vector.path.average", &json!({ "path": id, "points": "0:0\n0:2", "axis": "horizontal" }))
        .expect("the points are averaged");
    let after = path_of(&session, id);
    let handle = after
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(0))
        .and_then(|subpath| subpath.get("segments"))
        .and_then(|segments| segments.get(2))
        .and_then(|segment| segment.get("cubic"))
        .and_then(|cubic| cubic.get("c1"))
        .cloned()
        .expect("the segment is a curve");
    assert_eq!(handle.get("y").and_then(|y| y.as_f64()), Some(5.0), "the handle moved with the point");
}

/// An axis that is **not** an axis is refused: the tree treats an
/// unknown axis as `both` and says nothing.
#[test]
fn an_axis_that_is_not_an_axis_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome =
        session.execute("vector.path.average", &json!({ "path": id, "points": THREE, "axis": "sideways" }));
    assert!(outcome.is_err(), "the axis is refused");
}

/// A line that is **not** a point is refused.
#[test]
fn a_line_that_is_not_a_point_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute("vector.path.average", &json!({ "path": id, "points": "0" }));
    assert!(outcome.is_err(), "the point is refused");
    let outcome = session.execute("vector.path.average", &json!({ "path": id, "points": "a:b" }));
    assert!(outcome.is_err(), "the point is refused");
}

/// A point that is **not there** is refused.
#[test]
fn a_point_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute("vector.path.average", &json!({ "path": id, "points": "0:9" }));
    assert!(outcome.is_err(), "the point is refused");
}

/// **No points** is refused: the tree does nothing at all, and this
/// tree says that there is nothing to do.
#[test]
fn no_points_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute("vector.path.average", &json!({ "path": id, "points": "" }));
    assert!(outcome.is_err(), "there is nothing to average");
}

/// The same point **twice** is refused: it would be averaged twice.
#[test]
fn the_same_point_twice_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute("vector.path.average", &json!({ "path": id, "points": "0:0\n0:0" }));
    assert!(outcome.is_err(), "the point is in the list twice");
}
