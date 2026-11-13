//! The **SVG** read back: a picture file made into a drawing.
//!
//! The file is an SVG, and the drawing it names is the drawing that
//! was read.

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

/// A path of this test's own, so that two tests do not fight over one
/// file.
fn a_path(name: &str) -> String {
    craftkit_scratch::file(&format!("craftkit-v2-svgin-{name}-{}", std::process::id()))
        .expect("the scratch is made")
        .to_string_lossy()
        .to_string()
}

/// **Write** a file of this test's own.
fn a_file(name: &str, text: &str) -> String {
    let path = a_path(name);
    std::fs::write(&path, text.as_bytes()).expect("the file is written");
    path
}

/// The **entities** of the drawing that was read.
fn entities_of(session: &Session, drawing: u64) -> Vec<(String, Json)> {
    let node = session.document().node(drawing).expect("the drawing is there");
    let block = node.children.iter().find(|child| child.kind == "cad.block").expect("there is a block");
    block.children.iter().map(|child| (child.kind.clone(), child.data["geometry"].clone())).collect()
}

/// A picture of a **line**, a **circle** and a **rectangle** is read
/// as a drawing of the three.
#[test]
fn a_picture_is_read_as_a_drawing() {
    let mut session = session();
    let path = a_file(
        "shapes",
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50">
             <line x1="10" y1="10" x2="40" y2="10"/>
             <circle cx="20" cy="20" r="5"/>
             <rect x="0" y="30" width="10" height="10"/>
           </svg>"#,
    );
    let outcome = session
        .execute("cad.import.svg", &json!({ "path": path, "name": "the picture" }))
        .expect("the picture is read");
    assert_eq!(outcome.data["entities"], json!(3), "there are three entities: {outcome:?}");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let node = session.document().node(drawing).expect("the drawing is there");
    assert_eq!(node.name, "the picture", "the drawing has the name that was asked for");
    let entities = entities_of(&session, drawing);
    assert_eq!(entities.len(), 3, "there are three entities: {entities:?}");
    assert_eq!(entities[0].0, "cad.entity.line", "the line is the first: {entities:?}");
    assert_eq!(
        entities[0].1["from"],
        json!({ "x": 10.0, "y": 40.0 }),
        "the line starts where the picture says: {entities:?}"
    );
    assert_eq!(entities[0].1["to"], json!({ "x": 40.0, "y": 40.0 }), "and ends there: {entities:?}");
    assert_eq!(entities[1].0, "cad.entity.circle", "the circle is the second: {entities:?}");
    assert_eq!(
        entities[1].1["center"],
        json!({ "x": 20.0, "y": 30.0 }),
        "and stands where the picture says: {entities:?}"
    );
    assert_eq!(entities[1].1["radius"], json!(5.0), "with its radius: {entities:?}");
    assert_eq!(entities[2].0, "cad.entity.polyline", "the rectangle is a polyline: {entities:?}");
    assert_eq!(entities[2].1["closed"], json!(true), "and it is closed: {entities:?}");
    assert_eq!(
        entities[2].1["points"],
        json!([
            { "x": 0.0, "y": 20.0 },
            { "x": 10.0, "y": 20.0 },
            { "x": 10.0, "y": 10.0 },
            { "x": 0.0, "y": 10.0 }
        ]),
        "with the four corners of the picture: {entities:?}"
    );
}

/// The **Y of the page is turned over**: a page has its Y down and
/// this tree's world has it up.
#[test]
fn the_y_of_the_page_is_turned_over() {
    let mut session = session();
    let path = a_file(
        "flipped",
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="50mm" viewBox="0 0 100 50">
             <line x1="0" y1="0" x2="0" y2="50"/>
           </svg>"#,
    );
    let outcome = session.execute("cad.import.svg", &json!({ "path": path })).expect("the picture is read");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let entities = entities_of(&session, drawing);
    // The top of the page is the top of the drawing, and the bottom
    // of the page is the bottom of the drawing.
    assert_eq!(
        entities[0].1["from"],
        json!({ "x": 0.0, "y": 50.0 }),
        "the top of the page is the top of the drawing: {entities:?}"
    );
    assert_eq!(
        entities[0].1["to"],
        json!({ "x": 0.0, "y": 0.0 }),
        "and the bottom is the bottom: {entities:?}"
    );
}

/// The **units** of a page are the tree's own: 96 pixels to the inch,
/// and a width in millimetres over the width of the view box.
#[test]
fn the_units_of_a_page_are_the_tree_s_own() {
    let mut session = session();
    // A page of 200 pixels wide, and a view box of 100 units: two
    // millimetres to a unit.
    let path = a_file(
        "units",
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="200px" height="100px" viewBox="0 0 100 50">
             <line x1="0" y1="0" x2="10" y2="0"/>
           </svg>"#,
    );
    let outcome = session.execute("cad.import.svg", &json!({ "path": path })).expect("the picture is read");
    let scale = outcome.data["scale"].as_f64().expect("the scale is said");
    assert!((scale - 0.5291666666666666).abs() < 1e-9, "the scale is the tree's own: {scale}");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let entities = entities_of(&session, drawing);
    assert!(
        (entities[0].1["to"]["x"].as_f64().unwrap_or_default() - 5.291666666666666).abs() < 1e-9,
        "the line is that long in millimetres: {entities:?}"
    );
}

/// A **path** is a polyline, and a **curve** in a path is a run of
/// straight segments.
#[test]
fn a_path_is_a_polyline_and_a_curve_is_a_run_of_segments() {
    let mut session = session();
    let path = a_file(
        "path",
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
             <path d="M10 10 L20 10 L20 20 Z"/>
             <path d="M0 0 C 0 10 10 10 10 0"/>
           </svg>"#,
    );
    let outcome = session.execute("cad.import.svg", &json!({ "path": path })).expect("the picture is read");
    assert_eq!(outcome.data["entities"], json!(2), "there are two polylines: {outcome:?}");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let entities = entities_of(&session, drawing);
    assert_eq!(
        entities[0].1["points"],
        json!([
            { "x": 10.0, "y": 90.0 },
            { "x": 20.0, "y": 90.0 },
            { "x": 20.0, "y": 80.0 }
        ]),
        "the path is the points it says: {entities:?}"
    );
    assert_eq!(entities[0].1["closed"], json!(true), "and it is closed: {entities:?}");
    let points = entities[1].1["points"].as_array().expect("the curve has points");
    assert_eq!(points.len(), 17, "a curve is a run of sixteen segments: {entities:?}");
    assert_eq!(
        points[16],
        json!({ "x": 10.0, "y": 100.0 }),
        "and it ends where the curve ends: {entities:?}"
    );
}

/// A path of **relative** commands is a path of relative commands.
#[test]
fn a_path_of_relative_commands_is_read() {
    let mut session = session();
    let path = a_file(
        "relative",
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
             <path d="m10 10 l10 0 l0 10 z"/>
           </svg>"#,
    );
    let outcome = session.execute("cad.import.svg", &json!({ "path": path })).expect("the picture is read");
    let drawing = outcome.data["drawing"].as_u64().expect("the drawing has an id");
    let entities = entities_of(&session, drawing);
    assert_eq!(
        entities[0].1["points"],
        json!([
            { "x": 10.0, "y": 90.0 },
            { "x": 20.0, "y": 90.0 },
            { "x": 20.0, "y": 80.0 }
        ]),
        "the run is where the commands take it: {entities:?}"
    );
}

/// What the tree does not read is **left out**, and the answer says
/// what it left out.
#[test]
fn what_the_tree_does_not_read_is_left_out() {
    let mut session = session();
    let path = a_file(
        "leftout",
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="100mm" height="100mm" viewBox="0 0 100 100">
             <text x="0" y="0">the words</text>
             <g transform="translate(10,10)"><line x1="0" y1="0" x2="1" y2="1"/></g>
             <path d="M0 0 A 5 5 0 0 1 10 10"/>
           </svg>"#,
    );
    let outcome = session.execute("cad.import.svg", &json!({ "path": path })).expect("the picture is read");
    assert_eq!(outcome.data["entities"], json!(0), "nothing is read: {outcome:?}");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert!(left_out.contains(&json!("text")), "the text is said: {left_out:?}");
    assert!(left_out.contains(&json!("a transform")), "the transform is said: {left_out:?}");
    assert!(left_out.contains(&json!("a path with an arc")), "the arc is said: {left_out:?}");
}

/// A file that is **not an SVG file** is refused.
#[test]
fn a_file_that_is_not_an_svg_file_is_refused() {
    let mut session = session();
    let path = a_file("other", "<html><body>the beach</body></html>");
    let error =
        session.execute("cad.import.svg", &json!({ "path": path })).expect_err("a web page is not a picture");
    assert!(error.to_string().contains("not an SVG file"), "the refusal says why: {error}");
    let error = session.execute("cad.import.svg", &json!({ "path": "  " })).expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}
