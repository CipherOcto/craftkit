//! The **array** of an entity.

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

/// The **copies** an array made, and the entities they are.
fn copies(outcome: &craftkit_sdk::Outcome) -> Vec<u64> {
    outcome.data["copies"]
        .as_array()
        .expect("the copies are a list")
        .iter()
        .filter_map(|copy| copy.as_u64())
        .collect()
}

/// An entity may be arrayed in a **grid**: the rows and the columns.
#[test]
fn an_entity_may_be_arrayed_in_a_grid() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block);
    let outcome = session
        .execute(
            "cad.entity.array.rectangular",
            &json!({
                "entity": line,
                "rows": 2,
                "columns": 3,
                "row_spacing": 10.0,
                "column_spacing": 5.0,
            }),
        )
        .expect("the entity is arrayed");
    assert_eq!(outcome.data["items"], json!(6), "the array holds six items");
    assert_eq!(outcome.data["count"], json!(5), "and five of them are copies");
    let made = copies(&outcome);
    assert_eq!(made.len(), 5, "and five of them are copies");
    // The first copy is the second item: one column along.
    let node = session.document().node(made[0]).expect("the copy is in the document");
    assert_eq!(node.kind, "cad.entity.line", "the copy is a line");
    assert_eq!(node.data["geometry"]["from"], json!({ "x": 5.0, "y": 0.0 }), "the copy starts there");
    assert_eq!(node.data["geometry"]["to"], json!({ "x": 8.0, "y": 4.0 }), "and it ends there");
    // The third copy is the first item of the second row.
    let node = session.document().node(made[2]).expect("the copy is in the document");
    assert_eq!(node.data["geometry"]["from"], json!({ "x": 0.0, "y": 10.0 }), "the copy starts there");
    // The copies are named after the entity they are copies of.
    let node = session.document().node(made[0]).expect("the copy is in the document");
    assert_eq!(node.name, "the line 2", "and the first copy has the second name");
}

/// An entity may be arrayed **about a centre**.
#[test]
fn an_entity_may_be_arrayed_about_a_centre() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block);
    let outcome = session
        .execute(
            "cad.entity.array.polar",
            &json!({
                "entity": line,
                "centre": { "x": 0.0, "y": 0.0 },
                "count": 4,
            }),
        )
        .expect("the entity is arrayed");
    assert_eq!(outcome.data["items"], json!(4), "the array holds four items");
    assert_eq!(outcome.data["count"], json!(3), "and three of them are copies");
    let made = copies(&outcome);
    assert_eq!(made.len(), 3, "and three of them are copies");
    // The first copy is a quarter of a turn round: the centre of the
    // line is at (1.5, 2), and the turn puts it at (-2, 1.5).
    let node = session.document().node(made[0]).expect("the copy is in the document");
    let from = node.data["geometry"]["from"].as_object().expect("the copy has a start");
    let x = from["x"].as_f64().expect("the start has an x");
    let y = from["y"].as_f64().expect("the start has a y");
    assert!((x - -3.5).abs() < 0.0001, "the copy is at the turn: {x}");
    assert!((y - -0.5).abs() < 0.0001, "the copy is at the turn: {y}");
}

/// The copies of a polar array may be **turned** with the array.
#[test]
fn the_copies_of_a_polar_array_may_be_turned_with_it() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block);
    let outcome = session
        .execute(
            "cad.entity.array.polar",
            &json!({
                "entity": line,
                "centre": { "x": 0.0, "y": 0.0 },
                "count": 4,
                "rotate": true,
            }),
        )
        .expect("the entity is arrayed");
    let made = copies(&outcome);
    let node = session.document().node(made[0]).expect("the copy is in the document");
    // The line was from (0,0) to (3,4); a quarter turn about the
    // centre puts its start at (0, 0) and its end at (-4, 3).
    assert!(
        (node.data["geometry"]["from"]["x"].as_f64().unwrap_or(0.0)).abs() < 0.0001,
        "the copy starts there"
    );
    assert!(
        (node.data["geometry"]["to"]["x"].as_f64().unwrap_or(0.0) - -4.0).abs() < 0.0001,
        "and it ends there"
    );
    assert!(
        (node.data["geometry"]["to"]["y"].as_f64().unwrap_or(0.0) - 3.0).abs() < 0.0001,
        "and it ends there"
    );
}

/// A count that is **not a whole number** is refused.
#[test]
fn a_count_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block);
    let outcome = session.execute(
        "cad.entity.array.rectangular",
        &json!({
            "entity": line,
            "rows": 1,
            "columns": 2.5,
            "row_spacing": 10.0,
            "column_spacing": 5.0,
        }),
    );
    assert!(outcome.is_err(), "half a column is not a column and the command refuses it");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("whole number"), "and it says why: {error}");
}

/// An array may not be **endless**.
#[test]
fn an_array_may_not_be_endless() {
    let mut session = session();
    let (_, block) = a_block(&mut session);
    let line = a_line(&mut session, block);
    let outcome = session.execute(
        "cad.entity.array.rectangular",
        &json!({
            "entity": line,
            "rows": 100,
            "columns": 100,
            "row_spacing": 10.0,
            "column_spacing": 5.0,
        }),
    );
    assert!(outcome.is_err(), "ten thousand items is too many and the command refuses it");
    let error = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(error.contains("at most"), "and it says why: {error}");
}
