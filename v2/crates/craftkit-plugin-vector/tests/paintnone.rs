//! The paint that is **nothing**: the tree's *None*.
//!
//! The tree's rule: a shape may be filled with nothing, and the fill
//! is there and paints nothing.

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

/// The **fill** of a path, as the document carries it.
fn fill_of(session: &Session, path: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(path).expect("the path is there");
    node.data.get("fill").cloned().unwrap_or(craftkit_sdk::serde_json::Value::Null)
}

/// A path may be filled with **nothing**: the tree's *None*.
#[test]
fn a_path_may_be_filled_with_nothing() {
    let mut session = session();
    let path = a_rectangle(&mut session);
    session
        .execute("vector.path.fill", &json!({ "path": path, "colour": "#ff0000" }))
        .expect("the path is filled");
    assert_eq!(fill_of(&session, path)["r"].as_f64(), Some(1.0), "the path is red");
    session
        .execute("vector.path.fill", &json!({ "path": path, "none": true }))
        .expect("the fill is taken away");
    let fill = fill_of(&session, path);
    assert_eq!(fill["a"].as_f64(), Some(0.0), "the fill paints nothing");
}

/// The fill that paints nothing is **there**: it is not the same as a
/// path with no fill at all.
#[test]
fn the_fill_that_paints_nothing_is_there() {
    let mut session = session();
    let path = a_rectangle(&mut session);
    session
        .execute("vector.path.fill", &json!({ "path": path, "none": true }))
        .expect("the fill is taken away");
    let fill = fill_of(&session, path);
    assert!(!fill.is_null(), "the fill is there");
    assert_eq!(fill["a"].as_f64(), Some(0.0), "and it paints nothing");
}

/// A fill that says **neither** the colour nor the none is refused.
#[test]
fn a_fill_that_says_neither_is_refused() {
    let mut session = session();
    let path = a_rectangle(&mut session);
    let error =
        session.execute("vector.path.fill", &json!({ "path": path })).expect_err("the fill is refused");
    assert!(error.to_string().contains("name the colour of the fill"), "the refusal says so: {error}");
}
