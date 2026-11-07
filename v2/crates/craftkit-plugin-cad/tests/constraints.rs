//! The **constraints** of a drawing.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_cad::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the cad battery.
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

/// A **drawing** with a **block** in it: the drawing and the block.
fn a_block(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the model" }))
        .expect("the block is added");
    let block = outcome.data["id"].as_u64().expect("the block has an id");
    (drawing, block)
}

/// A **line** in a block, from one point to another.
fn a_line(session: &mut Session, block: u64, name: &str, from: (f64, f64), to: (f64, f64)) -> u64 {
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": name,
                "from": { "x": from.0, "y": from.1 },
                "to": { "x": to.0, "y": to.1 },
            }),
        )
        .expect("the line is added");
    outcome.data["id"].as_u64().expect("the line has an id")
}

/// A constraint may be **added** to a drawing, and whether it holds is
/// worked out from the geometry.
#[test]
fn a_constraint_may_be_added_to_a_drawing() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let first = a_line(&mut session, block, "the first", (0.0, 0.0), (10.0, 0.0));
    let second = a_line(&mut session, block, "the second", (0.0, 5.0), (10.0, 5.0));
    let outcome = session
        .execute("cad.constraint.add", &json!({ "first": first, "second": second, "kind": "parallel" }))
        .expect("the constraint is added");
    assert_eq!(outcome.data["kind"], json!("parallel"), "the constraint says what it is");
    assert_eq!(outcome.data["holds"], json!(true), "and the two lines are parallel");
}

/// A constraint that does **not hold** is said so.
#[test]
fn a_constraint_that_does_not_hold_is_said_so() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let first = a_line(&mut session, block, "the first", (0.0, 0.0), (10.0, 0.0));
    let second = a_line(&mut session, block, "the second", (0.0, 0.0), (5.0, 5.0));
    let outcome = session
        .execute("cad.constraint.add", &json!({ "first": first, "second": second, "kind": "perpendicular" }))
        .expect("the constraint is added");
    assert_eq!(outcome.data["holds"], json!(false), "the lines are not at right angles");
}

/// The constraints of a drawing are **read**.
#[test]
fn the_constraints_of_a_drawing_are_read() {
    let mut session = session();
    let (drawing, block) = a_block(&mut session);
    let first = a_line(&mut session, block, "the first", (0.0, 0.0), (10.0, 0.0));
    let second = a_line(&mut session, block, "the second", (0.0, 0.0), (0.0, 10.0));
    session
        .execute("cad.constraint.add", &json!({ "first": first, "second": second, "kind": "perpendicular" }))
        .expect("the constraint is added");
    let outcome = session
        .execute("cad.constraint.list", &json!({ "drawing": drawing }))
        .expect("the constraints are read");
    assert_eq!(outcome.data["count"], json!(1), "the drawing holds one constraint");
    let listed = outcome.data["constraints"].as_array().expect("the constraints are a list");
    assert_eq!(listed[0]["kind"], json!("perpendicular"), "the constraint says what it is");
    assert_eq!(listed[0]["first"], json!(first), "and which entities it is between");
    assert_eq!(listed[0]["second"], json!(second), "and which entities it is between");
    assert_eq!(listed[0]["holds"], json!(true), "and whether it holds");
}

/// A **kind** that is not a constraint is refused by name.
#[test]
fn a_kind_that_is_not_a_constraint_is_refused_by_name() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let first = a_line(&mut session, block, "the first", (0.0, 0.0), (10.0, 0.0));
    let second = a_line(&mut session, block, "the second", (0.0, 5.0), (10.0, 5.0));
    let outcome =
        session.execute("cad.constraint.add", &json!({ "first": first, "second": second, "kind": "wobbly" }));
    assert!(outcome.is_err(), "wobbly is not a constraint and the command refuses it");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("wobbly"), "and it says what it is: {error}");
}

/// A constraint is between **two** entities, and one is not enough.
#[test]
fn a_constraint_is_between_two_entities_and_one_is_not_enough() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block, "the line", (0.0, 0.0), (10.0, 0.0));
    let outcome = session
        .execute("cad.constraint.add", &json!({ "first": line, "second": line, "kind": "horizontal" }));
    assert!(outcome.is_err(), "a constraint with one entity in it is refused");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("two entities"), "and it says why: {error}");
}
