//! The **blending mode** of a layer.
//!
//! A layer says how it is laid over what is under it, and the tree's
//! list of modes is long and the tree's names are not the names they
//! are written with.

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

/// The **blending mode** of a layer, as the document holds it.
fn mode_of(session: &Session, layer: u64) -> String {
    let node = session.document().node(layer).expect("the layer is there");
    node.data.get("blend_mode").and_then(|mode| mode.as_str()).unwrap_or("Normal").to_string()
}

/// A new layer is blended **normally**.
#[test]
fn a_new_layer_is_blended_normally() {
    let mut session = session();
    let layer = a_layer(&mut session);
    assert_eq!(mode_of(&session, layer), "Normal", "the layer is blended normally");
}

/// A layer may be blended **differently**.
#[test]
fn a_layer_may_be_blended_differently() {
    let mut session = session();
    let layer = a_layer(&mut session);
    session
        .execute("fx.layer.blend", &json!({ "layer": layer, "mode": "Multiply" }))
        .expect("the layer is multiplied");
    assert_eq!(mode_of(&session, layer), "Multiply", "the layer is multiplied");
}

/// The tree's **names** are not the names they are written with: a
/// mode is found by its name with everything that is not a letter or
/// a digit taken out.
#[test]
fn the_trees_names_are_the_trees() {
    let mut session = session();
    let layer = a_layer(&mut session);
    session
        .execute("fx.layer.blend", &json!({ "layer": layer, "mode": "ColorBurn" }))
        .expect("the layer is burned");
    assert_eq!(mode_of(&session, layer), "Color Burn", "the mode is the tree's own");
}

/// A mode that **does not exist** is refused.
#[test]
fn a_mode_that_does_not_exist_is_refused() {
    let mut session = session();
    let layer = a_layer(&mut session);
    let error = session
        .execute("fx.layer.blend", &json!({ "layer": layer, "mode": "Wobble" }))
        .expect_err("there is no such mode");
    assert!(error.to_string().contains("no blending mode"), "the error says there is no mode: {error}");
    assert_eq!(mode_of(&session, layer), "Normal", "and the layer is unchanged");
}

/// A **step** goes to the next mode, and the list **wraps around**.
#[test]
fn a_step_goes_to_the_next_mode_and_wraps_around() {
    let mut session = session();
    let layer = a_layer(&mut session);
    session.execute("fx.layer.blend", &json!({ "layer": layer, "step": 1 })).expect("the mode steps on");
    assert_eq!(mode_of(&session, layer), "Dissolve", "the next mode is Dissolve");
    session.execute("fx.layer.blend", &json!({ "layer": layer, "step": -1 })).expect("the mode steps back");
    assert_eq!(mode_of(&session, layer), "Normal", "and back to Normal");
    // The list wraps: a step back from the first mode is the last one.
    session.execute("fx.layer.blend", &json!({ "layer": layer, "step": -1 })).expect("the mode wraps");
    assert_eq!(mode_of(&session, layer), "Luminescent Premul", "the last mode is the last one");
}

/// A step of **zero** is refused: it says nothing.
#[test]
fn a_step_of_zero_is_refused() {
    let mut session = session();
    let layer = a_layer(&mut session);
    let error = session
        .execute("fx.layer.blend", &json!({ "layer": layer, "step": 0 }))
        .expect_err("a step of zero says nothing");
    assert!(error.to_string().contains("step"), "the error says the step is nothing: {error}");
}

/// A mode **and** a step is two ways of saying one thing, and it is
/// refused.
#[test]
fn a_mode_and_a_step_is_refused() {
    let mut session = session();
    let layer = a_layer(&mut session);
    let error = session
        .execute("fx.layer.blend", &json!({ "layer": layer, "mode": "Multiply", "step": 1 }))
        .expect_err("the mode and the step are one thing said twice");
    assert!(error.to_string().contains("two ways"), "the error says so: {error}");
}

/// A command with **neither** does not know what the mode should be.
#[test]
fn a_command_with_neither_is_refused() {
    let mut session = session();
    let layer = a_layer(&mut session);
    let error = session
        .execute("fx.layer.blend", &json!({ "layer": layer }))
        .expect_err("the command needs a mode or a step");
    assert!(error.to_string().contains("needed"), "the error says what is needed: {error}");
}

/// The tree's **stencil** modes act on everything below the layer, and
/// the command says which modes those are.
#[test]
fn the_stencil_modes_are_the_trees() {
    let mut session = session();
    let layer = a_layer(&mut session);
    let outcome = session
        .execute("fx.layer.blend", &json!({ "layer": layer, "mode": "Stencil Alpha" }))
        .expect("the layer is a stencil");
    assert_eq!(outcome.data["stencil"].as_bool(), Some(true), "a stencil acts on everything below");
    let outcome = session
        .execute("fx.layer.blend", &json!({ "layer": layer, "mode": "Multiply" }))
        .expect("the layer is multiplied");
    assert_eq!(outcome.data["stencil"].as_bool(), Some(false), "and a multiply does not");
}

/// The mode is **saved** with the layer.
#[test]
fn the_mode_is_saved_with_the_layer() {
    let mut session = session();
    let layer = a_layer(&mut session);
    session
        .execute("fx.layer.blend", &json!({ "layer": layer, "mode": "Screen" }))
        .expect("the layer is screened");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(layer).expect("the layer is there");
    assert_eq!(node.data["blend_mode"].as_str(), Some("Screen"), "the layer is still screened");
}
