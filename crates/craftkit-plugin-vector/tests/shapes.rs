//! The **shapes** the tree makes that this tree did not: the star,
//! the line and the spiral.
//!
//! The geometry is the tree's own: the points of a star alternate
//! between two radii, a line is a contour of two points, and a spiral
//! is a run of quarter arcs that get smaller and smaller.

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

/// The **point** of a piece of JSON, as a pair.
fn point_of(value: &craftkit_sdk::serde_json::Value) -> Option<(f64, f64)> {
    let x = value.get("x").and_then(|x| x.as_f64())?;
    let y = value.get("y").and_then(|y| y.as_f64())?;
    Some((x, y))
}

/// The **paths** of the document: each one's points.
fn paths_of(session: &Session) -> Vec<Vec<(f64, f64)>> {
    session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .map(|node| {
            let Some(subpaths) = node
                .data
                .get("path")
                .and_then(|path| path.get("subpaths"))
                .and_then(|subpaths| subpaths.as_array())
            else {
                return Vec::new();
            };
            let mut points = Vec::new();
            for subpath in subpaths {
                if let Some(start) = subpath.get("start").and_then(point_of) {
                    points.push(start);
                }
                if let Some(segments) = subpath.get("segments").and_then(|segments| segments.as_array()) {
                    for segment in segments {
                        // A segment is a **line** or a **curve**, and
                        // both of them end at a point.
                        let end = segment
                            .get("line")
                            .and_then(|line| line.get("to"))
                            .or_else(|| segment.get("cubic").and_then(|cubic| cubic.get("to")));
                        if let Some(to) = end.and_then(point_of) {
                            points.push(to);
                        }
                    }
                }
            }
            points
        })
        .collect()
}

/// A **star** has points that alternate between two radii.
#[test]
fn a_star_alternates_between_two_radii() {
    let mut session = session();
    session
        .execute(
            "vector.star",
            &json!({ "x": 0.0, "y": 0.0, "radius": 10.0, "inner_radius": 5.0, "points": 5 }),
        )
        .expect("the star is added");
    let paths = paths_of(&session);
    assert_eq!(paths.len(), 1, "there is one path");
    let points = &paths[0];
    assert_eq!(points.len(), 10, "a star of five points has ten corners: {points:?}");
    for (index, (x, y)) in points.iter().enumerate() {
        let radius = (x * x + y * y).sqrt();
        let expected = if index % 2 == 0 { 10.0 } else { 5.0 };
        assert!((radius - expected).abs() < 1e-6, "corner {index} is at {expected}: {radius}");
    }
}

/// The star starts at the **top**.
#[test]
fn a_star_starts_at_the_top() {
    let mut session = session();
    session
        .execute("vector.star", &json!({ "x": 0.0, "y": 0.0, "radius": 10.0, "points": 5 }))
        .expect("the star is added");
    let paths = paths_of(&session);
    let first = paths[0][0];
    assert!((first.0).abs() < 1e-6, "the first corner is straight up: {first:?}");
    assert!((first.1 + 10.0).abs() < 1e-6, "and it is at the outer radius: {first:?}");
}

/// The inner radius is **half** the outer one when it is not given.
#[test]
fn the_inner_radius_is_half_the_outer_one() {
    let mut session = session();
    session
        .execute("vector.star", &json!({ "x": 0.0, "y": 0.0, "radius": 10.0, "points": 4 }))
        .expect("the star is added");
    let paths = paths_of(&session);
    let radius = |index: usize| {
        let (x, y) = paths[0][index];
        (x * x + y * y).sqrt()
    };
    assert!((radius(0) - 10.0).abs() < 1e-6, "the outer radius is the radius");
    assert!((radius(1) - 5.0).abs() < 1e-6, "and the inner one is half of it: {}", radius(1));
}

/// A star with **too many** points is refused, and so is one with too
/// few.
#[test]
fn a_star_outside_the_bound_is_refused() {
    let mut session = session();
    let error = session
        .execute("vector.star", &json!({ "x": 0.0, "y": 0.0, "radius": 10.0, "points": 1 }))
        .expect_err("the star is refused");
    assert!(error.to_string().contains("between two and"), "the error says why: {error}");
}

/// A **line** is a contour of two points, and it is not closed.
#[test]
fn a_line_is_two_points() {
    let mut session = session();
    session
        .execute("vector.line", &json!({ "x1": 0.0, "y1": 0.0, "x2": 10.0, "y2": 20.0 }))
        .expect("the line is added");
    let paths = paths_of(&session);
    assert_eq!(paths.len(), 1, "there is one path");
    assert_eq!(paths[0], vec![(0.0, 0.0), (10.0, 20.0)], "the line goes from here to there");
}

/// A **spiral** is a run of quarter arcs, and it winds inward.
#[test]
fn a_spiral_winds_inward() {
    let mut session = session();
    session
        .execute("vector.spiral", &json!({ "x": 0.0, "y": 0.0, "radius": 100.0, "segments": 4 }))
        .expect("the spiral is added");
    let paths = paths_of(&session);
    assert_eq!(paths.len(), 1, "there is one path");
    let points = &paths[0];
    assert!(points.len() >= 5, "four quarter turns have five anchors: {points:?}");
    let radius = |index: usize| {
        let (x, y) = points[index];
        (x * x + y * y).sqrt()
    };
    // The spiral is **reversed**: it starts at the centre and ends at
    // the outside.
    assert!(radius(0) < radius(points.len() - 1), "it starts at the centre: {points:?}");
    assert!((radius(points.len() - 1) - 100.0).abs() < 1e-3, "and ends at the radius");
}

/// A spiral whose **decay** is outside the bound is refused.
#[test]
fn a_spiral_outside_the_bound_is_refused() {
    let mut session = session();
    let error = session
        .execute("vector.spiral", &json!({ "x": 0.0, "y": 0.0, "radius": 100.0, "decay": 1.0 }))
        .expect_err("the spiral is refused");
    assert!(error.to_string().contains("decay is a number of percent"), "the error says why: {error}");
}
