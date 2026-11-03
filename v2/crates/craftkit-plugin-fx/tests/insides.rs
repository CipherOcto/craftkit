//! The **in side** of a keyframe.
//!
//! The tree's keyframe has an interpolation on **each** side: the side
//! it leaves on and the side it is entered on. The segment between two
//! keyframes is shaped by the side the first leaves on and by the side
//! the second is entered on.

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

/// A **property** that is animated, and its layer.
fn an_animated_layer(session: &mut Session) -> u64 {
    let layer = a_layer(session);
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "position", "value": { "keys": [] } }))
        .expect("the property is animated");
    layer
}

/// The **value** of a property at a frame.
fn value_at(session: &mut Session, layer: u64, path: &str, frame: u64) -> Value {
    let outcome = session
        .execute("fx.prop.at", &json!({ "layer": layer, "path": path, "frame": frame }))
        .expect("the value is read");
    outcome.data["value"].clone()
}

/// The **number** of a value.
fn number(value: &Value) -> f64 {
    value.as_f64().unwrap_or(f64::NAN)
}

/// A property with two keyframes: ten at frame zero and ten at frame
/// ten.
fn two_keys(session: &mut Session) -> u64 {
    let layer = an_animated_layer(session);
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 0, "value": 0 }))
        .expect("the keyframe is set");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 10, "value": 10 }))
        .expect("the keyframe is set");
    layer
}

/// The **in side** of a keyframe is set on its own.
#[test]
fn the_in_side_is_set_on_its_own() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 10, "interp": "bezier", "side": "in" }),
        )
        .expect("the interpolation is set");
    let node = session.document().node(layer).expect("the layer is there");
    let keys = node.data["props"]["position"]["keys"].as_array().expect("the keys are there");
    let second = &keys[1];
    assert_eq!(second["in_interp"].as_str(), Some("bezier"), "the in side is a bezier");
    assert_eq!(second["interp"].as_str(), Some("linear"), "the out side is not touched");
}

/// The side that is **not** named is the side the keyframe leaves on.
#[test]
fn the_side_that_is_not_named_is_the_out_side() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "bezier" }),
        )
        .expect("the interpolation is set");
    let node = session.document().node(layer).expect("the layer is there");
    let keys = node.data["props"]["position"]["keys"].as_array().expect("the keys are there");
    assert_eq!(keys[0]["interp"].as_str(), Some("bezier"), "the out side is the bezier");
}

/// The **in side** shapes the segment it ends.
#[test]
fn the_in_side_shapes_the_segment() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 10, "interp": "bezier", "side": "in" }),
        )
        .expect("the interpolation is set");
    let middle = number(&value_at(&mut session, layer, "position", 5));
    assert!(middle > 5.0, "the curve is above the line: {middle}");
}

/// A **hold** on the in side shapes the segment like a **linear** side.
#[test]
fn a_hold_on_the_in_side_is_a_linear_side() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 10, "interp": "hold", "side": "in" }),
        )
        .expect("the interpolation is set");
    let middle = number(&value_at(&mut session, layer, "position", 5));
    assert_eq!(middle, 5.0, "the segment is a line: {middle}");
}

/// The **out side** decides the shape of the segment: a hold out side
/// holds the value, whatever the in side says.
#[test]
fn the_out_side_decides_the_shape() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "hold" }),
        )
        .expect("the interpolation is set");
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 10, "interp": "bezier", "side": "in" }),
        )
        .expect("the interpolation is set");
    let middle = number(&value_at(&mut session, layer, "position", 5));
    assert_eq!(middle, 0.0, "the value is held: {middle}");
}

/// A **linear** out side with a **bezier** in side is a curve: the
/// segment is not the line.
#[test]
fn a_linear_out_side_with_a_bezier_in_side_is_a_curve() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 10, "interp": "bezier", "side": "in" }),
        )
        .expect("the interpolation is set");
    let middle = number(&value_at(&mut session, layer, "position", 5));
    assert!(middle > 5.0, "the curve is above the line: {middle}");
}

/// The **ease** of the in side is kept with it.
#[test]
fn the_ease_of_the_in_side_is_kept() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({
                "layer": layer,
                "path": "position",
                "frame": 10,
                "interp": "bezier",
                "side": "in",
                "speed": 2.0,
                "influence": 0.5,
            }),
        )
        .expect("the interpolation is set");
    let node = session.document().node(layer).expect("the layer is there");
    let keys = node.data["props"]["position"]["keys"].as_array().expect("the keys are there");
    assert_eq!(keys[1]["in_ease"]["speed"].as_f64(), Some(2.0), "the speed is kept");
    assert_eq!(keys[1]["in_ease"]["influence"].as_f64(), Some(0.5), "the influence is kept");
}

/// A side that is **not** a side is refused.
#[test]
fn a_side_that_is_not_a_side_is_refused() {
    let mut session = session();
    let layer = two_keys(&mut session);
    let outcome = session.execute(
        "fx.keyframe.interp",
        &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "linear", "side": "sideways" }),
    );
    assert!(outcome.is_err(), "the side that is not a side is refused");
}

/// The outcome says which **side** was set.
#[test]
fn the_outcome_says_the_side() {
    let mut session = session();
    let layer = two_keys(&mut session);
    let outcome = session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 10, "interp": "bezier", "side": "in" }),
        )
        .expect("the interpolation is set");
    assert_eq!(outcome.data["side"].as_str(), Some("in"), "the side is named");
}
