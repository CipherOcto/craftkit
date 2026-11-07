//! The **SVG** of a drawing: a drawing made into a picture file.
//!
//! The file is an SVG, and the drawing it shows is the drawing that
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
        .join(format!("craftkit-v2-svgout-{name}-{}", std::process::id()))
        .to_string_lossy()
        .to_string()
}

/// A **drawing** with a layer, a block and a line.
fn a_drawing(session: &mut Session) -> (u64, u64, u64) {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.layer.add", &json!({ "drawing": drawing, "name": "the walls" }))
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

/// A drawing is written as an **SVG file**: the picture of the
/// drawing.
#[test]
fn a_drawing_is_written_as_an_svg_file() {
    let mut session = session();
    let (drawing, _block, _entity) = a_drawing(&mut session);
    let path = a_path("file");
    let outcome = session
        .execute("cad.export.svg", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    assert_eq!(outcome.data["entities"], json!(1), "there is one entity: {outcome:?}");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    assert!(text.starts_with("<svg"), "the file is a picture: {text}");
    assert!(text.contains("xmlns=\"http://www.w3.org/2000/svg\""), "and it says so");
    assert!(text.contains("<rect"), "with a white ground");
    assert!(text.contains("fill=\"white\""), "that is white");
    assert!(text.contains("<path"), "and the line in it");
    assert!(text.trim_end().ends_with("</svg>"), "and it ends");
}

/// The **box** of the picture is the extents of the drawing with a
/// **margin** of two percent of the larger side, and the **world is
/// turned over**: the tree's Y is up and an SVG's Y is down.
#[test]
fn the_box_is_the_extents_with_a_margin_and_the_world_is_turned_over() {
    let mut session = session();
    let (drawing, _block, _entity) = a_drawing(&mut session);
    let path = a_path("box");
    session
        .execute("cad.export.svg", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    // The drawing is 3 by 4, so the larger side is 4 and the margin is
    // 0.08: the box is 3.16 by 4.16.
    assert!(
        text.contains("viewBox=\"0 0 3.160000 4.160000\""),
        "the box is the extents and the margin: {text}"
    );
    // The line runs from (1, 2) to (4, 6). The box's left is 1 - 0.08
    // and its top is 6 + 0.08, so the line starts at (0.08, 4.08) and
    // ends at (3.08, 0.08).
    assert!(text.contains("M0.080000 4.080000"), "the world is turned over: {text}");
    assert!(text.contains("L3.080000 0.080000"), "and it ends at the other end: {text}");
}

/// The **numbers** carry at least six decimals, and more when the
/// stroke would lose its leading digits: the tree's own rule.
#[test]
fn the_numbers_carry_the_decimals_of_the_rule() {
    let mut session = session();
    let (drawing, _block, _entity) = a_drawing(&mut session);
    let path = a_path("decimals");
    session
        .execute("cad.export.svg", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    // The stroke is 4.16 / 1500, which is 0.00277, so the rule asks
    // for 3 - (-3) = 6 decimals: the six it always has.
    let decimals = text
        .split("viewBox=\"0 0 ")
        .nth(1)
        .and_then(|rest| rest.split(' ').next())
        .and_then(|number| number.split('.').nth(1))
        .map(|tail| tail.len())
        .expect("the view box holds a number");
    assert_eq!(decimals, 6, "the numbers carry six decimals: {text}");
}

/// A **closed polyline** is a path that comes back to where it began.
#[test]
fn a_closed_polyline_is_a_path_that_comes_back() {
    let mut session = session();
    let (drawing, block, _entity) = a_drawing(&mut session);
    session
        .execute(
            "cad.entity.polyline.add",
            &json!({
                "block": block,
                "name": "the room",
                "points": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 2.0, "y": 0.0 },
                    { "x": 2.0, "y": 1.0 },
                ],
                "closed": true,
            }),
        )
        .expect("the polyline is added");
    let path = a_path("closed");
    session
        .execute("cad.export.svg", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    assert!(text.contains("<path"), "the closed polyline is a path: {text}");
    assert!(text.contains("Z\""), "and it comes back: {text}");
    assert!(!text.contains("<polyline"), "and no polyline is left: {text}");
}

/// The **colour** of an entity is the colour of its layer: the fill
/// the layer says.
#[test]
fn the_colour_of_an_entity_is_the_colour_of_its_layer() {
    let mut session = session();
    let (drawing, _block, _entity) = a_drawing(&mut session);
    let outcome =
        session.execute("cad.layer.list", &json!({ "drawing": drawing })).expect("the layers are read");
    let layers = outcome.data["layers"].as_array().expect("there are layers");
    let walls = layers.iter().find(|layer| layer["name"] == json!("the walls")).expect("the layer is there");
    let layer = walls["id"].as_u64().expect("the layer has an id");
    session
        .execute("cad.layer.fill", &json!({ "layer": layer, "fill": "#ff8800" }))
        .expect("the layer is drawn in orange");
    let path = a_path("colour");
    session
        .execute("cad.export.svg", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    let text = std::fs::read_to_string(&path).expect("the file is there");
    assert!(text.contains("stroke=\"#ff8800\""), "the line is drawn in the colour of its layer: {text}");
}

/// An entity this tree cannot draw is **left out**, and the answer
/// says what it left out.
#[test]
fn an_entity_that_cannot_be_drawn_is_left_out() {
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
        .execute("cad.export.svg", &json!({ "path": path, "drawing": drawing }))
        .expect("the drawing is written");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert_eq!(left_out, &vec![json!("the words")], "the text is said");
    assert_eq!(outcome.data["entities"], json!(1), "and only the line is drawn");
}

/// A **node** that is not a drawing is refused.
#[test]
fn a_node_that_is_not_a_drawing_is_refused() {
    let mut session = session();
    let (drawing, block, _entity) = a_drawing(&mut session);
    let error = session
        .execute("cad.export.svg", &json!({ "path": a_path("wrong"), "drawing": block }))
        .expect_err("the block is not a drawing");
    assert!(error.to_string().contains("not a drawing"), "the refusal says why: {error}");
    let error = session
        .execute("cad.export.svg", &json!({ "path": "  ", "drawing": drawing }))
        .expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}
