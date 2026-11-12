//! The **layer** is the kit's own.
//!
//! A layer is a node, and the things on it are said to be on it by an
//! **edge** — the kit's edge, `core.on_layer`. What follows from that
//! is the kit's rule and not a battery's: a node with no appearance of
//! its own takes the layer's, and a hidden layer hides what is on it.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A session with a **layer** in it.
fn a_layer() -> (Session, u64) {
    let mut session = ecosystem().session();
    let layer = session
        .execute("core.layer.add", &json!({ "name": "Walls" }))
        .expect("the layer is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    (session, layer)
}

/// The membership is the **kit's edge**.
///
/// Every battery that has layers uses it, so that the rule that
/// follows from it is one rule and not one per battery.
#[test]
fn the_membership_is_the_kits_edge() {
    let (mut session, layer) = a_layer();
    let shape = session
        .execute("node.add", &json!({ "name": "a shape", "kind": "geometry.rect" }))
        .expect("the node is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    session.execute("core.layer.on", &json!({ "node": shape, "layer": layer })).expect("it is on the layer");

    let relations = &session.document().relations;
    let edge = relations
        .iter()
        .find(|relation| relation.kind == craftkit_sdk::layer::ON_LAYER)
        .expect("there is an edge");
    assert_eq!(edge.from, shape, "the edge is from the node");
    assert_eq!(edge.to, layer, "and to the layer");
}

/// A node is on **one** layer.
///
/// The edge says which, and a second edge would be two answers to one
/// question.
#[test]
fn a_node_is_on_one_layer() {
    let (mut session, layer) = a_layer();
    let shape = session
        .execute("node.add", &json!({ "name": "a shape", "kind": "geometry.rect" }))
        .expect("the node is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    session.execute("core.layer.on", &json!({ "node": shape, "layer": layer })).expect("it is on the layer");

    let refused = session
        .execute("core.layer.on", &json!({ "node": shape, "layer": layer }))
        .expect_err("the second edge is refused");
    assert!(refused.to_string().contains("already"), "the error says why: {refused}");

    session.execute("core.layer.off", &json!({ "node": shape })).expect("it is off the layer");
    let edges =
        session.document().relations.iter().filter(|relation| relation.kind == craftkit_sdk::layer::ON_LAYER);
    assert_eq!(edges.count(), 0, "the edge is gone");
}

/// A **hidden layer** hides what is on it.
///
/// Turning a layer off is what a layer is for, and a layer that is
/// turned off takes its members with it.
#[test]
fn a_hidden_layer_hides_what_is_on_it() {
    let (mut session, layer) = a_layer();
    let shape = session
        .execute("node.add", &json!({ "name": "a shape", "kind": "geometry.rect" }))
        .expect("the node is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    session.execute("core.layer.on", &json!({ "node": shape, "layer": layer })).expect("it is on the layer");
    assert_eq!(session.document().walk_visible().count(), 2, "the layer and the node are shown");

    session.execute("node.hide", &json!({ "node": layer })).expect("the layer is hidden");
    assert_eq!(session.document().walk_visible().count(), 0, "what is on it is hidden too");
}

/// A node on a layer takes the **layer's appearance**.
///
/// The appearance is the kit's own and the layer is the kit's own, so
/// the rule that joins them is the kit's own.
#[cfg(feature = "vector")]
#[test]
fn a_node_on_a_layer_takes_the_layers_appearance() {
    let (mut session, layer) = a_layer();
    session
        .execute("core.appearance.set", &json!({ "node": layer, "fill": "#3366cc" }))
        .expect("the layer has an appearance");
    let shape = session
        .execute(
            "vector.rectangle",
            &json!({ "x": -5.0, "y": -5.0, "width": 10.0, "height": 10.0, "fill": "#ffffff" }),
        )
        .expect("the rectangle is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    session.execute("core.layer.on", &json!({ "node": shape, "layer": layer })).expect("it is on the layer");

    let picture = draw(&session, 64);
    assert!(picture.contains(&[0x33, 0x66, 0xcc, 0xff]), "the rectangle is drawn in the layer's colour");
}

/// And a node with an appearance **of its own** keeps it.
///
/// The layer is the fallback, not the master.
#[cfg(feature = "vector")]
#[test]
fn a_node_with_an_appearance_of_its_own_keeps_it() {
    let (mut session, layer) = a_layer();
    session
        .execute("core.appearance.set", &json!({ "node": layer, "fill": "#3366cc" }))
        .expect("the layer has an appearance");
    let shape = session
        .execute(
            "vector.rectangle",
            &json!({ "x": -5.0, "y": -5.0, "width": 10.0, "height": 10.0, "fill": "#ffffff" }),
        )
        .expect("the rectangle is added")
        .data
        .get("id")
        .and_then(|id| id.as_u64())
        .expect("there is an id");
    session
        .execute("core.appearance.set", &json!({ "node": shape, "fill": "#cc3366" }))
        .expect("the rectangle has an appearance");
    session.execute("core.layer.on", &json!({ "node": shape, "layer": layer })).expect("it is on the layer");

    let picture = draw(&session, 64);
    assert!(picture.contains(&[0xcc, 0x33, 0x66, 0xff]), "the rectangle is drawn in its own colour");
}

/// The picture of a document, as the pixels it has.
#[cfg(feature = "vector")]
fn draw(session: &Session, size: usize) -> Vec<[u8; 4]> {
    let camera = *session.camera();
    let mut raster = craftkit_raster::Raster::new(size, size, camera).expect("the picture is made");
    let document = session.document();
    for node in document.walk_visible() {
        let Some(kind) = session.kinds().get(&node.kind) else { continue };
        (kind.paint)(node, document, &mut raster);
    }
    let image = raster.image();
    let mut pixels = Vec::new();
    for y in 0..image.height() {
        for x in 0..image.width() {
            pixels.push(image.pixel(x, y).unwrap_or([0, 0, 0, 0]));
        }
    }
    pixels
}
