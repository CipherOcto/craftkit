//! The **dimension** of a drawing.

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

/// A **drawing** with a **block** in it.
fn a_block(session: &mut Session) -> u64 {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the model" }))
        .expect("the block is added");
    outcome.data["id"].as_u64().expect("the block has an id")
}

/// A **line** in a block, from one point to another.
fn a_line(session: &mut Session, block: u64) -> u64 {
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 3.0, "y": 4.0 },
            }),
        )
        .expect("the line is added");
    outcome.data["id"].as_u64().expect("the line has an id")
}

/// A **dimension** of an entity, in a block.
fn a_dimension(session: &mut Session, block: u64, entity: u64) -> u64 {
    let outcome = session
        .execute(
            "cad.dimension.add",
            &json!({
                "block": block,
                "name": "the dimension",
                "entity": entity,
                "at": { "x": 1.0, "y": 1.0 },
            }),
        )
        .expect("the dimension is added");
    outcome.data["id"].as_u64().expect("the dimension has an id")
}

/// What a dimension **means**, as the plan reports it.
fn facts_of(session: &mut Session, dimension: u64) -> Json {
    let plan = session.evaluate();
    let entry =
        plan.items.iter().find(|entry| entry.node == dimension).expect("the dimension is in the plan");
    entry.value.clone()
}

/// **A dimension measures the length of an entity.**
#[test]
fn a_dimension_measures_the_length_of_an_entity() {
    let mut session = session();
    let block = a_block(&mut session);
    let line = a_line(&mut session, block);
    let dimension = a_dimension(&mut session, block, line);
    let facts = facts_of(&mut session, dimension);
    assert_eq!(facts["value"], json!(5.0), "the line is five long");
    assert_eq!(facts["measures"], json!("the line"), "and the dimension says what it measures");
}

/// The **text** of a dimension is the measurement.
#[test]
fn the_text_of_a_dimension_is_the_measurement() {
    let mut session = session();
    let block = a_block(&mut session);
    let line = a_line(&mut session, block);
    let dimension = a_dimension(&mut session, block, line);
    let facts = facts_of(&mut session, dimension);
    assert_eq!(facts["text"], json!("5.000"), "the measurement is written with three decimals");
}

/// A dimension may say something **of its own**.
#[test]
fn a_dimension_may_say_something_of_its_own() {
    let mut session = session();
    let block = a_block(&mut session);
    let line = a_line(&mut session, block);
    let dimension = a_dimension(&mut session, block, line);
    session
        .execute("cad.dimension.text", &json!({ "dimension": dimension, "text": "the length" }))
        .expect("the dimension says its own text");
    let facts = facts_of(&mut session, dimension);
    assert_eq!(facts["text"], json!("the length"), "the text is what it says");
    assert_eq!(facts["value"], json!(5.0), "and the measurement is still there");
}

/// An **empty** text takes the text away.
#[test]
fn an_empty_text_takes_the_text_away() {
    let mut session = session();
    let block = a_block(&mut session);
    let line = a_line(&mut session, block);
    let dimension = a_dimension(&mut session, block, line);
    session
        .execute("cad.dimension.text", &json!({ "dimension": dimension, "text": "the length" }))
        .expect("the dimension says its own text");
    session
        .execute("cad.dimension.text", &json!({ "dimension": dimension, "text": "" }))
        .expect("the text is taken away");
    let facts = facts_of(&mut session, dimension);
    assert_eq!(facts["text"], json!("5.000"), "the measurement shows again");
}

/// The measurement is **derived**, so it cannot be out of date.
#[test]
fn the_measurement_is_derived() {
    let mut session = session();
    let block = a_block(&mut session);
    let line = a_line(&mut session, block);
    let dimension = a_dimension(&mut session, block, line);
    session
        .execute(
            "cad.entity.scale",
            &json!({ "entity": line, "about": { "x": 0.0, "y": 0.0 }, "factor": 2.0 }),
        )
        .expect("the line is scaled");
    let facts = facts_of(&mut session, dimension);
    assert_eq!(facts["value"], json!(10.0), "the dimension measures the line as it is now");
}

/// An entity that has **no length** may not be measured.
#[test]
fn an_entity_that_has_no_length_may_not_be_measured() {
    let mut session = session();
    let block = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.text.add",
            &json!({
                "block": block,
                "name": "the text",
                "text": "hello",
                "at": { "x": 0.0, "y": 0.0 },
            }),
        )
        .expect("the text is added");
    let text = outcome.data["id"].as_u64().expect("the text has an id");
    let refused = session
        .execute(
            "cad.dimension.add",
            &json!({
                "block": block,
                "name": "the dimension",
                "entity": text,
                "at": { "x": 1.0, "y": 1.0 },
            }),
        )
        .expect_err("the dimension is refused");
    assert!(refused.to_string().contains("no length"), "the refusal says why");
}

/// A node that is **not an entity** may not be measured.
#[test]
fn a_node_that_is_not_an_entity_may_not_be_measured() {
    let mut session = session();
    let block = a_block(&mut session);
    let refused = session
        .execute(
            "cad.dimension.add",
            &json!({
                "block": block,
                "name": "the dimension",
                "entity": block,
                "at": { "x": 1.0, "y": 1.0 },
            }),
        )
        .expect_err("the dimension is refused");
    assert!(refused.to_string().contains("not an entity"), "the refusal says why");
}

/// A node that is **not a dimension** may not be given a text.
#[test]
fn a_node_that_is_not_a_dimension_may_not_be_given_a_text() {
    let mut session = session();
    let block = a_block(&mut session);
    let line = a_line(&mut session, block);
    let refused = session
        .execute("cad.dimension.text", &json!({ "dimension": line, "text": "the length" }))
        .expect_err("the text is refused");
    assert!(refused.to_string().contains("not a dimension"), "the refusal says why");
}
