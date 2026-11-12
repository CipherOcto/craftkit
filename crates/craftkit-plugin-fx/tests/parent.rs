//! The **parent** of a layer.
//!
//! A layer may be under another layer, and the chain of parents may
//! not be a circle: a layer under a layer under itself would carry
//! itself forever.

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

/// A **composition** with two **layers** in it, and their ids.
fn two_layers(session: &mut Session) -> (u64, u64, u64) {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the child" }))
        .expect("the child is added");
    let child = outcome.data["id"].as_u64().expect("the child has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the parent" }))
        .expect("the parent is added");
    let parent = outcome.data["id"].as_u64().expect("the parent has an id");
    (comp, child, parent)
}

/// The **parent** of a layer, as the document holds it.
fn parent_of(session: &Session, layer: u64) -> Option<u64> {
    let node = session.document().node(layer).expect("the layer is there");
    node.data.get("parent").and_then(|parent| parent.as_u64())
}

/// A layer may be put **under** another layer.
#[test]
fn a_layer_may_be_put_under_another() {
    let mut session = session();
    let (_, child, parent) = two_layers(&mut session);
    session
        .execute("fx.layer.parent", &json!({ "layer": child, "parent": parent }))
        .expect("the child is under the parent");
    assert_eq!(parent_of(&session, child), Some(parent), "the child is under the parent");
}

/// The parent may be taken **away**.
#[test]
fn the_parent_may_be_taken_away() {
    let mut session = session();
    let (_, child, parent) = two_layers(&mut session);
    session
        .execute("fx.layer.parent", &json!({ "layer": child, "parent": parent }))
        .expect("the child is under the parent");
    session
        .execute("fx.layer.parent.delete", &json!({ "layer": child }))
        .expect("the child is under no layer");
    assert_eq!(parent_of(&session, child), None, "the child is under no layer");
}

/// A layer may **not be its own** parent.
#[test]
fn a_layer_may_not_be_its_own_parent() {
    let mut session = session();
    let (_, child, _) = two_layers(&mut session);
    let error = session
        .execute("fx.layer.parent", &json!({ "layer": child, "parent": child }))
        .expect_err("a layer cannot be its own parent");
    assert!(error.to_string().contains("circle"), "the error says it would make a circle: {error}");
    assert_eq!(parent_of(&session, child), None, "and the layer is unchanged");
}

/// A parent that would **close a circle** is refused: a layer under a
/// layer under it.
#[test]
fn a_parent_that_would_close_a_circle_is_refused() {
    let mut session = session();
    let (_, child, parent) = two_layers(&mut session);
    session
        .execute("fx.layer.parent", &json!({ "layer": child, "parent": parent }))
        .expect("the child is under the parent");
    let error = session
        .execute("fx.layer.parent", &json!({ "layer": parent, "parent": child }))
        .expect_err("that would make a circle");
    assert!(error.to_string().contains("circle"), "the error says it would make a circle: {error}");
    assert_eq!(parent_of(&session, parent), None, "and the parent is unchanged");
}

/// A **deeper** circle is refused too: three layers in a chain.
#[test]
fn a_deeper_circle_is_refused() {
    let mut session = session();
    let (comp, child, parent) = two_layers(&mut session);
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the grandparent" }))
        .expect("the grandparent is added");
    let grandparent = outcome.data["id"].as_u64().expect("the grandparent has an id");
    session
        .execute("fx.layer.parent", &json!({ "layer": parent, "parent": grandparent }))
        .expect("the parent is under the grandparent");
    // The chain runs child, parent, grandparent: putting the
    // grandparent under the child would close it.
    session
        .execute("fx.layer.parent", &json!({ "layer": child, "parent": parent }))
        .expect("the child is under the parent");
    let error = session
        .execute("fx.layer.parent", &json!({ "layer": grandparent, "parent": child }))
        .expect_err("that would make a circle");
    assert!(error.to_string().contains("circle"), "the error says it would make a circle: {error}");
    assert_eq!(parent_of(&session, grandparent), None, "and the grandparent is unchanged");
}

/// A parent that is **not a layer** is refused.
#[test]
fn a_parent_that_is_not_a_layer_is_refused() {
    let mut session = session();
    let (comp, child, _) = two_layers(&mut session);
    let error = session
        .execute("fx.layer.parent", &json!({ "layer": child, "parent": comp }))
        .expect_err("a composition is not a layer");
    assert!(error.to_string().contains("not a layer"), "the error says the node is not a layer: {error}");
}

/// A parent that is **not there** is refused.
#[test]
fn a_parent_that_is_not_there_is_refused() {
    let mut session = session();
    let (_, child, parent) = two_layers(&mut session);
    let error = session
        .execute("fx.layer.parent", &json!({ "layer": child, "parent": parent + 1000 }))
        .expect_err("there is no such layer");
    assert!(error.to_string().contains("no layer"), "the error says there is no layer: {error}");
}

/// The parent is **saved** with the layer.
#[test]
fn the_parent_is_saved_with_the_layer() {
    let mut session = session();
    let (_, child, parent) = two_layers(&mut session);
    session
        .execute("fx.layer.parent", &json!({ "layer": child, "parent": parent }))
        .expect("the child is under the parent");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(child).expect("the layer is there");
    assert_eq!(node.data["parent"].as_u64(), Some(parent), "the child is still under the parent");
}
