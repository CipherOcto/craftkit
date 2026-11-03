//! The **opacity** of a path.
//!
//! The opacity is how solid a path is: it multiplies into the fill and
//! into the line on the outline.

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

/// A **rectangle**, and its id.
fn a_path(session: &mut Session) -> u64 {
    let outcome = session
        .execute(
            "vector.rectangle",
            &json!({ "x": 0, "y": 0, "width": 100, "height": 50, "fill": "#FF0000" }),
        )
        .expect("the rectangle is added");
    outcome.data["id"].as_u64().expect("the path has an id")
}

/// The **opacity** of a path, as the document holds it.
fn opacity_of(session: &Session, path: u64) -> f64 {
    let node = session.document().node(path).expect("the path is there");
    node.data.get("opacity").and_then(|opacity| opacity.as_f64()).unwrap_or(1.0)
}

/// A new path is **fully solid**.
#[test]
fn a_new_path_is_fully_solid() {
    let mut session = session();
    let path = a_path(&mut session);
    assert_eq!(opacity_of(&session, path), 1.0, "the path is fully solid");
}

/// The opacity may be **set**.
#[test]
fn the_opacity_may_be_set() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.opacity", &json!({ "path": path, "opacity": 50 }))
        .expect("the path is half solid");
    assert_eq!(opacity_of(&session, path), 0.5, "the path is half solid");
}

/// An opacity outside **zero to a hundred** is refused: the tree's
/// command clamps it, and a clamped number is a number that was not
/// asked for.
#[test]
fn an_opacity_outside_the_range_is_refused() {
    let mut session = session();
    let path = a_path(&mut session);
    let error = session
        .execute("vector.path.opacity", &json!({ "path": path, "opacity": 1000 }))
        .expect_err("a thousand per cent is not an opacity");
    assert!(error.to_string().contains("opacity"), "the error says so: {error}");
    assert_eq!(opacity_of(&session, path), 1.0, "and the path is unchanged");
}

/// A path may be **nothing** at all.
#[test]
fn a_path_may_be_nothing_at_all() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.opacity", &json!({ "path": path, "opacity": 0 }))
        .expect("the path is nothing");
    assert_eq!(opacity_of(&session, path), 0.0, "the path is nothing");
}

/// The opacity multiplies into the **fill**: a path that is half solid
/// and filled with a colour is filled with half that colour.
#[test]
fn the_opacity_multiplies_into_the_fill() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.opacity", &json!({ "path": path, "opacity": 50 }))
        .expect("the path is half solid");
    let node = session.document().node(path).expect("the path is there");
    let opacity = node.data["opacity"].as_f64().expect("the path has an opacity");
    let fill = craftkit_plugin_vector::fill_at(
        opacity,
        &craftkit_sdk::Fill::Solid(craftkit_sdk::Colour([1.0, 0.0, 0.0, 1.0])),
    );
    let craftkit_sdk::Fill::Solid(colour) = fill else {
        panic!("the fill is a colour");
    };
    assert_eq!(colour.0[3], 0.5, "the fill is half solid");
}

/// The opacity multiplies into the **line** as well.
#[test]
fn the_opacity_multiplies_into_the_line() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.opacity", &json!({ "path": path, "opacity": 25 }))
        .expect("the path is a quarter solid");
    let stroke = craftkit_plugin_vector::stroke_at(
        0.25,
        &craftkit_sdk::Stroke { colour: craftkit_sdk::Colour([0.0, 0.0, 0.0, 1.0]), ..Default::default() },
    );
    assert_eq!(stroke.colour.0[3], 0.25, "the line is a quarter solid");
}

/// The opacity is **saved** with the path.
#[test]
fn the_opacity_is_saved_with_the_path() {
    let mut session = session();
    let path = a_path(&mut session);
    session
        .execute("vector.path.opacity", &json!({ "path": path, "opacity": 25 }))
        .expect("the path is a quarter solid");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(path).expect("the path is there");
    assert_eq!(node.data["opacity"].as_f64(), Some(0.25), "the path is still a quarter solid");
}
