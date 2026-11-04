//! The **mirror** of an entity: the same entity, turned over a line.

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

/// A **drawing** with a **block** in it.
fn a_block(session: &mut Session) -> u64 {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the model" }))
        .expect("the block is added");
    outcome.data["id"].as_u64().expect("the block has an id")
}

/// The **geometry** of an entity.
fn geometry(session: &mut Session, entity: u64) -> Json {
    let node = session.document().node(entity).expect("the entity is in the document");
    node.data.get("geometry").cloned().expect("the entity has a geometry")
}

/// A line may be turned over the **vertical** line through the origin.
#[test]
fn a_line_may_be_turned_over_a_line() {
    let mut session = session();
    let block = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 1.0, "y": 2.0 },
                "to": { "x": 3.0, "y": 2.0 },
            }),
        )
        .expect("the line is added");
    let line = outcome.data["id"].as_u64().expect("the line has an id");
    session
        .execute(
            "cad.entity.mirror",
            &json!({
                "entity": line,
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 0.0, "y": 1.0 },
            }),
        )
        .expect("the line is turned over the line");
    let geometry = geometry(&mut session, line);
    assert_eq!(geometry["from"], json!({ "x": -1.0, "y": 2.0 }), "the start is on the other side");
    assert_eq!(geometry["to"], json!({ "x": -3.0, "y": 2.0 }), "and so is the end");
}

/// An ellipse turned over a line has its **major axis turned over**
/// with it: the axis is a vector, and a vector is not a point.
#[test]
fn an_ellipse_has_its_major_axis_turned_over() {
    let mut session = session();
    let block = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.ellipse.add",
            &json!({
                "block": block,
                "name": "the ellipse",
                "center": { "x": 0.0, "y": 0.0 },
                "major": { "x": 10.0, "y": 0.0 },
                "ratio": 0.5,
            }),
        )
        .expect("the ellipse is added");
    let ellipse = outcome.data["id"].as_u64().expect("the ellipse has an id");
    session
        .execute(
            "cad.entity.mirror",
            &json!({
                "entity": ellipse,
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 0.0, "y": 1.0 },
            }),
        )
        .expect("the ellipse is turned over the line");
    let geometry = geometry(&mut session, ellipse);
    assert_eq!(geometry["center"], json!({ "x": 0.0, "y": 0.0 }), "the centre is where it was");
    assert_eq!(geometry["major"], json!({ "x": -10.0, "y": 0.0 }), "the major axis is turned over");
    assert_eq!(geometry["ratio"], json!(0.5), "and the ratio is the ratio");
}

/// A circle turned over a line keeps its **radius**.
#[test]
fn a_circle_keeps_its_radius() {
    let mut session = session();
    let block = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.circle.add",
            &json!({
                "block": block,
                "name": "the circle",
                "center": { "x": 4.0, "y": 0.0 },
                "radius": 3.0,
            }),
        )
        .expect("the circle is added");
    let circle = outcome.data["id"].as_u64().expect("the circle has an id");
    session
        .execute(
            "cad.entity.mirror",
            &json!({
                "entity": circle,
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 0.0, "y": 1.0 },
            }),
        )
        .expect("the circle is turned over the line");
    let geometry = geometry(&mut session, circle);
    assert_eq!(geometry["center"], json!({ "x": -4.0, "y": 0.0 }), "the centre is on the other side");
    assert_eq!(geometry["radius"], json!(3.0), "and the radius is the radius");
}

/// A line of **no length** is refused: there is no line to turn over.
#[test]
fn a_line_of_no_length_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 1.0, "y": 0.0 },
            }),
        )
        .expect("the line is added");
    let line = outcome.data["id"].as_u64().expect("the line has an id");
    let value = session
        .execute(
            "cad.entity.mirror",
            &json!({
                "entity": line,
                "from": { "x": 1.0, "y": 1.0 },
                "to": { "x": 1.0, "y": 1.0 },
            }),
        )
        .expect_err("the mirror is refused");
    assert!(value.to_string().contains("same point"), "the refusal says why, and it says {}", value);
}

/// A node that is **not an entity** is refused.
#[test]
fn a_node_that_is_not_an_entity_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let value = session
        .execute(
            "cad.entity.mirror",
            &json!({
                "entity": block,
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 0.0, "y": 1.0 },
            }),
        )
        .expect_err("the mirror is refused");
    assert!(value.to_string().contains("not an entity"), "the refusal says why, and it says {}", value);
}
