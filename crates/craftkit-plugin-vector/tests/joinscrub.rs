//! The **join scrub**: the ends near a drag joined together.
//!
//! The tree's rules: the ends near the drag are the ones that are
//! joined, they are ordered along the drag, and the first two are
//! joined — the two ends of one contour close it.

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

/// A path of **open** contours, and its id.
fn a_path(session: &mut Session, data: &str) -> u64 {
    session.execute("vector.path.create", &json!({ "d": data })).expect("the path is added");
    paths(session)[0]
}

/// The two ends near the drag are **joined** into one contour.
#[test]
fn the_two_ends_near_the_drag_are_joined() {
    let mut session = session();
    let id = a_path(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    session
        .execute("vector.path.join.scrub", &json!({ "path": id, "points": [[15.0, -5.0], [15.0, 5.0]] }))
        .expect("the ends are joined");
    let contours = contours(&session, id);
    assert_eq!(contours.len(), 1, "the path has one contour now");
    let sides = sides(&contours[0]);
    assert_eq!(sides.len(), 3, "the joined contour has three sides");
    assert_eq!(end_of(&sides[0]), (10.0, 0.0), "the first side is as it was");
    assert_eq!(end_of(&sides[1]), (20.0, 0.0), "the side between them is straight");
    assert_eq!(end_of(&sides[2]), (30.0, 0.0), "the last side is as it was");
}

/// The ends are **ordered along the drag**, and the first two are the
/// ones that are joined.
#[test]
fn the_ends_are_ordered_along_the_drag() {
    let mut session = session();
    let id = a_path(&mut session, "M 0 0 L 100 0 M 20 0 L 30 0");
    session
        .execute("vector.path.join.scrub", &json!({ "path": id, "points": [[-20.0, 0.0], [50.0, 0.0]] }))
        .expect("the ends are joined");
    let contours = contours(&session, id);
    assert_eq!(contours.len(), 1, "the path has one contour now");
    let sides = sides(&contours[0]);
    assert_eq!(end_of(&sides[0]), (0.0, 0.0), "the first contour was turned round");
    assert_eq!(end_of(&sides[1]), (20.0, 0.0), "the second contour is joined to it");
    assert_eq!(end_of(&sides[2]), (30.0, 0.0), "the second contour is as it was");
}

/// The two ends of **one** contour close it.
#[test]
fn the_two_ends_of_one_contour_close_it() {
    let mut session = session();
    let id = a_path(&mut session, "M 0 0 L 5 0 L 5 5");
    session
        .execute("vector.path.join.scrub", &json!({ "path": id, "points": [[2.0, 2.0]] }))
        .expect("the ends are joined");
    let contours = contours(&session, id);
    assert_eq!(contours.len(), 1, "the path has one contour");
    assert_eq!(contours[0]["closed"], json!(true), "the contour is closed: {contours:?}");
}

/// A scrub with **fewer than two** ends near it changes nothing.
#[test]
fn a_scrub_with_fewer_than_two_ends_changes_nothing() {
    let mut session = session();
    let id = a_path(&mut session, "M 0 0 L 100 0");
    let before = contours(&session, id);
    let outcome = session
        .execute("vector.path.join.scrub", &json!({ "path": id, "points": [[50.0, 0.0]] }))
        .expect("the scrub runs");
    assert!(!outcome.changed, "nothing was joined: {outcome:?}");
    assert_eq!(contours(&session, id), before, "the path is as it was");
}

/// A **tolerance** that is negative is refused.
#[test]
fn a_tolerance_that_is_negative_is_refused() {
    let mut session = session();
    let id = a_path(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    let outcome = session.execute(
        "vector.path.join.scrub",
        &json!({ "path": id, "points": [[15.0, 0.0]], "tolerance": -1.0 }),
    );
    assert!(outcome.is_err(), "the tolerance that is negative is refused");
}

/// A scrub that is **not a list of points** is refused.
#[test]
fn a_scrub_that_is_not_a_list_of_points_is_refused() {
    let mut session = session();
    let id = a_path(&mut session, "M 0 0 L 10 0 M 20 0 L 30 0");
    let outcome = session.execute("vector.path.join.scrub", &json!({ "path": id, "points": "here" }));
    assert!(outcome.is_err(), "the scrub that is not a list is refused");
}
