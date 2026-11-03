//! The **switches** of a layer.
//!
//! A switch is a fact about the layer, and what the switches *do* is
//! derived: a solo silences the others, a shy layer steps aside when
//! the composition says so, and a locked layer is not edited.

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

/// A **composition** with a **layer** in it, and their ids.
fn a_comp_and_a_layer(session: &mut Session) -> (u64, u64) {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    (comp, layer)
}

/// The **switches** of a layer, as the document holds them.
fn switches_of(session: &Session, layer: u64) -> Value {
    let node = session.document().node(layer).expect("the layer is there");
    node.data.get("switches").cloned().unwrap_or(Value::Null)
}

/// What the switches **do**, read from the plan.
fn what_they_do(session: &Session, layer: u64) -> Value {
    let plan = session.evaluate();
    let item = plan
        .items
        .iter()
        .find(|item| item.node == layer && item.kind == "fx.switches")
        .expect("the layer is in the plan");
    item.value.clone()
}

/// A new layer has the tree's **own** switches: the eye and the
/// speaker on, and the rest off.
#[test]
fn a_new_layer_has_the_trees_own_switches() {
    let mut session = session();
    let (_, layer) = a_comp_and_a_layer(&mut session);
    let switches = switches_of(&session, layer);
    assert_eq!(switches["video"].as_bool(), Some(true), "the eye is on");
    assert_eq!(switches["audio"].as_bool(), Some(true), "the speaker is on");
    assert_eq!(switches["solo"].as_bool(), Some(false), "the layer is not alone");
    assert_eq!(switches["locked"].as_bool(), Some(false), "the layer is not locked");
    assert_eq!(switches["effects"].as_bool(), Some(true), "the effects run");
}

/// A switch may be **put on**.
#[test]
fn a_switch_may_be_put_on() {
    let mut session = session();
    let (_, layer) = a_comp_and_a_layer(&mut session);
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "video", "value": false }))
        .expect("the switch is off");
    assert_eq!(switches_of(&session, layer)["video"].as_bool(), Some(false), "the eye is off");
}

/// A switch with **no value** is toggled.
#[test]
fn a_switch_with_no_value_is_toggled() {
    let mut session = session();
    let (_, layer) = a_comp_and_a_layer(&mut session);
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "solo" }))
        .expect("the switch is toggled");
    assert_eq!(switches_of(&session, layer)["solo"].as_bool(), Some(true), "the layer is alone");
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "solo" }))
        .expect("the switch is toggled again");
    assert_eq!(switches_of(&session, layer)["solo"].as_bool(), Some(false), "and it is not");
}

/// A switch that **does not exist** is refused.
#[test]
fn a_switch_that_does_not_exist_is_refused() {
    let mut session = session();
    let (_, layer) = a_comp_and_a_layer(&mut session);
    let error = session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "wobble", "value": true }))
        .expect_err("there is no such switch");
    assert!(error.to_string().contains("no switch"), "the error says there is no switch: {error}");
}

/// The tree's **names** are the tree's: the eye is the video switch,
/// the lock is the locked one.
#[test]
fn the_trees_names_are_the_trees() {
    let mut session = session();
    let (_, layer) = a_comp_and_a_layer(&mut session);
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "eye", "value": false }))
        .expect("the eye is off");
    assert_eq!(switches_of(&session, layer)["video"].as_bool(), Some(false), "the eye is the video switch");
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "lock", "value": true }))
        .expect("the layer is locked");
    assert_eq!(switches_of(&session, layer)["locked"].as_bool(), Some(true), "the lock is the locked switch");
}

/// A **solo** silences the others: the tree's renderer draws the
/// soloed layers alone.
#[test]
fn a_solo_silences_the_others() {
    let mut session = session();
    let (comp, layer) = a_comp_and_a_layer(&mut session);
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the other" }))
        .expect("the other layer is added");
    let other = outcome.data["id"].as_u64().expect("the other layer has an id");
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "solo", "value": true }))
        .expect("the layer is alone");
    let mine = what_they_do(&session, layer);
    let theirs = what_they_do(&session, other);
    assert_eq!(mine["drawn"].as_bool(), Some(true), "the soloed layer is drawn");
    assert_eq!(theirs["drawn"].as_bool(), Some(false), "and the other one is not");
    assert_eq!(theirs["heard"].as_bool(), Some(false), "and it is not heard");
}

/// A **shy** layer steps aside when the composition hides the shy
/// layers.
#[test]
fn a_shy_layer_steps_aside() {
    let mut session = session();
    let (comp, layer) = a_comp_and_a_layer(&mut session);
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "shy", "value": true }))
        .expect("the layer is shy");
    // The composition does not hide the shy layers yet, and the layer
    // is drawn.
    assert_eq!(what_they_do(&session, layer)["drawn"].as_bool(), Some(true), "the shy layer is drawn");
    session
        .execute("fx.comp.switch", &json!({ "comp": comp, "switch": "hide_shy", "value": true }))
        .expect("the composition hides the shy layers");
    let what = what_they_do(&session, layer);
    assert_eq!(what["drawn"].as_bool(), Some(false), "and it is not drawn now");
    assert_eq!(what["editable"].as_bool(), Some(false), "and it is not edited");
}

/// The composition's switch with **no value** is toggled.
#[test]
fn the_compositions_switch_with_no_value_is_toggled() {
    let mut session = session();
    let (comp, _) = a_comp_and_a_layer(&mut session);
    session
        .execute("fx.comp.switch", &json!({ "comp": comp, "switch": "hide_shy" }))
        .expect("the switch is toggled");
    let node = session.document().node(comp).expect("the comp is there");
    assert_eq!(node.data["hide_shy"].as_bool(), Some(true), "the composition hides the shy layers");
}

/// A **locked** layer is not edited.
#[test]
fn a_locked_layer_is_not_edited() {
    let mut session = session();
    let (_, layer) = a_comp_and_a_layer(&mut session);
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "lock", "value": true }))
        .expect("the layer is locked");
    assert_eq!(what_they_do(&session, layer)["editable"].as_bool(), Some(false), "the layer is not edited");
}

/// A switch that is **not** a switch of a composition is refused.
#[test]
fn a_switch_that_is_not_a_switch_of_a_composition_is_refused() {
    let mut session = session();
    let (comp, _) = a_comp_and_a_layer(&mut session);
    let error = session
        .execute("fx.comp.switch", &json!({ "comp": comp, "switch": "solo", "value": true }))
        .expect_err("a composition has no solo switch");
    assert!(error.to_string().contains("no switch"), "the error says there is no switch: {error}");
}

/// The switches are **saved** with the layer.
#[test]
fn the_switches_are_saved_with_the_layer() {
    let mut session = session();
    let (_, layer) = a_comp_and_a_layer(&mut session);
    session
        .execute("fx.layer.switch", &json!({ "layer": layer, "switch": "shy", "value": true }))
        .expect("the layer is shy");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(layer).expect("the layer is there");
    assert_eq!(node.data["switches"]["shy"].as_bool(), Some(true), "the shy layer is still shy");
}
