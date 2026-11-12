//! The **outline** of a line: the shape a stroke covers.
//!
//! A line with a width covers an area, and that area is a shape in its
//! own right. The command makes it a path of its own, so that it may
//! be filled, measured and cut like any other.

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

/// A **rectangle** of ten by ten, with a line on it, and its id.
fn a_stroked_rectangle(session: &mut Session) -> u64 {
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    let id = session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .last()
        .map(|node| node.id)
        .expect("the rectangle is there");
    session
        .execute("vector.path.stroke", &json!({ "path": id, "colour": "#000000", "width": 2.0 }))
        .expect("the path has a line");
    id
}

/// The **area** a node covers, as the evaluator says it.
fn area_of(session: &Session, id: u64) -> f64 {
    let plan = session.evaluate();
    let mut area = 0.0;
    for item in &plan.items {
        if item.node == id
            && let Some(value) = item.value.get("area").and_then(|area| area.as_f64())
        {
            area = value;
        }
    }
    area
}

/// A line's **outline** is a path of its own, and it covers more than
/// the path it came from.
#[test]
fn the_outline_of_a_line_is_a_path() {
    let mut session = session();
    let id = a_stroked_rectangle(&mut session);
    session.execute("vector.path.outline", &json!({ "path": id })).expect("the line is outlined");
    let nodes: Vec<_> = session.document().walk().filter(|node| node.kind == "vector.path").collect();
    assert_eq!(nodes.len(), 2, "the document holds the path and its outline");
    let outline = nodes.last().expect("the outline is there");
    let path = outline.data["path"].clone();
    let subpaths = path["subpaths"].as_array().expect("the path has contours");
    assert!(!subpaths.is_empty(), "the outline has a contour");
    // The outline of a square of ten with a line of two is the **band**
    // around it: it has area of its own, and it reaches a unit beyond
    // the square on every side.
    assert!(area_of(&session, outline.id) > 0.0, "the outline has area of its own");
    let box_of = |id: u64| session.document().node(id).expect("the node is there").rect;
    let outline_box = box_of(outline.id).expect("the outline has a box");
    let square_box = box_of(id).expect("the square has a box");
    assert!(outline_box.w > square_box.w, "the outline reaches beyond the square");
    assert!(outline_box.h > square_box.h, "and it does so the other way too");
}

/// A path with **no line** has no outline, and the refusal says so.
#[test]
fn a_path_with_no_line_has_no_outline() {
    let mut session = session();
    session
        .execute("vector.rectangle", &json!({ "x": 0.0, "y": 0.0, "width": 10.0, "height": 10.0 }))
        .expect("the rectangle is added");
    let id = session
        .document()
        .walk()
        .filter(|node| node.kind == "vector.path")
        .last()
        .map(|node| node.id)
        .expect("the rectangle is there");
    let refused = session.execute("vector.path.outline", &json!({ "path": id }));
    let message = match refused {
        Ok(_) => panic!("a path with no line has no outline"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("no line"), "the refusal says why: {message}");
}
