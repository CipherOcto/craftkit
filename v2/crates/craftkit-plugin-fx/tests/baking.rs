//! The **baking** of an expression into keyframes.

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

/// A property with one **keyframe** and an **expression**, and the
/// layer's id.
fn a_property_with_an_expression(session: &mut Session, path: &str, expression: &str) -> u64 {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": path, "value": { "keys": [] } }))
        .expect("the property is animated");
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": path, "frame": 0, "value": 1.0 }))
        .expect("the keyframe is set");
    session
        .execute("fx.prop.expression", &json!({ "layer": layer, "path": path, "expression": expression }))
        .expect("the expression is set");
    layer
}

/// The **value** of a property at a frame.
fn value_at(session: &mut Session, layer: u64, path: &str, frame: u64) -> f64 {
    let outcome = session
        .execute("fx.prop.at", &json!({ "layer": layer, "path": path, "frame": frame }))
        .expect("the value is read");
    outcome.data["value"].as_f64().expect("the value is a number")
}

/// An expression is **baked** into keyframes.
#[test]
fn an_expression_is_baked_into_keyframes() {
    let mut session = session();
    let layer = a_property_with_an_expression(&mut session, "position/x", "time * 2");
    let outcome = session
        .execute("fx.prop.bake", &json!({ "layer": layer, "path": "position/x", "from": 0, "to": 4 }))
        .expect("the expression is baked");
    assert_eq!(outcome.data["keyframes"], json!(5), "there are five keyframes");
    assert_eq!(value_at(&mut session, layer, "position/x", 3), 6.0, "the value at frame three is six");
}

/// The **step** between the samples may be more than one frame.
#[test]
fn the_step_between_the_samples_may_be_more_than_one_frame() {
    let mut session = session();
    let layer = a_property_with_an_expression(&mut session, "position/x", "time * 2");
    let outcome = session
        .execute(
            "fx.prop.bake",
            &json!({ "layer": layer, "path": "position/x", "from": 0, "to": 4, "step": 2 }),
        )
        .expect("the expression is baked");
    assert_eq!(outcome.data["keyframes"], json!(3), "there are three keyframes");
}

/// The expression is **taken away** when it is baked.
#[test]
fn the_expression_is_taken_away_when_it_is_baked() {
    let mut session = session();
    let layer = a_property_with_an_expression(&mut session, "position/x", "time * 2");
    session
        .execute("fx.prop.bake", &json!({ "layer": layer, "path": "position/x", "from": 0, "to": 2 }))
        .expect("the expression is baked");
    let outcome = session
        .execute("fx.prop.get", &json!({ "layer": layer, "path": "position/x" }))
        .expect("the property is read");
    assert!(outcome.data.get("expression").is_none(), "the expression is gone");
}

/// A property with **no expression** has nothing to bake.
#[test]
fn a_property_with_no_expression_has_nothing_to_bake() {
    let mut session = session();
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "position/x", "value": { "keys": [] } }))
        .expect("the property is animated");
    let outcome =
        session.execute("fx.prop.bake", &json!({ "layer": layer, "path": "position/x", "from": 0, "to": 2 }));
    assert!(outcome.is_err(), "there is no expression to bake");
}

/// A bake of more than a **thousand** samples is refused.
#[test]
fn a_long_bake_is_refused() {
    let mut session = session();
    let layer = a_property_with_an_expression(&mut session, "position/x", "time * 2");
    let outcome = session
        .execute("fx.prop.bake", &json!({ "layer": layer, "path": "position/x", "from": 0, "to": 2000 }));
    assert!(outcome.is_err(), "a thousand samples is the bound");
}
