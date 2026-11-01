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

/// A path may be filled with a **gradient**.
///
/// The gradient is the kit's own: the colours, the stops they are at,
/// and where the gradient is put. The command says where it is put, and
/// the path's own box is the answer when it does not.
#[test]
fn a_path_may_be_filled_with_a_gradient() {
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
        .expect("the path is filled with a gradient");
    let paint = paint_of(&session, id);
    let gradient = paint.get("fill").and_then(|fill| fill.get("gradient")).expect("there is a gradient");
    assert_eq!(gradient["kind"], json!("linear"), "the gradient says what it is");
    let stops = gradient["stops"].as_array().expect("there are stops");
    assert_eq!(stops.len(), 2, "the gradient has two stops");
    assert_eq!(
        stops[0]["colour"],
        json!({ "r": 1.0, "g": 0.0, "b": 0.0, "a": 1.0 }),
        "the first stop is red"
    );
    // The geometry is the path's **own box**: the gradient goes across
    // it, from the left to the right.
    let linear = &gradient["geometry"]["linear"];
    assert_eq!(linear["from"]["x"], json!(0.0), "it starts at the left of the box");
    assert_eq!(linear["to"]["x"], json!(10.0), "and ends at the right of it");
    assert_eq!(linear["from"]["y"], json!(5.0), "across the middle of it");
}

/// A gradient that cannot be made is **refused**, and it says why.
#[test]
fn a_gradient_that_cannot_be_made_is_refused() {
    let mut session = session();
    let id = a_rectangle(&mut session);
    let stops = json!([
        { "offset": 0.0, "colour": "#ff0000" },
        { "offset": 1.0, "colour": "#0000ff" },
    ]);
    // A kind that is not one of the two.
    let refused =
        session.execute("vector.path.gradient", &json!({ "path": id, "kind": "diagonal", "stops": stops }));
    assert!(refused.is_err(), "a gradient is linear or radial");
    // One stop.
    let refused = session.execute(
        "vector.path.gradient",
        &json!({ "path": id, "kind": "linear", "stops": [{ "offset": 0.0, "colour": "#ff0000" }] }),
    );
    assert!(refused.is_err(), "a gradient has two stops or more");
    // An offset that is out of the way.
    let refused = session.execute(
        "vector.path.gradient",
        &json!({
            "path": id,
            "kind": "linear",
            "stops": [
                { "offset": 0.0, "colour": "#ff0000" },
                { "offset": 4.0, "colour": "#0000ff" },
            ],
        }),
    );
    assert!(refused.is_err(), "a stop's offset is between zero and one");
}
