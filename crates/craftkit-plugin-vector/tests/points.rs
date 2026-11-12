//! The **points** of a path: the anchors that may be moved, and the
//! handles that may be set.
//!
//! The tree's path is a list of anchors, and each anchor carries two
//! handles: the one the curve leaves by and the one it arrives by. v2
//! had a path that could be created and combined and not edited.

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

/// The **segment** of a contour, as the document holds it.
fn segment_of(session: &Session, id: u64, index: usize) -> craftkit_sdk::serde_json::Value {
    path_of(session, id)
        .get("subpaths")
        .and_then(|subpaths| subpaths.get(0))
        .and_then(|subpath| subpath.get("segments"))
        .and_then(|segments| segments.get(index))
        .cloned()
        .expect("the segment is there")
}

/// A **point** of a path may be moved: the path is different and the
/// box of the node has moved with it.
#[test]
fn a_point_may_be_moved() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.move", &json!({ "path": id, "point": 1, "to": { "x": 20.0, "y": 0.0 } }))
        .expect("the point is moved");
    let segment = segment_of(&session, id, 0);
    let end = segment.get("line").and_then(|line| line.get("to")).expect("the segment is a line");
    assert_eq!(end.get("x").and_then(|x| x.as_f64()), Some(20.0), "the point has moved: {end}");
    let rect = session.document().node(id).and_then(|node| node.rect).expect("the node has a box");
    assert_eq!(rect.w, 20.0, "the box has moved with the path: {rect:?}");
}

/// A **handle** on a point makes the segment a curve: a straight
/// segment with a handle on it is a curve.
#[test]
fn a_handle_makes_a_curve() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.handle", &json!({ "path": id, "point": 1, "c1": { "x": 15.0, "y": -5.0 } }))
        .expect("the handle is set");
    // The curve that leaves the point one is the second segment.
    let segment = segment_of(&session, id, 1);
    let curve = segment.get("cubic").expect("the segment is a curve: {segment}");
    let c1 = curve.get("c1").expect("the curve has a first control point");
    assert_eq!(c1.get("x").and_then(|x| x.as_f64()), Some(15.0), "the handle is where it was put");
    assert_eq!(c1.get("y").and_then(|y| y.as_f64()), Some(-5.0), "the handle is where it was put");
}

/// The handles of a point move **with the point**: an anchor carries
/// its handles.
#[test]
fn the_handles_move_with_the_point() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.handle", &json!({ "path": id, "point": 1, "c1": { "x": 15.0, "y": -5.0 } }))
        .expect("the handle is set");
    session
        .execute("vector.path.move", &json!({ "path": id, "point": 1, "to": { "x": 20.0, "y": 0.0 } }))
        .expect("the point is moved");
    let segment = segment_of(&session, id, 1);
    let curve = segment.get("cubic").expect("the segment is a curve: {segment}");
    let c1 = curve.get("c1").expect("the curve has a first control point");
    // The handle was at fifteen and the point moved by ten.
    assert_eq!(c1.get("x").and_then(|x| x.as_f64()), Some(25.0), "the handle moved with it: {c1}");
    assert_eq!(c1.get("y").and_then(|y| y.as_f64()), Some(-5.0), "the handle moved with it: {c1}");
}

/// A point that is **not there** is refused by name, and so is a
/// number that is not a point at all.
#[test]
fn a_point_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let refused = session
        .execute("vector.path.move", &json!({ "path": id, "point": 99, "to": { "x": 1.0, "y": 1.0 } }));
    let message = match refused {
        Ok(_) => panic!("a point that is not there is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("not a point 99"), "the refusal says why: {message}");
    let half = session
        .execute("vector.path.move", &json!({ "path": id, "point": 1.5, "to": { "x": 1.0, "y": 1.0 } }));
    assert!(half.is_err(), "a number that is not a point is refused");
}
