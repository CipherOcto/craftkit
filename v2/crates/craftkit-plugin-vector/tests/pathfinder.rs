//! The **crop** and the **trim** of two paths.

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
fn a_rect(session: &mut Session, x: f64, y: f64, size: f64) -> u64 {
    let outcome = session
        .execute("vector.rectangle", &json!({ "x": x, "y": y, "width": size, "height": size }))
        .expect("the rectangle is made");
    outcome.data["id"].as_u64().expect("the path has an id")
}

/// The **facts** of a node, as the plan reports them.
fn facts_of(session: &Session, id: u64) -> Json {
    let plan = session.evaluate();
    plan.items.iter().find(|item| item.node == id).map(|item| item.value.clone()).unwrap_or(Json::Null)
}

/// The **area** a path covers, as the evaluator says it.
fn area_of(session: &Session, id: u64) -> f64 {
    facts_of(session, id)["area"].as_f64().unwrap_or_default()
}

/// The **bounds** of a path: where it starts and how big it is.
fn bounds_of(session: &Session, id: u64) -> Json {
    let node = session.document().node(id).expect("the path is there");
    let rect = node.rect.expect("the path has a box");
    json!({ "x": rect.x, "y": rect.y, "width": rect.w, "height": rect.h })
}

/// Two overlapping **rectangles**: the first is in front.
fn two_rects(session: &mut Session) -> (u64, u64) {
    let back = a_rect(session, 0.0, 0.0, 100.0);
    let front = a_rect(session, 50.0, 50.0, 100.0);
    (front, back)
}

/// The **crop** keeps what the front path was covering.
#[test]
fn the_crop_keeps_what_the_front_path_was_covering() {
    let mut session = session();
    let (front, back) = two_rects(&mut session);
    session
        .execute("vector.path.crop", &json!({ "front": front, "back": back }))
        .expect("the paths are cropped");
    assert!((area_of(&session, back) - 2500.0).abs() < 1.0, "the overlap is what is left");
    let bounds = bounds_of(&session, back);
    assert_eq!(bounds["x"], json!(50.0), "and it is where the overlap was");
    assert_eq!(bounds["y"], json!(50.0), "and where the overlap was");
}

/// The **crop** takes the front path away.
#[test]
fn the_crop_takes_the_front_path_away() {
    let mut session = session();
    let (front, back) = two_rects(&mut session);
    session
        .execute("vector.path.crop", &json!({ "front": front, "back": back }))
        .expect("the paths are cropped");
    assert!(session.document().node(front).is_none(), "the path in front is gone");
}

/// The **trim** takes the hidden part of the path behind away.
#[test]
fn the_trim_takes_the_hidden_part_of_the_path_behind_away() {
    let mut session = session();
    let (front, back) = two_rects(&mut session);
    session
        .execute("vector.path.trim", &json!({ "front": front, "back": back }))
        .expect("the paths are trimmed");
    assert!((area_of(&session, back) - 7500.0).abs() < 1.0, "the hidden part is gone");
    assert!((area_of(&session, front) - 10000.0).abs() < 1.0, "and the front keeps all of itself");
}

/// The **trim** leaves the two paths not overlapping.
#[test]
fn the_trim_leaves_the_two_paths_not_overlapping() {
    let mut session = session();
    let (front, back) = two_rects(&mut session);
    session
        .execute("vector.path.trim", &json!({ "front": front, "back": back }))
        .expect("the paths are trimmed");
    let bounds = bounds_of(&session, back);
    assert_eq!(bounds["x"], json!(0.0), "the path behind still starts at the left");
    assert_eq!(bounds["width"], json!(100.0), "and is as wide as it was");
    assert_eq!(bounds["height"], json!(100.0), "and as tall as it was");
}

/// A **crop** of two paths that do not meet is refused.
#[test]
fn a_crop_of_two_paths_that_do_not_meet_is_refused() {
    let mut session = session();
    let back = a_rect(&mut session, 0.0, 0.0, 10.0);
    let front = a_rect(&mut session, 50.0, 0.0, 10.0);
    let refused = session
        .execute("vector.path.crop", &json!({ "front": front, "back": back }))
        .expect_err("the crop is refused");
    assert!(refused.to_string().contains("do not meet"), "the refusal says why: {refused}");
}

/// The **selection** says which path is in front.
#[test]
fn the_selection_says_which_path_is_in_front() {
    let mut session = session();
    let (_front, back) = two_rects(&mut session);
    session.execute("vector.path.crop", &json!({})).expect_err("there is nothing selected");
    session.execute("select.all", &json!({})).expect("the paths are selected");
    session.execute("vector.path.crop", &json!({})).expect("the selection is cropped");
    assert!((area_of(&session, back) - 2500.0).abs() < 1.0, "the overlap is what is left");
}
