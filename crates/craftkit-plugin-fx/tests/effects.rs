//! The **effects** of a layer.
//!
//! An effect is a named set of parameters, and an instance of one on a
//! layer is a group in the layer's property tree.

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

/// The **catalogue** says what effects there are and what they take.
#[test]
fn the_catalogue_says_what_there_is() {
    let mut session = session();
    let outcome = session.execute("fx.effect.list", &json!({})).expect("the catalogue is read");
    let effects = outcome.data["effects"].as_array().expect("the effects are a list");
    assert_eq!(effects.len(), 6, "there are six effects");
    let blur = effects.iter().find(|effect| effect["id"] == "blur").expect("there is a blur");
    assert_eq!(blur["name"], "Blur", "the blur has a name");
    assert_eq!(blur["category"], "blur", "and a category");
    let radius = blur["params"].as_array().expect("the parameters are a list");
    assert_eq!(radius.len(), 1, "the blur takes one parameter");
    assert_eq!(radius[0]["id"], "radius", "and it is the radius");
    assert_eq!(radius[0]["kind"], "number", "and it is a number");
    assert_eq!(radius[0]["default"], 0.0, "that is zero when it is new");
    assert_eq!(radius[0]["min"], 0.0, "and it is not below zero");
    assert_eq!(radius[0]["max"], 100.0, "and not above a hundred");
}

/// The catalogue says the **kinds** of parameter there are.
#[test]
fn the_catalogue_says_the_kinds() {
    let mut session = session();
    let outcome = session.execute("fx.effect.list", &json!({})).expect("the catalogue is read");
    let effects = outcome.data["effects"].as_array().expect("the effects are a list");
    let noise = effects.iter().find(|effect| effect["id"] == "noise").expect("there is a noise");
    let params = noise["params"].as_array().expect("the parameters are a list");
    let monochrome =
        params.iter().find(|param| param["id"] == "monochrome").expect("the noise has a monochrome");
    assert_eq!(monochrome["kind"], "boolean", "a yes or a no");
    assert_eq!(monochrome["default"], false, "and it is no when it is new");
    let posterize = effects.iter().find(|effect| effect["id"] == "posterize").expect("there is a posterize");
    let params = posterize["params"].as_array().expect("the parameters are a list");
    let steps = params[0].as_object().expect("the steps are there");
    assert_eq!(steps["kind"], "popup", "one of a list");
    let options = steps["options"].as_array().expect("the options are a list");
    assert_eq!(options.len(), 4, "there are four of them");
    assert_eq!(options[0]["value"], 2.0, "and the first is two");
}

/// An effect is **put on** a layer, with its parameters at their
/// defaults.
#[test]
fn an_effect_is_put_on_a_layer() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    assert_eq!(at(&mut session, layer, "effects/#1/effect"), Some(json!("blur")), "the effect is there");
    assert_eq!(at(&mut session, layer, "effects/#1/radius"), Some(json!(0.0)), "with its radius at zero");
}

/// Two effects may be on a layer, and the second is the **second**.
#[test]
fn two_effects_may_be_on_a_layer() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "glow" }))
        .expect("the second effect is added");
    assert_eq!(at(&mut session, layer, "effects/#1/effect"), Some(json!("blur")), "the first is the blur");
    assert_eq!(at(&mut session, layer, "effects/#2/effect"), Some(json!("glow")), "the second is the glow");
    assert_eq!(at(&mut session, layer, "effects/#2/intensity"), Some(json!(0.0)), "with its intensity");
}

/// The **same** effect may be put on twice: the tree allows it and so
/// does this one.
#[test]
fn the_same_effect_may_be_put_on_twice() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the second effect is added");
    assert_eq!(at(&mut session, layer, "effects/#2/effect"), Some(json!("blur")), "both are blurs");
}

/// A parameter of an effect is an ordinary **property**: it may be set.
#[test]
fn a_parameter_is_a_property() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects/#1/radius", "value": 12 }))
        .expect("the radius is set");
    assert_eq!(at(&mut session, layer, "effects/#1/radius"), Some(json!(12)), "the radius is twelve");
}

/// An effect that is **not there** is refused, and the refusal says
/// what there is.
#[test]
fn an_effect_that_is_not_there_is_refused() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let error = session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "sharpen" }))
        .expect_err("the effect is refused");
    assert!(error.to_string().contains("there is no effect `sharpen`"), "the error says so: {error}");
    assert!(error.to_string().contains("blur"), "and it says what there is");
}

/// A node that is **not a layer** is refused.
#[test]
fn a_node_that_is_not_a_layer_is_refused() {
    let mut session = session();
    let (comp, _layer) = a_layer(&mut session);
    let error = session
        .execute("fx.effect.add", &json!({ "layer": comp, "effect": "blur" }))
        .expect_err("the effect is refused");
    assert!(error.to_string().contains("not a layer"), "the error says so: {error}");
}

/// An effect may be **taken away**, and the one after it moves up.
#[test]
fn an_effect_may_be_taken_away() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "glow" }))
        .expect("the second effect is added");
    session
        .execute("fx.effect.remove", &json!({ "layer": layer, "index": 1 }))
        .expect("the first effect is taken away");
    assert_eq!(
        at(&mut session, layer, "effects/#1/effect"),
        Some(json!("glow")),
        "the glow is the first now"
    );
    assert_eq!(at(&mut session, layer, "effects/#2/effect"), None, "and there is no second");
}

/// An effect that is **not there** may not be taken away, and the
/// refusal says so.
#[test]
fn an_effect_that_is_not_there_may_not_be_taken_away() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    let error = session
        .execute("fx.effect.remove", &json!({ "layer": layer, "index": 3 }))
        .expect_err("the effect is refused");
    assert!(error.to_string().contains("there is no effect number 3"), "the error says so: {error}");
    assert_eq!(at(&mut session, layer, "effects/#1/effect"), Some(json!("blur")), "and the effect is there");
}

/// An effect is **counted from one**: a number that is not one is
/// refused.
#[test]
fn an_effect_is_counted_from_one() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let error = session
        .execute("fx.effect.remove", &json!({ "layer": layer, "index": 0 }))
        .expect_err("the effect is refused");
    assert!(error.to_string().contains("counted from one"), "the error says so: {error}");
}

/// A **property** may not make an effect: the stack is made by
/// `fx.effect.add` and taken away by `fx.effect.remove`.
#[test]
fn a_property_may_not_make_an_effect() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let error = session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects", "value": [] }))
        .expect_err("the property is refused");
    assert!(error.to_string().contains("fx.effect.add"), "the error says so: {error}");
}

/// And a property may not make an **effect** by writing into a number
/// that is not there.
#[test]
fn a_property_may_not_make_an_effect_by_number() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let error = session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects/#2/radius", "value": 5 }))
        .expect_err("the property is refused");
    assert!(error.to_string().contains("there is no effect number 2"), "the error says so: {error}");
}

/// And a property may not write the **effect itself**, only a
/// parameter of it.
#[test]
fn a_property_may_not_write_the_effect_itself() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    let error = session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects/#1", "value": 3 }))
        .expect_err("the property is refused");
    assert!(error.to_string().contains("not a property"), "the error says so: {error}");
}

/// The effect is **saved**: it is in the document after it is done.
#[test]
fn the_effect_is_saved() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "levels" }))
        .expect("the effect is added");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(layer).expect("the layer is in the document");
    let effects = node.data["props"]["effects"].as_array().expect("the effects are a list");
    assert_eq!(effects.len(), 1, "there is one effect");
    assert_eq!(effects[0]["effect"], "levels", "and it is the levels");
    assert_eq!(effects[0]["input_high"], 100.0, "with its parameters at their defaults");
}
