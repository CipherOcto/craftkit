//! The **copy** of an entity.

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

/// An entity may be copied.
#[test]
fn an_entity_may_be_copied() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block);
    let outcome = session
        .execute(
            "cad.entity.copy",
            &json!({
                "entity": line,
                "name": "the copy",
                "by": { "x": 10.0, "y": 0.0 },
            }),
        )
        .expect("the entity is copied");
    let copy = outcome.data["copy"].as_u64().expect("the copy has an id");
    let node = session.document().node(copy).expect("the copy is in the document");
    assert_eq!(node.kind, "cad.entity.line", "the copy is a line");
    assert_eq!(node.name, "the copy", "and it has the name it was given");
    // The copy is the line, moved by the vector.
    assert_eq!(node.data["geometry"]["from"], json!({ "x": 10.0, "y": 0.0 }), "the copy starts there");
    assert_eq!(node.data["geometry"]["to"], json!({ "x": 13.0, "y": 4.0 }), "and it ends there");
}

/// The copy is in the **same block** as the entity it copies.
#[test]
fn the_copy_is_in_the_same_block() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block);
    let outcome = session
        .execute(
            "cad.entity.copy",
            &json!({
                "entity": line,
                "name": "the copy",
                "by": { "x": 0.0, "y": 0.0 },
            }),
        )
        .expect("the entity is copied");
    let copy = outcome.data["copy"].as_u64().expect("the copy has an id");
    let entities =
        session.execute("cad.entity.list", &json!({ "block": block })).expect("the entities are read");
    let listed = entities.data["entities"].as_array().expect("the entities are a list");
    assert!(
        listed.iter().any(|entity| *entity == json!(copy)),
        "the copy is in the block, and the block holds {:?}",
        listed
    );
}

/// The copy is on the **same layer** as the entity it copies.
#[test]
fn the_copy_is_on_the_same_layer() {
    let mut session = session();
    let (drawing, block) = a_block(&mut session);
    let outcome = session
        .execute("cad.layer.add", &json!({ "drawing": drawing, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    let line = a_line(&mut session, block);
    session
        .execute("cad.entity.layer", &json!({ "entity": line, "layer": layer }))
        .expect("the line is on the layer");
    let outcome = session
        .execute(
            "cad.entity.copy",
            &json!({
                "entity": line,
                "name": "the copy",
                "by": { "x": 5.0, "y": 0.0 },
            }),
        )
        .expect("the entity is copied");
    let copy = outcome.data["copy"].as_u64().expect("the copy has an id");
    let relations = session.document().relations_from(copy, Some(craftkit_sdk::layer::ON_LAYER));
    assert_eq!(relations.len(), 1, "the copy is on one layer");
    assert_eq!(relations[0].to, layer, "and it is the layer of the entity it copies");
}

/// The copy may be put in **another block**.
#[test]
fn the_copy_may_be_put_in_another_block() {
    let mut session = session();
    let (drawing, block) = a_block(&mut session);
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the other block" }))
        .expect("the block is added");
    let other = outcome.data["id"].as_u64().expect("the block has an id");
    let line = a_line(&mut session, block);
    let outcome = session
        .execute(
            "cad.entity.copy",
            &json!({
                "entity": line,
                "name": "the copy",
                "by": { "x": 0.0, "y": 0.0 },
                "block": other,
            }),
        )
        .expect("the entity is copied");
    let _copy = outcome.data["copy"].as_u64().expect("the copy has an id");
    let entities =
        session.execute("cad.entity.list", &json!({ "block": other })).expect("the entities are read");
    let listed = entities.data["entities"].as_array().expect("the entities are a list");
    assert_eq!(listed.len(), 1, "the other block holds the copy, and it holds {:?}", listed);
}

/// A node that is **not an entity** is refused.
#[test]
fn a_node_that_is_not_an_entity_is_refused() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let value = session
        .execute(
            "cad.entity.copy",
            &json!({
                "entity": block,
                "name": "the copy",
                "by": { "x": 0.0, "y": 0.0 },
            }),
        )
        .expect_err("the copy is refused");
    assert!(value.to_string().contains("not an entity"), "the refusal says why, and it says {}", value);
}
