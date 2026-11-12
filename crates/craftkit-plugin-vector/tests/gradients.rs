//! The **stops** of a gradient.

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

/// A **rectangle** filled with a gradient, and its id.
fn a_gradient_path(session: &mut Session) -> u64 {
    let outcome = session
        .execute(
            "vector.rectangle",
            &json!({ "x": 0, "y": 0, "width": 100, "height": 50, "fill": "#FF0000" }),
        )
        .expect("the rectangle is added");
    let path = outcome.data["id"].as_u64().expect("the path has an id");
    session
        .execute(
            "vector.path.gradient",
            &json!({
                "path": path,
                "kind": "linear",
                "stops": [
                    { "offset": 0, "colour": "#ff0000" },
                    { "offset": 1, "colour": "#0000ff" },
                ],
            }),
        )
        .expect("the gradient is put on the path");
    path
}

/// The **stops** of a gradient, as the command reports them.
fn reported(session: &mut Session, path: u64) -> Vec<Json> {
    let outcome =
        session.execute("vector.gradient.read", &json!({ "path": path })).expect("the gradient is read");
    outcome.data["stops"].as_array().cloned().expect("the stops are a list")
}

/// The gradient of a path may be **read**.
#[test]
fn the_gradient_of_a_path_may_be_read() {
    let mut session = session();
    let path = a_gradient_path(&mut session);
    let outcome =
        session.execute("vector.gradient.read", &json!({ "path": path })).expect("the gradient is read");
    assert_eq!(outcome.data["kind"], json!("linear"), "the gradient is a line");
    let stops = reported(&mut session, path);
    assert_eq!(stops.len(), 2, "it has two stops");
    assert_eq!(stops[0]["offset"], json!(0.0), "the first is at the start");
    assert_eq!(stops[0]["colour"], json!("#ff0000"), "and it is red");
}

/// A **stop** may be added, and the stops stay in order.
#[test]
fn a_stop_may_be_added() {
    let mut session = session();
    let path = a_gradient_path(&mut session);
    session
        .execute("vector.gradient.stop.add", &json!({ "path": path, "offset": 0.5, "colour": "#00ff00" }))
        .expect("the stop is added");
    let stops = reported(&mut session, path);
    assert_eq!(stops.len(), 3, "there are three stops");
    assert_eq!(stops[1]["offset"], json!(0.5), "the new one is in the middle");
    assert_eq!(stops[1]["colour"], json!("#00ff00"), "and it is green");
}

/// A stop added at an offset that is **already taken** goes after the
/// one that is there: the stops keep the order they were in.
#[test]
fn a_stop_added_at_an_offset_already_taken_goes_after_it() {
    let mut session = session();
    let path = a_gradient_path(&mut session);
    session
        .execute("vector.gradient.stop.add", &json!({ "path": path, "offset": 0.0, "colour": "#00ff00" }))
        .expect("the stop is added");
    let stops = reported(&mut session, path);
    assert_eq!(stops[0]["colour"], json!("#ff0000"), "the stop that was there is first");
    assert_eq!(stops[1]["colour"], json!("#00ff00"), "and the new one follows it");
}

/// A **stop** may be changed.
#[test]
fn a_stop_may_be_changed() {
    let mut session = session();
    let path = a_gradient_path(&mut session);
    session
        .execute(
            "vector.gradient.stop.set",
            &json!({ "path": path, "stop": 0, "colour": "#ffffff", "opacity": 0.5 }),
        )
        .expect("the stop is changed");
    let stops = reported(&mut session, path);
    assert_eq!(stops[0]["colour"], json!("#ffffff"), "the colour is changed");
    assert_eq!(stops[0]["opacity"], json!(0.5), "and so is the opacity");
    assert_eq!(stops[0]["offset"], json!(0.0), "and the offset is where it was");
}

/// A stop may be **taken away**.
#[test]
fn a_stop_may_be_taken_away() {
    let mut session = session();
    let path = a_gradient_path(&mut session);
    session
        .execute("vector.gradient.stop.add", &json!({ "path": path, "offset": 0.5, "colour": "#00ff00" }))
        .expect("the stop is added");
    session
        .execute("vector.gradient.stop.remove", &json!({ "path": path, "stop": 1 }))
        .expect("the stop is taken away");
    let stops = reported(&mut session, path);
    assert_eq!(stops.len(), 2, "there are two stops again");
    assert_eq!(stops[1]["colour"], json!("#0000ff"), "and the second one is the last");
}

/// A gradient has **two stops or more**, and the last one may not be
/// taken away.
#[test]
fn the_last_stop_of_a_gradient_may_not_be_taken_away() {
    let mut session = session();
    let path = a_gradient_path(&mut session);
    let value = session
        .execute("vector.gradient.stop.remove", &json!({ "path": path, "stop": 0 }))
        .expect_err("the stop is refused");
    assert!(value.to_string().contains("two stops or more"), "the refusal says why, and it says {}", value);
}

/// A path that is filled with **one colour** has no gradient.
#[test]
fn a_path_filled_with_one_colour_has_no_gradient() {
    let mut session = session();
    let outcome = session
        .execute(
            "vector.rectangle",
            &json!({ "x": 0, "y": 0, "width": 100, "height": 50, "fill": "#FF0000" }),
        )
        .expect("the rectangle is added");
    let path = outcome.data["id"].as_u64().expect("the path has an id");
    let value = session
        .execute("vector.gradient.read", &json!({ "path": path }))
        .expect_err("the gradient is refused");
    assert!(value.to_string().contains("one colour"), "the refusal says why, and it says {}", value);
}

/// An offset that is **not between zero and one** is refused.
#[test]
fn an_offset_that_is_not_between_zero_and_one_is_refused() {
    let mut session = session();
    let path = a_gradient_path(&mut session);
    let value = session
        .execute("vector.gradient.stop.add", &json!({ "path": path, "offset": 2.0, "colour": "#00ff00" }))
        .expect_err("the stop is refused");
    assert!(
        value.to_string().contains("between zero and one"),
        "the refusal says why, and it says {}",
        value
    );
}

/// A stop that is **not there** is refused.
#[test]
fn a_stop_that_is_not_there_is_refused() {
    let mut session = session();
    let path = a_gradient_path(&mut session);
    let value = session
        .execute("vector.gradient.stop.set", &json!({ "path": path, "stop": 5, "colour": "#ffffff" }))
        .expect_err("the stop is refused");
    assert!(value.to_string().contains("no stop"), "the refusal says why, and it says {}", value);
}
