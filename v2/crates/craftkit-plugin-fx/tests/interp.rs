//! The **interpolation** of a keyframe.
//!
//! The tree's keyframes leave on a `linear`, a `bezier` or a `hold`,
//! and a bezier has an ease: a speed and an influence.

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

/// A **linear** keyframe moves the value in a straight line.
#[test]
fn a_linear_keyframe_moves_the_value_in_a_line() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "linear" }),
        )
        .expect("the interpolation is set");
    assert_eq!(number(&value_at(&mut session, layer, "position", 5)), 5.0, "the middle is the middle");
}

/// A **hold** keyframe does not move the value at all.
#[test]
fn a_hold_keyframe_does_not_move_the_value() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "hold" }),
        )
        .expect("the interpolation is set");
    assert_eq!(number(&value_at(&mut session, layer, "position", 5)), 0.0, "the value holds");
    assert_eq!(number(&value_at(&mut session, layer, "position", 9)), 0.0, "and it holds");
}

/// A **bezier** is not a line: the tree's default ease has no speed,
/// and so the value moves slowly at the start.
#[test]
fn a_bezier_is_not_a_line() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "bezier" }),
        )
        .expect("the interpolation is set");
    let middle = number(&value_at(&mut session, layer, "position", 5));
    assert!((middle - 4.450419).abs() < 1e-5, "the middle is {middle} and not five");
}

/// A bezier whose speed is the speed of the **straight line** is the
/// straight line.
#[test]
fn a_bezier_at_the_speed_of_the_line_is_the_line() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({
                "layer": layer,
                "path": "position",
                "frame": 0,
                "interp": "bezier",
                "speed": 1.0,
            }),
        )
        .expect("the interpolation is set");
    let middle = number(&value_at(&mut session, layer, "position", 5));
    assert!((middle - 5.0).abs() < 1e-9, "the middle is {middle} and is five");
}

/// The **influence** is the fraction of the segment the handle takes,
/// and a big influence is a slow start: the handle reaches across the
/// whole segment and the value waits for it.
#[test]
fn an_influence_is_a_fraction_of_the_segment() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({
                "layer": layer,
                "path": "position",
                "frame": 0,
                "interp": "bezier",
                "influence": 1.0,
            }),
        )
        .expect("the interpolation is set");
    let middle = number(&value_at(&mut session, layer, "position", 5));
    assert!((middle - 0.922332).abs() < 1e-5, "the middle is {middle} and is well behind");
}

/// A **list** of numbers is eased number by number.
#[test]
fn a_list_of_numbers_is_eased_number_by_number() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "position", "frame": 0, "value": [0, 100] }),
        )
        .expect("the keyframe is set");
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "position", "frame": 10, "value": [10, 200] }),
        )
        .expect("the keyframe is set");
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "bezier" }),
        )
        .expect("the interpolation is set");
    let value = value_at(&mut session, layer, "position", 5);
    let list = value.as_array().expect("the value is a list");
    let first = list[0].as_f64().unwrap_or(f64::NAN);
    let second = list[1].as_f64().unwrap_or(f64::NAN);
    assert!((first - 4.450419).abs() < 1e-5, "the first is {first}");
    assert!((second - 144.50419).abs() < 1e-4, "and the second is {second}");
}

/// The **value** of a keyframe may be changed and its interpolation is
/// kept, as the tree's `set_key` keeps it.
#[test]
fn the_interpolation_is_kept_when_the_value_is_changed() {
    let mut session = session();
    let layer = two_keys(&mut session);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "hold" }),
        )
        .expect("the interpolation is set");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 0, "value": 5 }))
        .expect("the keyframe is set");
    assert_eq!(number(&value_at(&mut session, layer, "position", 5)), 5.0, "the value holds still");
}

/// An interpolation that is **not one of the three** is refused.
#[test]
fn an_interpolation_that_is_not_one_of_the_three_is_refused() {
    let mut session = session();
    let layer = two_keys(&mut session);
    let error = session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "spring" }),
        )
        .expect_err("the interpolation is refused");
    let message = error.to_string();
    assert!(message.contains("is not an interpolation"), "the refusal says why: {message}");
    assert!(message.contains("hold"), "and it names the three: {message}");
}

/// An influence that is **not a fraction** is refused.
#[test]
fn an_influence_that_is_not_a_fraction_is_refused() {
    let mut session = session();
    let layer = two_keys(&mut session);
    let error = session
        .execute(
            "fx.keyframe.interp",
            &json!({
                "layer": layer,
                "path": "position",
                "frame": 0,
                "interp": "bezier",
                "influence": 2.0,
            }),
        )
        .expect_err("the influence is refused");
    assert!(error.to_string().contains("fraction of the segment"), "the refusal says why: {error}");
}

/// A keyframe that is **not there** is refused.
#[test]
fn a_keyframe_that_is_not_there_is_refused() {
    let mut session = session();
    let layer = two_keys(&mut session);
    let error = session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 5, "interp": "hold" }),
        )
        .expect_err("the interpolation is refused");
    assert!(error.to_string().contains("has no keyframe at frame 5"), "the refusal says why: {error}");
}
