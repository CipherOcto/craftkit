//! The **reading** of an SVG file.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_vector::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the vector battery.
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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-svgimport-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// Write an SVG file and **read** it into the document.
fn read(session: &mut Session, name: &str, svg: &str) -> craftkit_sdk::serde_json::Value {
    let directory = directory(name);
    let path = format!("{directory}/drawing.svg");
    std::fs::write(&path, svg).expect("the file is written");
    let outcome = session.execute("vector.import.svg", &json!({ "path": path })).expect("the file is read");
    outcome.data.clone()
}

/// A **path** of the document, and where it is.
fn placed(session: &Session, id: u64) -> (f64, f64, f64, f64) {
    let node = session.document().node(id).expect("the path is there");
    let rect = node.rect.expect("the path has a box");
    (rect.x, rect.y, rect.w, rect.h)
}

/// The paths of an SVG file are read, and the file is **turned over**:
/// the world's Y is up and an SVG file's Y is down.
#[test]
fn the_paths_of_an_svg_file_are_read() {
    let mut session = session();
    let read = read(
        &mut session,
        "plain",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50">
             <path d="M 0 0 L 100 0 L 100 50 L 0 50 Z" fill="#ff0000"/>
           </svg>"##,
    );
    assert_eq!(read["paths"], json!(1), "the file holds one path");
    let id = read["ids"][0].as_u64().expect("the path has an id");
    let (_x, y, w, h) = placed(&session, id);
    // A pixel is 25.4/96 of a millimetre.
    let scale = 25.4 / 96.0;
    assert!((w - 100.0 * scale).abs() < 0.001, "the path is as wide as the file: {w}");
    assert!((h - 50.0 * scale).abs() < 0.001, "and as tall: {h}");
    assert!(y.abs() < 0.001, "and its bottom stands at the origin: {y}");
}

/// The **fill** and the **stroke** of a path are read.
#[test]
fn the_fill_and_the_stroke_are_read() {
    let mut session = session();
    let read = read(
        &mut session,
        "paint",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
             <path d="M 0 0 L 10 0 L 10 10 Z" fill="rgb(255,0,0)" stroke="none"/>
           </svg>"##,
    );
    let id = read["ids"][0].as_u64().expect("the path has an id");
    let node = session.document().node(id).expect("the path is there");
    assert_eq!(node.data["fill"]["r"].as_f64(), Some(1.0), "the fill is red");
    assert_eq!(node.data["fill"]["g"].as_f64(), Some(0.0), "and no green");
    assert!(node.data["stroke"].is_null(), "and there is no stroke");
}

/// A **transform** puts the drawing somewhere else.
#[test]
fn a_transform_puts_the_drawing_somewhere_else() {
    let mut session = session();
    let read = read(
        &mut session,
        "moved",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
             <g transform="translate(10,0)">
               <path d="M 0 0 L 10 0 L 10 10 Z" fill="#000000"/>
             </g>
           </svg>"##,
    );
    let id = read["ids"][0].as_u64().expect("the path has an id");
    let (x, _, _, _) = placed(&session, id);
    let scale = 25.4 / 96.0;
    assert!((x - 10.0 * scale).abs() < 0.001, "the group moved the path across: {x}");
}

/// An element this tree cannot read is **skipped**, and the answer says
/// what it skipped.
#[test]
fn an_element_the_tree_cannot_read_is_skipped() {
    let mut session = session();
    let read = read(
        &mut session,
        "skipped",
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
             <rect x="0" y="0" width="10" height="10"/>
             <path d="M 0 0 L 10 0 L 10 10 Z" fill="#000000"/>
           </svg>"##,
    );
    assert_eq!(read["paths"], json!(1), "the path is read");
    let skipped = read["skipped"].as_array().expect("the skipped are a list");
    assert_eq!(skipped.len(), 1, "and the rectangle is skipped");
    assert_eq!(skipped[0], json!("rect"), "and the answer says which");
}

/// A file with **no path** in it is refused.
#[test]
fn a_file_with_no_path_is_refused() {
    let mut session = session();
    let directory = directory("empty");
    let path = format!("{directory}/drawing.svg");
    std::fs::write(&path, "<svg xmlns=\"http://www.w3.org/2000/svg\"></svg>").expect("the file is written");
    let outcome = session.execute("vector.import.svg", &json!({ "path": path }));
    assert!(outcome.is_err(), "a file with no path is refused");
    let message = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("no path"), "and it says why: {message}");
}

/// A path that holds an **arc** is not read: an arc is a curve, and
/// this tree does not write a curve it cannot say. A file with no
/// other path is refused, and the refusal says why.
#[test]
fn a_path_that_holds_an_arc_is_refused() {
    let mut session = session();
    let directory = directory("arc");
    let path = format!("{directory}/drawing.svg");
    std::fs::write(
        &path,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 100 100\"><path d=\"M 0 0 A 10 10 0 0 1 20 20\"/></svg>",
    )
    .expect("the file is written");
    let outcome = session.execute("vector.import.svg", &json!({ "path": path }));
    assert!(outcome.is_err(), "a file with no readable path is refused");
    let message = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("arc"), "and it says why: {message}");
}
