//! The **switch** of one effect.
//!
//! An effect on a layer may be switched off and leave its values where
//! they are: the tree's `effect.toggle`.

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
fn a_layer(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 100, "height": 100, "duration": 10 }))
        .expect("the composition is added");
    let comp = outcome.data["id"].as_u64().expect("the composition has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    (comp, layer)
}

/// The value at a path on a layer.
fn at(session: &mut Session, layer: u64, path: &str) -> Option<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("fx.prop.get", &json!({ "layer": layer, "path": path }))
        .expect("the property is read");
    let value = outcome.data["value"].clone();
    if value.is_null() { None } else { Some(value) }
}

/// What the switches **do**, read from the plan.
fn switches(session: &mut Session, layer: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan
        .items
        .iter()
        .find(|item| item.node == layer && item.kind == "fx.switches")
        .expect("the layer is in the plan");
    item.value.clone()
}

/// An effect is **on** when it is put on a layer.
#[test]
fn an_effect_is_on_when_it_is_put_on() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    assert_eq!(at(&mut session, layer, "effects/#1/enabled"), Some(json!(true)), "the effect is on");
}

/// An effect may be switched **off**, and its values stay where they
/// are.
#[test]
fn an_effect_may_be_switched_off() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects/#1/radius", "value": 12 }))
        .expect("the radius is set");
    session
        .execute("fx.effect.toggle", &json!({ "layer": layer, "effect": 1, "on": false }))
        .expect("the effect is switched off");
    assert_eq!(at(&mut session, layer, "effects/#1/enabled"), Some(json!(false)), "the effect is off");
    assert_eq!(at(&mut session, layer, "effects/#1/radius"), Some(json!(12)), "and its value is there");
}

/// An effect with no value given is **toggled**: that is the tree's
/// own rule.
#[test]
fn an_effect_with_no_value_is_toggled() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    session
        .execute("fx.effect.toggle", &json!({ "layer": layer, "effect": 1 }))
        .expect("the effect is toggled");
    assert_eq!(at(&mut session, layer, "effects/#1/enabled"), Some(json!(false)), "the effect is off");
    session
        .execute("fx.effect.toggle", &json!({ "layer": layer, "effect": 1 }))
        .expect("the effect is toggled");
    assert_eq!(at(&mut session, layer, "effects/#1/enabled"), Some(json!(true)), "and on again");
}

/// The **effects that run** are derived: the ones that are enabled and
/// the layer's own effects switch is on.
#[test]
fn the_effects_that_run_are_derived() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "glow" }))
        .expect("the second effect is added");
    let value = switches(&mut session, layer);
    assert_eq!(value["effects"], json!(2), "both effects run");
    session
        .execute("fx.effect.toggle", &json!({ "layer": layer, "effect": 2, "on": false }))
        .expect("the second effect is switched off");
    let value = switches(&mut session, layer);
    assert_eq!(value["effects"], json!(1), "and now one does");
}

/// The layer's own **effects switch** says whether its effects run at
/// all.
#[test]
fn the_layers_own_switch_says_whether_they_run() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "effects", "value": false }))
        .expect("the switch is off");
    let value = switches(&mut session, layer);
    assert_eq!(value["effects"], json!(0), "and no effect runs");
}

/// An effect that is **not there** may not be switched.
#[test]
fn an_effect_that_is_not_there_is_refused() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    let error = session
        .execute("fx.effect.toggle", &json!({ "layer": layer, "effect": 3, "on": false }))
        .expect_err("the effect is refused");
    assert!(error.to_string().contains("there is no effect number 3"), "the error says so: {error}");
}

/// An effect is **counted from one**.
#[test]
fn an_effect_is_counted_from_one() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let error = session
        .execute("fx.effect.toggle", &json!({ "layer": layer, "effect": 0, "on": false }))
        .expect_err("the effect is refused");
    assert!(error.to_string().contains("counted from one"), "the error says so: {error}");
}

/// A node that is **not a layer** is refused.
#[test]
fn a_node_that_is_not_a_layer_is_refused() {
    let mut session = session();
    let (comp, _layer) = a_layer(&mut session);
    let error = session
        .execute("fx.effect.toggle", &json!({ "layer": comp, "effect": 1, "on": false }))
        .expect_err("the effect is refused");
    assert!(error.to_string().contains("not a layer"), "the error says so: {error}");
}

/// The switch is **saved**: it is in the document after it is done.
#[test]
fn the_switch_is_saved() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    session
        .execute("fx.effect.toggle", &json!({ "layer": layer, "effect": 1, "on": false }))
        .expect("the effect is switched off");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(layer).expect("the layer is in the document");
    assert_eq!(node.data["props"]["effects"][0]["enabled"].as_bool(), Some(false), "the switch is there");
}
