//! The **writing** of a drawing as an SVG file.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-svg-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **rectangle** of ten by ten, and its id.
fn a_rectangle(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "vector.path")
        .map(|node| node.id)
        .expect("the rectangle is there")
}

/// The **file** a drawing is written as.
fn the_file(session: &mut Session, name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/drawing.svg");
    session.execute("vector.export.svg", &json!({ "path": path })).expect("the drawing is written");
    std::fs::read_to_string(&path).expect("the file is there")
}

/// A drawing is written as an SVG file: the path is in it, and the box
/// of the drawing is the box of the file.
#[test]
fn a_drawing_is_written_as_an_svg_file() {
    let mut session = session();
    a_rectangle(&mut session);
    let file = the_file(&mut session, "rectangle");
    assert!(file.contains("<svg xmlns=\"http://www.w3.org/2000/svg\""), "it is an SVG: {file}");
    assert!(file.contains("width=\"10\""), "and it is as wide as the drawing: {file}");
    assert!(file.contains("height=\"10\""), "and as tall: {file}");
    assert!(file.contains("<path d=\"M 0 0 L 10 0 L 10 10 L 0 10 Z\""), "and the path is in it: {file}");
    assert!(file.contains("scale(1,-1)"), "and the world is turned over: {file}");
}

/// The **view box** of the file is the box of the world turned over.
#[test]
fn the_view_box_is_the_box_of_the_world_turned_over() {
    let mut session = session();
    a_rectangle(&mut session);
    let file = the_file(&mut session, "viewbox");
    assert!(file.contains("viewBox=\"0 -10 10 10\""), "the box is the world's, turned over: {file}");
}

/// The **fill** and the **stroke** of a path are written as the
/// attributes of the path.
#[test]
fn the_fill_and_the_stroke_are_written() {
    let mut session = session();
    let path = a_rectangle(&mut session);
    session
        .execute("vector.path.fill", &json!({ "path": path, "colour": "#ff0000" }))
        .expect("the fill is set");
    let file = the_file(&mut session, "fill");
    assert!(file.contains("fill=\"#ff0000\""), "the fill is written: {file}");
    assert!(file.contains("stroke=\"none\""), "and a path with no stroke says so: {file}");
}

/// A drawing with **no path** is refused.
#[test]
fn a_drawing_with_no_path_is_refused() {
    let mut session = session();
    let directory = directory("empty");
    let path = format!("{directory}/drawing.svg");
    let outcome = session.execute("vector.export.svg", &json!({ "path": path }));
    assert!(outcome.is_err(), "a drawing with no path is refused");
    let message = outcome.err().map(|error| error.to_string()).unwrap_or_default();
    assert!(message.contains("no path"), "and it says why: {message}");
}
