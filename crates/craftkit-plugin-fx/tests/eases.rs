//! The **ease** of a keyframe: the easy ease, and the hold.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **property** that is animated, with a keyframe on it.
fn an_animated_layer(session: &mut Session) -> u64 {
    let layer = a_layer(session);
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "position", "value": { "keys": [] } }))
        .expect("the property is animated");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 0, "value": 0 }))
        .expect("the keyframe is set");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": "position", "frame": 24, "value": 100 }))
        .expect("the keyframe is set");
    layer
}

/// The **keyframe** at a frame, as the payload holds it.
fn key_at(session: &mut Session, layer: u64, path: &str, frame: u64) -> Json {
    let outcome = session
        .execute("fx.prop.get", &json!({ "layer": layer, "path": path }))
        .expect("the property is read");
    let value = outcome.data["value"].clone();
    let keys = value["keys"].as_array().cloned().expect("the keys are a list");
    keys.into_iter()
        .find(|key| key["frame"] == json!(frame))
        .unwrap_or_else(|| panic!("there is a keyframe at frame {frame}"))
}

/// The easy ease gives the keyframe an **influence of a third** and a
/// **speed of nothing**.
#[test]
fn the_easy_ease_is_a_third_and_no_speed() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.easy", &json!({ "layer": layer, "path": "position", "frame": 0 }))
        .expect("the keyframe has an easy ease");
    let key = key_at(&mut session, layer, "position", 0);
    assert_eq!(key["interp"], json!("bezier"), "the side it leaves on is a bezier");
    assert_eq!(key["ease"]["influence"], json!(1.0 / 3.0), "the influence is a third");
    assert_eq!(key["ease"]["speed"], json!(0.0), "and the speed is nothing");
}

/// The easy ease may be given to **one side** only.
#[test]
fn the_easy_ease_may_be_given_to_one_side() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute(
            "fx.keyframe.easy",
            &json!({ "layer": layer, "path": "position", "frame": 0, "side": "out" }),
        )
        .expect("the keyframe has an easy ease");
    let key = key_at(&mut session, layer, "position", 0);
    assert_eq!(key["interp"], json!("bezier"), "the side it leaves on is a bezier");
    assert_eq!(key["ease"]["influence"], json!(1.0 / 3.0), "the influence is a third");
    assert_eq!(key["in_interp"], json!("linear"), "and the side it is entered on is untouched");
}

/// The easy ease stops the keyframe being an **auto bezier**: the ease
/// is written down now.
#[test]
fn the_easy_ease_stops_the_keyframe_being_an_auto_bezier() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.auto", &json!({ "layer": layer, "path": "position", "frame": 0, "auto": true }))
        .expect("the keyframe is an auto bezier");
    session
        .execute("fx.keyframe.easy", &json!({ "layer": layer, "path": "position", "frame": 0 }))
        .expect("the keyframe has an easy ease");
    let key = key_at(&mut session, layer, "position", 0);
    assert_eq!(key["auto"], json!(false), "the keyframe is not an auto bezier any more");
}

/// A side that is **not a side** is refused.
#[test]
fn a_side_that_is_not_a_side_is_refused() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    let value = session
        .execute(
            "fx.keyframe.easy",
            &json!({ "layer": layer, "path": "position", "frame": 0, "side": "sideways" }),
        )
        .expect_err("the ease is refused");
    assert!(value.to_string().contains("not a side"), "the refusal says why, and it says {}", value);
}

/// A **hold** keyframe does not move the value until the next one.
#[test]
fn a_hold_keyframe_does_not_move_the_value() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.hold", &json!({ "layer": layer, "path": "position", "frame": 0 }))
        .expect("the keyframe holds");
    let key = key_at(&mut session, layer, "position", 0);
    assert_eq!(key["interp"], json!("hold"), "the side it leaves on holds");
    let outcome = session
        .execute("fx.prop.at", &json!({ "layer": layer, "path": "position", "frame": 12 }))
        .expect("the value is read");
    assert_eq!(outcome.data["value"], json!(0), "and the value has not moved");
}

/// The **hold** turned off again gives a **straight line** back, when
/// the keyframe does not ease.
#[test]
fn the_hold_turned_off_again_gives_a_straight_line_back() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.hold", &json!({ "layer": layer, "path": "position", "frame": 0 }))
        .expect("the keyframe holds");
    session
        .execute("fx.keyframe.hold", &json!({ "layer": layer, "path": "position", "frame": 0 }))
        .expect("the keyframe holds no more");
    let key = key_at(&mut session, layer, "position", 0);
    assert_eq!(key["interp"], json!("linear"), "the side it leaves on is a line");
}

/// The **hold** turned off again gives a **bezier** back, when the
/// keyframe eases.
#[test]
fn the_hold_turned_off_again_gives_a_bezier_back() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    session
        .execute("fx.keyframe.auto", &json!({ "layer": layer, "path": "position", "frame": 0, "auto": true }))
        .expect("the keyframe is an auto bezier");
    session
        .execute("fx.keyframe.hold", &json!({ "layer": layer, "path": "position", "frame": 0 }))
        .expect("the keyframe holds");
    session
        .execute("fx.keyframe.hold", &json!({ "layer": layer, "path": "position", "frame": 0 }))
        .expect("the keyframe holds no more");
    let key = key_at(&mut session, layer, "position", 0);
    assert_eq!(key["interp"], json!("bezier"), "the side it leaves on is a bezier");
}

/// A keyframe that is **not there** is refused.
#[test]
fn a_keyframe_that_is_not_there_is_refused() {
    let mut session = session();
    let layer = an_animated_layer(&mut session);
    let value = session
        .execute("fx.keyframe.easy", &json!({ "layer": layer, "path": "position", "frame": 99 }))
        .expect_err("the ease is refused");
    assert!(value.to_string().contains("no keyframe"), "the refusal says why, and it says {}", value);
}
