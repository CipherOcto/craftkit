//! The **paint** of a path: the fill inside it and the line on it.
//!
//! The tree's path carries a fill and a stroke, and two commands say
//! what they are. v2's path carried both and no command could write
//! either.

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

/// The paint of a node, as the document holds it.
fn paint_of(session: &Session, id: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(id).expect("the node is there");
    node.data.clone()
}

/// A path may be **filled** with a colour.
#[test]
fn a_path_may_be_filled() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.fill", &json!({ "path": id, "colour": "#ff0000" }))
        .expect("the path is filled");
    let paint = paint_of(&session, id);
    let fill = paint.get("fill").expect("the path has a fill");
    assert_eq!(fill.get("r").and_then(|r| r.as_f64()), Some(1.0), "the fill is the colour: {fill}");
    assert_eq!(fill.get("g").and_then(|g| g.as_f64()), Some(0.0), "the fill is the colour: {fill}");
}

/// A path may have a **line** on it: a colour and a width.
#[test]
fn a_path_may_have_a_line() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.stroke", &json!({ "path": id, "colour": "#0000ff", "width": 2.0 }))
        .expect("the path has a line");
    let paint = paint_of(&session, id);
    let stroke = paint.get("stroke").expect("the path has a line");
    let colour = stroke.get("colour").expect("the line has a colour");
    assert_eq!(colour.get("b").and_then(|b| b.as_f64()), Some(1.0), "the line is the colour: {colour}");
    assert_eq!(stroke.get("width").and_then(|width| width.as_f64()), Some(2.0), "the line is that wide");
}

/// The line may be **taken away** again.
#[test]
fn the_line_may_be_taken_away() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    session
        .execute("vector.path.stroke", &json!({ "path": id, "colour": "#0000ff", "width": 2.0 }))
        .expect("the path has a line");
    session
        .execute(
            "vector.path.stroke",
            &json!({ "path": id, "colour": "#0000ff", "width": 2.0, "none": true }),
        )
        .expect("the line is gone");
    let paint = paint_of(&session, id);
    assert!(paint.get("stroke").map(|stroke| stroke.is_null()).unwrap_or(false), "the line is gone");
}

/// A colour that is **not a colour** is refused by name, and so is a
/// width that is not a width.
#[test]
fn a_colour_that_is_not_a_colour_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let refused = session.execute("vector.path.fill", &json!({ "path": id, "colour": "blue" }));
    let message = match refused {
        Ok(_) => panic!("a colour that is not a colour is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains('#'), "the refusal says how a colour is written: {message}");
    let thin =
        session.execute("vector.path.stroke", &json!({ "path": id, "colour": "#000000", "width": -1.0 }));
    assert!(thin.is_err(), "a negative width is refused");
}
