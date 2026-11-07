//! The **DXF** written out: a drawing made into a drawing file.
//!
//! The file is a DXF, and the drawing it names is the drawing that
//! was written.

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

/// A path of this test's own, so that two tests do not fight over one
/// file.
fn a_path(name: &str) -> String {
    std::env::temp_dir()
        .join(format!("craftkit-v2-dxfout-{name}-{}", std::process::id()))
        .to_string_lossy()
        .to_string()
}

/// A **drawing** with a layer, a block and a line.
fn a_drawing(session: &mut Session) -> (u64, u64, u64) {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.layer.add", &json!({ "drawing": drawing, "name": "the walls", "color": "3" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "*Model_Space" }))
        .expect("the block is added");
    let block = outcome.data["id"].as_u64().expect("the block has an id");
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 1.0, "y": 2.0 },
                "to": { "x": 4.0, "y": 6.0 },
            }),
        )
        .expect("the line is added");
    let entity = outcome.data["id"].as_u64().expect("the line has an id");
    session
        .execute("cad.entity.layer", &json!({ "entity": entity, "layer": layer }))
        .expect("the line is on the layer");
    (drawing, block, entity)
}

/// A drawing is written as a **DXF file**: the header, the tables and
/// the entities.
#[test]
fn a_drawing_is_written_as_a_dxf_file() {
    let mut session = session();
    let (drawing, _block, _entity) = a_drawing(&mut session);
    let path = a_path("file");
    let outcome = session
        .execute("cad.export.dxf", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    assert_eq!(outcome.data["entities"], json!(1), "there is one entity: {outcome:?}");
    let text = std::fs::read_to_string(outcome.data["path"].as_str().unwrap_or_default())
        .expect("the file is there");
    assert!(text.contains("SECTION"), "the file has its sections");
    assert!(text.contains("$ACADVER"), "and its header");
    assert!(text.contains("AC1032"), "with the tree's version");
    assert!(text.contains("ENTITIES"), "and its entities");
    assert!(text.contains("LINE"), "with the line in it");
    assert!(text.contains("the walls"), "and the layer of the line");
    assert!(text.trim_end().ends_with("EOF"), "and it ends");
}

/// The **layers** of the drawing are the table of the file, and a
/// layer that is off has a **negative** colour.
#[test]
fn the_layers_are_the_table() {
    let mut session = session();
    let (drawing, _block, _entity) = a_drawing(&mut session);
    let outcome =
        session.execute("cad.layer.list", &json!({ "drawing": drawing })).expect("the layers are read");
    let layers = outcome.data["layers"].as_array().expect("there are layers");
    let walls = layers.iter().find(|layer| layer["name"] == json!("the walls")).expect("the layer is there");
    let id = walls["id"].as_u64().expect("the layer has an id");
    session.execute("cad.layer.on", &json!({ "layer": id, "on": false })).expect("the layer is turned off");
    let path = a_path("off");
    session
        .execute("cad.export.dxf", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    assert!(text.contains("\n62\n-3\n"), "the colour of a layer that is off is negative: {text}");
}

/// A drawing that is written and read again is the same drawing: the
/// **round trip**.
#[test]
fn a_drawing_that_is_written_is_read_back() {
    let mut session = session();
    let (drawing, _block, _entity) = a_drawing(&mut session);
    let path = a_path("roundtrip");
    session
        .execute("cad.export.dxf", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    let mut other = crate::session();
    let outcome = other
        .execute("cad.import.dxf", &json!({ "path": path, "name": "the drawing again" }))
        .expect("the file is read");
    assert_eq!(outcome.data["entities"], json!(1), "the line is read: {outcome:?}");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let node = other.document().node(drawing).expect("the drawing is there");
    let block = node.children.iter().find(|child| child.kind == "cad.block").expect("there is a block");
    let line = block.children.iter().find(|child| child.kind == "cad.entity.line").expect("there is a line");
    assert_eq!(
        line.data["geometry"]["from"],
        json!({ "x": 1.0, "y": 2.0 }),
        "the line starts where it did: {line:?}"
    );
    assert_eq!(line.data["geometry"]["to"], json!({ "x": 4.0, "y": 6.0 }), "and ends there");
}

/// An entity this tree cannot write is **left out**, and the answer
/// says what it left out.
#[test]
fn an_entity_that_cannot_be_written_is_left_out() {
    let mut session = session();
    let (drawing, block, _entity) = a_drawing(&mut session);
    session
        .execute(
            "cad.entity.text.add",
            &json!({
                "block": block,
                "name": "the words",
                "text": "the beach",
                "at": { "x": 0.0, "y": 0.0 },
            }),
        )
        .expect("the text is added");
    let path = a_path("leftout");
    let outcome = session
        .execute("cad.export.dxf", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert_eq!(left_out, &vec![json!("the words")], "the text is said");
}

/// A **node** that is not a drawing is refused.
#[test]
fn a_node_that_is_not_a_drawing_is_refused() {
    let mut session = session();
    let (drawing, block, _entity) = a_drawing(&mut session);
    let error = session
        .execute("cad.export.dxf", &json!({ "path": a_path("wrong"), "drawing": block }))
        .expect_err("the block is not a drawing");
    assert!(error.to_string().contains("not a drawing"), "the refusal says why: {error}");
    let error = session
        .execute("cad.export.dxf", &json!({ "path": "  ", "drawing": drawing }))
        .expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}
