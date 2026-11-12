//! The **making of a path by hand**.
//!
//! The tree's pen makes a path one of two ways: out of path data
//! written in the SVG spelling, or out of a list of anchors. The rules
//! here are the tree's own: a handle that is not given is the point
//! itself, a side with no handle is a straight line, a malformed anchor
//! is refused rather than given a default, and an arc is not read.

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

/// The path of the one path in the document.
fn path_of(session: &Session) -> Json {
    let node = session.document().walk().find(|node| node.kind == "vector.path").expect("there is a path");
    node.data.get("path").cloned().expect("the node has a path")
}

/// The one contour of the path.
fn contour_of(session: &Session) -> Json {
    path_of(session)
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(0))
        .cloned()
        .expect("the path has a contour")
}

/// The **segments** of the contour.
fn segments_of(session: &Session) -> Vec<Json> {
    contour_of(session)
        .get("segments")
        .and_then(|segments| segments.as_array())
        .cloned()
        .expect("the contour has segments")
}

/// The **end** of a segment.
fn end_of(segment: &Json) -> (f64, f64) {
    let end = segment
        .get("line")
        .and_then(|line| line.get("to"))
        .or_else(|| segment.get("cubic").and_then(|cubic| cubic.get("to")))
        .expect("the segment has an end");
    (
        end.get("x").and_then(|x| x.as_f64()).expect("the end is across"),
        end.get("y").and_then(|y| y.as_f64()).expect("the end is down"),
    )
}

/// Three anchors, with no handles: a triangle that is not closed.
fn three_anchors() -> Json {
    json!([
        { "x": 0.0, "y": 0.0 },
        { "x": 10.0, "y": 0.0 },
        { "x": 10.0, "y": 10.0 }
    ])
}

/// A path made of anchors is **in the document**, with its sides.
#[test]
fn a_path_may_be_made_of_anchors() {
    let mut session = session();
    session.execute("vector.path.create", &json!({ "anchors": three_anchors() })).expect("the path is added");
    let segments = segments_of(&session);
    assert_eq!(segments.len(), 2, "three anchors are two sides");
    assert_eq!(end_of(&segments[0]), (10.0, 0.0), "the first side goes to the second anchor");
    assert_eq!(end_of(&segments[1]), (10.0, 10.0), "the second side goes to the third anchor");
    let contour = contour_of(&session);
    assert_eq!(contour.get("closed").and_then(|closed| closed.as_bool()), Some(false));
}

/// A path made of anchors is **named**, as the tree names it.
#[test]
fn a_path_made_of_anchors_is_named() {
    let mut session = session();
    session.execute("vector.path.create", &json!({ "anchors": three_anchors() })).expect("the path is added");
    let node = session.document().walk().find(|node| node.kind == "vector.path").expect("there is a path");
    assert_eq!(node.name, "Pen", "the tree calls a path made by hand a Pen");
}

/// A **closed** path has the side that comes back to the first anchor.
#[test]
fn a_closed_path_has_the_side_that_comes_back() {
    let mut session = session();
    session
        .execute("vector.path.create", &json!({ "anchors": three_anchors(), "closed": true }))
        .expect("the path is added");
    let segments = segments_of(&session);
    assert_eq!(segments.len(), 3, "a closed path of three anchors has three sides");
    assert_eq!(end_of(&segments[2]), (0.0, 0.0), "the last side comes back to the first anchor");
    let contour = contour_of(&session);
    assert_eq!(contour.get("closed").and_then(|closed| closed.as_bool()), Some(true));
}

/// A **handle** on an anchor makes the side a curve.
#[test]
fn a_handle_makes_the_side_a_curve() {
    let mut session = session();
    session
        .execute(
            "vector.path.create",
            &json!({
                "anchors": [
                    { "x": 0.0, "y": 0.0, "out": [5.0, 0.0] },
                    { "x": 10.0, "y": 0.0 }
                ]
            }),
        )
        .expect("the path is added");
    let segments = segments_of(&session);
    let curve = segments[0].get("cubic").expect("the side is a curve");
    let c1 = curve.get("c1").expect("the curve has a first control point");
    assert_eq!(c1.get("x").and_then(|x| x.as_f64()), Some(5.0), "the handle is where it was put");
}

/// A handle that is **not given** is the point itself, so the side is a
/// straight line.
#[test]
fn a_handle_that_is_not_given_is_the_point() {
    let mut session = session();
    session
        .execute(
            "vector.path.create",
            &json!({
                "anchors": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 10.0, "y": 0.0, "in": null }
                ]
            }),
        )
        .expect("the path is added");
    let segments = segments_of(&session);
    assert!(segments[0].get("line").is_some(), "the side is a line: {}", segments[0]);
}

/// A path made of **path data** is in the document too.
#[test]
fn a_path_may_be_made_of_path_data() {
    let mut session = session();
    session
        .execute("vector.path.create", &json!({ "d": "M 0 0 L 10 0 L 10 10" }))
        .expect("the path is added");
    let segments = segments_of(&session);
    assert_eq!(segments.len(), 2, "the path data has two sides");
    assert_eq!(end_of(&segments[1]), (10.0, 10.0), "the second side goes where the data says");
}

/// The path data says whether the path is **closed**.
#[test]
fn the_path_data_may_close_the_path() {
    let mut session = session();
    session
        .execute("vector.path.create", &json!({ "d": "M 0 0 L 10 0 L 10 10 Z" }))
        .expect("the path is added");
    let contour = contour_of(&session);
    assert_eq!(contour.get("closed").and_then(|closed| closed.as_bool()), Some(true));
}

/// An **arc** is not read: this tree does not write a curve it cannot
/// say.
#[test]
fn an_arc_is_refused() {
    let mut session = session();
    let outcome = session.execute("vector.path.create", &json!({ "d": "M 0 0 A 5 5 0 0 1 10 10" }));
    assert!(outcome.is_err(), "the arc is refused");
}

/// An anchor that is **malformed** is refused, and not given a default.
#[test]
fn a_malformed_anchor_is_refused() {
    let mut session = session();
    let outcome =
        session.execute("vector.path.create", &json!({ "anchors": [{ "x": 0.0 }, { "x": 10.0, "y": 0.0 }] }));
    assert!(outcome.is_err(), "the anchor without a `y` is refused");
}

/// An anchor whose handle is **malformed** is refused too.
#[test]
fn a_malformed_handle_is_refused() {
    let mut session = session();
    let outcome = session.execute(
        "vector.path.create",
        &json!({ "anchors": [{ "x": 0.0, "y": 0.0, "out": [5.0] }, { "x": 10.0, "y": 0.0 }] }),
    );
    assert!(outcome.is_err(), "the handle that is not a pair is refused");
}

/// An anchor that is **smooth** is refused: a path here does not keep
/// an anchor's kind.
#[test]
fn a_smooth_anchor_is_refused() {
    let mut session = session();
    let outcome = session.execute(
        "vector.path.create",
        &json!({
            "anchors": [
                { "x": 0.0, "y": 0.0, "smooth": true },
                { "x": 10.0, "y": 0.0 }
            ]
        }),
    );
    assert!(outcome.is_err(), "the smooth anchor is refused");
}

/// A path made of **one** anchor has no side, and is refused.
#[test]
fn a_path_of_one_anchor_is_refused() {
    let mut session = session();
    let outcome = session.execute("vector.path.create", &json!({ "anchors": [{ "x": 0.0, "y": 0.0 }] }));
    assert!(outcome.is_err(), "the path of one anchor is refused");
}

/// The two ways of making a path are **one** way: they may not be used
/// together.
#[test]
fn the_two_ways_may_not_be_used_together() {
    let mut session = session();
    let outcome =
        session.execute("vector.path.create", &json!({ "d": "M 0 0 L 10 0", "anchors": three_anchors() }));
    assert!(outcome.is_err(), "the path may not be made two ways at once");
}

/// A path is made one of the two ways, and a command that says neither
/// is refused.
#[test]
fn a_path_may_not_be_made_of_nothing() {
    let mut session = session();
    let outcome = session.execute("vector.path.create", &json!({}));
    assert!(outcome.is_err(), "the path with no way of being made is refused");
}
