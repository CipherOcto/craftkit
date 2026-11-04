//! The **audit** of a drawing: what is wrong with it.

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

/// The **issues** of a drawing.
fn issues(session: &mut Session, drawing: u64) -> Vec<Json> {
    let outcome =
        session.execute("cad.audit", &json!({ "drawing": drawing })).expect("the drawing is audited");
    outcome.data["issues"].as_array().cloned().expect("the issues are a list")
}

/// The kinds of the issues of a drawing.
fn kinds(session: &mut Session, drawing: u64) -> Vec<String> {
    issues(session, drawing)
        .iter()
        .filter_map(|issue| issue["kind"].as_str().map(|kind| kind.to_string()))
        .collect()
}

/// A drawing with nothing wrong with it has **no issues**.
#[test]
fn a_drawing_with_nothing_wrong_with_it_has_no_issues() {
    let mut session = session();
    let (drawing, _block) = a_block(&mut session);
    let found = issues(&mut session, drawing);
    assert!(found.is_empty(), "there is nothing wrong with it, and it says {:?}", found);
}

/// Two layers with the **same name** are an issue.
#[test]
fn two_layers_with_the_same_name_are_an_issue() {
    let mut session = session();
    let (drawing, _block) = a_block(&mut session);
    for _ in 0..2 {
        session
            .execute("cad.layer.add", &json!({ "drawing": drawing, "name": "the layer" }))
            .expect("the layer is added");
    }
    let found = kinds(&mut session, drawing);
    assert_eq!(found, vec!["duplicateLayerName"], "the two layers are an issue, and it says {:?}", found);
}

/// An entity on a **locked** layer is an issue.
#[test]
fn an_entity_on_a_locked_layer_is_an_issue() {
    let mut session = session();
    let (drawing, block) = a_block(&mut session);
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
    session.execute("cad.layer.lock", &json!({ "layer": layer, "on": true })).expect("the layer is locked");
    let found = kinds(&mut session, drawing);
    assert_eq!(found, vec!["onALockedLayer"], "the line is an issue, and it says {:?}", found);
}

/// An entity on a **frozen** layer is an issue.
#[test]
fn an_entity_on_a_frozen_layer_is_an_issue() {
    let mut session = session();
    let (drawing, block) = a_block(&mut session);
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
    session.execute("cad.layer.freeze", &json!({ "layer": layer, "on": true })).expect("the layer is frozen");
    let found = kinds(&mut session, drawing);
    assert_eq!(found, vec!["onAFrozenLayer"], "the line is an issue, and it says {:?}", found);
}

/// An insert of a block of **another drawing** is an issue.
#[test]
fn an_insert_of_a_block_of_another_drawing_is_an_issue() {
    let mut session = session();
    let (drawing, block) = a_block(&mut session);
    let outcome = session
        .execute("cad.drawing.add", &json!({ "name": "the other drawing" }))
        .expect("the drawing is added");
    let other = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": other, "name": "the other block" }))
        .expect("the block is added");
    let other_block = outcome.data["id"].as_u64().expect("the block has an id");
    session
        .execute(
            "cad.entity.insert.add",
            &json!({
                "block": block,
                "name": "the insert",
                "inserts": other_block,
                "at": { "x": 0.0, "y": 0.0 },
            }),
        )
        .expect("the insert is added");
    let found = kinds(&mut session, drawing);
    assert_eq!(found, vec!["insertFromAnotherDrawing"], "the insert is an issue, and it says {:?}", found);
}

/// An insert of the block it is **in** is an issue.
#[test]
fn an_insert_of_the_block_it_is_in_is_an_issue() {
    let mut session = session();
    let (drawing, block) = a_block(&mut session);
    session
        .execute(
            "cad.entity.insert.add",
            &json!({
                "block": block,
                "name": "the insert",
                "inserts": block,
                "at": { "x": 0.0, "y": 0.0 },
            }),
        )
        .expect("the insert is added");
    let found = kinds(&mut session, drawing);
    assert_eq!(found, vec!["insertRefersToItsOwnBlock"], "the insert is an issue, and it says {:?}", found);
}

/// A node that is **not a drawing** is refused.
#[test]
fn a_node_that_is_not_a_drawing_is_refused() {
    let mut session = session();
    let (_drawing, block) = a_block(&mut session);
    let value = session.execute("cad.audit", &json!({ "drawing": block })).expect_err("the audit is refused");
    assert!(value.to_string().contains("not a drawing"), "the refusal says why, and it says {}", value);
}
