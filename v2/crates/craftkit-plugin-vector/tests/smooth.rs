//! The **smooth** point: a point whose two handles are one.
//!
//! A smooth point's handles are collinear and of the same length: the
//! curve passes through the point as one curve and not as two. The
//! command makes it so, and a side that was a straight line becomes a
//! curve with a handle.

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

/// A **triangle** of three straight sides, and its id.
fn a_triangle(session: &mut Session) -> u64 {
    session
        .execute("vector.polygon", &json!({ "sides": 3, "radius": 10.0, "x": 0.0, "y": 0.0 }))
        .expect("the triangle is added");
    session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .last()
        .map(|node| node.id)
        .expect("the triangle is there")
}

/// The **start** of a path's first contour.
fn start_of(session: &Session, id: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"][0]["start"].clone()
}

/// The **segments** of a path, as the document holds them.
fn segments_of(session: &Session, id: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"][0]["segments"].as_array().expect("the path has segments").clone()
}

/// A point with a curve on one side and a straight line on the other
/// may be made **smooth**: the straight side becomes a curve, and the
/// two handles are collinear.
#[test]
fn a_point_with_a_curve_on_one_side_may_be_made_smooth() {
    let mut session = session();
    let id = a_triangle(&mut session);
    // A handle on the side that leaves the first point.
    session
        .execute("vector.path.handle", &json!({ "path": id, "point": 0, "c1": { "x": 5.0, "y": 0.0 } }))
        .expect("the handle is there");
    let segments = segments_of(&session, id);
    assert!(
        segments[0].as_object().expect("a side is an object").contains_key("cubic"),
        "the side is a curve"
    );
    // Now the point is smooth.
    session.execute("vector.path.smooth", &json!({ "path": id, "point": 0 })).expect("the point is smooth");
    let segments = segments_of(&session, id);
    // The closing edge of the contour was **implicit**, and a handle
    // needs a segment to hold it: the edge is a segment of its own
    // now, and it is a curve.
    assert_eq!(segments.len(), 3, "the contour has a closing edge of its own");
    // The two sides that meet at the point are curves: the one that
    // had the handle, and the one that was a straight line.
    let leaving = segments[0]["cubic"].as_object().expect("the side is a curve");
    let arriving = segments[2]["cubic"].as_object().expect("the side is a curve");
    // The two handles are collinear: they point in opposite directions
    // from the point, and they are the same length.
    let point = start_of(&session, id).as_object().expect("the point is there").clone();
    let out = leaving["c1"].as_object().expect("the handle is there");
    let back = arriving["c2"].as_object().expect("the handle is there");
    let dx = out["x"].as_f64().unwrap_or(0.0) - point["x"].as_f64().unwrap_or(0.0);
    let dy = out["y"].as_f64().unwrap_or(0.0) - point["y"].as_f64().unwrap_or(0.0);
    let ex = back["x"].as_f64().unwrap_or(0.0) - point["x"].as_f64().unwrap_or(0.0);
    let ey = back["y"].as_f64().unwrap_or(0.0) - point["y"].as_f64().unwrap_or(0.0);
    assert!((dx + ex).abs() < 1e-9 && (dy + ey).abs() < 1e-9, "the two handles are collinear");
    let out_len = (dx * dx + dy * dy).sqrt();
    let back_len = (ex * ex + ey * ey).sqrt();
    assert!((out_len - back_len).abs() < 1e-9, "and they are the same length");
}

/// A point with **no curve at all** has nothing to smooth, and a
/// point that is not there is refused by name.
#[test]
fn a_point_that_cannot_be_made_smooth_is_refused() {
    let mut session = session();
    let id = a_triangle(&mut session);
    // A point between two straight lines has no curvature to smooth.
    let refused = session.execute("vector.path.smooth", &json!({ "path": id, "point": 1 }));
    let message = match refused {
        Ok(_) => panic!("a point with no handles is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("handles"), "the refusal says why: {message}");
    // A point that is not there is refused too.
    let refused = session.execute("vector.path.smooth", &json!({ "path": id, "point": 99 }));
    let message = match refused {
        Ok(_) => panic!("a point that is not there is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("points"), "the refusal says why: {message}");
}
