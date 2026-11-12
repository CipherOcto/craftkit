//! The **divide** and the **merge** of two paths.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **rectangle** path, and its id.
fn a_rect(session: &mut Session, x: f64, y: f64, size: f64, fill: &str) -> u64 {
    let outcome = session
        .execute("vector.rectangle", &json!({ "x": x, "y": y, "width": size, "height": size, "fill": fill }))
        .expect("the rectangle is made");
    outcome.data["id"].as_u64().expect("the path has an id")
}

/// The **area** a path covers, as the evaluator says it.
fn area_of(session: &Session, id: u64) -> f64 {
    let plan = session.evaluate();
    let mut area = 0.0;
    for item in &plan.items {
        if item.node == id
            && let Some(value) = item.value.get("area").and_then(|area| area.as_f64())
        {
            area = value;
        }
    }
    area
}

/// The **paths** of the document, in the order they are in it.
fn paths_of(session: &Session) -> Vec<u64> {
    let plan = session.evaluate();
    plan.items.iter().filter(|item| item.node != 0).map(|item| item.node).collect()
}

/// The **fill** a path is painted with.
fn fill_of(session: &Session, id: u64) -> Json {
    let plan = session.evaluate();
    plan.items
        .iter()
        .find(|item| item.node == id)
        .map(|item| item.value.get("fill").cloned().unwrap_or(Json::Null))
        .unwrap_or(Json::Null)
}

/// Two overlapping **rectangles**: the first is in front.
fn two_rects(session: &mut Session) -> (u64, u64) {
    let back = a_rect(session, 0.0, 0.0, 100.0, "#ff0000");
    let front = a_rect(session, 50.0, 50.0, 100.0, "#0000ff");
    (front, back)
}

/// The **divide** cuts the paths at the places where they meet.
#[test]
fn the_divide_cuts_the_paths_at_the_places_where_they_meet() {
    let mut session = session();
    let (front, back) = two_rects(&mut session);
    session
        .execute("vector.path.divide", &json!({ "front": front, "back": back }))
        .expect("the paths are divided");
    // The part of the path behind that the path in front does not
    // cover.
    assert!((area_of(&session, back) - 7500.0).abs() < 1.0, "the back keeps its own part");
    // The part of the path in front that the path behind does not
    // cover.
    assert!((area_of(&session, front) - 7500.0).abs() < 1.0, "the front keeps its own part");
}

/// The **divide** makes a path for the part the paths **share**.
#[test]
fn the_divide_makes_a_path_for_the_part_the_paths_share() {
    let mut session = session();
    let (front, back) = two_rects(&mut session);
    session
        .execute("vector.path.divide", &json!({ "front": front, "back": back }))
        .expect("the paths are divided");
    let paths = paths_of(&session);
    assert_eq!(paths.len(), 3, "there is a face for each part");
    let shared = paths.into_iter().find(|id| *id != front && *id != back).expect("the shared face is there");
    assert!((area_of(&session, shared) - 2500.0).abs() < 1.0, "the shared face is the overlap");
}

/// The shared face takes the **style of the path in front**.
#[test]
fn the_shared_face_takes_the_style_of_the_path_in_front() {
    let mut session = session();
    let (front, back) = two_rects(&mut session);
    let front_fill = fill_of(&session, front);
    session
        .execute("vector.path.divide", &json!({ "front": front, "back": back }))
        .expect("the paths are divided");
    let shared = paths_of(&session)
        .into_iter()
        .find(|id| *id != front && *id != back)
        .expect("the shared face is there");
    assert_eq!(fill_of(&session, shared), front_fill, "the face is painted as the front path is");
}

/// The **divide** of two paths that do not meet makes two faces.
#[test]
fn the_divide_of_two_paths_that_do_not_meet_makes_two_faces() {
    let mut session = session();
    let back = a_rect(&mut session, 0.0, 0.0, 10.0, "#ff0000");
    let front = a_rect(&mut session, 50.0, 0.0, 10.0, "#0000ff");
    session
        .execute("vector.path.divide", &json!({ "front": front, "back": back }))
        .expect("the paths are divided");
    assert_eq!(paths_of(&session).len(), 2, "there is nothing shared");
}

/// The **merge** of two paths painted the same way makes one path.
#[test]
fn the_merge_of_two_paths_painted_the_same_way_makes_one_path() {
    let mut session = session();
    let back = a_rect(&mut session, 0.0, 0.0, 100.0, "#0000ff");
    let front = a_rect(&mut session, 50.0, 50.0, 100.0, "#0000ff");
    session
        .execute("vector.path.merge", &json!({ "front": front, "back": back }))
        .expect("the paths are merged");
    let paths = paths_of(&session);
    assert_eq!(paths.len(), 1, "the paths are one now");
    assert!((area_of(&session, paths[0]) - 17500.0).abs() < 1.0, "and they cover both");
}

/// The **merge** of two paths painted differently keeps them apart.
#[test]
fn the_merge_of_two_paths_painted_differently_keeps_them_apart() {
    let mut session = session();
    let (front, back) = two_rects(&mut session);
    session
        .execute("vector.path.merge", &json!({ "front": front, "back": back }))
        .expect("the paths are merged");
    assert_eq!(paths_of(&session).len(), 2, "the paths are two still");
    assert!((area_of(&session, back) - 7500.0).abs() < 1.0, "the hidden part is gone");
    assert!((area_of(&session, front) - 10000.0).abs() < 1.0, "and the front keeps all of itself");
}
