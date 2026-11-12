//! The **corner** of a point.
//!
//! A point with handles on it is a curve; a point with its handles
//! pulled back to it is a corner, and the two sides leave it as
//! straight lines.

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

/// The **segments** of the first contour, as the document holds them.
fn segments_of(session: &Session, id: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    path_of(session, id)
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(0))
        .and_then(|contour| contour.get("segments"))
        .and_then(|segments| segments.as_array().cloned())
        .expect("the contour has segments")
}

/// The **control point** of a segment: the one the curve leaves by, or
/// the one it arrives by.
fn control_of(segment: &craftkit_sdk::serde_json::Value, which: &str) -> Option<(f64, f64)> {
    let cubic = segment.get("cubic")?;
    let point = cubic.get(which)?;
    Some((point.get("x").and_then(|x| x.as_f64())?, point.get("y").and_then(|y| y.as_f64())?))
}

/// A rectangle whose **second point** has handles on both sides.
fn a_curved_rectangle(session: &mut Session) -> u64 {
    let id = a_rectangle(session);
    session
        .execute(
            "vector.path.handle",
            &json!({
                "path": id,
                "point": 1,
                "c1": { "x": 12.0, "y": 0.0 },
                "c2": { "x": 8.0, "y": 0.0 },
            }),
        )
        .expect("the handles are set");
    id
}

/// The handles at a point are **pulled back** to it.
#[test]
fn the_handles_at_the_point_are_pulled_back_to_it() {
    let mut session = session();
    let id = a_curved_rectangle(&mut session);
    let before = segments_of(&session, id);
    assert_eq!(control_of(&before[1], "c1"), Some((12.0, 0.0)), "the curve leaves the point by its handle");
    assert_eq!(control_of(&before[0], "c2"), Some((8.0, 0.0)), "and arrives by its other one");
    session.execute("vector.path.corner", &json!({ "path": id, "point": 1 })).expect("the point is a corner");
    let after = segments_of(&session, id);
    assert_eq!(
        control_of(&after[1], "c1"),
        Some((10.0, 0.0)),
        "the handle that leaves the point is at the point"
    );
    assert_eq!(control_of(&after[0], "c2"), Some((10.0, 0.0)), "and the one that arrives is too");
}

/// The handles at the **other ends** of the two sides are not touched:
/// they belong to other points.
#[test]
fn the_handles_at_the_other_ends_are_not_touched() {
    let mut session = session();
    let id = a_curved_rectangle(&mut session);
    let before = segments_of(&session, id);
    session.execute("vector.path.corner", &json!({ "path": id, "point": 1 })).expect("the point is a corner");
    let after = segments_of(&session, id);
    assert_eq!(control_of(&after[0], "c1"), control_of(&before[0], "c1"), "the first handle has not moved");
    assert_eq!(control_of(&after[1], "c2"), control_of(&before[1], "c2"), "and the last one has not");
}

/// The point itself does not **move**.
#[test]
fn the_point_does_not_move() {
    let mut session = session();
    let id = a_curved_rectangle(&mut session);
    let before = segments_of(&session, id);
    session.execute("vector.path.corner", &json!({ "path": id, "point": 1 })).expect("the point is a corner");
    let after = segments_of(&session, id);
    for index in 0..before.len() {
        let end = |segment: &craftkit_sdk::serde_json::Value| {
            segment
                .get("line")
                .and_then(|line| line.get("to"))
                .or_else(|| segment.get("cubic").and_then(|cubic| cubic.get("to")))
                .cloned()
        };
        assert_eq!(end(&after[index]), end(&before[index]), "the segment ends where it did");
    }
}

/// A point with **no handles** is refused: it is a corner already.
#[test]
fn a_point_with_no_handles_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let outcome = session.execute("vector.path.corner", &json!({ "path": id, "point": 1 }));
    assert!(outcome.is_err(), "there are no handles to pull back");
}

/// A point that is **not there** is refused.
#[test]
fn a_point_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_curved_rectangle(&mut session);
    let outcome = session.execute("vector.path.corner", &json!({ "path": id, "point": 9 }));
    assert!(outcome.is_err(), "the point is refused");
}

/// A contour that is **not there** is refused.
#[test]
fn a_contour_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_curved_rectangle(&mut session);
    let outcome = session.execute("vector.path.corner", &json!({ "path": id, "contour": 3, "point": 0 }));
    assert!(outcome.is_err(), "the contour is refused");
}
