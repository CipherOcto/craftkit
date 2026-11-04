//! The **settings** of a layer.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_cad::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the cad battery.
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

/// A **drawing** with a **layer** and a **line** on it.
fn a_drawing(session: &mut Session) -> (u64, u64, u64) {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.layer.add", &json!({ "drawing": drawing, "name": "walls" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    let outcome = session
        .execute("cad.block.add", &json!({ "drawing": drawing, "name": "the model" }))
        .expect("the block is added");
    let block = outcome.data["id"].as_u64().expect("the block has an id");
    let outcome = session
        .execute(
            "cad.entity.line.add",
            &json!({
                "block": block,
                "name": "the line",
                "from": { "x": 0.0, "y": 0.0 },
                "to": { "x": 10.0, "y": 10.0 },
            }),
        )
        .expect("the line is added");
    let entity = outcome.data["id"].as_u64().expect("the line has an id");
    session
        .execute("cad.entity.layer", &json!({ "entity": entity, "layer": layer }))
        .expect("the line is put on the layer");
    (drawing, layer, entity)
}

/// The **layers** of a drawing, read back.
fn layers_of(session: &mut Session, drawing: u64) -> Json {
    let outcome =
        session.execute("cad.layer.list", &json!({ "drawing": drawing })).expect("the layers are read");
    outcome.data["layers"].clone()
}

/// The settings of a layer may be **set**.
#[test]
fn the_settings_of_a_layer_may_be_set() {
    let mut session = session();
    let (drawing, layer, _entity) = a_drawing(&mut session);
    session.execute("cad.layer.color", &json!({ "layer": layer, "color": "3" })).expect("the colour is set");
    session
        .execute("cad.layer.fill", &json!({ "layer": layer, "fill": "#336699" }))
        .expect("the fill is set");
    session
        .execute("cad.layer.linetype", &json!({ "layer": layer, "linetype": "Dashed" }))
        .expect("the line type is set");
    session
        .execute("cad.layer.lineweight", &json!({ "layer": layer, "mm": 0.25 }))
        .expect("the line weight is set");
    let layers = layers_of(&mut session, drawing);
    let layers = layers.as_array().expect("the layers are there");
    assert_eq!(layers[0]["color"], json!("3"), "the colour is the colour");
    assert_eq!(layers[0]["linetype"], json!("Dashed"), "the line type is the line type");
    assert_eq!(layers[0]["lineweight_mm"], json!(0.25), "the line weight is the weight");
    assert_eq!(layers[0]["on"], json!(true), "the layer is on");
    assert_eq!(layers[0]["frozen"], json!(false), "and it is not frozen");
    assert_eq!(layers[0]["locked"], json!(false), "and it is not locked");
}

/// The flags of a layer may be **set**.
#[test]
fn the_flags_of_a_layer_may_be_set() {
    let mut session = session();
    let (drawing, layer, _entity) = a_drawing(&mut session);
    session
        .execute("cad.layer.on", &json!({ "layer": layer, "on": false }))
        .expect("the layer is turned off");
    session.execute("cad.layer.freeze", &json!({ "layer": layer, "on": true })).expect("the layer is frozen");
    session.execute("cad.layer.lock", &json!({ "layer": layer, "on": true })).expect("the layer is locked");
    let layers = layers_of(&mut session, drawing);
    assert_eq!(layers[0]["on"], json!(false), "the layer is off");
    assert_eq!(layers[0]["frozen"], json!(true), "the layer is frozen");
    assert_eq!(layers[0]["locked"], json!(true), "the layer is locked");
}

/// A colour that is **no colour** is refused.
#[test]
fn a_colour_that_is_no_colour_is_refused() {
    let mut session = session();
    let (_drawing, layer, _entity) = a_drawing(&mut session);
    let error = session
        .execute("cad.layer.color", &json!({ "layer": layer, "color": "0" }))
        .expect_err("a colour index of nothing is refused");
    assert!(error.to_string().contains("index"), "and the error says why");
    let error = session
        .execute("cad.layer.color", &json!({ "layer": layer, "color": "ByLayer" }))
        .expect_err("the colour of another is refused");
    assert!(error.to_string().contains("ByLayer"), "and the error says which");
    let error = session
        .execute("cad.layer.fill", &json!({ "layer": layer, "fill": "blue" }))
        .expect_err("a colour that is not a colour is refused");
    assert!(!error.to_string().is_empty(), "and the error says what is wrong");
}

/// A layer that is **off** is not drawn.
#[test]
fn a_layer_that_is_off_is_not_drawn() {
    let mut session = session();
    let (drawing, layer, _entity) = a_drawing(&mut session);
    let extents = |session: &mut Session| {
        let plan = session.evaluate();
        let item = plan
            .items
            .iter()
            .find(|item| item.node == drawing && item.kind == "cad.extents")
            .expect("the drawing is in the plan");
        item.value.clone()
    };
    assert_eq!(extents(&mut session)["max"]["x"], json!(10.0), "the line is in the extents");
    session
        .execute("cad.layer.on", &json!({ "layer": layer, "on": false }))
        .expect("the layer is turned off");
    assert_eq!(
        extents(&mut session)["max"]["x"],
        Json::Null,
        "and a line on a layer that is off is not in the extents"
    );
    session.execute("cad.layer.on", &json!({ "layer": layer, "on": true })).expect("the layer is turned on");
    session.execute("cad.layer.freeze", &json!({ "layer": layer, "on": true })).expect("the layer is frozen");
    assert_eq!(extents(&mut session)["max"]["x"], Json::Null, "and a frozen layer is not drawn either");
}
