//! The **reshape** of a path.
//!
//! The reshape tool grabs the path at a point and drags it, and the
//! anchors around the grab follow with a **smooth falloff** over a
//! radius measured **along** the path.

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

/// The first **contour** of a path, as the document holds it.
fn contour_of(session: &Session, id: u64) -> craftkit_sdk::serde_json::Value {
    path_of(session, id)
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(0))
        .cloned()
        .expect("the contour is there")
}

/// The **segments** of the first contour.
fn segments_of(session: &Session, id: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    contour_of(session, id)
        .get("segments")
        .and_then(|segments| segments.as_array())
        .cloned()
        .expect("the contour has segments")
}

/// A point of the document, as a pair of numbers.
fn point_of(value: &craftkit_sdk::serde_json::Value) -> (f64, f64) {
    let x = value.get("x").and_then(|x| x.as_f64()).expect("the point has an x");
    let y = value.get("y").and_then(|y| y.as_f64()).expect("the point has a y");
    (x, y)
}

/// The **points** of the first contour: where it starts and where each
/// segment ends.
fn points_of(session: &Session, id: u64) -> Vec<(f64, f64)> {
    let contour = contour_of(session, id);
    let mut points = vec![point_of(contour.get("start").expect("the contour has a start"))];
    for segment in segments_of(session, id) {
        let end = segment
            .get("line")
            .and_then(|line| line.get("to"))
            .or_else(|| segment.get("cubic").and_then(|cubic| cubic.get("to")))
            .expect("the segment has an end");
        points.push(point_of(end));
    }
    points
}

/// The **control point** of a segment: the one the curve leaves by, or
/// the one it arrives by.
fn control_of(segment: &craftkit_sdk::serde_json::Value, which: &str) -> Option<(f64, f64)> {
    let cubic = segment.get("cubic")?;
    let point = cubic.get(which)?;
    Some(point_of(point))
}

/// The **grabbed** anchor moves the whole way.
#[test]
fn the_grabbed_anchor_moves_the_whole_way() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.reshape", &json!({ "path": id, "x": 0.0, "y": 0.0, "dx": 5.0, "dy": 0.0 }))
        .expect("the path is reshaped");
    let points = points_of(&session, id);
    assert!((points[0].0 - 5.0).abs() < 1e-6, "the grabbed point moved: {points:?}");
    assert!(points[0].1.abs() < 1e-6, "and only sideways: {points:?}");
}

/// The anchors **beyond the radius** do not move: the default radius
/// is half the size of the contour, and the far corners are further
/// away than that.
#[test]
fn the_anchors_beyond_the_radius_do_not_move() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.reshape", &json!({ "path": id, "x": 0.0, "y": 0.0, "dx": 5.0, "dy": 0.0 }))
        .expect("the path is reshaped");
    let points = points_of(&session, id);
    assert!((points[1].0 - 10.0).abs() < 1e-6, "the far corner stays: {points:?}");
    assert!((points[2].0 - 10.0).abs() < 1e-6, "and so does the other one: {points:?}");
}

/// An anchor **within the radius** follows, and it follows by less
/// than the whole way.
#[test]
fn an_anchor_within_the_radius_follows() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.reshape",
            &json!({ "path": id, "x": 0.0, "y": 0.0, "dx": 10.0, "dy": 0.0, "radius": 20.0 }),
        )
        .expect("the path is reshaped");
    let points = points_of(&session, id);
    let moved = points[1].0 - 10.0;
    assert!(moved > 0.0, "the anchor follows: {points:?}");
    assert!(moved < 10.0, "but it does not move the whole way: {points:?}");
    // The weight of an anchor at half the radius is (1 - 1/4) squared.
    assert!((moved - 10.0 * 0.5625).abs() < 1e-6, "and it follows by its weight: {points:?}");
}

/// The falloff is measured **along** the path: the anchor at the far
/// corner of a rectangle is further along the path than it is in a
/// straight line.
#[test]
fn the_falloff_is_measured_along_the_path() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    // The radius is the length of the edge, and the anchor at the end
    // of the edge is exactly that far along the path.
    session
        .execute(
            "vector.path.reshape",
            &json!({ "path": id, "x": 0.0, "y": 0.0, "dx": 4.0, "dy": 0.0, "radius": 10.0 }),
        )
        .expect("the path is reshaped");
    let points = points_of(&session, id);
    // The anchor at the end of the edge is at the radius, so its
    // weight is zero.
    assert!((points[1].0 - 10.0).abs() < 1e-6, "the anchor at the radius does not move: {points:?}");
}

/// A grab on a point that is **not an anchor** adds one.
#[test]
fn a_grab_that_is_not_on_an_anchor_adds_one() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let before = segments_of(&session, id).len();
    session
        .execute(
            "vector.path.reshape",
            &json!({ "path": id, "x": 5.0, "y": 0.0, "dx": 0.0, "dy": 5.0, "tol": 0.5 }),
        )
        .expect("the path is reshaped");
    let after = segments_of(&session, id).len();
    assert_eq!(after, before + 1, "an anchor was added");
    let points = points_of(&session, id);
    assert!((points[1].0 - 5.0).abs() < 1e-6, "and it is where the path was grabbed: {points:?}");
    assert!((points[1].1 - 5.0).abs() < 1e-6, "and it moved the whole way: {points:?}");
}

/// A grab **near** an anchor grabs that one, and adds nothing.
#[test]
fn a_grab_near_an_anchor_grabs_that_one() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let before = segments_of(&session, id).len();
    session
        .execute(
            "vector.path.reshape",
            &json!({ "path": id, "x": 0.5, "y": 0.0, "dx": 5.0, "dy": 0.0, "tol": 3.0 }),
        )
        .expect("the path is reshaped");
    assert_eq!(segments_of(&session, id).len(), before, "no anchor was added");
    let points = points_of(&session, id);
    assert!((points[0].0 - 5.0).abs() < 1e-6, "and the anchor moved the whole way: {points:?}");
}

/// The **handles** move with their anchor.
#[test]
fn the_handles_move_with_the_anchor() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.handle",
            &json!({
                "path": id,
                "point": 0,
                "c1": { "x": -3.0, "y": 0.0 },
            }),
        )
        .expect("the handles are set");
    let before = control_of(&segments_of(&session, id)[0], "c1").expect("the curve has a handle");
    session
        .execute(
            "vector.path.reshape",
            &json!({ "path": id, "x": 0.0, "y": 0.0, "dx": 5.0, "dy": 0.0, "radius": 20.0 }),
        )
        .expect("the path is reshaped");
    let after = control_of(&segments_of(&session, id)[0], "c1").expect("the curve has a handle");
    assert!(
        (after.0 - before.0 - 5.0).abs() < 1e-6,
        "the handle moved with its anchor: {before:?} and {after:?}"
    );
}

/// A **negative** radius is refused, and so is a negative tolerance.
#[test]
fn a_negative_distance_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let refused = session.execute(
        "vector.path.reshape",
        &json!({ "path": id, "x": 0.0, "y": 0.0, "dx": 1.0, "dy": 0.0, "radius": -1.0 }),
    );
    assert!(refused.is_err(), "a radius is a distance");
    let refused = session.execute(
        "vector.path.reshape",
        &json!({ "path": id, "x": 0.0, "y": 0.0, "dx": 1.0, "dy": 0.0, "tol": -1.0 }),
    );
    assert!(refused.is_err(), "a tolerance is a distance");
}

/// A **radius of zero** moves only the grabbed anchor.
#[test]
fn a_radius_of_zero_moves_only_the_grabbed_anchor() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.reshape",
            &json!({ "path": id, "x": 0.0, "y": 0.0, "dx": 5.0, "dy": 0.0, "radius": 0.0 }),
        )
        .expect("the path is reshaped");
    let points = points_of(&session, id);
    assert!((points[0].0 - 5.0).abs() < 1e-6, "the grabbed point moved: {points:?}");
    assert!((points[1].0 - 10.0).abs() < 1e-6, "and nothing else did: {points:?}");
}

/// An **empty** path cannot be reshaped.
#[test]
fn an_empty_path_is_refused() {
    let mut session = session();
    let outcome = session
        .execute("vector.path.reshape", &json!({ "path": 999, "x": 0.0, "y": 0.0, "dx": 1.0, "dy": 1.0 }));
    assert!(outcome.is_err(), "there is no such path");
}
