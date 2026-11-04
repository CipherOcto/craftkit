//! The **selection** of layers.

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

/// A **composition** with two layers in it.
fn a_stack(session: &mut Session) -> (u64, Vec<u64>) {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let mut layers = Vec::new();
    for name in ["one", "two"] {
        let outcome = session
            .execute("fx.layer.add", &json!({ "comp": comp, "name": name }))
            .expect("the layer is added");
        layers.push(outcome.data["id"].as_u64().expect("the layer has an id"));
    }
    (comp, layers)
}

/// The **selected** layers.
fn selected(session: &mut Session) -> Vec<u64> {
    let outcome = session.execute("fx.selection.get", &json!({})).expect("the selection is read");
    outcome.data["layers"]
        .as_array()
        .expect("the layers are there")
        .iter()
        .map(|layer| layer.as_u64().expect("a layer has an id"))
        .collect()
}

/// Layers may be **selected**.
#[test]
fn layers_may_be_selected() {
    let mut session = session();
    let (_comp, layers) = a_stack(&mut session);
    assert!(selected(&mut session).is_empty(), "nothing is selected to begin with");
    session.execute("fx.layer.select", &json!({ "layers": [layers[0]] })).expect("the layer is selected");
    assert_eq!(selected(&mut session), vec![layers[0]], "and it is selected");
    session
        .execute("fx.layer.select", &json!({ "layers": [layers[1]] }))
        .expect("the other layer is selected");
    assert_eq!(selected(&mut session), vec![layers[1]], "and a select replaces the selection");
}

/// Layers may be **taken out** of the selection.
#[test]
fn layers_may_be_taken_out_of_the_selection() {
    let mut session = session();
    let (_comp, layers) = a_stack(&mut session);
    session.execute("fx.layer.select", &json!({ "layers": layers })).expect("the layers are selected");
    assert_eq!(selected(&mut session).len(), 2, "both of them are selected");
    session.execute("fx.layer.deselect", &json!({ "layers": [layers[0]] })).expect("the layer is taken out");
    assert_eq!(selected(&mut session), vec![layers[1]], "and the other one is still selected");
}

/// A layer that is **not there** is refused.
#[test]
fn a_layer_that_is_not_there_is_refused() {
    let mut session = session();
    let (_comp, layers) = a_stack(&mut session);
    let error = session
        .execute("fx.layer.select", &json!({ "layers": [9999] }))
        .expect_err("a layer that is not there is refused");
    assert!(error.to_string().contains("9999"), "and the error says which");
    session
        .execute("fx.layer.select", &json!({ "layers": [layers[0]] }))
        .expect("and a layer that is there is not refused");
}
