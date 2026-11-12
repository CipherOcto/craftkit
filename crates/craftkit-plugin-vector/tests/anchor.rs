//! The **anchor** in the middle of a segment.
//!
//! The tree's editor adds an anchor point on a segment: the segment is
//! split in two, and the two halves are the same curve.

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

/// A session with a **line** on it: two points and one segment.
fn a_line(session: &mut Session) -> u64 {
    let outcome = session
        .execute("vector.line", &json!({ "x1": 0.0, "y1": 0.0, "x2": 10.0, "y2": 0.0 }))
        .expect("the line is added");
    outcome.data["id"].as_u64().expect("the path has an id")
}

/// The **points** of the path's contour.
fn points_of(session: &Session) -> Vec<(f64, f64)> {
    let node = session.document().walk().find(|node| node.kind == "vector.path").expect("the path is there");
    let Some(subpaths) =
        node.data.get("path").and_then(|path| path.get("subpaths")).and_then(|s| s.as_array())
    else {
        return Vec::new();
    };
    let Some(subpath) = subpaths.first() else { return Vec::new() };
    let mut points = Vec::new();
    if let Some(start) = subpath.get("start")
        && let (Some(x), Some(y)) =
            (start.get("x").and_then(|x| x.as_f64()), start.get("y").and_then(|y| y.as_f64()))
    {
        points.push((x, y));
    }
    if let Some(segments) = subpath.get("segments").and_then(|segments| segments.as_array()) {
        for segment in segments {
            let end = segment
                .get("line")
                .and_then(|line| line.get("to"))
                .or_else(|| segment.get("cubic").and_then(|cubic| cubic.get("to")));
            if let Some(end) = end
                && let (Some(x), Some(y)) =
                    (end.get("x").and_then(|x| x.as_f64()), end.get("y").and_then(|y| y.as_f64()))
            {
                points.push((x, y));
            }
        }
    }
    points
}

/// An anchor is **added** to a segment, and the path has one more
/// point.
#[test]
fn an_anchor_is_added_to_a_segment() {
    let mut session = session();
    let node = a_line(&mut session);
    assert_eq!(points_of(&session).len(), 2, "the line has two points");
    session
        .execute("vector.path.anchor.add", &json!({ "node": node, "segment": 0, "t": 0.5 }))
        .expect("the anchor is added");
    let points = points_of(&session);
    assert_eq!(points.len(), 3, "and now it has three: {points:?}");
    assert!((points[1].0 - 5.0).abs() < 1e-12, "the anchor is halfway along: {points:?}");
}

/// The **shape** of the path does not change: the points are the same
/// ones the line had, with one more in the middle.
#[test]
fn the_shape_of_the_path_does_not_change() {
    let mut session = session();
    let node = a_line(&mut session);
    session
        .execute("vector.path.anchor.add", &json!({ "node": node, "segment": 0, "t": 0.25 }))
        .expect("the anchor is added");
    let points = points_of(&session);
    assert_eq!(points[0], (0.0, 0.0), "the line starts where it did");
    assert_eq!(points[2], (10.0, 0.0), "and ends where it did");
}

/// A segment that does **not** exist is refused.
#[test]
fn a_segment_that_is_not_there_is_refused() {
    let mut session = session();
    let node = a_line(&mut session);
    let error = session
        .execute("vector.path.anchor.add", &json!({ "node": node, "segment": 9, "t": 0.5 }))
        .expect_err("the anchor is refused");
    assert!(error.to_string().contains("no segment 9"), "the error says why: {error}");
    assert_eq!(points_of(&session).len(), 2, "and the path is as it was");
}
