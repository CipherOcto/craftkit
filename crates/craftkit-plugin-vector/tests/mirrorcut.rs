//! The **mirror and cut**: a shape mirrored in an axis.
//!
//! The tree's rules: the shape is cut at the axis, the side that is
//! named is kept, and what is kept is joined to its own mirror — so
//! that the shape that is left is symmetric about the axis.

use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, FormatValue, Path, Session};

use craftkit_vector_geom as geom;

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

/// The **shape** of the path, as the geometry library reads it.
fn shape_of(session: &Session, id: u64) -> Path {
    let node = session.document().node(id).expect("the path is there");
    Path::from_json(&node.data["path"]).expect("the path is read")
}

/// The **area** the shape covers.
fn area_of(session: &Session, id: u64) -> f64 {
    geom::area(&shape_of(session, id)).abs()
}

/// The **contours** of the path, as they are in the document.
fn contours(session: &Session, id: u64) -> Vec<Json> {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"].as_array().expect("the path has contours").clone()
}

/// A **triangle**, and its id: a right triangle with its corner at the
/// origin.
fn a_triangle(session: &mut Session) -> u64 {
    session
        .execute(
            "vector.path.create",
            &json!({
                "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 100.0, "y": 0.0 }, { "x": 0.0, "y": 100.0 }],
                "closed": true
            }),
        )
        .expect("the triangle is added");
    paths(session)[0]
}

/// The shape is **mirrored** and joined to its mirror.
#[test]
fn the_shape_is_mirrored_and_joined() {
    let mut session = session();
    let id = a_triangle(&mut session);
    let before = area_of(&session, id);
    assert!((before - 5000.0).abs() < 1.0, "the triangle is five thousand: {before}");
    session
        .execute("vector.path.cut.mirror", &json!({ "path": id, "axis": "vertical", "from": [50.0, 0.0] }))
        .expect("the shape is mirrored");
    assert_eq!(paths(&session).len(), 1, "there is still one path");
    let after = area_of(&session, id);
    assert!((after - 7500.0).abs() < 1.0, "the shape is the half and its mirror: {after}");
}

/// The shape that is left is **symmetric** about the axis.
#[test]
fn the_shape_that_is_left_is_symmetric() {
    let mut session = session();
    let id = a_triangle(&mut session);
    session
        .execute("vector.path.cut.mirror", &json!({ "path": id, "axis": "vertical", "from": [50.0, 0.0] }))
        .expect("the shape is mirrored");
    let shape = shape_of(&session, id);
    let bounds = geom::bounds(&shape);
    assert!((bounds.x - 0.0).abs() < 1e-6, "the shape begins at the origin: {bounds:?}");
    assert!((bounds.x + bounds.w - 100.0).abs() < 1e-6, "the shape ends at the axis: {bounds:?}");
}

/// The **side that is named** is the side that is kept.
#[test]
fn the_side_that_is_named_is_the_side_that_is_kept() {
    let mut right = session();
    let right_id = a_triangle(&mut right);
    right
        .execute(
            "vector.path.cut.mirror",
            &json!({ "path": right_id, "axis": "vertical", "from": [50.0, 0.0], "keep": "right" }),
        )
        .expect("the shape is mirrored");
    let kept = contours(&right, right_id);
    let mut left = session();
    let left_id = a_triangle(&mut left);
    left.execute(
        "vector.path.cut.mirror",
        &json!({ "path": left_id, "axis": "vertical", "from": [50.0, 0.0], "keep": "left" }),
    )
    .expect("the shape is mirrored");
    assert_ne!(kept, contours(&left, left_id), "the two sides give two shapes");
}

/// An axis that is **not one of the three** is refused.
#[test]
fn an_unknown_axis_is_refused() {
    let mut session = session();
    let id = a_triangle(&mut session);
    let outcome = session
        .execute("vector.path.cut.mirror", &json!({ "path": id, "axis": "diagonal", "from": [50.0, 0.0] }));
    assert!(outcome.is_err(), "the unknown axis is refused");
}

/// A side that is **not one of the four** is refused.
#[test]
fn an_unknown_side_is_refused() {
    let mut session = session();
    let id = a_triangle(&mut session);
    let outcome = session.execute(
        "vector.path.cut.mirror",
        &json!({ "path": id, "axis": "vertical", "from": [50.0, 0.0], "keep": "middle" }),
    );
    assert!(outcome.is_err(), "the unknown side is refused");
}

/// A side that is **not a side of the axis** is refused: a vertical axis
/// has no top.
#[test]
fn a_side_that_is_not_a_side_of_the_axis_is_refused() {
    let mut session = session();
    let id = a_triangle(&mut session);
    let outcome = session.execute(
        "vector.path.cut.mirror",
        &json!({ "path": id, "axis": "vertical", "from": [50.0, 0.0], "keep": "top" }),
    );
    assert!(outcome.is_err(), "the top of a vertical axis is refused");
}

/// A **free** axis needs two points that differ.
#[test]
fn a_free_axis_needs_two_points_that_differ() {
    let mut session = session();
    let id = a_triangle(&mut session);
    let outcome = session.execute(
        "vector.path.cut.mirror",
        &json!({ "path": id, "axis": "free", "from": [50.0, 0.0], "to": [50.0, 0.0] }),
    );
    assert!(outcome.is_err(), "the free axis with two equal points is refused");
}

/// A **free** axis is a line through two points.
#[test]
fn a_free_axis_is_a_line_through_two_points() {
    let mut session = session();
    let id = a_triangle(&mut session);
    session
        .execute(
            "vector.path.cut.mirror",
            &json!({ "path": id, "axis": "free", "from": [50.0, -10.0], "to": [50.0, 10.0] }),
        )
        .expect("the shape is mirrored");
    let after = area_of(&session, id);
    assert!((after - 7500.0).abs() < 1.0, "the shape is the half and its mirror: {after}");
}

/// A shape that is **wholly on the other side** is taken away.
#[test]
fn a_shape_wholly_on_the_other_side_is_taken_away() {
    let mut session = session();
    let id = a_triangle(&mut session);
    session
        .execute(
            "vector.path.cut.mirror",
            &json!({ "path": id, "axis": "vertical", "from": [200.0, 0.0], "keep": "right" }),
        )
        .expect("the mirror runs");
    assert!(paths(&session).is_empty(), "the shape is gone");
}

/// An **open** path is refused: the mirror cuts closed shapes.
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
        .execute("vector.path.cut.mirror", &json!({ "path": id, "axis": "vertical", "from": [10.0, 0.0] }));
    assert!(outcome.is_err(), "the open path is refused");
}
