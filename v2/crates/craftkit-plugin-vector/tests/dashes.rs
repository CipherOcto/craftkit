//! The **dash** of a path.

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

/// A **path** in the document.
fn a_path(session: &mut Session) -> u64 {
    let outcome = session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    outcome.data["id"].as_u64().expect("the path has an id")
}

/// The **facts** of a path, as the plan reports them.
fn facts_of(session: &mut Session, path: u64) -> Json {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == path).expect("the path is in the plan");
    entry.value.clone()
}

/// **A line may be dashed.**
#[test]
fn a_line_may_be_dashed() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.dash", &json!({ "path": path, "pattern": [6.0, 3.0], "offset": 0.0 }))
        .expect("the line is dashed");
    let facts = facts_of(&mut session, path);
    assert_eq!(facts["dash"]["pattern"], json!([6.0, 3.0]), "the pattern is there");
    assert_eq!(facts["dash"]["offset"], json!(0.0), "and the offset is there");
}

/// A pattern of an **odd** number of lengths is run **twice**.
#[test]
fn a_pattern_of_an_odd_number_of_lengths_is_run_twice() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.dash", &json!({ "path": path, "pattern": [6.0] }))
        .expect("the line is dashed");
    let facts = facts_of(&mut session, path);
    assert_eq!(facts["dash"]["run"], json!([6.0, 6.0]), "the run is the pattern twice");
    assert_eq!(facts["dash"]["round"], json!(12.0), "and a round is twelve long");
}

/// A pattern of an **even** number of lengths is run as it is.
#[test]
fn a_pattern_of_an_even_number_of_lengths_is_run_as_it_is() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.dash", &json!({ "path": path, "pattern": [6.0, 3.0] }))
        .expect("the line is dashed");
    let facts = facts_of(&mut session, path);
    assert_eq!(facts["dash"]["run"], json!([6.0, 3.0]), "the run is the pattern");
    assert_eq!(facts["dash"]["round"], json!(9.0), "and a round is nine long");
}

/// A pattern that is **empty** takes the dash away.
#[test]
fn a_pattern_that_is_empty_takes_the_dash_away() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.dash", &json!({ "path": path, "pattern": [6.0, 3.0] }))
        .expect("the line is dashed");
    session
        .execute("vector.path.dash", &json!({ "path": path, "pattern": [] }))
        .expect("the dash is taken away");
    let facts = facts_of(&mut session, path);
    assert!(facts["dash"].is_null(), "there is no dash");
}

/// A path that is **not** dashed has no dash.
#[test]
fn a_path_that_is_not_dashed_has_no_dash() {
    let mut session = session();
    let path = a_path(&mut session);
    let facts = facts_of(&mut session, path);
    assert!(facts["dash"].is_null(), "there is no dash");
}

/// A length that is **negative** is refused.
#[test]
fn a_length_that_is_negative_is_refused() {
    let mut session = session();
    let path = a_path(&mut session);
    let refused = session
        .execute("vector.path.dash", &json!({ "path": path, "pattern": [6.0, -3.0] }))
        .expect_err("the pattern is refused");
    assert!(refused.to_string().contains("negative"), "the refusal says why");
}

/// A length that is **not a number** is refused.
#[test]
fn a_length_that_is_not_a_number_is_refused() {
    let mut session = session();
    let path = a_path(&mut session);
    let refused = session
        .execute("vector.path.dash", &json!({ "path": path, "pattern": ["six"] }))
        .expect_err("the pattern is refused");
    assert!(refused.to_string().contains("numbers"), "the refusal says why");
}

/// An **offset** that is not a number is refused.
#[test]
fn an_offset_that_is_not_a_number_is_refused() {
    let mut session = session();
    let path = a_path(&mut session);
    let refused = session
        .execute("vector.path.dash", &json!({ "path": path, "pattern": [6.0, 3.0], "offset": "here" }))
        .expect_err("the offset is refused");
    assert!(refused.to_string().contains("number"), "the refusal says: {}", refused);
}
