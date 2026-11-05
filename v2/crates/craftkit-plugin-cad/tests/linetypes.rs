//! The **line type** of a layer: the table of the patterns.

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

/// A **drawing** with a **layer** in it.
fn a_layer(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("cad.drawing.add", &json!({ "name": "the drawing" })).expect("the drawing is added");
    let drawing = outcome.data["id"].as_u64().expect("the drawing has an id");
    let outcome = session
        .execute("cad.layer.add", &json!({ "drawing": drawing, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    (drawing, layer)
}

/// The **line types** of this tree, as the answer holds them.
fn linetypes_of(session: &mut Session) -> Vec<Json> {
    let outcome = session.execute("cad.linetype.list", &json!({})).expect("the line types are read");
    outcome.data["linetypes"].as_array().cloned().expect("the line types are a list")
}

/// The **facts** of a layer, as the plan reports them.
fn facts_of(session: &mut Session, layer: u64) -> Json {
    let plan = session.evaluate();
    let entry = plan.items.iter().find(|entry| entry.node == layer).expect("the layer is in the plan");
    entry.value.clone()
}

/// **The line types of this tree are listed.**
#[test]
fn the_line_types_of_this_tree_are_listed() {
    let mut session = session();
    let linetypes = linetypes_of(&mut session);
    assert_eq!(linetypes.len(), 18, "there are eighteen line types");
    let dashed = linetypes
        .iter()
        .find(|linetype| linetype["name"] == json!("DASHED"))
        .expect("there is a dashed line type");
    assert_eq!(dashed["description"], json!("Dashed __ __ __ __"), "and it says what it is");
}

/// **A layer may name one of them.**
#[test]
fn a_layer_may_name_one_of_them() {
    let mut session = session();
    let (_drawing, layer) = a_layer(&mut session);
    let outcome = session
        .execute("cad.layer.linetype", &json!({ "layer": layer, "linetype": "DASHED" }))
        .expect("the layer is dashed");
    assert_eq!(outcome.data["linetype"], json!("DASHED"), "the line type is the tree's own");
}

/// The **pattern** of the line type is derived.
#[test]
fn the_pattern_of_the_line_type_is_derived() {
    let mut session = session();
    let (_drawing, layer) = a_layer(&mut session);
    session
        .execute("cad.layer.linetype", &json!({ "layer": layer, "linetype": "DASHED" }))
        .expect("the layer is dashed");
    let facts = facts_of(&mut session, layer);
    assert_eq!(facts["pattern"], json!([12.7, -6.35]), "twelve and seven tenths on, six and a third off");
}

/// The name of a line type is found **without regard to its case**.
#[test]
fn the_name_of_a_line_type_is_found_without_regard_to_its_case() {
    let mut session = session();
    let (_drawing, layer) = a_layer(&mut session);
    session
        .execute("cad.layer.linetype", &json!({ "layer": layer, "linetype": "dashed" }))
        .expect("the layer is dashed");
    let facts = facts_of(&mut session, layer);
    assert_eq!(facts["pattern"], json!([12.7, -6.35]), "and it is the same line type");
}

/// A name that is **not** a line type is refused.
#[test]
fn a_name_that_is_not_a_line_type_is_refused() {
    let mut session = session();
    let (_drawing, layer) = a_layer(&mut session);
    let refused = session
        .execute("cad.layer.linetype", &json!({ "layer": layer, "linetype": "wobbly" }))
        .expect_err("the line type is refused");
    assert!(refused.to_string().contains("DASHED"), "the refusal says what the line types are");
}

/// A layer with **no line type** has no pattern.
#[test]
fn a_layer_with_no_line_type_has_no_pattern() {
    let mut session = session();
    let (_drawing, layer) = a_layer(&mut session);
    let facts = facts_of(&mut session, layer);
    assert!(facts["linetype"].is_null(), "there is no line type");
    assert!(facts["pattern"].is_null(), "and so there is no pattern");
}
