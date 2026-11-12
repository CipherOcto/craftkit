//! The **closing** and the **turning** of a path.
//!
//! The tree's editor closes a path's open contour and turns a path the
//! other way round.

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

/// A **line** in the document, with the number of points it has and
/// whether it is closed.
fn path_of(session: &Session) -> (usize, bool, Vec<(f64, f64)>) {
    let node = session.document().walk().find(|node| node.kind == "vector.path").expect("the path is there");
    let subpaths = node.data.get("path").and_then(|path| path.get("subpaths")).and_then(|s| s.as_array());
    let Some(subpaths) = subpaths else { return (0, false, Vec::new()) };
    let Some(subpath) = subpaths.first() else { return (0, false, Vec::new()) };
    let closed = subpath.get("closed").and_then(|closed| closed.as_bool()).unwrap_or(false);
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
    (points.len(), closed, points)
}

/// A session with a **line** on it: two points and an open contour.
fn a_line(session: &mut Session) -> u64 {
    let outcome = session
        .execute("vector.line", &json!({ "x1": 0.0, "y1": 0.0, "x2": 10.0, "y2": 0.0 }))
        .expect("the line is added");
    outcome.data["id"].as_u64().expect("the path has an id")
}

/// A path's open contour is **closed**.
#[test]
fn an_open_contour_is_closed() {
    let mut session = session();
    let node = a_line(&mut session);
    let (_, closed, _) = path_of(&session);
    assert!(!closed, "the contour is open to begin with");
    session.execute("vector.path.close", &json!({ "node": node })).expect("the path is closed");
    let (_, closed, points) = path_of(&session);
    assert!(closed, "and now it is closed");
    assert_eq!(points.len(), 2, "the points are the same: closing is not a point");
}

/// A path with **no open contour** is refused: there is nothing to
/// close.
#[test]
fn a_path_with_no_open_contour_is_refused() {
    let mut session = session();
    let node = a_line(&mut session);
    session.execute("vector.path.close", &json!({ "node": node })).expect("the path is closed");
    let error =
        session.execute("vector.path.close", &json!({ "node": node })).expect_err("the closing is refused");
    assert!(error.to_string().contains("no open contour"), "the error says why: {error}");
}

/// A path is **turned** the other way round: it starts where it ended.
#[test]
fn a_path_is_turned_the_other_way_round() {
    let mut session = session();
    let node = a_line(&mut session);
    let (_, _, before) = path_of(&session);
    session.execute("vector.path.reverse", &json!({ "node": node })).expect("the path is turned");
    let (_, _, after) = path_of(&session);
    assert_eq!(after.first().copied(), before.last().copied(), "it starts where it ended: {after:?}");
    assert_eq!(after.last().copied(), before.first().copied(), "and ends where it began");
    assert_eq!(after.len(), before.len(), "and it has the same points");
}

/// Turning a path **twice** gives the path back.
#[test]
fn turning_a_path_twice_gives_it_back() {
    let mut session = session();
    let node = a_line(&mut session);
    let (_, _, before) = path_of(&session);
    session.execute("vector.path.reverse", &json!({ "node": node })).expect("the path is turned");
    session.execute("vector.path.reverse", &json!({ "node": node })).expect("the path is turned");
    let (_, _, after) = path_of(&session);
    assert_eq!(after, before, "the path is the path it was: {after:?}");
}
