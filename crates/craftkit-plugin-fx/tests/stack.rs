//! The **stack of layers** of a composition.

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

/// A **composition** with three layers in it.
fn a_stack(session: &mut Session) -> (u64, Vec<u64>) {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let mut layers = Vec::new();
    for name in ["one", "two", "three"] {
        let outcome = session
            .execute("fx.layer.add", &json!({ "comp": comp, "name": name }))
            .expect("the layer is added");
        layers.push(outcome.data["id"].as_u64().expect("the layer has an id"));
    }
    (comp, layers)
}

/// The **stack**, the front first.
fn stack(session: &mut Session, comp: u64) -> Vec<u64> {
    let outcome = session.execute("fx.layer.list", &json!({ "comp": comp })).expect("the stack is read");
    outcome.data["layers"]
        .as_array()
        .expect("the layers are there")
        .iter()
        .map(|layer| layer.as_u64().expect("a layer has an id"))
        .collect()
}

/// The stack may be **read**.
#[test]
fn the_stack_may_be_read() {
    let mut session = session();
    let (comp, layers) = a_stack(&mut session);
    let order = stack(&mut session, comp);
    assert_eq!(order, vec![layers[2], layers[1], layers[0]], "the front is the last one added");
}

/// A layer may be **moved** in the stack.
#[test]
fn a_layer_may_be_moved_in_the_stack() {
    let mut session = session();
    let (comp, layers) = a_stack(&mut session);
    let (one, two, three) = (layers[0], layers[1], layers[2]);
    session
        .execute("fx.layer.arrange", &json!({ "comp": comp, "layer": three, "to": "back" }))
        .expect("the layer goes to the back");
    assert_eq!(stack(&mut session, comp), vec![two, one, three], "and the stack is the stack");
    session
        .execute("fx.layer.arrange", &json!({ "comp": comp, "layer": three, "to": "forward" }))
        .expect("the layer goes forward");
    assert_eq!(stack(&mut session, comp), vec![two, three, one], "and one step forward is one step");
    session
        .execute("fx.layer.arrange", &json!({ "comp": comp, "layer": three, "to": "front" }))
        .expect("the layer goes to the front");
    assert_eq!(stack(&mut session, comp), vec![three, two, one], "and to the front is to the front");
    session
        .execute("fx.layer.arrange", &json!({ "comp": comp, "layer": three, "to": "backward" }))
        .expect("the layer goes backward");
    assert_eq!(stack(&mut session, comp), vec![two, three, one], "and one step backward is one step");
}

/// A place that is **no place** is refused.
#[test]
fn a_place_that_is_no_place_is_refused() {
    let mut session = session();
    let (comp, layers) = a_stack(&mut session);
    let error = session
        .execute("fx.layer.arrange", &json!({ "comp": comp, "layer": layers[0], "to": "sideways" }))
        .expect_err("a place that is not a place is refused");
    assert!(error.to_string().contains("sideways"), "and the error says which");
    let error = session
        .execute("fx.layer.arrange", &json!({ "comp": comp, "layer": 9999, "to": "front" }))
        .expect_err("a layer that is not there is refused");
    assert!(error.to_string().contains("9999"), "and the error says which too");
}

/// A layer may be **renamed**.
#[test]
fn a_layer_may_be_renamed() {
    let mut session = session();
    let (_comp, layers) = a_stack(&mut session);
    session
        .execute("fx.layer.rename", &json!({ "layer": layers[0], "name": "the pictures" }))
        .expect("the layer is renamed");
    let outcome = session
        .execute("fx.prop.get", &json!({ "layer": layers[0], "path": "transform/position" }))
        .expect("the layer is still there");
    assert!(outcome.data.is_object(), "and the layer answers");
    assert!(
        session.execute("fx.layer.rename", &json!({ "layer": layers[0], "name": "  " })).is_err(),
        "and an empty name is refused"
    );
}
