//! The **removal of an anchor**.
//!
//! The tree's rule: the segments that touch the anchor go with it, and
//! what is left is the stretches that survive — the surviving
//! stretches stay separate rather than joining across the gap.

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

/// A **spiral** of four quarter turns: an open contour with five
/// points.
fn a_spiral(session: &mut Session) -> u64 {
    session
        .execute("vector.spiral", &json!({ "x": 0.0, "y": 0.0, "radius": 10.0, "segments": 4 }))
        .expect("the spiral is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .map(|node| node.id)
        .last()
        .expect("the spiral is there")
}

/// A **line**: an open contour with two points.
fn a_line(session: &mut Session) -> u64 {
    session
        .execute("vector.line", &json!({ "x1": 0.0, "y1": 0.0, "x2": 10.0, "y2": 0.0 }))
        .expect("the line is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .map(|node| node.id)
        .last()
        .expect("the line is there")
}

/// The path of a node, as the document holds it.
fn path_of(session: &Session, id: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(id).expect("the node is there");
    node.data.get("path").cloned().expect("the node has a path")
}

/// The **contours** of a path, as the document holds them.
fn contours_of(session: &Session, id: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    path_of(session, id)
        .get("subpaths")
        .and_then(|subpaths| subpaths.as_array().cloned())
        .expect("the path has contours")
}

/// The **points** of a contour, as the document holds them.
fn points_of(contour: &craftkit_sdk::serde_json::Value) -> Vec<(f64, f64)> {
    let mut points: Vec<(f64, f64)> = Vec::new();
    let start = contour.get("start").expect("the contour has a start");
    points.push((
        start.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0),
        start.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0),
    ));
    for segment in segments_of(contour) {
        let end = segment
            .get("line")
            .and_then(|line| line.get("to"))
            .or_else(|| segment.get("cubic").and_then(|cubic| cubic.get("to")))
            .cloned()
            .expect("the segment ends at a point");
        points.push((
            end.get("x").and_then(|x| x.as_f64()).unwrap_or(0.0),
            end.get("y").and_then(|y| y.as_f64()).unwrap_or(0.0),
        ));
    }
    points
}

/// The **segments** of a contour, as the document holds them.
fn segments_of(contour: &craftkit_sdk::serde_json::Value) -> Vec<craftkit_sdk::serde_json::Value> {
    contour.get("segments").and_then(|segments| segments.as_array().cloned()).unwrap_or_default()
}

/// An anchor may be **taken away**.
#[test]
fn an_anchor_may_be_taken_away() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.anchor.remove", &json!({ "node": id, "anchor": 1 }))
        .expect("the anchor is taken away");
    let contours = contours_of(&session, id);
    assert_eq!(contours.len(), 1, "the contour is one stretch");
    assert_eq!(points_of(&contours[0]).len(), 3, "and it has three points");
}

/// A closed contour is a **cycle**, so one anchor away leaves **one**
/// stretch.
#[test]
fn a_closed_contour_stays_one_stretch() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.anchor.remove", &json!({ "node": id, "anchor": 2 }))
        .expect("the anchor is taken away");
    let contours = contours_of(&session, id);
    assert_eq!(contours.len(), 1, "the contour is one stretch");
    let points = points_of(&contours[0]);
    assert_eq!(points.len(), 3, "and it has three points");
    assert_eq!(points[0], (0.0, 10.0), "it starts where the segments that went ended");
    assert_eq!(points[1], (0.0, 0.0), "then it goes home");
    assert_eq!(points[2], (10.0, 0.0), "and on to the corner that is left");
}

/// An open contour is a **line** of points, so one anchor away leaves
/// the stretches on either side of it.
#[test]
fn an_open_contour_splits_into_two_stretches() {
    let mut session = session();
    let id = a_spiral(&mut session);
    let before = contours_of(&session, id);
    assert_eq!(before.len(), 1, "the spiral is one contour");
    assert_eq!(before[0].get("closed").and_then(|closed| closed.as_bool()), Some(false), "and it is open");
    session
        .execute("vector.path.anchor.remove", &json!({ "node": id, "anchor": 2 }))
        .expect("the anchor is taken away");
    let contours = contours_of(&session, id);
    assert_eq!(contours.len(), 2, "the contour is two stretches");
    assert_eq!(points_of(&contours[0]).len(), 2, "the first stretch has two points");
    assert_eq!(points_of(&contours[1]).len(), 2, "and the second has two");
}

/// The points that **survive** keep their places.
#[test]
fn the_points_that_survive_keep_their_places() {
    let mut session = session();
    let id = a_spiral(&mut session);
    let before = points_of(&contours_of(&session, id)[0]);
    session
        .execute("vector.path.anchor.remove", &json!({ "node": id, "anchor": 2 }))
        .expect("the anchor is taken away");
    let after: Vec<(f64, f64)> = contours_of(&session, id).iter().flat_map(points_of).collect();
    assert_eq!(after.len(), 4, "the contour has four points");
    assert_eq!(after[0], before[0], "the first point is still there");
    assert_eq!(after[1], before[1], "and the one after it");
    assert_eq!(after[2], before[3], "and the one after the anchor that went");
    assert_eq!(after[3], before[4], "and the last one");
}

/// A contour that was **closed** is open afterwards.
#[test]
fn a_closed_contour_is_open_afterwards() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    assert_eq!(
        contours_of(&session, id)[0].get("closed").and_then(|closed| closed.as_bool()),
        Some(true),
        "the rectangle is closed"
    );
    session
        .execute("vector.path.anchor.remove", &json!({ "node": id, "anchor": 1 }))
        .expect("the anchor is taken away");
    let contours = contours_of(&session, id);
    assert_eq!(
        contours[0].get("closed").and_then(|closed| closed.as_bool()),
        Some(false),
        "and the stretch is open"
    );
}

/// A contour that is left with **fewer than two points** is gone.
#[test]
fn a_contour_with_one_point_is_gone() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let line = a_line(&mut session);
    session
        .execute("vector.path.anchor.remove", &json!({ "node": line, "anchor": 0 }))
        .expect("the anchor is taken away");
    assert!(contours_of(&session, line).is_empty(), "the contour is gone");
    assert_eq!(contours_of(&session, id).len(), 1, "and the rectangle is still there");
}

/// The **segments** that survive keep their shape.
#[test]
fn the_segments_that_survive_keep_their_shape() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    // The **second** segment of the rectangle becomes a curve.
    session
        .execute("vector.path.handle", &json!({ "path": id, "point": 2, "c1": { "x": 8.0, "y": 10.0 } }))
        .expect("the handle is set");
    let before = segments_of(&contours_of(&session, id)[0]);
    session
        .execute("vector.path.anchor.remove", &json!({ "node": id, "anchor": 1 }))
        .expect("the anchor is taken away");
    let after: Vec<craftkit_sdk::serde_json::Value> =
        contours_of(&session, id).iter().flat_map(segments_of).collect();
    assert!(after.contains(&before[2]), "the last segment is still there");
    assert!(!after.contains(&before[0]), "and the two that touched the anchor are gone");
    assert!(!after.contains(&before[1]), "both of them");
}

/// An anchor that is **not there** is refused.
#[test]
fn an_anchor_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute("vector.path.anchor.remove", &json!({ "node": id, "anchor": 9 }));
    assert!(outcome.is_err(), "the anchor is refused");
}

/// A contour that is **not there** is refused.
#[test]
fn a_contour_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome =
        session.execute("vector.path.anchor.remove", &json!({ "node": id, "subpath": 3, "anchor": 0 }));
    assert!(outcome.is_err(), "the contour is refused");
}

/// An anchor that is **not a number** is refused.
#[test]
fn an_anchor_that_is_not_a_number_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute("vector.path.anchor.remove", &json!({ "node": id, "anchor": -1.0 }));
    assert!(outcome.is_err(), "the anchor is refused");
}
