//! The **rule** the smooth point follows.
//!
//! The tree's rule: the handles at the point are the ones the
//! **curvature tool** gives it — along the circle through the point and
//! its two neighbours — and an end of an open contour takes its
//! neighbour's tangent, mirrored in the chord.

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

/// The **segments** of a path, as the document holds them.
fn segments_of(session: &Session, id: u64) -> Vec<Json> {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"][0]["segments"].as_array().expect("the path has segments").clone()
}

/// The **control point** of a side, if it is a curve.
fn control_of(side: &Json, name: &str) -> Option<(f64, f64)> {
    let curve = side.get("cubic")?;
    let value = curve.get(name)?;
    Some((value.get("x").and_then(|x| x.as_f64())?, value.get("y").and_then(|y| y.as_f64())?))
}

/// A path of three points, and its id.
fn a_path(session: &mut Session, anchors: Json) -> u64 {
    session.execute("vector.path.create", &json!({ "anchors": anchors })).expect("the path is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .last()
        .map(|node| node.id)
        .expect("the path is there")
}

/// The tree's own numbers: the handles of the middle point are along
/// the circle through the three points, and each is between a third and
/// two thirds of its own chord.
#[test]
fn the_handles_follow_the_circle_through_the_point() {
    let mut session = session();
    let id = a_path(
        &mut session,
        json!([{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 200.0, "y": 100.0 }]),
    );
    session.execute("vector.path.smooth", &json!({ "path": id, "point": 1 })).expect("the point is smooth");
    let segments = segments_of(&session, id);
    let back = control_of(&segments[0], "c2").expect("the side that arrives is a curve");
    let forward = control_of(&segments[1], "c1").expect("the side that leaves is a curve");
    let at: (f64, f64) = (10.0, 0.0);
    let neighbours: [(f64, f64); 2] = [(0.0, 0.0), (200.0, 100.0)];
    for (handle, neighbour) in [(back, neighbours[0]), (forward, neighbours[1])] {
        let chord = (at.0 - neighbour.0).hypot(at.1 - neighbour.1);
        let length = (handle.0 - at.0).hypot(handle.1 - at.1);
        let ratio = length / chord;
        assert!(
            (1.0 / 3.0 - 1e-9..=2.0 / 3.0 + 1e-9).contains(&ratio),
            "the handle is between a third and two thirds of its chord: {ratio}"
        );
    }
    // The two handles are in line: they point in opposite directions.
    let into = (at.0 - back.0, at.1 - back.1);
    let out = (forward.0 - at.0, forward.1 - at.1);
    let cross = into.0 * out.1 - into.1 * out.0;
    assert!(cross.abs() < 1e-9, "the handles are in line: {cross}");
    let dot = into.0 * out.0 + into.1 * out.1;
    assert!(dot > 0.0, "the point is between its two handles: {dot}");
}

/// A point with **no handles** may be made smooth: the rule makes the
/// handles, and it does not need them there first.
#[test]
fn a_corner_point_may_be_made_smooth() {
    let mut session = session();
    let id = a_path(
        &mut session,
        json!([{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 20.0, "y": 10.0 }]),
    );
    let before = segments_of(&session, id);
    assert!(before[0].get("line").is_some(), "the side is straight to begin with");
    session.execute("vector.path.smooth", &json!({ "path": id, "point": 1 })).expect("the point is smooth");
    let after = segments_of(&session, id);
    assert!(after[0].get("cubic").is_some(), "the side that arrives is a curve now");
    assert!(after[1].get("cubic").is_some(), "the side that leaves is a curve now");
}

/// The far end of a side that was straight keeps the **line's
/// direction**: the curve leaves it along the line it was.
#[test]
fn the_far_end_keeps_the_line() {
    let mut session = session();
    let id = a_path(
        &mut session,
        json!([{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 20.0, "y": 10.0 }]),
    );
    session.execute("vector.path.smooth", &json!({ "path": id, "point": 1 })).expect("the point is smooth");
    let segments = segments_of(&session, id);
    // The side that arrives started at the first point, which had no
    // handle of its own: the curve leaves it along the line.
    let start = control_of(&segments[0], "c1").expect("the side is a curve");
    assert_eq!(start, (0.0, 0.0), "the far end keeps the line's direction");
}

/// An end of an open contour takes its **neighbour's tangent**, mirrored
/// in the chord: the two handles are in line.
#[test]
fn an_end_takes_its_neighbours_tangent() {
    let mut session = session();
    let id = a_path(
        &mut session,
        json!([{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 20.0, "y": 10.0 }]),
    );
    // The neighbour gets its handle first: an end takes the tangent of
    // a neighbour that has one.
    session.execute("vector.path.smooth", &json!({ "path": id, "point": 1 })).expect("the point is smooth");
    session.execute("vector.path.smooth", &json!({ "path": id, "point": 0 })).expect("the end is smooth");
    let segments = segments_of(&session, id);
    let end = control_of(&segments[0], "c1").expect("the side that leaves the end is a curve");
    let tangent = control_of(&segments[1], "c1").expect("the side that leaves the neighbour is a curve");
    // The end leaves the chord at the angle the curve meets its
    // neighbour at: the same angle, the other side of the chord.
    let chord = (10.0_f64, 0.0);
    let handle = (end.0 - 0.0, end.1 - 0.0);
    let neighbour = (tangent.0 - 10.0, tangent.1 - 0.0);
    let angle = |one: (f64, f64), other: (f64, f64)| {
        (one.0 * other.0 + one.1 * other.1) / (one.0.hypot(one.1) * other.0.hypot(other.1))
    };
    assert!(
        (angle(handle, chord) - angle(neighbour, chord)).abs() < 1e-9,
        "the end leaves the chord at the angle the curve meets its neighbour at"
    );
}

/// A point whose **neighbours are one point** has no direction, and it
/// is refused by name.
#[test]
fn a_point_with_no_direction_is_refused() {
    let mut session = session();
    let id = a_path(
        &mut session,
        json!([{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 0.0, "y": 0.0 }]),
    );
    let refused = session.execute("vector.path.smooth", &json!({ "path": id, "point": 1 }));
    let message = match refused {
        Ok(_) => panic!("the point with no direction is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("direction"), "the refusal says why: {message}");
}
