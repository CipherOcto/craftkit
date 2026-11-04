//! The **arc** of a drawing: a piece of a circle.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Point, RecordingCanvas, Session};

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

/// An **arc** in a block: a quarter of a circle of the radius ten.
fn an_arc(session: &mut Session, block: u64) -> u64 {
    let outcome = session
        .execute(
            "cad.entity.arc.add",
            &json!({
                "block": block,
                "name": "the arc",
                "center": { "x": 0.0, "y": 0.0 },
                "radius": 10.0,
                "start": 0.0,
                "end": std::f64::consts::FRAC_PI_2,
            }),
        )
        .expect("the arc is added");
    outcome.data["id"].as_u64().expect("the arc has an id")
}

/// An arc may be put in a block.
#[test]
fn an_arc_may_be_put_in_a_block() {
    let mut session = session();
    let block = a_block(&mut session);
    let arc = an_arc(&mut session, block);
    let node = session.document().node(arc).expect("the arc is in the document");
    assert_eq!(node.kind, "cad.entity.arc", "the arc is an arc");
    let geometry = node.data.get("geometry").expect("the arc has a geometry");
    assert_eq!(geometry["radius"], json!(10.0), "the radius is ten");
}

/// The **box** of an arc is the box of the curve and not the box of its
/// two ends: the bulge is in the box.
#[test]
fn the_box_of_an_arc_includes_the_bulge() {
    let mut session = session();
    let block = a_block(&mut session);
    let arc = an_arc(&mut session, block);
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == arc).expect("the arc is in the plan");
    let max_y = item.value["extents"]["max"]["y"].as_f64().expect("the box has a top");
    assert!((max_y - 10.0).abs() < 1e-6, "the top of the box is ten, and it says {}", max_y);
}

/// An arc is **drawn**.
#[test]
fn an_arc_is_drawn() {
    let mut session = session();
    let block = a_block(&mut session);
    let _ = an_arc(&mut session, block);
    let mut canvas = RecordingCanvas::default();
    session.paint(&mut canvas);
    assert!(
        canvas.drawn.iter().any(|drawn| drawn.starts_with("path(")),
        "the arc is drawn, and what was drawn is {:?}",
        canvas.drawn
    );
}

/// A point on an arc is **on** it, and a point beside it is not.
#[test]
fn a_point_on_an_arc_is_on_it() {
    let mut session = session();
    let block = a_block(&mut session);
    let arc = an_arc(&mut session, block);
    let kind = session.kinds().get("cad.entity.arc").expect("the kind of an arc is there");
    let node = session.document().node(arc).expect("the arc is in the document");
    let on = Point::new(10.0, 0.0);
    assert!((kind.hit)(node, session.document(), on), "the end of the arc is on it");
    let beside = Point::new(12.0, 0.0);
    assert!(!(kind.hit)(node, session.document(), beside), "a point beside the arc is not on it");
}

/// The **length** of an arc is the radius times the sweep.
#[test]
fn the_length_of_an_arc_is_the_radius_times_the_sweep() {
    let mut session = session();
    let block = a_block(&mut session);
    let arc = an_arc(&mut session, block);
    let outcome =
        session.execute("cad.measure.length", &json!({ "entity": arc })).expect("the length is measured");
    let measured = outcome.data["length"].as_f64().expect("the length is a number");
    let expected = 10.0 * std::f64::consts::FRAC_PI_2;
    assert!((measured - expected).abs() < 1e-9, "the arc is {} long, and it says {}", expected, measured);
}

/// An arc with a **radius of nothing** is refused.
#[test]
fn an_arc_with_a_radius_of_nothing_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let value = session
        .execute(
            "cad.entity.arc.add",
            &json!({
                "block": block,
                "name": "the flat arc",
                "center": { "x": 0.0, "y": 0.0 },
                "radius": 0.0,
                "start": 0.0,
                "end": 1.0,
            }),
        )
        .expect_err("the arc is refused");
    assert!(value.to_string().contains("above nothing"), "the refusal says why, and it says {}", value);
}

/// An arc with **no sweep** is refused: it is a point and not an arc.
#[test]
fn an_arc_with_no_sweep_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let value = session
        .execute(
            "cad.entity.arc.add",
            &json!({
                "block": block,
                "name": "the arc of no sweep",
                "center": { "x": 0.0, "y": 0.0 },
                "radius": 10.0,
                "start": 1.0,
                "end": 1.0,
            }),
        )
        .expect_err("the arc is refused");
    assert!(value.to_string().contains("no sweep"), "the refusal says why, and it says {}", value);
}
