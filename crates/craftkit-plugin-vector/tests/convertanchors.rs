//! The **converting** of points: smooth, or corner.
//!
//! The tree's rules: a point made smooth takes the direction from the
//! point before it to the point after it, and each handle is a third
//! of the way to that side's neighbour; a point that is already smooth
//! keeps its handles; a corner's handles come back to the point.

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

/// A **path** of three points — a corner — and its id.
fn a_corner(session: &mut Session) -> u64 {
    session
        .execute(
            "vector.path.create",
            &json!({
                "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 0.0, "y": 10.0 }]
            }),
        )
        .expect("the path is added");
    paths(session)[0]
}

/// The point becomes **smooth**: the handles are in line with the
/// direction from the point before it to the point after it, and each
/// is a third of the way to that side's neighbour.
#[test]
fn the_point_becomes_smooth_by_the_neighbours_rule() {
    let mut session = session();
    let id = a_corner(&mut session);
    session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1], "to": "smooth" }))
        .expect("the point is converted");
    let sides = sides(&session, id);
    let (_, c2) = handles(&sides[0]).expect("the arriving side is a curve");
    assert!((c2.0 - 10.0).abs() < 1e-6, "the handle is at the point: {c2:?}");
    assert!((c2.1 - (-10.0 / 3.0)).abs() < 1e-6, "the handle is a third of the way: {c2:?}");
    let (c1, c2) = handles(&sides[1]).expect("the leaving side is a curve");
    assert!((c1.0 - 10.0).abs() < 1e-6, "the handle is at the point: {c1:?}");
    let expected = 10.0 * 2.0_f64.sqrt() / 3.0;
    assert!((c1.1 - expected).abs() < 1e-6, "the handle is a third of the way: {c1:?}");
    assert!((c2.0 - 0.0).abs() < 1e-6, "the far end keeps what it had: {c2:?}");
}

/// **Many** points are converted at once.
#[test]
fn many_points_are_converted_at_once() {
    let mut session = session();
    let id = a_corner(&mut session);
    let outcome = session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [0, 1], "to": "smooth" }))
        .expect("the points are converted");
    let converted: Vec<u64> = outcome
        .data
        .get("converted")
        .and_then(|list| list.as_array())
        .map(|list| list.iter().filter_map(|point| point.as_u64()).collect())
        .unwrap_or_default();
    assert_eq!(converted, vec![0, 1], "both points are converted: {converted:?}");
}

/// A point that is **already smooth** keeps its handles.
#[test]
fn a_point_that_is_already_smooth_keeps_its_handles() {
    let mut session = session();
    let id = a_corner(&mut session);
    session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1], "to": "smooth" }))
        .expect("the point is converted");
    let before = sides(&session, id);
    let outcome = session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1], "to": "smooth" }))
        .expect("the point is converted");
    assert_eq!(sides(&session, id), before, "the path is as it was");
    let left: Vec<u64> = outcome
        .data
        .get("left")
        .and_then(|list| list.as_array())
        .map(|list| list.iter().filter_map(|point| point.as_u64()).collect())
        .unwrap_or_default();
    assert_eq!(left, vec![1], "the point is left as it is: {left:?}");
}

/// A point becomes a **corner**: its handles come back to it.
#[test]
fn the_point_becomes_a_corner() {
    let mut session = session();
    let id = a_corner(&mut session);
    session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1], "to": "smooth" }))
        .expect("the point is converted");
    session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1], "to": "corner" }))
        .expect("the point is converted");
    let sides = sides(&session, id);
    let (_, c2) = handles(&sides[0]).expect("the arriving side is a curve");
    assert!((c2.0 - 10.0).abs() < 1e-6, "the handle is at the point: {c2:?}");
    assert!((c2.1 - 0.0).abs() < 1e-6, "the handle is at the point: {c2:?}");
    let (c1, _) = handles(&sides[1]).expect("the leaving side is a curve");
    assert!((c1.0 - 10.0).abs() < 1e-6, "the handle is at the point: {c1:?}");
    assert!((c1.1 - 0.0).abs() < 1e-6, "the handle is at the point: {c1:?}");
}

/// An end of an open contour has **one** neighbour, and it takes that
/// neighbour's direction.
#[test]
fn an_end_of_an_open_contour_takes_its_neighbours_direction() {
    let mut session = session();
    let id = a_corner(&mut session);
    session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [2], "to": "smooth" }))
        .expect("the point is converted");
    let sides = sides(&session, id);
    let (_, c2) = handles(&sides[1]).expect("the arriving side is a curve");
    // The last point is (0, 10) and its neighbour is (10, 0): the
    // handle is a third of the way back towards it.
    let expected_x = 10.0 / 3.0;
    let expected_y = 20.0 / 3.0;
    assert!((c2.0 - expected_x).abs() < 1e-6, "the handle is a third of the way: {c2:?}");
    assert!((c2.1 - expected_y).abs() < 1e-6, "the handle is a third of the way: {c2:?}");
}

/// A point whose **neighbours are one point** has no direction, and it
/// is left as it is.
#[test]
fn a_point_with_one_neighbour_point_is_left_alone() {
    let mut session = session();
    session
        .execute(
            "vector.path.create",
            &json!({
                "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 0.0, "y": 0.0 }]
            }),
        )
        .expect("the path is added");
    let id = paths(&session)[0];
    let before = sides(&session, id);
    let outcome =
        session.execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1], "to": "smooth" }));
    match outcome {
        Ok(_outcome) => {
            assert_eq!(sides(&session, id), before, "the path is as it was");
        }
        Err(_) => {
            assert_eq!(sides(&session, id), before, "the path is as it was");
        }
    }
}

/// A `to` that is **neither** smooth nor corner is refused.
#[test]
fn a_to_that_is_neither_is_refused() {
    let mut session = session();
    let id = a_corner(&mut session);
    let outcome =
        session.execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1], "to": "bent" }));
    assert!(outcome.is_err(), "the `to` that is neither is refused");
}

/// A point that is **not there** is refused.
#[test]
fn a_point_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_corner(&mut session);
    let outcome =
        session.execute("vector.path.anchors.convert", &json!({ "path": id, "points": [9], "to": "smooth" }));
    assert!(outcome.is_err(), "the point that is not there is refused");
}

/// The points that are **not a list** are refused.
#[test]
fn points_that_are_not_a_list_are_refused() {
    let mut session = session();
    let id = a_corner(&mut session);
    let outcome = session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": "here", "to": "smooth" }));
    assert!(outcome.is_err(), "the points that are not a list are refused");
}

/// A point that is **not a whole number** is refused.
#[test]
fn a_point_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let id = a_corner(&mut session);
    let outcome = session
        .execute("vector.path.anchors.convert", &json!({ "path": id, "points": [1.5], "to": "smooth" }));
    assert!(outcome.is_err(), "the point that is not whole is refused");
}
