//! The **keyframes** of a property.
//!
//! The value of an animated property at a time is derived: it is the
//! interpolation of the keyframes around it.

use craftkit_sdk::serde_json::json;
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
fn value_at(session: &mut Session, layer: u64, path: &str, frame: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("fx.prop.at", &json!({ "layer": layer, "path": path, "frame": frame }))
        .expect("the value is read");
    outcome.data["value"].clone()
}

/// A keyframe is **set** on a property.
#[test]
fn a_keyframe_is_set() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 0, "value": 10 }))
        .expect("the keyframe is set");
    assert_eq!(value_at(&mut session, layer, "position", 0), json!(10), "the property is at the keyframe");
}

/// A keyframe that is **there** is changed.
#[test]
fn a_keyframe_that_is_there_is_changed() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 0, "value": 10 }))
        .expect("the keyframe is set");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 0, "value": 20 }))
        .expect("the keyframe is set again");
    assert_eq!(value_at(&mut session, layer, "position", 0), json!(20), "the keyframe has the new value");
}

/// The keyframes stay **sorted by frame**, and the value in between is
/// the line between them.
#[test]
fn the_value_in_between_is_the_line() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 24, "value": 100 }))
        .expect("the keyframe is set");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 0, "value": 0 }))
        .expect("the keyframe is set");
    assert_eq!(value_at(&mut session, layer, "position", 0), json!(0), "the first keyframe");
    assert_eq!(value_at(&mut session, layer, "position", 24), json!(100), "the last keyframe");
    assert_eq!(value_at(&mut session, layer, "position", 12), json!(50.0), "the line between them");
}

/// Before the first keyframe the value is the first keyframe's, and
/// after the last one it is the last keyframe's.
#[test]
fn the_value_outside_the_keyframes_is_held() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 10, "value": 5 }))
        .expect("the keyframe is set");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 20, "value": 15 }))
        .expect("the keyframe is set");
    assert_eq!(value_at(&mut session, layer, "position", 0), json!(5), "before the first keyframe");
    assert_eq!(value_at(&mut session, layer, "position", 40), json!(15), "after the last keyframe");
}

/// A list of numbers is interpolated, one number at a time.
#[test]
fn a_list_of_numbers_is_interpolated() {
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
            &json!({ "layer": layer, "path": "position", "frame": 10, "value": [100, 200] }),
        )
        .expect("the keyframe is set");
    assert_eq!(value_at(&mut session, layer, "position", 5), json!([50.0, 150.0]), "the line between them");
}

/// A value that does not interpolate is **held**.
#[test]
fn a_value_that_does_not_interpolate_is_held() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "position", "frame": 0, "value": "red" }),
        )
        .expect("the keyframe is set");
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "position", "frame": 10, "value": "blue" }),
        )
        .expect("the keyframe is set");
    assert_eq!(value_at(&mut session, layer, "position", 5), json!("red"), "the earlier value is held");
}

/// A property that is **not animated** has the same value at every
/// frame.
#[test]
fn a_property_that_is_not_animated_has_one_value() {
    let mut session = session();
    let layer = a_layer(&mut session);
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "opacity", "value": 100 }))
        .expect("the property is set");
    assert_eq!(value_at(&mut session, layer, "opacity", 0), json!(100), "at the first frame");
    assert_eq!(value_at(&mut session, layer, "opacity", 50), json!(100), "and at the last one");
}

/// A keyframe is **taken away**.
#[test]
fn a_keyframe_is_taken_away() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 0, "value": 0 }))
        .expect("the keyframe is set");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 10, "value": 10 }))
        .expect("the keyframe is set");
    session
        .execute("fx.keyframe.remove", &json!({ "layer": layer, "path": "position", "frame": 0 }))
        .expect("the keyframe is taken away");
    assert_eq!(value_at(&mut session, layer, "position", 0), json!(10), "the first keyframe is gone");
}

/// A frame that has **no keyframe** is refused: a keyframe that was
/// never there is not a keyframe that was taken away.
#[test]
fn a_frame_that_has_no_keyframe_is_refused() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    let error = session
        .execute("fx.keyframe.remove", &json!({ "layer": layer, "path": "position", "frame": 7 }))
        .expect_err("the removal is refused");
    assert!(error.to_string().contains("no keyframe at frame 7"), "the error says why: {error}");
}

/// A property that is **not animated** cannot have a keyframe: it is
/// refused, and it says so.
#[test]
fn a_property_that_is_not_animated_is_refused() {
    let mut session = session();
    let layer = a_layer(&mut session);
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "opacity", "value": 100 }))
        .expect("the property is set");
    let error = session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "opacity", "frame": 0, "value": 50 }))
        .expect_err("the keyframe is refused");
    assert!(error.to_string().contains("is not animated"), "the error says why: {error}");
}

/// A path that names **nothing** is refused.
#[test]
fn a_path_that_names_nothing_is_refused() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    let error = session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "nothing", "frame": 0, "value": 1 }))
        .expect_err("the keyframe is refused");
    assert!(error.to_string().contains("there is no `nothing` here"), "the error says why: {error}");
}

/// A frame that is **not a whole number** is refused.
#[test]
fn a_frame_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    let error = session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 1.5, "value": 1 }))
        .expect_err("the keyframe is refused");
    assert!(error.to_string().contains("a whole number"), "the error says why: {error}");
}

/// A node that is **not a layer** is refused.
#[test]
fn a_node_that_is_not_a_layer_is_refused() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    let error = session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer + 1_000_000, "path": "position", "frame": 0, "value": 1 }),
        )
        .expect_err("the keyframe is refused");
    assert!(error.to_string().contains("there is no layer"), "the error says why: {error}");
}
