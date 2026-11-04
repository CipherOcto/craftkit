//! The **curves** of a drawing: the ellipse and the polyline.

use craftkit_sdk::serde_json::json;
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

/// An **ellipse** may be put in a block.
#[test]
fn an_ellipse_may_be_put_in_a_block() {
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
    let entity = outcome.data["id"].as_u64().expect("the ellipse has an id");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == entity).expect("the ellipse is in the plan");
    // The box of the ellipse: the major axis is ten and the minor one
    // is five.
    assert_eq!(item.value["extents"]["max"]["x"], json!(10.0), "the box is ten wide");
    assert_eq!(item.value["extents"]["max"]["y"], json!(5.0), "and five tall");
}

/// An ellipse with a **bad ratio** is refused.
#[test]
fn an_ellipse_with_a_bad_ratio_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let error = session
        .execute(
            "cad.entity.ellipse.add",
            &json!({
                "block": block,
                "name": "the ellipse",
                "center": { "x": 0.0, "y": 0.0 },
                "major": { "x": 10.0, "y": 0.0 },
                "ratio": 0.0,
            }),
        )
        .expect_err("a ratio of nothing is refused");
    assert!(error.to_string().contains("ratio"), "and the error says why");
    let error = session
        .execute(
            "cad.entity.ellipse.add",
            &json!({
                "block": block,
                "name": "the ellipse",
                "center": { "x": 0.0, "y": 0.0 },
                "major": { "x": 10.0, "y": 0.0 },
                "ratio": 2.0,
            }),
        )
        .expect_err("a ratio above one is refused");
    assert!(error.to_string().contains("ratio"), "and the error says why too");
    let error = session
        .execute(
            "cad.entity.ellipse.add",
            &json!({
                "block": block,
                "name": "the ellipse",
                "center": { "x": 0.0, "y": 0.0 },
                "major": { "x": 0.0, "y": 0.0 },
                "ratio": 1.0,
            }),
        )
        .expect_err("a major axis of no length is refused");
    assert!(error.to_string().contains("major"), "and the error says which");
}

/// A **polyline** may be put in a block.
#[test]
fn a_polyline_may_be_put_in_a_block() {
    let mut session = session();
    let block = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.polyline.add",
            &json!({
                "block": block,
                "name": "the polyline",
                "points": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 10.0, "y": 0.0 },
                    { "x": 10.0, "y": 10.0 },
                ],
            }),
        )
        .expect("the polyline is added");
    let entity = outcome.data["id"].as_u64().expect("the polyline has an id");
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == entity).expect("the polyline is in the plan");
    assert_eq!(item.value["extents"]["max"]["x"], json!(10.0), "the box is ten wide");
    assert_eq!(item.value["extents"]["max"]["y"], json!(10.0), "and ten tall");
    assert_eq!(item.value["extents"]["min"]["x"], json!(0.0), "and it starts at nothing");
}

/// A polyline of **no points** is refused.
#[test]
fn a_polyline_of_no_points_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let error = session
        .execute("cad.entity.polyline.add", &json!({ "block": block, "name": "the polyline", "points": [] }))
        .expect_err("a polyline of no points is refused");
    assert!(error.to_string().contains("points"), "and the error says why");
    let error = session
        .execute(
            "cad.entity.polyline.add",
            &json!({
                "block": block,
                "name": "the polyline",
                "points": [{ "x": "a", "y": 0.0 }],
            }),
        )
        .expect_err("a point that is not a point is refused");
    assert!(error.to_string().contains("point"), "and the error says why too");
}
