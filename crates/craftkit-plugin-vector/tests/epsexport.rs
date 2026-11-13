//! The **writing** of a drawing as an EPS file.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-eps-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **rectangle** of ten by ten.
fn a_rectangle(session: &mut Session) {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
}

/// The **file** a drawing is written as.
fn the_file(session: &mut Session, name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/drawing.eps");
    session.execute("vector.export.eps", &json!({ "path": path })).expect("the drawing is written");
    std::fs::read_to_string(&path).expect("the file is there")
}

/// A drawing is written as an EPS file: the prolog, the box and the
/// path are in it.
#[test]
fn a_drawing_is_written_as_an_eps_file() {
    let mut session = session();
    a_rectangle(&mut session);
    let file = the_file(&mut session, "rectangle");
    assert!(file.starts_with("%!PS-Adobe-3.0 EPSF-3.0\n"), "it is an EPS: {file}");
    assert!(file.contains("%%BoundingBox: 0 0 10 10"), "and it says its box: {file}");
    assert!(file.contains("/f* {eofill} bd"), "the prolog of the tree is in it: {file}");
    assert!(file.contains("1 0 0 -1 0 0 cm"), "and the world is turned over: {file}");
    assert!(file.contains("m\n"), "the path is in it: {file}");
    assert!(file.contains("l\n"), "with its lines: {file}");
    assert!(file.contains("h\n"), "and it is closed: {file}");
    assert!(file.contains("rg\n"), "and the ink is written: {file}");
    assert!(file.ends_with("%%Trailer\n%%EOF\n"), "and the file ends as it should: {file}");
}

/// A drawing with **no path** in it is refused.
#[test]
fn a_drawing_with_no_path_in_it_is_refused() {
    let mut session = session();
    let directory = directory("empty");
    let path = format!("{directory}/drawing.eps");
    let refusal =
        session.execute("vector.export.eps", &json!({ "path": path })).expect_err("the drawing is refused");
    let message = refusal.to_string();
    assert!(message.contains("no path"), "the refusal says so: {message}");
}
