//! The **rectangle cut**: a shape cut to a rectangle.
//!
//! The tree's rules: the part of the shape inside the rectangle is
//! kept and the part outside is taken away; a shape wholly inside is
//! left as it was; a shape wholly outside is taken away whole; and the
//! rectangle is four numbers or two corners, and all four or nothing.

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

/// The **contours** of the path, as they are in the document.
fn contours(session: &Session, id: u64) -> Vec<Json> {
    let node = session.document().node(id).expect("the path is there");
    node.data["path"]["subpaths"].as_array().expect("the path has contours").clone()
}

/// The **box** around the path.
fn box_of(session: &Session, id: u64) -> (f64, f64, f64, f64) {
    let node = session.document().node(id).expect("the path is there");
    let rect = node.rect.expect("the path has a place");
    (rect.x, rect.y, rect.w, rect.h)
}

/// A **rectangle**, and its id.
fn a_rectangle(session: &mut Session, x: f64, y: f64, w: f64, h: f64) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": x, "y": y, "width": w, "height": h }))
        .expect("the rectangle is added");
    paths(session).last().copied().expect("the rectangle is there")
}

/// A shape is **cut to the rectangle**: the part inside is kept and the
/// part outside is taken away.
#[test]
fn a_shape_is_cut_to_the_rectangle() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0, 20.0, 20.0);
    session
        .execute("vector.path.cut.rect", &json!({ "path": id, "rect": [10.0, 0.0, 20.0, 20.0] }))
        .expect("the shape is cut");
    assert_eq!(paths(&session).len(), 1, "there is still one path");
    let (x, y, w, h) = box_of(&session, id);
    assert!((x - 10.0).abs() < 1e-6, "the shape begins at the rectangle: {x}");
    assert!((y - 0.0).abs() < 1e-6, "the shape begins at the rectangle: {y}");
    assert!((w - 10.0).abs() < 1e-6, "the shape is as wide as the rectangle: {w}");
    assert!((h - 20.0).abs() < 1e-6, "the shape is as tall as the rectangle: {h}");
}

/// A shape **wholly inside** the rectangle is left as it was.
#[test]
fn a_shape_wholly_inside_is_left_alone() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0, 10.0, 10.0);
    let before = contours(&session, id);
    let outcome = session
        .execute("vector.path.cut.rect", &json!({ "path": id, "rect": [0.0, 0.0, 40.0, 40.0] }))
        .expect("the cut runs");
    assert!(!outcome.changed, "nothing was cut: {outcome:?}");
    assert_eq!(contours(&session, id), before, "the shape is as it was");
}

/// A shape **wholly outside** the rectangle is taken away whole.
#[test]
fn a_shape_wholly_outside_is_taken_away() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0, 10.0, 10.0);
    let outcome = session
        .execute("vector.path.cut.rect", &json!({ "path": id, "rect": [100.0, 100.0, 10.0, 10.0] }))
        .expect("the cut runs");
    assert!(outcome.changed, "the shape was taken away: {outcome:?}");
    assert_eq!(paths(&session).len(), 0, "there is no path now");
}

/// The rectangle may also be **two corners**.
#[test]
fn two_corners_give_the_rectangle() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0, 20.0, 20.0);
    session
        .execute("vector.path.cut.rect", &json!({ "path": id, "from": [10.0, 0.0], "to": [20.0, 20.0] }))
        .expect("the shape is cut");
    let (x, _, w, _) = box_of(&session, id);
    assert!((x - 10.0).abs() < 1e-6, "the shape begins at the corner: {x}");
    assert!((w - 10.0).abs() < 1e-6, "the shape is as wide as the rectangle: {w}");
}

/// A rectangle with **three numbers** is refused: all four or nothing.
#[test]
fn a_rectangle_with_three_numbers_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0, 20.0, 20.0);
    let before = contours(&session, id);
    let outcome = session.execute("vector.path.cut.rect", &json!({ "path": id, "rect": [0.0, 0.0, 10.0] }));
    assert!(outcome.is_err(), "the rectangle with three numbers is refused");
    assert_eq!(contours(&session, id), before, "the shape is as it was");
}

/// A rectangle with **no width** is refused.
#[test]
fn a_rectangle_with_no_width_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0, 20.0, 20.0);
    let outcome =
        session.execute("vector.path.cut.rect", &json!({ "path": id, "rect": [0.0, 0.0, 0.0, 10.0] }));
    assert!(outcome.is_err(), "the rectangle with no width is refused");
}

/// A rectangle **beyond the bound** is refused.
#[test]
fn a_rectangle_beyond_the_bound_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0, 20.0, 20.0);
    let outcome =
        session.execute("vector.path.cut.rect", &json!({ "path": id, "rect": [0.0, 0.0, 1.0e9, 10.0] }));
    assert!(outcome.is_err(), "the rectangle beyond the bound is refused");
}

/// A rectangle that is **not there** is refused.
#[test]
fn a_rectangle_that_is_not_there_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session, 0.0, 0.0, 20.0, 20.0);
    let outcome = session.execute("vector.path.cut.rect", &json!({ "path": id }));
    assert!(outcome.is_err(), "the rectangle that is not there is refused");
}

/// An **open** path is refused: the rectangle cuts closed shapes.
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
    let outcome =
        session.execute("vector.path.cut.rect", &json!({ "path": id, "rect": [0.0, 0.0, 10.0, 10.0] }));
    assert!(outcome.is_err(), "the open path is refused");
}
