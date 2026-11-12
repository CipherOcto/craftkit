//! The **joining** of two open contours.
//!
//! The tree's rules: the two contours must be open, the ends that are
//! joined are the ones named or the nearest pair, the ends that are
//! already one place are joined without a jump, and the joined point
//! may be a corner or smooth.

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

/// The **sides** of a contour.
fn sides(contour: &Json) -> Vec<Json> {
    contour["segments"].as_array().expect("the contour has sides").clone()
}

/// Where a side ends.
fn end_of(side: &Json) -> (f64, f64) {
    let end = side
        .get("line")
        .and_then(|line| line.get("to"))
        .or_else(|| side.get("cubic").and_then(|cubic| cubic.get("to")))
        .expect("the side has an end");
    (
        end.get("x").and_then(|x| x.as_f64()).expect("the end is across"),
        end.get("y").and_then(|y| y.as_f64()).expect("the end is down"),
    )
}

/// The **handle** of a side at one of its ends.
fn handle_of(side: &Json, at_start: bool) -> Option<(f64, f64)> {
    let cubic = side.get("cubic")?;
    let key = if at_start { "c1" } else { "c2" };
    let point = cubic.get(key)?;
    Some((point.get("x")?.as_f64()?, point.get("y")?.as_f64()?))
}

/// A path of **two** open contours, and its id.
fn two_contours(session: &mut Session, data: &str) -> u64 {
    session.execute("vector.path.create", &json!({ "d": data })).expect("the path is added");
    paths(session)[0]
}

/// The two contours are **joined** into one.
#[test]
fn the_two_contours_are_joined() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    session
        .execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 1 }))
        .expect("the contours are joined");
    let contours = contours(&session, id);
    assert_eq!(contours.len(), 1, "the path has one contour now");
    let sides = sides(&contours[0]);
    assert_eq!(sides.len(), 3, "the joined contour has three sides");
    assert_eq!(end_of(&sides[0]), (10.0, 0.0), "the first side is as it was");
    assert_eq!(end_of(&sides[1]), (20.0, 0.0), "the side between them is straight");
    assert_eq!(end_of(&sides[2]), (30.0, 0.0), "the last side is as it was");
}

/// The ends that are already **one place** are joined without a jump:
/// no side is put between them.
#[test]
fn ends_that_are_one_place_are_joined_without_a_jump() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 10 0 L 20 0");
    session
        .execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 1 }))
        .expect("the contours are joined");
    let contours = contours(&session, id);
    assert_eq!(contours.len(), 1, "the path has one contour now");
    let sides = sides(&contours[0]);
    assert_eq!(sides.len(), 2, "the joined contour has two sides");
    assert_eq!(end_of(&sides[0]), (10.0, 0.0), "the first side is as it was");
    assert_eq!(end_of(&sides[1]), (20.0, 0.0), "the last side is as it was");
}

/// The ends that are **named** are the ends that are joined.
#[test]
fn the_ends_that_are_named_are_the_ends_that_are_joined() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    session
        .execute(
            "vector.path.join.ends",
            &json!({ "path": id, "first": 0, "second": 1, "ends": ["first", "last"] }),
        )
        .expect("the contours are joined");
    let contours = contours(&session, id);
    let sides = sides(&contours[0]);
    assert_eq!(sides.len(), 3, "the joined contour has three sides");
    assert_eq!(end_of(&sides[0]), (0.0, 0.0), "the first contour was turned round");
    assert_eq!(end_of(&sides[2]), (20.0, 0.0), "the second contour was turned round");
}

/// The joined point may be a **corner**: its handles come back to it.
#[test]
fn the_joined_point_may_be_a_corner() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    session
        .execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 1, "to": "corner" }))
        .expect("the contours are joined");
    let contours = contours(&session, id);
    let sides = sides(&contours[0]);
    let handle = handle_of(&sides[0], false).expect("the side is a curve");
    assert!((handle.0 - 10.0).abs() < 1e-6, "the handle is at the point: {handle:?}");
    assert!((handle.1 - 0.0).abs() < 1e-6, "the handle is at the point: {handle:?}");
}

/// The joined point may be **smooth**: the neighbours' rule.
#[test]
fn the_joined_point_may_be_smooth() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    session
        .execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 1, "to": "smooth" }))
        .expect("the contours are joined");
    let contours = contours(&session, id);
    let sides = sides(&contours[0]);
    // The point is (10, 0), its neighbours are (0, 0) and (20, 0): each
    // handle is a third of the way to that side's neighbour.
    let handle = handle_of(&sides[0], false).expect("the side is a curve");
    assert!((handle.0 - (10.0 - 10.0 / 3.0)).abs() < 1e-6, "the handle is a third of the way: {handle:?}");
    let handle = handle_of(&sides[1], true).expect("the side is a curve");
    assert!((handle.0 - (10.0 + 10.0 / 3.0)).abs() < 1e-6, "the handle is a third of the way: {handle:?}");
}

/// A **closed** contour is refused: the join needs open paths.
#[test]
fn a_closed_contour_is_refused() {
    let mut session = session();
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 20.0, "height": 20.0 }))
        .expect("the rectangle is added");
    let square = paths(&session)[0];
    let id = two_contours(&mut session, "M 0 0 L 10 0");
    let outcome = session.execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 1 }));
    assert!(outcome.is_err(), "the closed contour is refused");
    assert_eq!(paths(&session).len(), 2, "both paths are as they were");
    assert_eq!(contours(&session, square).len(), 1, "the rectangle is as it was");
}

/// A contour that is **not there** is refused.
#[test]
fn a_contour_that_is_not_there_is_refused() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    let outcome = session.execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 9 }));
    assert!(outcome.is_err(), "the contour that is not there is refused");
}

/// A contour cannot be joined **to itself**.
#[test]
fn a_contour_cannot_be_joined_to_itself() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    let outcome = session.execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 0 }));
    assert!(outcome.is_err(), "the contour joined to itself is refused");
}

/// A `to` that is **neither** corner nor smooth is refused.
#[test]
fn a_to_that_is_neither_is_refused() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    let outcome = session
        .execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 1, "to": "bent" }));
    assert!(outcome.is_err(), "the `to` that is neither is refused");
}

/// The ends that are **not** two of first and last are refused.
#[test]
fn ends_that_are_not_two_of_first_and_last_are_refused() {
    let mut session = session();
    let id = two_contours(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    let outcome = session
        .execute("vector.path.join.ends", &json!({ "path": id, "first": 0, "second": 1, "ends": ["first"] }));
    assert!(outcome.is_err(), "the ends that are not two of them are refused");
}
