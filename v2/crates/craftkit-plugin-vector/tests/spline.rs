//! The **curve through the points**.
//!
//! The tree's curvature tool makes a path that curves through the points
//! a person clicks, and the curve is the tree's own: a smooth point's
//! handles follow the circle through the point and its two neighbours, a
//! turn tighter than thirty degrees is cut back, and a point with no
//! direction is a corner.

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

/// The two **control points** of a curve, and where it ends.
fn curve_of(segment: &Json) -> ((f64, f64), (f64, f64), (f64, f64)) {
    let curve = segment.get("cubic").expect("the segment is a curve");
    let point = |name: &str| {
        let value = curve.get(name).expect("the curve has the point");
        (
            value.get("x").and_then(|x| x.as_f64()).expect("the point is across"),
            value.get("y").and_then(|y| y.as_f64()).expect("the point is down"),
        )
    };
    (point("c1"), point("c2"), point("to"))
}

/// A curve through three points has **two sides**.
#[test]
fn a_curve_through_three_points_has_two_sides() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }, { "x": 20.0, "y": 0.0 }] }),
        )
        .expect("the curve is added");
    let segments = segments_of(&session);
    assert_eq!(segments.len(), 2, "three points are two sides");
}

/// The handles of a smooth point follow the **circle** through the three
/// points: they are equal in length and opposite in direction.
#[test]
fn the_handles_follow_the_circle_through_the_points() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }, { "x": 20.0, "y": 0.0 }] }),
        )
        .expect("the curve is added");
    let segments = segments_of(&session);
    let (_, c2, _) = curve_of(&segments[0]);
    let (c1, _, _) = curve_of(&segments[1]);
    let into = (c2.0 - 10.0, c2.1 - 10.0);
    let out = (c1.0 - 10.0, c1.1 - 10.0);
    let length = |vector: (f64, f64)| vector.0.hypot(vector.1);
    assert!(
        (length(into) - length(out)).abs() < 1e-9,
        "the two handles are as long as each other: {into:?} and {out:?}"
    );
    assert!(
        (into.0 + out.0).abs() < 1e-9 && (into.1 + out.1).abs() < 1e-9,
        "the two handles are opposite: {into:?} and {out:?}"
    );
}

/// Three points **in a line** give a straight tangent: the curve does
/// not leave the line.
#[test]
fn three_points_in_a_line_give_a_straight_tangent() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 20.0, "y": 0.0 }] }),
        )
        .expect("the curve is added");
    for segment in segments_of(&session) {
        let (c1, c2, to) = curve_of(&segment);
        for point in [c1, c2, to] {
            assert!(point.1.abs() < 1e-9, "the curve stays on the line: {point:?} is not on it");
        }
    }
}

/// A turn **tighter than thirty degrees** is cut back, so that a narrow
/// V comes to a point instead of swinging round a near-whole circle.
#[test]
fn a_tight_turn_is_cut_back() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 10.0, "y": 0.0 }, { "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 1.0 }] }),
        )
        .expect("the curve is added");
    let segments = segments_of(&session);
    let (_, c2, _) = curve_of(&segments[0]);
    let length = (c2.0 - 0.0).hypot(c2.1 - 0.0);
    assert!(length < 1.0, "the handle of the tight turn is short, not a whole circle: {length}");
}

/// A **corner** point has no handles at all, and the sides at it are
/// straight: the end of a path takes its tangent from its neighbour, and
/// a corner has none to give.
#[test]
fn a_corner_point_has_no_handles() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({
                "points": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 10.0, "y": 10.0, "corner": true },
                    { "x": 20.0, "y": 0.0 }
                ]
            }),
        )
        .expect("the curve is added");
    let segments = segments_of(&session);
    for segment in &segments {
        assert!(segment.get("line").is_some(), "the side of a corner is straight: {segment}");
    }
    let end = segments[0].get("line").and_then(|line| line.get("to")).expect("the side has an end");
    assert_eq!(end.get("x").and_then(|x| x.as_f64()), Some(10.0), "the side goes to the corner");
}

/// A **closed** curve comes back to its first point.
#[test]
fn a_closed_curve_comes_back() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({
                "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }, { "x": 20.0, "y": 0.0 }],
                "closed": true
            }),
        )
        .expect("the curve is added");
    let segments = segments_of(&session);
    assert_eq!(segments.len(), 3, "a closed curve of three points has three sides");
    let contour = contour_of(&session);
    assert_eq!(contour.get("closed").and_then(|closed| closed.as_bool()), Some(true));
}

/// A closed curve needs **more than two points**: the tree's own rule.
#[test]
fn a_closed_curve_needs_more_than_two_points() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }], "closed": true }),
        )
        .expect("the curve is added");
    let contour = contour_of(&session);
    assert_eq!(
        contour.get("closed").and_then(|closed| closed.as_bool()),
        Some(false),
        "two points do not make a closed curve"
    );
}

/// The curve is **named**, as the tree names it.
#[test]
fn the_curve_is_named() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }, { "x": 20.0, "y": 0.0 }] }),
        )
        .expect("the curve is added");
    let node = session.document().walk().find(|node| node.kind == "vector.path").expect("there is a path");
    assert_eq!(node.name, "Curvature", "the tree calls a curve made this way a Curvature");
}

/// The curve may **replace** the path that is named, as the tree's own
/// command does.
#[test]
fn the_curve_may_replace_the_path_that_is_there() {
    let mut session = session();
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    let id = session
        .document()
        .walk()
        .find(|node| node.kind == "vector.path")
        .map(|node| node.id)
        .expect("there is a path");
    session
        .execute(
            "vector.path.spline",
            &json!({
                "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }, { "x": 20.0, "y": 0.0 }],
                "path": id
            }),
        )
        .expect("the path is curved");
    let segments = segments_of(&session);
    assert_eq!(segments.len(), 2, "the rectangle is now the curve: {segments:?}");
}

/// The handles of a smooth point are **between a third and two thirds**
/// of their own chord, and the shorter side's handle is the shorter:
/// the tree's own rule.
#[test]
fn the_handles_are_between_a_third_and_two_thirds_of_their_chord() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 200.0, "y": 100.0 }] }),
        )
        .expect("the curve is added");
    let segments = segments_of(&session);
    let (_, c2, _) = curve_of(&segments[0]);
    let (c1, _, _) = curve_of(&segments[1]);
    let middle = (10.0, 0.0);
    let length = |point: (f64, f64)| (point.0 - middle.0).hypot(point.1 - middle.1);
    let chord_in = 10.0_f64;
    let chord_out = (200.0_f64 - 10.0).hypot(100.0 - 0.0);
    for (handle, chord) in [(length(c2), chord_in), (length(c1), chord_out)] {
        let ratio = handle / chord;
        assert!(
            (1.0 / 3.0 - 1e-9..=2.0 / 3.0 + 1e-9).contains(&ratio),
            "the handle is between a third and two thirds of its chord: {ratio}"
        );
    }
    assert!(length(c2) < length(c1), "the shorter side's handle is the shorter");
}

/// The **ends** of an open curve take the tangent of their neighbour,
/// and the sides at a smooth point are curves: the tree's own numbers.
#[test]
fn the_ends_of_an_open_curve_take_their_neighbours_tangent() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 100.0, "y": -100.0 }, { "x": 200.0, "y": 0.0 }] }),
        )
        .expect("the curve is added");
    let segments = segments_of(&session);
    for segment in &segments {
        assert!(segment.get("cubic").is_some(), "the side at a smooth point is a curve: {segment}");
    }
    let (c1, _, _) = curve_of(&segments[0]);
    assert_ne!(c1, (0.0, 0.0), "the first point has a handle that leaves it");
    let (_, c2, _) = curve_of(&segments[1]);
    assert_ne!(c2, (200.0, 0.0), "the last point has a handle that arrives at it");
}

/// **Two points** stay a straight line: the tree's own rule.
#[test]
fn two_points_stay_a_straight_line() {
    let mut session = session();
    session
        .execute(
            "vector.path.spline",
            &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 100.0, "y": -100.0 }] }),
        )
        .expect("the curve is added");
    let segments = segments_of(&session);
    assert_eq!(segments.len(), 1, "two points are one side");
    assert!(segments[0].get("line").is_some(), "the side is straight: {}", segments[0]);
}

/// A curve of **one point** has no side at all, and is refused.
#[test]
fn a_curve_of_one_point_is_refused() {
    let mut session = session();
    let outcome = session.execute("vector.path.spline", &json!({ "points": [{ "x": 0.0, "y": 0.0 }] }));
    assert!(outcome.is_err(), "the curve of one point is refused");
}

/// A curve of **no points** is refused.
#[test]
fn a_curve_of_no_points_is_refused() {
    let mut session = session();
    let outcome = session.execute("vector.path.spline", &json!({ "points": [] }));
    assert!(outcome.is_err(), "the curve of no points is refused");
}

/// A point that is **not a point** is refused.
#[test]
fn a_point_that_is_not_a_point_is_refused() {
    let mut session = session();
    let outcome =
        session.execute("vector.path.spline", &json!({ "points": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0 }] }));
    assert!(outcome.is_err(), "the point without a `y` is refused");
}
