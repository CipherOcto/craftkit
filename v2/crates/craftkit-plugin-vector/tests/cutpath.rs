//! The **cutting** of a path.
//!
//! The tree's rule: a **closed** path opens at the cut and an **open**
//! one becomes one path per piece, and the shape does not change — the
//! pieces are the sides that were there.

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

/// The path of a node, as the document holds it.
fn path_of(session: &Session, id: u64) -> Json {
    let node = session.document().node(id).expect("the node is there");
    node.data.get("path").cloned().expect("the node has a path")
}

/// The **contours** of a path.
fn contours_of(session: &Session, id: u64) -> Vec<Json> {
    path_of(session, id)
        .get("subpaths")
        .and_then(|subpaths| subpaths.as_array())
        .cloned()
        .expect("the path has contours")
}

/// The **sides** of a contour.
fn sides_of(contour: &Json) -> Vec<Json> {
    contour.get("segments").and_then(|segments| segments.as_array()).cloned().expect("the contour has sides")
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

/// An **open path** of two sides: a triangle that is not closed.
fn an_open_path(session: &mut Session) -> u64 {
    session
        .execute(
            "vector.path.create",
            &json!({
                "anchors": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 10.0, "y": 0.0 },
                    { "x": 10.0, "y": 10.0 }
                ]
            }),
        )
        .expect("the path is added");
    paths(session)[0]
}

/// A **closed path**: a rectangle.
fn a_closed_path(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    paths(session)[0]
}

/// An **open path** is cut into **two**, and the pieces are paths of
/// their own.
#[test]
fn an_open_path_is_cut_in_two() {
    let mut session = session();
    let id = an_open_path(&mut session);
    session.execute("vector.path.cut", &json!({ "path": id, "point": 1 })).expect("the path is cut");
    assert_eq!(paths(&session).len(), 2, "the path is two paths now");
    let first = contours_of(&session, id);
    assert_eq!(first.len(), 1, "the first piece is one contour");
    let sides = sides_of(&first[0]);
    assert_eq!(sides.len(), 1, "the first piece is one side");
    assert_eq!(end_of(&sides[0]), (10.0, 0.0), "the first piece ends where the cut was");
}

/// The piece that is **not** the first is a path of its own, with the
/// same name.
#[test]
fn the_pieces_are_paths_of_their_own() {
    let mut session = session();
    let id = an_open_path(&mut session);
    let outcome =
        session.execute("vector.path.cut", &json!({ "path": id, "point": 1 })).expect("the path is cut");
    let ids = outcome.data.get("ids").and_then(|ids| ids.as_array());
    let ids: Vec<u64> = ids.map(|ids| ids.iter().filter_map(|id| id.as_u64()).collect()).unwrap_or_default();
    assert_eq!(ids.len(), 2, "the cut says what it made: {ids:?}");
    assert_eq!(ids[0], id, "the first piece keeps the node");
    let other = ids[1];
    let name = session.document().node(id).map(|node| node.name.clone());
    let other_name = session.document().node(other).map(|node| node.name.clone());
    assert_eq!(name, other_name, "the piece is named as the path it was cut from");
    let sides = sides_of(&contours_of(&session, other)[0]);
    assert_eq!(sides.len(), 1, "the second piece is one side");
    assert_eq!(end_of(&sides[0]), (10.0, 10.0), "the second piece ends where the path did");
}

/// A **closed** path opens at the cut: it is one piece whose ends meet
/// where the cut was.
#[test]
fn a_closed_path_opens_at_the_cut() {
    let mut session = session();
    let id = a_closed_path(&mut session);
    session.execute("vector.path.cut", &json!({ "path": id, "point": 0 })).expect("the path is cut");
    assert_eq!(paths(&session).len(), 1, "the path is one path still");
    let contours = contours_of(&session, id);
    assert_eq!(contours.len(), 1, "the path has one contour");
    assert_eq!(
        contours[0].get("closed").and_then(|closed| closed.as_bool()),
        Some(false),
        "the contour is open now"
    );
    let sides = sides_of(&contours[0]);
    assert_eq!(sides.len(), 4, "the three sides of the rectangle and the one that closed it");
    assert_eq!(end_of(&sides[3]), (0.0, 0.0), "the ends meet where the cut was");
}

/// The shape does **not** change: the piece is where the path was.
#[test]
fn the_shape_does_not_change() {
    let mut session = session();
    let id = a_closed_path(&mut session);
    let before = session.document().node(id).and_then(|node| node.rect);
    session.execute("vector.path.cut", &json!({ "path": id, "point": 0 })).expect("the path is cut");
    let after = session.document().node(id).and_then(|node| node.rect);
    assert_eq!(before, after, "the path is in the same place: {before:?} and {after:?}");
}

/// A path of **several contours** keeps its contours: the pieces stay
/// contours of it.
#[test]
fn a_path_of_several_contours_keeps_its_contours() {
    let mut session = session();
    let id = a_closed_path(&mut session);
    session
        .execute(
            "vector.path.anchors.set",
            &json!({
                "path": id,
                "subpaths": [
                    { "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 10.0, "y": 0.0 }, { "x": 10.0, "y": 10.0 }] },
                    { "anchors": [{ "x": 0.0, "y": 0.0 }, { "x": 5.0, "y": 5.0 }], "closed": true }
                ]
            }),
        )
        .expect("the anchors are set");
    session
        .execute("vector.path.cut", &json!({ "path": id, "contour": 1, "point": 0 }))
        .expect("the path is cut");
    assert_eq!(paths(&session).len(), 1, "the path is one path still");
    let contours = contours_of(&session, id);
    assert_eq!(contours.len(), 2, "the path has its two contours");
    assert_eq!(
        contours[1].get("closed").and_then(|closed| closed.as_bool()),
        Some(false),
        "the cut contour is open now"
    );
}

/// The **ends** of an open contour are not cut: a piece with no side has
/// no shape.
#[test]
fn the_ends_of_an_open_contour_are_not_cut() {
    let mut session = session();
    let id = an_open_path(&mut session);
    let outcome = session.execute("vector.path.cut", &json!({ "path": id, "point": 0 }));
    assert!(outcome.is_err(), "the end of an open contour is not cut");
    assert_eq!(paths(&session).len(), 1, "the path is as it was");
}

/// A point that is not a **whole number** is refused.
#[test]
fn a_point_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let id = an_open_path(&mut session);
    let outcome = session.execute("vector.path.cut", &json!({ "path": id, "point": 1.5 }));
    assert!(outcome.is_err(), "the point that is not whole is refused");
}

/// A contour that is **not there** is refused.
#[test]
fn a_contour_that_is_not_there_is_refused() {
    let mut session = session();
    let id = an_open_path(&mut session);
    let outcome = session.execute("vector.path.cut", &json!({ "path": id, "contour": 3, "point": 1 }));
    assert!(outcome.is_err(), "the contour that is not there is refused");
}
