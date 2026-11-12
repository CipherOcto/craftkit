//! What may be **edited**: a layer that is locked or frozen locks what
//! is on it.

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

/// A **drawing** with a **block** in it.
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

/// A **line** on a layer, and the layer.
fn a_line_on_a_layer(session: &mut Session) -> (u64, u64) {
    let (drawing, block) = a_block(session);
    let outcome = session
        .execute("cad.layer.add", &json!({ "drawing": drawing, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 1.0, "y": 0.0 },
            }),
        )
        .expect("the line is added");
    let line = outcome.data["id"].as_u64().expect("the line has an id");
    session
        .execute("cad.entity.layer", &json!({ "entity": line, "layer": layer }))
        .expect("the line is on the layer");
    (layer, line)
}

/// An entity on a **locked** layer may not be **moved**.
#[test]
fn an_entity_on_a_locked_layer_may_not_be_moved() {
    let mut session = session();
    let (layer, line) = a_line_on_a_layer(&mut session);
    session.execute("cad.layer.lock", &json!({ "layer": layer, "on": true })).expect("the layer is locked");
    let value = session
        .execute("cad.entity.move", &json!({ "entity": line, "by": { "x": 1.0, "y": 0.0 } }))
        .expect_err("the move is refused");
    assert!(value.to_string().contains("locked layer"), "the refusal says why, and it says {}", value);
}

/// An entity on a locked layer may not be **taken away**.
#[test]
fn an_entity_on_a_locked_layer_may_not_be_taken_away() {
    let mut session = session();
    let (layer, line) = a_line_on_a_layer(&mut session);
    session.execute("cad.layer.lock", &json!({ "layer": layer, "on": true })).expect("the layer is locked");
    let value =
        session.execute("cad.entity.remove", &json!({ "entity": line })).expect_err("the removal is refused");
    assert!(value.to_string().contains("locked layer"), "the refusal says why, and it says {}", value);
}

/// An entity on a **frozen** layer may not be **turned**.
#[test]
fn an_entity_on_a_frozen_layer_may_not_be_turned() {
    let mut session = session();
    let (layer, line) = a_line_on_a_layer(&mut session);
    session.execute("cad.layer.freeze", &json!({ "layer": layer, "on": true })).expect("the layer is frozen");
    let value = session
        .execute(
            "cad.entity.rotate",
            &json!({ "entity": line, "about": { "x": 0.0, "y": 0.0 }, "degrees": 90.0 }),
        )
        .expect_err("the turn is refused");
    assert!(value.to_string().contains("frozen layer"), "the refusal says why, and it says {}", value);
}

/// An entity on a locked layer may not be **copied**.
#[test]
fn an_entity_on_a_locked_layer_may_not_be_copied() {
    let mut session = session();
    let (layer, line) = a_line_on_a_layer(&mut session);
    session.execute("cad.layer.lock", &json!({ "layer": layer, "on": true })).expect("the layer is locked");
    let value = session
        .execute(
            "cad.entity.copy",
            &json!({ "entity": line, "name": "the copy", "by": { "x": 1.0, "y": 0.0 } }),
        )
        .expect_err("the copy is refused");
    assert!(value.to_string().contains("locked layer"), "the refusal says why, and it says {}", value);
}

/// An entity on a layer that is **not locked** may be moved.
#[test]
fn an_entity_on_a_layer_that_is_not_locked_may_be_moved() {
    let mut session = session();
    let (_layer, line) = a_line_on_a_layer(&mut session);
    session
        .execute("cad.entity.move", &json!({ "entity": line, "by": { "x": 1.0, "y": 0.0 } }))
        .expect("the line is moved");
}

/// An entity that is on **no layer** may be moved: there is no layer to
/// lock it.
#[test]
fn an_entity_that_is_on_no_layer_may_be_moved() {
    let mut session = session();
    let (_drawing, block) = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 1.0, "y": 0.0 },
            }),
        )
        .expect("the line is added");
    let line = outcome.data["id"].as_u64().expect("the line has an id");
    session
        .execute("cad.entity.move", &json!({ "entity": line, "by": { "x": 1.0, "y": 0.0 } }))
        .expect("the line is moved");
}

/// A layer that is **unlocked** lets its entities be edited again.
#[test]
fn a_layer_that_is_unlocked_lets_its_entities_be_edited_again() {
    let mut session = session();
    let (layer, line) = a_line_on_a_layer(&mut session);
    session.execute("cad.layer.lock", &json!({ "layer": layer, "on": true })).expect("the layer is locked");
    session
        .execute("cad.entity.move", &json!({ "entity": line, "by": { "x": 1.0, "y": 0.0 } }))
        .expect_err("the move is refused");
    session
        .execute("cad.layer.lock", &json!({ "layer": layer, "on": false }))
        .expect("the layer is unlocked");
    session
        .execute("cad.entity.move", &json!({ "entity": line, "by": { "x": 1.0, "y": 0.0 } }))
        .expect("the line is moved");
}
