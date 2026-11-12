//! The **continuous** keyframe.
//!
//! The tree's keyframe has a mode in which its two sides are locked to
//! each other: the outgoing side follows the incoming one, and the
//! keyframe is marked continuous.

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
fn value_at(session: &mut Session, layer: u64, frame: u64) -> f64 {
    let outcome = session
        .execute("fx.prop.at", &json!({ "layer": layer, "path": "position", "frame": frame }))
        .expect("the value is read");
    let value: &Value = &outcome.data["value"];
    value.as_f64().unwrap_or(f64::NAN)
}

/// A property with three keyframes: zero at frame zero, ten at frame
/// ten and twenty at frame twenty.
fn three_keys(session: &mut Session) -> u64 {
    let layer = an_animated_layer(session);
    for (frame, value) in [(0, 0), (10, 10), (20, 20)] {
        session
            .execute(
                "fx.keyframe.set",
                &json!({ "layer": layer, "path": "position", "frame": frame, "value": value }),
            )
            .expect("the keyframe is set");
    }
    layer
}

/// The **keyframe** at a frame, as the payload holds it.
fn key_at(session: &Session, layer: u64, frame: u64) -> Value {
    let node = session.document().node(layer).expect("the layer is there");
    let keys = node.data["props"]["position"]["keys"].as_array().expect("the keys are there");
    keys.iter().find(|key| key["frame"].as_u64() == Some(frame)).cloned().expect("the keyframe is there")
}

/// The two sides of a keyframe are **locked**: they are the same ease.
#[test]
fn the_two_sides_are_locked() {
    let mut session = session();
    let layer = three_keys(&mut session);
    session
        .execute(
            "fx.keyframe.continuous",
            &json!({ "layer": layer, "path": "position", "frame": 10, "speed": 1.0, "influence": 0.5 }),
        )
        .expect("the keyframe is continuous");
    let key = key_at(&session, layer, 10);
    assert_eq!(key["ease"]["speed"].as_f64(), Some(1.0), "the out side has the ease");
    assert_eq!(key["in_ease"]["speed"].as_f64(), Some(1.0), "and the in side has it too");
    assert_eq!(key["ease"]["influence"].as_f64(), Some(0.5), "the influence is on both");
    assert_eq!(key["in_ease"]["influence"].as_f64(), Some(0.5), "the influence is on both");
}

/// A continuous keyframe's sides are **beziers**.
#[test]
fn the_sides_are_beziers() {
    let mut session = session();
    let layer = three_keys(&mut session);
    session
        .execute("fx.keyframe.continuous", &json!({ "layer": layer, "path": "position", "frame": 10 }))
        .expect("the keyframe is continuous");
    let key = key_at(&session, layer, 10);
    assert_eq!(key["interp"].as_str(), Some("bezier"), "the out side is a bezier");
    assert_eq!(key["in_interp"].as_str(), Some("bezier"), "and the in side is");
}

/// The **continuous** flag is kept with the keyframe.
#[test]
fn the_continuous_flag_is_kept() {
    let mut session = session();
    let layer = three_keys(&mut session);
    session
        .execute("fx.keyframe.continuous", &json!({ "layer": layer, "path": "position", "frame": 10 }))
        .expect("the keyframe is continuous");
    let key = key_at(&session, layer, 10);
    assert_eq!(key["continuous"].as_bool(), Some(true), "the keyframe is marked");
}

/// The flag may be **cleared**, and the eases stay as they are.
#[test]
fn the_flag_may_be_cleared() {
    let mut session = session();
    let layer = three_keys(&mut session);
    session
        .execute("fx.keyframe.continuous", &json!({ "layer": layer, "path": "position", "frame": 10 }))
        .expect("the keyframe is continuous");
    session
        .execute(
            "fx.keyframe.continuous",
            &json!({ "layer": layer, "path": "position", "frame": 10, "continuous": false }),
        )
        .expect("the flag is cleared");
    let key = key_at(&session, layer, 10);
    assert_eq!(key["continuous"].as_bool(), Some(false), "the keyframe is not marked");
    assert_eq!(key["interp"].as_str(), Some("bezier"), "and the sides are still beziers");
}

/// A **manual** ease is not an auto one: the continuous flag clears the
/// auto flag, as the tree's does.
#[test]
fn the_auto_flag_is_cleared() {
    let mut session = session();
    let layer = three_keys(&mut session);
    session
        .execute(
            "fx.keyframe.auto",
            &json!({ "layer": layer, "path": "position", "frame": 10, "auto": true }),
        )
        .expect("the auto is set");
    session
        .execute("fx.keyframe.continuous", &json!({ "layer": layer, "path": "position", "frame": 10 }))
        .expect("the keyframe is continuous");
    let key = key_at(&session, layer, 10);
    assert_eq!(key["auto"].as_bool(), Some(false), "the keyframe is not an auto one");
}

/// The **curve** of a continuous keyframe is **smooth**: the value a
/// frame before the keyframe and the value a frame after it are the
/// same distance from the keyframe's value.
#[test]
fn the_curve_is_smooth_at_the_keyframe() {
    let mut session = session();
    let layer = three_keys(&mut session);
    session
        .execute(
            "fx.keyframe.continuous",
            &json!({ "layer": layer, "path": "position", "frame": 10, "speed": 1.0, "influence": 0.4 }),
        )
        .expect("the keyframe is continuous");
    let before = value_at(&mut session, layer, 9);
    let at = value_at(&mut session, layer, 10);
    let after = value_at(&mut session, layer, 11);
    assert!((before + after - 2.0 * at).abs() < 1e-9, "the curve is smooth: {before}, {at}, {after}");
}

/// The side that is **not** named takes the ease of the side that is:
/// the keyframe's own ease is the master when nothing is given.
#[test]
fn the_in_side_is_the_master() {
    let mut session = session();
    let layer = three_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({
                "layer": layer,
                "path": "position",
                "frame": 10,
                "interp": "bezier",
                "side": "in",
                "speed": 3.0,
                "influence": 0.25,
            }),
        )
        .expect("the interpolation is set");
    session
        .execute("fx.keyframe.continuous", &json!({ "layer": layer, "path": "position", "frame": 10 }))
        .expect("the keyframe is continuous");
    let key = key_at(&session, layer, 10);
    assert_eq!(key["ease"]["speed"].as_f64(), Some(3.0), "the out side took the in side's ease");
    assert_eq!(key["ease"]["influence"].as_f64(), Some(0.25), "and its influence");
}

/// A keyframe that is **not there** is refused.
#[test]
fn a_keyframe_that_is_not_there_is_refused() {
    let mut session = session();
    let layer = three_keys(&mut session);
    let outcome = session
        .execute("fx.keyframe.continuous", &json!({ "layer": layer, "path": "position", "frame": 99 }));
    assert!(outcome.is_err(), "the keyframe that is not there is refused");
}

/// An influence that is **not** a fraction is refused.
#[test]
fn an_influence_that_is_not_a_fraction_is_refused() {
    let mut session = session();
    let layer = three_keys(&mut session);
    let outcome = session.execute(
        "fx.keyframe.continuous",
        &json!({ "layer": layer, "path": "position", "frame": 10, "influence": 2.0 }),
    );
    assert!(outcome.is_err(), "the influence that is not a fraction is refused");
}
