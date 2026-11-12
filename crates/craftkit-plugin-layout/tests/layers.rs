//! The **layers** of a layout document.
//!
//! The tree's own rules: the items of a layer go with it, one layer
//! always stays, and the layers may be merged and moved.

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

/// A **document** with a **spread** and **two layers**, and their ids.
fn a_document(session: &mut Session) -> (u64, u64, Vec<u64>) {
    let outcome = session
        .execute("layout.document.add", &json!({ "name": "the document" }))
        .expect("the document is added");
    let document = outcome.data["id"].as_u64().expect("the document has an id");
    let outcome = session
        .execute("layout.spread.add", &json!({ "document": document, "name": "the spread" }))
        .expect("the spread is added");
    let spread = outcome.data["id"].as_u64().expect("the spread has an id");
    let mut layers = Vec::new();
    for name in ["the first layer", "the second layer"] {
        let outcome = session
            .execute("layout.layer.add", &json!({ "document": document, "name": name }))
            .expect("the layer is added");
        layers.push(outcome.data["id"].as_u64().expect("the layer has an id"));
    }
    (document, spread, layers)
}

/// An **item** on a **layer**.
fn an_item(session: &mut Session, spread: u64, layer: u64, name: &str) -> u64 {
    let outcome = session
        .execute("layout.item.add", &json!({ "spread": spread, "name": name }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    session
        .execute("layout.item.layer", &json!({ "item": item, "layer": layer }))
        .expect("the item is on the layer");
    item
}

/// The **layers** of a document, in the order they stand in.
fn layers_of(session: &Session, document: u64) -> Vec<u64> {
    let node = session.document().node(document).expect("the document is there");
    node.children.iter().filter(|child| child.kind == "layout.layer").map(|child| child.id).collect()
}

/// The **items** on a layer.
fn items_on(session: &Session, layer: u64) -> Vec<u64> {
    session
        .document()
        .relations
        .iter()
        .filter(|relation| relation.kind == craftkit_sdk::layer::ON_LAYER && relation.to == layer)
        .map(|relation| relation.from)
        .collect()
}

/// A layer is **renamed**, and the name is not empty and is not one
/// that is already there.
#[test]
fn a_layer_is_renamed() {
    let mut session = session();
    let (document, _spread, layers) = a_document(&mut session);
    session
        .execute("layout.layer.rename", &json!({ "layer": layers[0], "name": "the art layer" }))
        .expect("the layer is renamed");
    let node = session.document().node(layers[0]).expect("the layer is there");
    assert_eq!(node.name, "the art layer", "the layer has the new name");
    let outcome = session
        .execute("layout.layer.rename", &json!({ "layer": layers[0], "name": "  " }))
        .expect_err("an empty name is refused");
    assert!(outcome.to_string().contains("a layer has a name"), "the refusal says why: {outcome}");
    let outcome = session
        .execute("layout.layer.rename", &json!({ "layer": layers[0], "name": "the second layer" }))
        .expect_err("a name that is there is refused");
    assert!(outcome.to_string().contains("is already there"), "the refusal says why: {outcome}");
    let _ = document;
}

/// The items of a layer go with it when it is **deleted**, and a
/// document needs at least one layer.
#[test]
fn the_items_of_a_layer_go_with_it() {
    let mut session = session();
    let (_document, spread, layers) = a_document(&mut session);
    let item = an_item(&mut session, spread, layers[0], "the item");
    session.execute("layout.layer.delete", &json!({ "layer": layers[0] })).expect("the layer is deleted");
    assert_eq!(items_on(&session, layers[0]), Vec::<u64>::new(), "the layer has no items");
    assert!(session.document().node(item).is_none(), "and the item is gone");
    let outcome = session
        .execute("layout.layer.delete", &json!({ "layer": layers[1] }))
        .expect_err("the last layer may not be deleted");
    assert!(outcome.to_string().contains("at least one layer"), "the refusal says why: {outcome}");
}

/// The layers with **nothing on them** are deleted, and one layer
/// always stays.
#[test]
fn the_unused_layers_are_deleted() {
    let mut session = session();
    let (document, spread, layers) = a_document(&mut session);
    an_item(&mut session, spread, layers[0], "the item");
    let outcome = session
        .execute("layout.layer.unused.delete", &json!({ "document": document }))
        .expect("the unused layers are deleted");
    assert_eq!(outcome.data["count"], json!(1), "one layer is gone: {outcome:?}");
    assert_eq!(layers_of(&session, document), vec![layers[0]], "and the used one stays");
}

/// A layer is **moved** in the order of the layers: the first is the
/// frontmost.
#[test]
fn a_layer_is_moved_in_the_order() {
    let mut session = session();
    let (document, _spread, layers) = a_document(&mut session);
    // The layers are read in the order of the children, and the
    // frontmost is the last of them.
    session
        .execute("layout.layer.move", &json!({ "layer": layers[0], "to": 0 }))
        .expect("the layer is moved");
    assert_eq!(layers_of(&session, document), vec![layers[1], layers[0]], "the layer is at the front");
    let outcome = session
        .execute("layout.layer.move", &json!({ "layer": layers[0], "to": 7 }))
        .expect_err("a place that is not there is refused");
    assert!(outcome.to_string().contains("the place is a number"), "the refusal says why: {outcome}");
}

/// The layers are **merged**: the items move to one, and the others
/// are gone.
#[test]
fn the_layers_are_merged() {
    let mut session = session();
    let (document, spread, layers) = a_document(&mut session);
    let first = an_item(&mut session, spread, layers[0], "the first item");
    let second = an_item(&mut session, spread, layers[1], "the second item");
    session
        .execute("layout.layer.merge", &json!({ "layers": [layers[0], layers[1]], "into": layers[0] }))
        .expect("the layers are merged");
    assert_eq!(items_on(&session, layers[0]), vec![first, second], "the items are on the layer that stays");
    assert_eq!(layers_of(&session, document), vec![layers[0]], "and the other layer is gone");
}
