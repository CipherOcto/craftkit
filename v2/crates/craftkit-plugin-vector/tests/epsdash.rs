//! The **finish** of a line in the EPS file: the width, the end, the
//! corner and the dash.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-epsdash-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **rectangle** of ten by ten, with a line on it, and its id.
fn a_rectangle(session: &mut Session) -> u64 {
    let outcome = session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    let id = outcome.data["id"].as_u64().expect("the path has an id");
    session
        .execute("vector.path.stroke", &json!({ "path": id, "colour": "#000000", "width": 2.0 }))
        .expect("the path has a line");
    id
}

/// The **file** a drawing is written as.
fn the_file(session: &mut Session, name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/drawing.eps");
    session.execute("vector.export.eps", &json!({ "path": path })).expect("the drawing is written");
    std::fs::read_to_string(&path).expect("the file is there")
}

/// **A dashed line is written with its pattern and its offset.**
#[test]
fn a_dashed_line_is_written_with_its_pattern_and_its_offset() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.dash", &json!({ "path": id, "pattern": [6.0, 3.0], "offset": 1.5 }))
        .expect("the line is dashed");
    let file = the_file(&mut session, "dashed");
    assert!(file.contains("[6 3] 1.5 d\n"), "the dash is written: {file}");
}

/// **A line that is whole is written with no dash.**
#[test]
fn a_line_that_is_whole_is_written_with_no_dash() {
    let mut session = session();
    a_rectangle(&mut session);
    let file = the_file(&mut session, "whole");
    assert!(!file.contains(" d\n"), "there is no dash in it: {file}");
}

/// **A pattern with no length in it is no dash.**
///
/// The tree's own rule: a pattern that is not dashed takes the dash
/// away.
#[test]
fn a_pattern_with_no_length_in_it_is_no_dash() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.dash", &json!({ "path": id, "pattern": [0.0, 0.0] }))
        .expect("the line is dashed");
    let file = the_file(&mut session, "zeros");
    assert!(!file.contains(" d\n"), "there is no dash in it: {file}");
}

/// **The ends and the corners of a line are written.**
#[test]
fn the_ends_and_the_corners_of_a_line_are_written() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session.execute("vector.path.cap", &json!({ "path": id, "cap": "square" })).expect("the ends are set");
    session
        .execute("vector.path.join", &json!({ "path": id, "join": "round" }))
        .expect("the corners are set");
    let file = the_file(&mut session, "ends");
    assert!(file.contains("2 J\n"), "the end is written: {file}");
    assert!(file.contains("1 j\n"), "and the corner: {file}");
}
