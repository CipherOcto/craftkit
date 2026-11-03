//! The **spatial** property: a value that is a point follows a **path**.
//!
//! The tree's spatial property does not move along the straight line
//! from one keyframe to the next — it follows a path in the plane, and
//! the time maps to the **length** along that path. The path is a cubic
//! with the **auto** tangents: a sixth of the way from the keyframe
//! before to the keyframe after.

use craftkit_sdk::serde_json::{Value, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_fx::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the fx battery.
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

/// A **composition** with a **layer** in it, and the layer's id.
fn a_layer(session: &mut Session) -> u64 {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    outcome.data["id"].as_u64().expect("the layer has an id")
}

/// A property on a layer, and the layer's id.
fn a_property(session: &mut Session, path: &str) -> u64 {
    let layer = a_layer(session);
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": path, "value": { "keys": [] } }))
        .expect("the property is animated");
    layer
}

/// A property with the given keyframes, and the layer's id.
fn keys_at(session: &mut Session, path: &str, points: &[(u64, [f64; 2])]) -> u64 {
    let layer = a_property(session, path);
    for (frame, point) in points {
        session
            .execute(
                "fx.keyframe.set",
                &json!({ "layer": layer, "path": path, "frame": frame, "value": [point[0], point[1]] }),
            )
            .expect("the keyframe is set");
    }
    layer
}

/// The **value** of a property at a frame.
fn value_at(session: &mut Session, layer: u64, path: &str, frame: u64) -> Value {
    let outcome = session
        .execute("fx.prop.at", &json!({ "layer": layer, "path": path, "frame": frame }))
        .expect("the value is read");
    outcome.data["value"].clone()
}

/// The **point** of a value: its two components.
fn point(value: &Value) -> [f64; 2] {
    let list = value.as_array().expect("the value is a list");
    [list[0].as_f64().unwrap_or(f64::NAN), list[1].as_f64().unwrap_or(f64::NAN)]
}

/// The **distance** between two points.
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

/// Make the property **spatial**.
fn make_spatial(session: &mut Session, layer: u64, path: &str) {
    session
        .execute("fx.prop.spatial", &json!({ "layer": layer, "path": path, "spatial": true }))
        .expect("the property is spatial");
}

/// A property with two keyframes and a **straight** path between them.
fn a_straight_path(session: &mut Session) -> u64 {
    let layer = keys_at(session, "position", &[(0, [0.0, 0.0]), (10, [100.0, 0.0])]);
    make_spatial(session, layer, "position");
    layer
}

/// A property with three keyframes: the path **turns** at the middle.
fn a_turning_path(session: &mut Session) -> u64 {
    let layer = keys_at(session, "position", &[(0, [0.0, 0.0]), (10, [100.0, 0.0]), (20, [100.0, 100.0])]);
    make_spatial(session, layer, "position");
    layer
}

/// A straight path is the **chord** itself: the value at the middle
/// frame is the middle of the two points.
#[test]
fn a_straight_path_is_the_chord() {
    let mut session = session();
    let layer = a_straight_path(&mut session);
    let middle = point(&value_at(&mut session, layer, "position", 5));
    assert!((middle[0] - 50.0).abs() < 1e-6, "the point is halfway along: {middle:?}");
    assert!(middle[1].abs() < 1e-6, "the point is on the line: {middle:?}");
}

/// The path of two keyframes is the line: the **ends** are the
/// keyframes' own values.
#[test]
fn the_ends_of_the_path_are_the_keyframes() {
    let mut session = session();
    let layer = a_straight_path(&mut session);
    let first = point(&value_at(&mut session, layer, "position", 0));
    let last = point(&value_at(&mut session, layer, "position", 10));
    assert!((first[0] - 0.0).abs() < 1e-9 && first[1].abs() < 1e-9, "the first point: {first:?}");
    assert!((last[0] - 100.0).abs() < 1e-9 && last[1].abs() < 1e-9, "the last point: {last:?}");
}

/// A path that **turns** does not go along the straight line: the
/// value at the middle frame is off the chord.
#[test]
fn a_turning_path_leaves_the_chord() {
    let mut session = session();
    let layer = a_turning_path(&mut session);
    let middle = point(&value_at(&mut session, layer, "position", 5));
    // The chord from the origin to the first corner is the x axis, so
    // a point off it has a y of its own.
    assert!(middle[1].abs() > 1e-6, "the path leaves the chord: {middle:?}");
}

/// The time maps to the **length** along the path: the point is
/// **farther** from the start than the straight line's middle, because
/// the path is longer than the chord.
#[test]
fn the_time_maps_to_the_length_of_the_path() {
    let mut session = session();
    let layer = a_turning_path(&mut session);
    let middle = point(&value_at(&mut session, layer, "position", 5));
    let chord = distance([0.0, 0.0], [50.0, 0.0]);
    let along = distance([0.0, 0.0], middle);
    assert!(along > chord, "the path is longer than the chord: {along} and {chord}");
}

/// The **same** keyframes without the spatial flag move along the
/// straight line: the flag is what makes the path.
#[test]
fn the_spatial_flag_is_what_makes_the_path() {
    let mut session = session();
    let layer =
        keys_at(&mut session, "position", &[(0, [0.0, 0.0]), (10, [100.0, 0.0]), (20, [100.0, 100.0])]);
    let straight = point(&value_at(&mut session, layer, "position", 5));
    assert!(straight[1].abs() < 1e-6, "the straight line stays on the x axis: {straight:?}");
    make_spatial(&mut session, layer, "position");
    let curved = point(&value_at(&mut session, layer, "position", 5));
    assert!(curved[1].abs() > 1e-6, "the path leaves the chord: {curved:?}");
}

/// The **flag** is kept with the property.
#[test]
fn the_spatial_flag_is_kept() {
    let mut session = session();
    let layer = a_straight_path(&mut session);
    let node = session.document().node(layer).expect("the layer is there");
    assert_eq!(node.data["props"]["position"]["spatial"].as_bool(), Some(true), "the property is spatial");
}

/// The flag is **cleared** again.
#[test]
fn the_spatial_flag_is_cleared() {
    let mut session = session();
    let layer = a_straight_path(&mut session);
    session
        .execute("fx.prop.spatial", &json!({ "layer": layer, "path": "position", "spatial": false }))
        .expect("the property is not spatial");
    let node = session.document().node(layer).expect("the layer is there");
    assert_eq!(
        node.data["props"]["position"]["spatial"].as_bool(),
        Some(false),
        "the property is not spatial"
    );
}

/// A property that is **not animated** has no path, and is refused.
#[test]
fn a_property_that_is_not_animated_is_refused() {
    let mut session = session();
    let layer = a_layer(&mut session);
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "position", "value": 3.0 }))
        .expect("the property is set");
    let outcome =
        session.execute("fx.prop.spatial", &json!({ "layer": layer, "path": "position", "spatial": true }));
    assert!(outcome.is_err(), "a property without keyframes has no path");
}

/// A property whose value is **not a point** has no path, and is
/// refused.
#[test]
fn a_property_that_is_not_a_point_is_refused() {
    let mut session = session();
    let layer = a_property(&mut session, "opacity");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "opacity", "frame": 0, "value": 0.5 }))
        .expect("the keyframe is set");
    let outcome =
        session.execute("fx.prop.spatial", &json!({ "layer": layer, "path": "opacity", "spatial": true }));
    assert!(outcome.is_err(), "a number is not a point");
}

/// A property whose keyframes are **not the same width** is not a
/// path, and is refused.
#[test]
fn a_property_of_two_widths_is_refused() {
    let mut session = session();
    let layer = a_property(&mut session, "position");
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "position", "frame": 0, "value": [0.0, 0.0] }),
        )
        .expect("the keyframe is set");
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "position", "frame": 10, "value": [1.0, 1.0, 1.0] }),
        )
        .expect("the keyframe is set");
    let outcome =
        session.execute("fx.prop.spatial", &json!({ "layer": layer, "path": "position", "spatial": true }));
    assert!(outcome.is_err(), "a path has one width");
}

/// A property that is **not there** is refused.
#[test]
fn a_property_that_is_not_there_is_refused() {
    let mut session = session();
    let layer = a_layer(&mut session);
    let outcome =
        session.execute("fx.prop.spatial", &json!({ "layer": layer, "path": "position", "spatial": true }));
    assert!(outcome.is_err(), "there is no such property");
}

/// A **triple** is a point too, and the path keeps the third
/// component.
#[test]
fn a_triple_is_a_point() {
    let mut session = session();
    let layer = a_property(&mut session, "position");
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "position", "frame": 0, "value": [0.0, 0.0, 0.0] }),
        )
        .expect("the keyframe is set");
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "position", "frame": 10, "value": [10.0, 0.0, 5.0] }),
        )
        .expect("the keyframe is set");
    make_spatial(&mut session, layer, "position");
    let middle = value_at(&mut session, layer, "position", 5);
    let list = middle.as_array().expect("the value is a list");
    assert_eq!(list.len(), 3, "the point keeps its three components");
    assert!((list[2].as_f64().unwrap_or(f64::NAN) - 2.5).abs() < 1e-6, "the third component: {list:?}");
}
