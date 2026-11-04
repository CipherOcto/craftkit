//! The **order** the entities of a block are drawn in.

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

/// A **block** with three entities in it.
fn a_block(session: &mut Session) -> (u64, Vec<u64>) {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the model" }))
        .expect("the block is added");
    let block = outcome.data["id"].as_u64().expect("the block has an id");
    let mut entities = Vec::new();
    for index in 0..3 {
        let outcome = session
            .execute(
                "cad.entity.line.add",
                &json!({
                    "block": block,
                    "name": format!("line {index}"),
                    "from": { "x": index as f64, "y": 0.0 },
                    "to": { "x": index as f64 + 1.0, "y": 1.0 },
                }),
            )
            .expect("the line is added");
        entities.push(outcome.data["id"].as_u64().expect("the line has an id"));
    }
    (block, entities)
}

/// The **order** of the entities of a block.
fn order(session: &mut Session, block: u64) -> Vec<u64> {
    let outcome =
        session.execute("cad.entity.list", &json!({ "block": block })).expect("the entities are read");
    outcome.data["entities"]
        .as_array()
        .expect("the entities are there")
        .iter()
        .map(|entity| entity.as_u64().expect("an entity has an id"))
        .collect()
}

/// The order may be **read**.
#[test]
fn the_order_may_be_read() {
    let mut session = session();
    let (block, entities) = a_block(&mut session);
    assert_eq!(order(&mut session, block), entities, "the entities are in the order they were added");
}

/// An entity may be brought to the **front** and sent to the **back**.
#[test]
fn an_entity_may_be_brought_to_the_front_and_sent_to_the_back() {
    let mut session = session();
    let (block, entities) = a_block(&mut session);
    let (one, two, three) = (entities[0], entities[1], entities[2]);
    session.execute("cad.entity.front", &json!({ "entity": one })).expect("the entity comes to the front");
    assert_eq!(order(&mut session, block), vec![two, three, one], "and it is drawn last");
    session.execute("cad.entity.back", &json!({ "entity": one })).expect("the entity goes to the back");
    assert_eq!(order(&mut session, block), vec![one, two, three], "and it is drawn first");
}

/// An entity may be moved **one step**.
#[test]
fn an_entity_may_be_moved_one_step() {
    let mut session = session();
    let (block, entities) = a_block(&mut session);
    let (one, two, three) = (entities[0], entities[1], entities[2]);
    session.execute("cad.entity.forward", &json!({ "entity": one })).expect("the entity moves forward");
    assert_eq!(order(&mut session, block), vec![two, one, three], "one step towards the front");
    session.execute("cad.entity.backward", &json!({ "entity": one })).expect("the entity moves backward");
    assert_eq!(order(&mut session, block), vec![one, two, three], "and one step back");
    session
        .execute("cad.entity.backward", &json!({ "entity": one }))
        .expect("the entity moves backward again");
    assert_eq!(order(&mut session, block), vec![one, two, three], "and an entity at the back stays");
}

/// A node that is **not an entity** is refused.
#[test]
fn a_node_that_is_not_an_entity_is_refused() {
    let mut session = session();
    let (block, _entities) = a_block(&mut session);
    let error = session
        .execute("cad.entity.front", &json!({ "entity": block }))
        .expect_err("a block is not an entity");
    assert!(error.to_string().contains("entity"), "and the error says why");
}
