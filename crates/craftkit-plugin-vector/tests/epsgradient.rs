//! The **gradients** of a path in the EPS file: the shading of
//! PostScript.

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

/// A **directory** of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-epsgrad-{name}-{}", std::process::id()))
        .expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **rectangle** of ten by ten, and its id.
fn a_rectangle(session: &mut Session) -> u64 {
    let outcome = session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    outcome.data["id"].as_u64().expect("the path has an id")
}

/// The **file** a drawing is written as.
fn the_file(session: &mut Session, name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/drawing.eps");
    session.execute("vector.export.eps", &json!({ "path": path })).expect("the drawing is written");
    std::fs::read_to_string(&path).expect("the file is there")
}

/// **A linear gradient is written as a shading.**
#[test]
fn a_linear_gradient_is_written_as_a_shading() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.gradient",
            &json!({
                "path": id,
                "kind": "linear",
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 10.0, "y": 0.0 },
                "stops": [
                    { "offset": 0.0, "colour": "#ff0000" },
                    { "offset": 1.0, "colour": "#0000ff" },
                ],
            }),
        )
        .expect("the path has a gradient");
    let file = the_file(&mut session, "linear");
    assert!(file.contains("/ShadingType 2"), "the shading is a linear one: {file}");
    assert!(file.contains("/Coords [0 0 10 0]"), "and it says where it goes: {file}");
    assert!(file.contains("/Extend [true true]"), "and it is extended both ways: {file}");
    assert!(file.contains("shfill"), "and the path is filled with it: {file}");
}

/// **A radial gradient is written as a shading of kind three.**
#[test]
fn a_radial_gradient_is_written_as_a_shading_of_kind_three() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.gradient",
            &json!({
                "path": id,
                "kind": "radial",
                "centre": { "x": 5.0, "y": 5.0 },
                "radius": 5.0,
                "stops": [
                    { "offset": 0.0, "colour": "#ffffff" },
                    { "offset": 1.0, "colour": "#000000" },
                ],
            }),
        )
        .expect("the path has a gradient");
    let file = the_file(&mut session, "radial");
    assert!(file.contains("/ShadingType 3"), "the shading is a radial one: {file}");
    assert!(file.contains("/Coords [5 5 0 5 5 5]"), "and it says where it goes: {file}");
}

/// **Two stops are one exponential function.**
#[test]
fn two_stops_are_one_exponential_function() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.gradient",
            &json!({
                "path": id,
                "kind": "linear",
                "stops": [
                    { "offset": 0.0, "colour": "#ff0000" },
                    { "offset": 1.0, "colour": "#0000ff" },
                ],
            }),
        )
        .expect("the path has a gradient");
    let file = the_file(&mut session, "two");
    assert!(file.contains("/FunctionType 2"), "the function is an exponential one: {file}");
    assert!(file.contains("/C0 [1 0 0]"), "and it holds the first colour: {file}");
    assert!(file.contains("/C1 [0 0 1]"), "and the second: {file}");
}

/// **Three stops are a stitching function.**
#[test]
fn three_stops_are_a_stitching_function() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.gradient",
            &json!({
                "path": id,
                "kind": "linear",
                "stops": [
                    { "offset": 0.0, "colour": "#ff0000" },
                    { "offset": 0.5, "colour": "#00ff00" },
                    { "offset": 1.0, "colour": "#0000ff" },
                ],
            }),
        )
        .expect("the path has a gradient");
    let file = the_file(&mut session, "three");
    assert!(file.contains("/FunctionType 3"), "the function stitches the pieces: {file}");
    assert!(file.contains("/Bounds [0.5]"), "and it says where they meet: {file}");
    assert!(file.contains("/Encode [0 1 0 1]"), "and how each piece is mapped: {file}");
    assert!(file.contains("/Domain [0 0.5]"), "and the stretch of each piece: {file}");
}

/// **A path with a gradient is drawn at all.**
///
/// Before this, a path whose fill was a gradient was not in the file.
#[test]
fn a_path_with_a_gradient_is_drawn_at_all() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute(
            "vector.path.gradient",
            &json!({
                "path": id,
                "kind": "linear",
                "stops": [
                    { "offset": 0.0, "colour": "#ff0000" },
                    { "offset": 1.0, "colour": "#0000ff" },
                ],
            }),
        )
        .expect("the path has a gradient");
    let file = the_file(&mut session, "drawn");
    assert!(file.contains("m\n"), "the path is in the file: {file}");
    assert!(file.contains("shfill"), "and it is filled with the shading: {file}");
}
