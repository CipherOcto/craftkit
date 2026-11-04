//! The **measure** of a drawing: how long, how big, how wide, and how
//! far apart.

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

/// A **line** in a block, from one point to another.
fn a_line(session: &mut Session, block: u64) -> u64 {
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 3.0, "y": 4.0 },
            }),
        )
        .expect("the line is added");
    outcome.data["id"].as_u64().expect("the line has an id")
}

/// A **circle** in a block.
fn a_circle(session: &mut Session, block: u64) -> u64 {
    let outcome = session
        .execute(
            "cad.entity.circle.add",
            &json!({
                "block": block,
                "name": "the circle",
                "center": { "x": 0.0, "y": 0.0 },
                "radius": 2.0,
            }),
        )
        .expect("the circle is added");
    outcome.data["id"].as_u64().expect("the circle has an id")
}

/// The **length** of an entity.
fn length(session: &mut Session, entity: u64) -> f64 {
    let outcome =
        session.execute("cad.measure.length", &json!({ "entity": entity })).expect("the length is measured");
    outcome.data["length"].as_f64().expect("the length is a number")
}

/// The **area** an entity encloses.
fn area(session: &mut Session, entity: u64) -> f64 {
    let outcome =
        session.execute("cad.measure.area", &json!({ "entity": entity })).expect("the area is measured");
    outcome.data["area"].as_f64().expect("the area is a number")
}

/// The **radius** of an entity.
fn radius(session: &mut Session, entity: u64) -> f64 {
    let outcome =
        session.execute("cad.measure.radius", &json!({ "entity": entity })).expect("the radius is measured");
    outcome.data["radius"].as_f64().expect("the radius is a number")
}

/// The **length** of a line is the distance between its ends.
#[test]
fn the_length_of_a_line_is_the_distance_between_its_ends() {
    let mut session = session();
    let block = a_block(&mut session);
    let line = a_line(&mut session, block);
    let measured = length(&mut session, line);
    assert!((measured - 5.0).abs() < 1e-9, "the line is five long, and it says {}", measured);
}

/// The **length** of a circle is its circumference.
#[test]
fn the_length_of_a_circle_is_its_circumference() {
    let mut session = session();
    let block = a_block(&mut session);
    let circle = a_circle(&mut session, block);
    let measured = length(&mut session, circle);
    let expected = std::f64::consts::TAU * 2.0;
    assert!(
        (measured - expected).abs() < 1e-9,
        "the circumference is {}, and it says {}",
        expected,
        measured
    );
}

/// The **length** of a polyline is the sum of its segments, and a
/// closed one is the sum with the closing segment too.
#[test]
fn the_length_of_a_polyline_is_the_sum_of_its_segments() {
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
                    { "x": 3.0, "y": 0.0 },
                    { "x": 3.0, "y": 4.0 },
                ],
            }),
        )
        .expect("the polyline is added");
    let open = outcome.data["id"].as_u64().expect("the polyline has an id");
    let measured = length(&mut session, open);
    assert!((measured - 7.0).abs() < 1e-9, "the polyline is seven long, and it says {}", measured);

    let outcome = session
        .execute(
            "cad.entity.polyline.add",
            &json!({
                "block": block,
                "name": "the closed polyline",
                "points": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 3.0, "y": 0.0 },
                    { "x": 3.0, "y": 4.0 },
                ],
                "closed": true,
            }),
        )
        .expect("the closed polyline is added");
    let closed = outcome.data["id"].as_u64().expect("the polyline has an id");
    let measured = length(&mut session, closed);
    assert!((measured - 12.0).abs() < 1e-9, "the closed polyline is twelve long, and it says {}", measured);
}

/// The **area** of a circle is pi times the radius squared.
#[test]
fn the_area_of_a_circle_is_pi_times_the_radius_squared() {
    let mut session = session();
    let block = a_block(&mut session);
    let circle = a_circle(&mut session, block);
    let measured = area(&mut session, circle);
    let expected = std::f64::consts::PI * 4.0;
    assert!((measured - expected).abs() < 1e-9, "the area is {}, and it says {}", expected, measured);
}

/// The **area** of a closed polyline is the area it encloses.
#[test]
fn the_area_of_a_closed_polyline_is_the_area_it_encloses() {
    let mut session = session();
    let block = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.polyline.add",
            &json!({
                "block": block,
                "name": "the square",
                "points": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 4.0, "y": 0.0 },
                    { "x": 4.0, "y": 4.0 },
                    { "x": 0.0, "y": 4.0 },
                ],
                "closed": true,
            }),
        )
        .expect("the square is added");
    let square = outcome.data["id"].as_u64().expect("the polyline has an id");
    let measured = area(&mut session, square);
    assert!((measured - 16.0).abs() < 1e-9, "the square is sixteen big, and it says {}", measured);
}

/// The **area** of an open polyline is **refused**: it encloses none,
/// and a zero would be a number that means nothing.
#[test]
fn the_area_of_an_open_polyline_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let outcome = session
        .execute(
            "cad.entity.polyline.add",
            &json!({
                "block": block,
                "name": "the open polyline",
                "points": [
                    { "x": 0.0, "y": 0.0 },
                    { "x": 4.0, "y": 0.0 },
                ],
            }),
        )
        .expect("the polyline is added");
    let open = outcome.data["id"].as_u64().expect("the polyline has an id");
    let value = session
        .execute("cad.measure.area", &json!({ "entity": open }))
        .expect_err("the area of an open polyline is refused");
    assert!(value.to_string().contains("not supported yet"), "the refusal says why, and it says {}", value);
}

/// The **radius** of a circle is its radius, and its **diameter** is
/// twice it.
#[test]
fn the_radius_of_a_circle_is_its_radius() {
    let mut session = session();
    let block = a_block(&mut session);
    let circle = a_circle(&mut session, block);
    let measured = radius(&mut session, circle);
    assert!((measured - 2.0).abs() < 1e-9, "the radius is two, and it says {}", measured);
    let outcome =
        session.execute("cad.measure.radius", &json!({ "entity": circle })).expect("the radius is measured");
    let diameter = outcome.data["diameter"].as_f64().expect("the diameter is a number");
    assert!((diameter - 4.0).abs() < 1e-9, "the diameter is four, and it says {}", diameter);
}

/// The **radius** of a line is **refused**: a line has none.
#[test]
fn the_radius_of_a_line_is_refused() {
    let mut session = session();
    let block = a_block(&mut session);
    let line = a_line(&mut session, block);
    let value = session
        .execute("cad.measure.radius", &json!({ "entity": line }))
        .expect_err("the radius of a line is refused");
    assert!(value.to_string().contains("not supported yet"), "the refusal says why, and it says {}", value);
}

/// The **angle** at a point is the angle between two directions, and it
/// is never more than a straight one.
#[test]
fn the_angle_at_a_point_is_the_angle_between_two_directions() {
    let mut session = session();
    let outcome = session
        .execute(
            "cad.measure.angle",
            &json!({
                "vertex": { "x": 0.0, "y": 0.0 },
                "first": { "x": 1.0, "y": 0.0 },
                "second": { "x": 0.0, "y": 1.0 },
            }),
        )
        .expect("the angle is measured");
    let measured = outcome.data["angle"].as_f64().expect("the angle is a number");
    assert!((measured - 90.0).abs() < 1e-9, "the angle is ninety, and it says {}", measured);

    let outcome = session
        .execute(
            "cad.measure.angle",
            &json!({
                "vertex": { "x": 0.0, "y": 0.0 },
                "first": { "x": 1.0, "y": 0.0 },
                "second": { "x": -1.0, "y": 0.0 },
            }),
        )
        .expect("the angle is measured");
    let measured = outcome.data["angle"].as_f64().expect("the angle is a number");
    assert!((measured - 180.0).abs() < 1e-9, "the angle is a straight one, and it says {}", measured);
}

/// An **angle** with a direction of no length is **refused**.
#[test]
fn an_angle_with_a_direction_of_no_length_is_refused() {
    let mut session = session();
    let value = session
        .execute(
            "cad.measure.angle",
            &json!({
                "vertex": { "x": 0.0, "y": 0.0 },
                "first": { "x": 0.0, "y": 0.0 },
                "second": { "x": 0.0, "y": 1.0 },
            }),
        )
        .expect_err("the angle is refused");
    assert!(value.to_string().contains("direction"), "the refusal says why, and it says {}", value);
}
