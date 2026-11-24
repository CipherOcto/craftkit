//! The **kinds** of the fx battery: what a marker and a mask mean.
//!
//! A kind is a rule the kit publishes as much as a command is: it says
//! what a node of that kind is worth. A kind that no test names is a
//! kind that nothing proves means what it says.

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

/// A **composition** with a **layer** in it, and their ids.
fn a_layer(session: &mut Session) -> (u64, u64) {
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

/// What the plan says about a node.
fn said_of(session: &Session, node: u64) -> craftkit_sdk::serde_json::Value {
    let plan = session.evaluate();
    let item = plan.items.iter().find(|item| item.node == node).expect("the node is in the plan");
    item.value.clone()
}

/// A **marker** is a kind, and it means where it is and what it says.
#[test]
fn a_marker_means_where_it_is_and_what_it_says() {
    let mut session = session();
    let (comp, _) = a_layer(&mut session);
    let outcome = session
        .execute("fx.marker.add", &json!({ "comp": comp, "frame": 12, "comment": "the beat" }))
        .expect("the marker is added");
    let marker = outcome.data["id"].as_u64().expect("the marker has an id");
    let node = session.document().node(marker).expect("the marker is there");
    assert_eq!(node.kind, "fx.marker", "the node is a marker");
    let said = said_of(&session, marker);
    assert_eq!(said["frame"], json!(12), "the marker says where it is");
    assert_eq!(said["comment"], json!("the beat"), "and what it says");
}

/// A **mask** is a kind, and it means its path and its settings.
#[test]
fn a_mask_means_its_path_and_its_settings() {
    let mut session = session();
    let (_, layer) = a_layer(&mut session);
    let outcome = session
        .execute(
            "fx.mask.add",
            &json!({ "layer": layer, "vertices": [[0.0, 0.0], [10.0, 0.0], [10.0, 10.0]], "closed": true }),
        )
        .expect("the mask is added");
    let mask = outcome.data["id"].as_u64().expect("the mask has an id");
    let node = session.document().node(mask).expect("the mask is there");
    assert_eq!(node.kind, "fx.mask", "the node is a mask");
    let said = said_of(&session, mask);
    assert_eq!(said["vertices"].as_array().map(|points| points.len()), Some(3), "the mask says its path");
    assert_eq!(said["closed"], json!(true), "and whether the path is closed");
    assert_eq!(said["mode"], json!("Add"), "and the mode it works in");
}
