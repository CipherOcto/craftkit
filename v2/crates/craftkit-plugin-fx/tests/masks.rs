//! The **masks** of a layer.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **composition** with a **layer** in it.
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

/// The **masks** of a layer, read back.
fn masks_of(session: &mut Session, layer: u64) -> Json {
    let outcome = session.execute("fx.mask.list", &json!({ "layer": layer })).expect("the masks are read");
    outcome.data["masks"].clone()
}

/// A **mask** may be made on a layer.
#[test]
fn a_mask_may_be_made_on_a_layer() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let outcome = session
        .execute(
            "fx.mask.add",
            &json!({
                "layer": layer,
                "vertices": [[0.0, 0.0], [100.0, 0.0], [100.0, 100.0]],
                "closed": true,
            }),
        )
        .expect("the mask is made");
    let mask = outcome.data["id"].as_u64().expect("the mask has an id");
    let masks = masks_of(&mut session, layer);
    let masks = masks.as_array().expect("the masks are there");
    assert_eq!(masks.len(), 1, "the layer has one mask");
    assert_eq!(masks[0]["id"], json!(mask), "and it is the mask that was made");
    assert_eq!(masks[0]["closed"], json!(true), "and it is closed");
    assert_eq!(masks[0]["mode"], json!("Add"), "and its mode is the tree's default");
    assert_eq!(masks[0]["inverted"], json!(false), "and it is not inverted");
}

/// A mask with no vertices is **refused**.
#[test]
fn a_mask_with_no_vertices_is_refused() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let error = session
        .execute("fx.mask.add", &json!({ "layer": layer, "vertices": [] }))
        .expect_err("a mask of no vertices is refused");
    assert!(error.to_string().contains("vertices"), "and the error says why");
    let error = session
        .execute("fx.mask.add", &json!({ "layer": layer, "vertices": [["a", "b"]] }))
        .expect_err("a point that is not a point is refused");
    assert!(error.to_string().contains("numbers"), "and the error says why too");
}

/// The **mode** of a mask may be set.
#[test]
fn the_mode_of_a_mask_may_be_set() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let outcome = session
        .execute("fx.mask.add", &json!({ "layer": layer, "vertices": [[0.0, 0.0], [10.0, 10.0]] }))
        .expect("the mask is made");
    let mask = outcome.data["id"].as_u64().expect("the mask has an id");
    session
        .execute("fx.mask.mode", &json!({ "layer": layer, "mask": mask, "mode": "subtract" }))
        .expect("the mode is set");
    let masks = masks_of(&mut session, layer);
    assert_eq!(masks[0]["mode"], json!("Subtract"), "and the mode is the tree's own");
    let error = session
        .execute("fx.mask.mode", &json!({ "layer": layer, "mask": mask, "mode": "Puce" }))
        .expect_err("a mode that is not a mode is refused");
    assert!(error.to_string().contains("Puce"), "and the error says which");
}

/// A mask may be **inverted** and **locked**.
#[test]
fn a_mask_may_be_inverted_and_locked() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let outcome = session
        .execute("fx.mask.add", &json!({ "layer": layer, "vertices": [[0.0, 0.0], [10.0, 10.0]] }))
        .expect("the mask is made");
    let mask = outcome.data["id"].as_u64().expect("the mask has an id");
    session
        .execute("fx.mask.invert", &json!({ "layer": layer, "mask": mask }))
        .expect("the mask is inverted");
    session.execute("fx.mask.lock", &json!({ "layer": layer, "mask": mask })).expect("the mask is locked");
    let masks = masks_of(&mut session, layer);
    assert_eq!(masks[0]["inverted"], json!(true), "the mask is inverted");
    assert_eq!(masks[0]["locked"], json!(true), "and it is locked");
    session
        .execute("fx.mask.invert", &json!({ "layer": layer, "mask": mask }))
        .expect("the mask is inverted again");
    let masks = masks_of(&mut session, layer);
    assert_eq!(masks[0]["inverted"], json!(false), "and a toggle is a toggle");
    session
        .execute("fx.mask.invert", &json!({ "layer": layer, "mask": mask, "value": true }))
        .expect("the mask is inverted again");
    let masks = masks_of(&mut session, layer);
    assert_eq!(masks[0]["inverted"], json!(true), "and a value says what to do");
}

/// A mask may be **taken away**.
#[test]
fn a_mask_may_be_taken_away() {
    let mut session = session();
    let (_comp, layer) = a_layer(&mut session);
    let outcome = session
        .execute("fx.mask.add", &json!({ "layer": layer, "vertices": [[0.0, 0.0], [10.0, 10.0]] }))
        .expect("the mask is made");
    let mask = outcome.data["id"].as_u64().expect("the mask has an id");
    session
        .execute("fx.mask.remove", &json!({ "layer": layer, "mask": mask }))
        .expect("the mask is taken away");
    let masks = masks_of(&mut session, layer);
    assert_eq!(masks.as_array().expect("the masks are there").len(), 0, "and the layer has none");
}
