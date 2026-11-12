//! The **auto bezier** of a keyframe.
//!
//! The tree's auto keyframe does not have an ease written down: its
//! ease is the **slope** to the keyframes on the other side of it, and
//! its influence is a sixth. So an auto keyframe follows its
//! neighbours.

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

/// A property with three keyframes: zero at frame zero, ten at frame
/// ten and thirty at frame twenty.
fn three_keys(session: &mut Session) -> u64 {
    let layer = an_animated_layer(session);
    for (frame, value) in [(0, 0), (10, 10), (20, 30)] {
        session
            .execute(
                "fx.keyframe.set",
                &json!({ "layer": layer, "path": "position", "frame": frame, "value": value }),
            )
            .expect("the keyframe is set");
    }
    layer
}

/// The middle keyframe leaves on a **bezier**.
fn a_bezier_middle(session: &mut Session, layer: u64) {
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 10, "interp": "bezier" }),
        )
        .expect("the interpolation is set");
}

/// The **first** keyframe leaves on a bezier.
fn a_bezier_first(session: &mut Session, layer: u64) {
    session
        .execute(
            "fx.keyframe.interp",
            &json!({ "layer": layer, "path": "position", "frame": 0, "interp": "bezier" }),
        )
        .expect("the interpolation is set");
}

/// The middle keyframe is an **auto** bezier.
fn an_auto_middle(session: &mut Session, layer: u64) {
    session
        .execute(
            "fx.keyframe.auto",
            &json!({ "layer": layer, "path": "position", "frame": 10, "auto": true }),
        )
        .expect("the auto is set");
}

/// An auto keyframe follows its **neighbours**: the curve is not the
/// curve of the ease that is written down.
#[test]
fn an_auto_keyframe_follows_its_neighbours() {
    let mut session = session();
    let layer = three_keys(&mut session);
    a_bezier_middle(&mut session, layer);
    let written = number(&value_at(&mut session, layer, "position", 15));
    an_auto_middle(&mut session, layer);
    let automatic = number(&value_at(&mut session, layer, "position", 15));
    assert!(automatic > written, "the auto curve is above the written one: {automatic} and {written}");
}

/// The **auto** flag is kept with the keyframe.
#[test]
fn the_auto_flag_is_kept() {
    let mut session = session();
    let layer = three_keys(&mut session);
    an_auto_middle(&mut session, layer);
    let node = session.document().node(layer).expect("the layer is there");
    let keys = node.data["props"]["position"]["keys"].as_array().expect("the keys are there");
    assert_eq!(keys[1]["auto"].as_bool(), Some(true), "the middle keyframe is an auto one");
}

/// The auto flag is **cleared** again.
#[test]
fn the_auto_flag_is_cleared() {
    let mut session = session();
    let layer = three_keys(&mut session);
    an_auto_middle(&mut session, layer);
    session
        .execute(
            "fx.keyframe.auto",
            &json!({ "layer": layer, "path": "position", "frame": 10, "auto": false }),
        )
        .expect("the auto is cleared");
    let node = session.document().node(layer).expect("the layer is there");
    let keys = node.data["props"]["position"]["keys"].as_array().expect("the keys are there");
    assert_eq!(keys[1]["auto"].as_bool(), Some(false), "the middle keyframe is not an auto one");
}

/// A **written** ease is a manual one, and it is not an auto one.
#[test]
fn a_written_ease_clears_the_auto_flag() {
    let mut session = session();
    let layer = three_keys(&mut session);
    an_auto_middle(&mut session, layer);
    session
        .execute(
            "fx.keyframe.interp",
            &json!({
                "layer": layer,
                "path": "position",
                "frame": 10,
                "interp": "bezier",
                "speed": 1.0,
                "influence": 0.5,
            }),
        )
        .expect("the interpolation is set");
    let node = session.document().node(layer).expect("the layer is there");
    let keys = node.data["props"]["position"]["keys"].as_array().expect("the keys are there");
    assert_eq!(keys[1]["auto"].as_bool(), Some(false), "the manual ease is not an auto one");
}

/// A keyframe with **no neighbour** on one side is not an auto one: the
/// ease that is written down is used instead.
#[test]
fn a_keyframe_with_no_neighbour_is_not_an_auto_one() {
    let mut session = session();
    let layer = three_keys(&mut session);
    a_bezier_first(&mut session, layer);
    let written = number(&value_at(&mut session, layer, "position", 5));
    session
        .execute("fx.keyframe.auto", &json!({ "layer": layer, "path": "position", "frame": 0, "auto": true }))
        .expect("the auto is set");
    let automatic = number(&value_at(&mut session, layer, "position", 5));
    assert_eq!(automatic, written, "the first keyframe has no neighbour before it");
}

/// The auto flag on a keyframe that is **not there** is refused.
#[test]
fn the_auto_on_a_keyframe_that_is_not_there_is_refused() {
    let mut session = session();
    let layer = three_keys(&mut session);
    let outcome = session.execute(
        "fx.keyframe.auto",
        &json!({ "layer": layer, "path": "position", "frame": 99, "auto": true }),
    );
    assert!(outcome.is_err(), "the keyframe that is not there is refused");
}

/// The outcome says whether the keyframe is an **auto** one.
#[test]
fn the_outcome_says_the_auto() {
    let mut session = session();
    let layer = three_keys(&mut session);
    let outcome = session
        .execute(
            "fx.keyframe.auto",
            &json!({ "layer": layer, "path": "position", "frame": 10, "auto": true }),
        )
        .expect("the auto is set");
    assert_eq!(outcome.data["auto"].as_bool(), Some(true), "the outcome says it");
}
