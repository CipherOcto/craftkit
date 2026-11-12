//! The other **commands on an effect instance**: reset, duplicate and
//! reorder.

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

/// The parameters of an effect may be put back to their **defaults**.
#[test]
fn the_parameters_may_be_reset() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects/#1/radius", "value": 12 }))
        .expect("the radius is set");
    session.execute("fx.effect.reset", &json!({ "layer": layer, "effect": 1 })).expect("the effect is reset");
    assert_eq!(
        at(&mut session, layer, "effects/#1/radius"),
        Some(json!(0.0)),
        "the radius is back to nothing"
    );
}

/// A reset of an effect whose parameter has **keyframes** is refused: it
/// would lose them.
#[test]
fn a_reset_that_would_lose_the_keyframes_is_refused() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    session
        .execute(
            "fx.prop.set",
            &json!({ "layer": layer, "path": "effects/#1/radius", "value": { "keys": [] } }),
        )
        .expect("the property is animated");
    session
        .execute(
            "fx.keyframe.set",
            &json!({ "layer": layer, "path": "effects/#1/radius", "frame": 0, "value": 5 }),
        )
        .expect("the keyframe is set");
    let error = session
        .execute("fx.effect.reset", &json!({ "layer": layer, "effect": 1 }))
        .expect_err("the reset is refused");
    assert!(error.to_string().contains("has keyframes"), "the error says so: {error}");
}

/// An effect may be **copied**, and the copy holds the same values.
#[test]
fn an_effect_may_be_copied() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects/#1/radius", "value": 12 }))
        .expect("the radius is set");
    session
        .execute("fx.effect.duplicate", &json!({ "layer": layer, "effect": 1 }))
        .expect("the effect is copied");
    assert_eq!(at(&mut session, layer, "effects/#2/effect"), Some(json!("blur")), "the copy is the second");
    assert_eq!(at(&mut session, layer, "effects/#2/radius"), Some(json!(12)), "and it holds the same value");
}

/// The copy goes **right after** the one it is a copy of.
#[test]
fn the_copy_goes_right_after() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "glow" }))
        .expect("the second effect is added");
    session
        .execute("fx.effect.duplicate", &json!({ "layer": layer, "effect": 1 }))
        .expect("the effect is copied");
    assert_eq!(at(&mut session, layer, "effects/#2/effect"), Some(json!("blur")), "the copy is the second");
    assert_eq!(
        at(&mut session, layer, "effects/#3/effect"),
        Some(json!("glow")),
        "and the glow is the third"
    );
}

/// An effect may be **moved** in the stack.
#[test]
fn an_effect_may_be_moved() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "glow" }))
        .expect("the second effect is added");
    session
        .execute("fx.effect.reorder", &json!({ "layer": layer, "effect": 1, "to": 2 }))
        .expect("the effect is moved");
    assert_eq!(at(&mut session, layer, "effects/#1/effect"), Some(json!("glow")), "the glow is the first");
    assert_eq!(
        at(&mut session, layer, "effects/#2/effect"),
        Some(json!("blur")),
        "and the blur is the second"
    );
}

/// A move says which **number** the effect is now, because the paths
/// address an effect by its number.
#[test]
fn a_move_says_where_the_effect_is_now() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "glow" }))
        .expect("the second effect is added");
    let outcome = session
        .execute("fx.effect.reorder", &json!({ "layer": layer, "effect": 2, "to": 1 }))
        .expect("the effect is moved");
    assert_eq!(outcome.data["to"], json!(1), "the answer says where it is now");
    assert_eq!(outcome.data["name"], json!("glow"), "and which effect it is");
}

/// A place that is **not there** is refused: the tree clamps it and
/// says nothing.
#[test]
fn a_place_that_is_not_there_is_refused() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    let error = session
        .execute("fx.effect.reorder", &json!({ "layer": layer, "effect": 1, "to": 5 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("the 5 is not one of them"), "the error says so: {error}");
}

/// An effect that is already in that place may not be moved: a
/// command that does nothing did not do anything.
#[test]
fn an_effect_that_is_already_there_is_refused() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    let error = session
        .execute("fx.effect.reorder", &json!({ "layer": layer, "effect": 1, "to": 1 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("already the 1"), "the error says so: {error}");
}

/// An effect that is **not there** may not be reset, copied or moved.
#[test]
fn an_effect_that_is_not_there_is_refused() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    for command in ["fx.effect.reset", "fx.effect.duplicate"] {
        let error = session
            .execute(command, &json!({ "layer": layer, "effect": 3 }))
            .expect_err("the effect is refused");
        assert!(error.to_string().contains("there is no effect number 3"), "the error says so: {error}");
    }
    let error = session
        .execute("fx.effect.reorder", &json!({ "layer": layer, "effect": 3, "to": 1 }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("there is no effect number 3"), "the error says so: {error}");
}

/// A node that is **not a layer** is refused.
#[test]
fn a_node_that_is_not_a_layer_is_refused() {
    let mut session = session();
    let (comp, _layer) = a_layer(&mut session);
    let error = session
        .execute("fx.effect.reset", &json!({ "layer": comp, "effect": 1 }))
        .expect_err("the reset is refused");
    assert!(error.to_string().contains("not a layer"), "the error says so: {error}");
}

/// The commands are **saved**: they are in the document after they are
/// done.
#[test]
fn the_commands_are_saved() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "glow" }))
        .expect("the second effect is added");
    session
        .execute("fx.effect.reorder", &json!({ "layer": layer, "effect": 1, "to": 2 }))
        .expect("the effect is moved");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(layer).expect("the layer is in the document");
    let effects = node.data["props"]["effects"].as_array().expect("the effects are a list");
    assert_eq!(effects[0]["effect"].as_str(), Some("glow"), "the glow is the first now");
}
