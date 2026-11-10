//! The **knife**: a closed shape cut along a run of points.
//!
//! The tree's rules: the knife needs two points, the slit between the
//! pieces has a width of its own, and only closed shapes are cut — an
//! open path has no inside, and there is nothing to cut it into.

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

/// The **left and right edges** of the shape, as the box around it.
fn box_of(session: &Session, id: u64) -> (f64, f64, f64, f64) {
    let node = session.document().node(id).expect("the path is there");
    let rect = node.rect.expect("the path has a place");
    (rect.x, rect.y, rect.w, rect.h)
}

/// A **rectangle** twenty by twenty, and its id.
fn a_square(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 20.0, "height": 20.0 }))
        .expect("the rectangle is added");
    paths(session)[0]
}

/// The **ids** the outcome says it made.
fn ids_of(outcome: &craftkit_sdk::Outcome) -> Vec<u64> {
    outcome
        .data
        .get("ids")
        .and_then(|ids| ids.as_array())
        .map(|ids| ids.iter().filter_map(|id| id.as_u64()).collect())
        .unwrap_or_default()
}

/// A closed shape is **cut in two** by a knife that crosses it.
#[test]
fn a_closed_shape_is_cut_in_two() {
    let mut session = session();
    let id = a_square(&mut session);
    session
        .execute("vector.path.knife", &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 25.0]] }))
        .expect("the shape is cut");
    assert_eq!(paths(&session).len(), 2, "the square is two paths now");
}

/// The pieces are **nodes of their own**, and the first keeps the node
/// that was cut.
#[test]
fn the_pieces_are_nodes_of_their_own() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session
        .execute("vector.path.knife", &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 25.0]] }))
        .expect("the shape is cut");
    let ids = ids_of(&outcome);
    assert_eq!(ids.len(), 2, "the knife says what it made: {ids:?}");
    assert_eq!(ids[0], id, "the first piece keeps the node");
    assert_ne!(ids[1], id, "the second piece is a new node");
}

/// The knife leaves a **slit** between the pieces, so that they are
/// apart and not merely touching.
#[test]
fn the_pieces_are_apart() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session
        .execute("vector.path.knife", &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 25.0]] }))
        .expect("the shape is cut");
    let ids = ids_of(&outcome);
    let (first_x, _, first_width, _) = box_of(&session, ids[0]);
    let (second_x, _, _, _) = box_of(&session, ids[1]);
    let gap = (second_x - (first_x + first_width)).abs();
    assert!(gap > 0.0, "the pieces are apart: {gap}");
    assert!(gap < 0.2, "the slit is narrow: {gap}");
}

/// A knife that **misses** the shape leaves it as it was.
#[test]
fn a_knife_that_misses_leaves_the_shape_alone() {
    let mut session = session();
    let id = a_square(&mut session);
    let before = contours(&session, id);
    let outcome = session
        .execute("vector.path.knife", &json!({ "path": id, "points": [[50.0, -5.0], [50.0, 25.0]] }))
        .expect("the knife runs");
    assert!(!outcome.changed, "nothing was cut: {outcome:?}");
    assert_eq!(contours(&session, id), before, "the shape is as it was");
    assert_eq!(paths(&session).len(), 1, "there is still one path");
}

/// An **open** path is refused: it has no inside, and there is nothing
/// to cut it into.
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
        session.execute("vector.path.knife", &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 5.0]] }));
    assert!(outcome.is_err(), "the open path is refused");
}

/// A knife with **one point** is refused: a run needs two.
#[test]
fn a_knife_with_one_point_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session.execute("vector.path.knife", &json!({ "path": id, "points": [[10.0, -5.0]] }));
    assert!(outcome.is_err(), "the knife with one point is refused");
}

/// A run that is **not a list of points** is refused.
#[test]
fn a_run_that_is_not_a_list_of_points_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session.execute("vector.path.knife", &json!({ "path": id, "points": "here" }));
    assert!(outcome.is_err(), "the run that is not a list is refused");
}

/// A point of the run that has **no numbers** in it is refused.
#[test]
fn a_point_with_no_numbers_is_refused() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session
        .execute("vector.path.knife", &json!({ "path": id, "points": [[10.0, -5.0], ["there", 25.0]] }));
    assert!(outcome.is_err(), "the point with no numbers is refused");
}

/// The pieces are **named** after the shape they were cut from.
#[test]
fn the_pieces_are_named_after_the_shape() {
    let mut session = session();
    let id = a_square(&mut session);
    let outcome = session
        .execute("vector.path.knife", &json!({ "path": id, "points": [[10.0, -5.0], [10.0, 25.0]] }))
        .expect("the shape is cut");
    let ids = ids_of(&outcome);
    let node = session.document().node(ids[1]).expect("the piece is there");
    let name = node.name.clone();
    assert!(!name.is_empty(), "the piece has a name");
}
