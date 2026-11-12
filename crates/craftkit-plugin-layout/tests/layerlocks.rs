//! The **layers** of a layout are the kit's layers.
//!
//! A locked layer locks what is on it, and a hidden layer hides it:
//! the kit's own rules.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_layout::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the layout battery.
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

/// A **document** with a **spread**, a **layer** and an **item** on it.
fn a_document(session: &mut Session) -> (u64, u64, u64) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let outcome = session
        .execute("layout.layer.add", &json!({ "document": document, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    let outcome = session
        .execute("layout.item.add", &json!({ "spread": spread, "name": "the item" }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    session
        .execute("layout.item.layer", &json!({ "item": item, "layer": layer }))
        .expect("the item is on the layer");
    (document, layer, item)
}

/// A **locked layer** locks what is on it: the kit's own rule.
#[test]
fn a_locked_layer_locks_what_is_on_it() {
    let mut session = session();
    let (_document, layer, item) = a_document(&mut session);
    // The item may be edited while the layer is open.
    session.execute("layout.item.rotate", &json!({ "item": item, "degrees": 10 })).expect("the item turns");
    session.execute("layout.layer.lock", &json!({ "layer": layer })).expect("the layer is locked");
    let outcome = session
        .execute("layout.item.rotate", &json!({ "item": item, "degrees": 10 }))
        .expect_err("the item may not be edited");
    assert!(outcome.to_string().contains("locked layer"), "the refusal says why: {outcome}");
    session.execute("layout.layer.unlock", &json!({ "layer": layer })).expect("the layer is unlocked");
    session.execute("layout.item.rotate", &json!({ "item": item, "degrees": 10 })).expect("the item turns");
}

/// A **hidden layer** hides what is on it: the kit's own rule.
#[test]
fn a_hidden_layer_hides_what_is_on_it() {
    let mut session = session();
    let (_document, layer, item) = a_document(&mut session);
    assert!(
        session.document().is_visible(&session.document().node(item).expect("the item is there").clone())
    );
    session.execute("layout.layer.hide", &json!({ "layer": layer })).expect("the layer is hidden");
    let node = session.document().node(item).expect("the item is there");
    assert!(!session.document().is_visible(node), "the item is hidden with the layer");
    session.execute("layout.layer.show", &json!({ "layer": layer })).expect("the layer is shown");
    let node = session.document().node(item).expect("the item is there");
    assert!(session.document().is_visible(node), "and the item is shown again");
}

/// The edge that says an item is on a layer is the **kit's** edge, and
/// not one of this battery's.
#[test]
fn the_edge_of_a_layer_is_the_kits_edge() {
    let mut session = session();
    let (_document, layer, item) = a_document(&mut session);
    let held =
        session.document().relations.iter().find(|relation| relation.from == item).expect("there is an edge");
    assert_eq!(held.kind, craftkit_sdk::layer::ON_LAYER, "the edge is the kit's: {held:?}");
    assert_eq!(held.to, layer, "and it points at the layer");
}
