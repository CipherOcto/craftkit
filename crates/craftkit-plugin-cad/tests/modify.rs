//! The **modification** of an entity.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **block** with a line and a circle in it.
fn a_block(session: &mut Session) -> (u64, u64, u64) {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the model" }))
        .expect("the block is added");
    let block = outcome.data["id"].as_u64().expect("the block has an id");
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 10.0, "y": 0.0 },
            }),
        )
        .expect("the line is added");
    let line = outcome.data["id"].as_u64().expect("the line has an id");
    let outcome = session
        .execute(
            "cad.entity.circle.add",
            &json!({
                "block": block,
                "name": "the circle",
                "center": { "x": 0.0, "y": 0.0 },
                "radius": 5.0,
            }),
        )
        .expect("the circle is added");
    let circle = outcome.data["id"].as_u64().expect("the circle has an id");
    (block, line, circle)
}

/// The **geometry** of an entity, read from the plan.
fn geometry_of(session: &mut Session, entity: u64) -> Json {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == entity).expect("the entity is in the plan");
    item.value["geometry"].clone()
}

/// An entity may be **moved**.
#[test]
fn an_entity_may_be_moved() {
    let mut session = session();
    let (_block, line, _circle) = a_block(&mut session);
    session
        .execute("cad.entity.move", &json!({ "entity": line, "by": { "x": 5.0, "y": 3.0 } }))
        .expect("the entity is moved");
    let geometry = geometry_of(&mut session, line);
    assert_eq!(geometry["from"]["x"], json!(5.0), "the line moved across");
    assert_eq!(geometry["from"]["y"], json!(3.0), "and up");
    assert_eq!(geometry["to"]["x"], json!(15.0), "and the other end moved too");
}

/// An entity may be **turned**.
#[test]
fn an_entity_may_be_turned() {
    let mut session = session();
    let (_block, line, _circle) = a_block(&mut session);
    session
        .execute(
            "cad.entity.rotate",
            &json!({ "entity": line, "about": { "x": 0.0, "y": 0.0 }, "degrees": 90.0 }),
        )
        .expect("the entity is turned");
    let geometry = geometry_of(&mut session, line);
    // A quarter turn about the origin: the end of the line is up. The
    // turn is a sine and a cosine, and so the answer is near the
    // number and not the number.
    let x = geometry["to"]["x"].as_f64().expect("a number");
    let y = geometry["to"]["y"].as_f64().expect("a number");
    assert!(x.abs() < 1e-9, "the line is turned: {x}");
    assert!((y - 10.0).abs() < 1e-9, "and it is up: {y}");
}

/// An entity may be **scaled**.
#[test]
fn an_entity_may_be_scaled() {
    let mut session = session();
    let (_block, _line, circle) = a_block(&mut session);
    session
        .execute(
            "cad.entity.scale",
            &json!({ "entity": circle, "about": { "x": 0.0, "y": 0.0 }, "factor": 2.0 }),
        )
        .expect("the entity is scaled");
    let geometry = geometry_of(&mut session, circle);
    assert_eq!(geometry["radius"], json!(10.0), "the circle is twice as big");
    let error = session
        .execute(
            "cad.entity.scale",
            &json!({ "entity": circle, "about": { "x": 0.0, "y": 0.0 }, "factor": 0.0 }),
        )
        .expect_err("a factor of nothing is refused");
    assert!(error.to_string().contains("nothing"), "and the error says why");
}

/// An entity may be **taken away**.
#[test]
fn an_entity_may_be_taken_away() {
    let mut session = session();
    let (block, line, _circle) = a_block(&mut session);
    session.execute("cad.entity.remove", &json!({ "entity": line })).expect("the entity is taken away");
    let outcome =
        session.execute("cad.entity.list", &json!({ "block": block })).expect("the entities are read");
    let entities = outcome.data["entities"].as_array().expect("the entities are there");
    assert_eq!(entities.len(), 1, "and the block has one entity left");
}
