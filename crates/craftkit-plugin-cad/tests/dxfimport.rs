//! The **DXF** read back: a drawing file made into a drawing.
//!
//! The entities of the file are entities of the model space, and the
//! layers of the file are layers of the drawing.

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

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-dxf-{name}-{}", std::process::id()));
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **file** with a table of layers and a section of entities.
fn a_file_with(entities: &str) -> String {
    let mut text = String::new();
    text.push_str("0\nSECTION\n2\nHEADER\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    text.push_str("0\nSECTION\n2\nTABLES\n");
    text.push_str(
        "0\nTABLE\n2\nLAYER\n0\nLAYER\n2\n0\n62\n7\n0\nLAYER\n2\nthe walls\n62\n3\n0\nENDTAB\n0\nENDSEC\n",
    );
    text.push_str("0\nSECTION\n2\nENTITIES\n");
    text.push_str(entities);
    text.push_str("0\nENDSEC\n0\nEOF\n");
    text
}

/// A **line** of the file.
fn a_line() -> String {
    "0\nLINE\n8\nthe walls\n10\n1.0\n20\n2.0\n30\n0.0\n11\n4.0\n21\n6.0\n31\n0.0\n".to_string()
}

/// A file is read into a **drawing**: the entities are entities of the
/// model space, and the layers are layers of the drawing.
#[test]
fn a_file_is_read_into_a_drawing() {
    let mut session = session();
    let path = a_file("drawing", &a_file_with(&a_line()));
    let outcome = session
        .execute("cad.import.dxf", &json!({ "path": path, "name": "the drawing" }))
        .expect("the file is read");
    assert_eq!(outcome.data["entities"], json!(1), "there is one entity: {outcome:?}");
    assert_eq!(outcome.data["layers"], json!(2), "and two layers: {outcome:?}");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let node = session.document().node(drawing).expect("the drawing is there");
    let block = node.children.iter().find(|child| child.kind == "cad.block").expect("there is a block");
    assert_eq!(block.name, "*Model_Space", "the block is the tree's own: {}", block.name);
    let entity =
        block.children.iter().find(|child| child.kind == "cad.entity.line").expect("there is a line");
    assert_eq!(
        entity.data["geometry"]["from"],
        json!({ "x": 1.0, "y": 2.0 }),
        "the line starts where the file says: {entity:?}"
    );
    assert_eq!(entity.data["geometry"]["to"], json!({ "x": 4.0, "y": 6.0 }), "and ends there");
}

/// The **layers** of the file are the layers of the drawing, with
/// their colour indices.
#[test]
fn the_layers_of_the_file_are_layers() {
    let mut session = session();
    let path = a_file("layers", &a_file_with(&a_line()));
    let outcome = session.execute("cad.import.dxf", &json!({ "path": path })).expect("the file is read");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let node = session.document().node(drawing).expect("the drawing is there");
    let layers: Vec<(&str, &str)> = node
        .children
        .iter()
        .filter(|child| child.kind == "cad.layer")
        .map(|layer| (layer.name.as_str(), layer.data["color"].as_str().unwrap_or_default()))
        .collect();
    assert_eq!(layers, vec![("0", "7"), ("the walls", "3")], "the layers are the file's: {layers:?}");
}

/// The **layer** of an entity is a relation, as it is everywhere else
/// in this tree.
#[test]
fn an_entity_is_on_its_layer() {
    let mut session = session();
    let path = a_file("layer", &a_file_with(&a_line()));
    let outcome = session.execute("cad.import.dxf", &json!({ "path": path })).expect("the file is read");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let node = session.document().node(drawing).expect("the drawing is there");
    let block = node.children.iter().find(|child| child.kind == "cad.block").expect("there is a block");
    let entity =
        block.children.iter().find(|child| child.kind == "cad.entity.line").expect("there is a line");
    let relations = session.document().relations_from(entity.id, Some(craftkit_sdk::layer::ON_LAYER));
    let relation = relations.first().expect("the entity is on a layer");
    assert_eq!(relation.kind, craftkit_sdk::layer::ON_LAYER, "and it is the layer relation");
    let layer = node.children.iter().find(|child| child.id == relation.to).expect("the layer is there");
    assert_eq!(layer.name, "the walls", "the layer the file named");
}

/// A **circle** and an **arc** are read, and the **degrees** of a file
/// are **radians** here.
#[test]
fn a_circle_and_an_arc_are_read() {
    let mut session = session();
    let mut entities = String::new();
    entities.push_str("0\nCIRCLE\n8\n0\n10\n5.0\n20\n6.0\n40\n2.5\n");
    entities.push_str("0\nARC\n8\n0\n10\n1.0\n20\n1.0\n40\n4.0\n50\n0.0\n51\n90.0\n");
    let path = a_file("circle", &a_file_with(&entities));
    let outcome = session.execute("cad.import.dxf", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["entities"], json!(2), "there are two: {outcome:?}");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let node = session.document().node(drawing).expect("the drawing is there");
    let block = node.children.iter().find(|child| child.kind == "cad.block").expect("there is a block");
    let circle = block.children.iter().find(|child| child.kind == "cad.entity.circle").expect("a circle");
    assert_eq!(circle.data["geometry"]["radius"], json!(2.5), "the radius is the file's");
    let arc = block.children.iter().find(|child| child.kind == "cad.entity.arc").expect("an arc");
    let start = arc.data["geometry"]["start"].as_f64().expect("the start is a number");
    let end = arc.data["geometry"]["end"].as_f64().expect("the end is a number");
    assert!((start - 0.0).abs() < 1e-9, "the start is zero radians: {start}");
    assert!((end - std::f64::consts::FRAC_PI_2).abs() < 1e-9, "and the end is ninety degrees: {end}");
}

/// A **polyline** is read, and the **closed** flag of the file is
/// kept.
#[test]
fn a_polyline_is_read() {
    let mut session = session();
    let mut entities = String::new();
    entities.push_str(
        "0\nLWPOLYLINE\n8\n0\n90\n3\n70\n1\n10\n0.0\n20\n0.0\n10\n4.0\n20\n0.0\n10\n4.0\n20\n3.0\n",
    );
    let path = a_file("polyline", &a_file_with(&entities));
    let outcome = session.execute("cad.import.dxf", &json!({ "path": path })).expect("the file is read");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let node = session.document().node(drawing).expect("the drawing is there");
    let block = node.children.iter().find(|child| child.kind == "cad.block").expect("there is a block");
    let polyline =
        block.children.iter().find(|child| child.kind == "cad.entity.polyline").expect("a polyline");
    let points = polyline.data["geometry"]["points"].as_array().expect("there are points");
    assert_eq!(points.len(), 3, "the three of the file");
    assert_eq!(polyline.data["geometry"]["closed"], json!(true), "and it is closed");
}

/// An entity this tree cannot read is **skipped**, and the answer says
/// what it skipped.
#[test]
fn an_entity_that_cannot_be_read_is_skipped() {
    let mut session = session();
    let mut entities = a_line();
    entities.push_str("0\nTEXT\n8\n0\n10\n0.0\n20\n0.0\n1\nthe words\n");
    let path = a_file("skipped", &a_file_with(&entities));
    let outcome = session.execute("cad.import.dxf", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["entities"], json!(1), "only the line is read: {outcome:?}");
    let skipped = outcome.data["skipped"].as_array().expect("the skipped are a list");
    assert_eq!(skipped, &vec![json!("TEXT")], "and the text is said");
}

/// A file that is **not DXF** is refused, and so is a file that
/// cannot be read.
#[test]
fn a_file_that_is_not_dxf_is_refused() {
    let mut session = session();
    let path = a_file("notdxf", "this is not a drawing file\n");
    let error = session.execute("cad.import.dxf", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("not a DXF file"), "the refusal says why: {error}");
    let error = session
        .execute("cad.import.dxf", &json!({ "path": "/there/is/no/such/file.dxf" }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("cannot be read"), "the refusal says why: {error}");
    let error = session.execute("cad.import.dxf", &json!({ "path": "  " })).expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}

/// The **units** of the file are said, and they are the file's own.
#[test]
fn the_units_of_the_file_are_said() {
    let mut session = session();
    let path = a_file("units", &a_file_with(&a_line()));
    let outcome = session.execute("cad.import.dxf", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["units"], json!(4), "the file declares millimetres: {outcome:?}");
}
