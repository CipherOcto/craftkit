//! The pathfinder on a whole **selection**.

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

/// A **rectangle** path, and its id.
fn a_rect(session: &mut Session, x: f64, y: f64, size: f64) -> u64 {
    let outcome = session
        .execute("vector.rectangle", &json!({ "x": x, "y": y, "width": size, "height": size }))
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

/// Three **rectangles** in a row, each overlapping the next: the first
/// is the one in front.
fn three_rects(session: &mut Session) -> (u64, u64, u64) {
    let back = a_rect(session, 0.0, 0.0, 100.0);
    let middle = a_rect(session, 50.0, 0.0, 100.0);
    let front = a_rect(session, 100.0, 0.0, 100.0);
    (front, middle, back)
}

/// The **trim** takes the hidden part of **every** path away.
#[test]
fn the_trim_takes_the_hidden_part_of_every_path_away() {
    let mut session = session();
    let (front, middle, back) = three_rects(&mut session);
    session.execute("select.all", &json!({})).expect("the paths are selected");
    session.execute("vector.path.trim", &json!({})).expect("the paths are trimmed");
    // The path in front keeps all of itself.
    assert!((area_of(&session, front) - 10000.0).abs() < 1.0, "the front keeps all of itself");
    // The path in the middle loses the part the front was covering.
    assert!((area_of(&session, middle) - 5000.0).abs() < 1.0, "the middle loses what the front covered");
    // The path behind loses what the front and the middle were
    // covering.
    assert!((area_of(&session, back) - 5000.0).abs() < 1.0, "the back loses what the two covered");
}

/// The **trim** takes a path that is hidden altogether away.
#[test]
fn the_trim_takes_a_path_that_is_hidden_altogether_away() {
    let mut session = session();
    let back = a_rect(&mut session, 0.0, 0.0, 100.0);
    let front = a_rect(&mut session, 0.0, 0.0, 100.0);
    session.execute("select.all", &json!({})).expect("the paths are selected");
    session.execute("vector.path.trim", &json!({})).expect("the paths are trimmed");
    assert_eq!(paths_of(&session), vec![front], "the hidden path is gone");
    assert!(session.document().node(back).is_none(), "and it is not a path any more");
}

/// The **crop** keeps what the path in front was covering, for every
/// path behind it.
#[test]
fn the_crop_keeps_what_the_path_in_front_was_covering_for_every_path_behind_it() {
    let mut session = session();
    let (front, middle, back) = three_rects(&mut session);
    session.execute("select.all", &json!({})).expect("the paths are selected");
    session.execute("vector.path.crop", &json!({})).expect("the paths are cropped");
    assert!(session.document().node(front).is_none(), "the path in front is gone");
    // The path in the middle keeps the part inside the front.
    assert!((area_of(&session, middle) - 5000.0).abs() < 1.0, "the middle keeps what the front covered");
    // The path behind keeps the part of it that is inside the front.
    assert!(
        (area_of(&session, back) - 0.0).abs() < 1.0,
        "the back keeps nothing, for it does not meet the front"
    );
    assert!(session.document().node(back).is_none(), "and a path with nothing left is gone");
}

/// The **crop** of two paths is what it always was.
#[test]
fn the_crop_of_two_paths_is_what_it_always_was() {
    let mut session = session();
    let back = a_rect(&mut session, 0.0, 0.0, 100.0);
    let front = a_rect(&mut session, 50.0, 50.0, 100.0);
    session
        .execute("vector.path.crop", &json!({ "front": front, "back": back }))
        .expect("the paths are cropped");
    assert!((area_of(&session, back) - 2500.0).abs() < 1.0, "the overlap is what is left");
}

/// The **divide** of more than two paths is refused.
#[test]
fn the_divide_of_more_than_two_paths_is_refused() {
    let mut session = session();
    three_rects(&mut session);
    session.execute("select.all", &json!({})).expect("the paths are selected");
    let refused = session.execute("vector.path.divide", &json!({})).expect_err("the divide is refused");
    assert!(refused.to_string().contains("two paths"), "the refusal says why: {refused}");
}
