//! The fx battery: the document model of a **compositing application**.
//!
//! A composition is a stack of layers, and every layer has a tree of
//! properties, addressed by **name and order** rather than by id.

use craftkit_sdk::Ecosystem;
// The tests that use it are the ones for the battery, and the battery is
// optional: the import is with them.
#[cfg(feature = "fx")]
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A composition with one layer in it.
#[cfg(feature = "fx")]
fn a_comp() -> (craftkit_sdk::Session, u64) {
    let mut session = ecosystem().session();
    session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 250 }))
        .expect("the comp is added");
    session.execute("fx.layer.add", &json!({ "comp": 1, "name": "title" })).expect("the layer is added");
    (session, 2)
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "fx")]
#[test]
fn the_fx_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("fx.comp.add").is_some(), "the battery is in");
    assert!(ecosystem.commands.get("fx.prop.set").is_some());
    assert!(ecosystem.kinds.get("fx.comp").is_some());
    assert!(ecosystem.kinds.get("fx.layer").is_some());
    assert!(ecosystem.evaluators.get("fx.props").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "fx"))]
#[test]
fn the_fx_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("fx.comp.add").is_none(), "the battery is out");
}

/// A property is set and read by its **path**.
#[cfg(feature = "fx")]
#[test]
fn a_property_is_set_and_read_by_its_path() {
    let (mut session, layer) = a_comp();
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "transform/position", "value": [10, 20] }))
        .expect("the property is set");
    let outcome = session
        .execute("fx.prop.get", &json!({ "layer": layer, "path": "transform/position" }))
        .expect("the property is read");
    assert_eq!(outcome.data["value"], json!([10, 20]));
}

/// A path with a **number** in it picks the nth child.
///
/// The effects of a layer are made by `fx.effect.add` and not by a
/// property: an effect is a thing with a name and its parameters, and
/// a property is a value.
#[cfg(feature = "fx")]
#[test]
fn a_path_with_a_number_in_it_picks_the_nth_child() {
    let (mut session, layer) = a_comp();
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the first effect is added");
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "glow" }))
        .expect("the second effect is added");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects/#2/intensity", "value": 9 }))
        .expect("the property is set");
    let outcome = session
        .execute("fx.prop.get", &json!({ "layer": layer, "path": "effects/#2/intensity" }))
        .expect("the property is read");
    assert_eq!(outcome.data["value"], json!(9), "the second effect");
}

/// A path that names a child that is not there is refused, and says
/// which.
///
/// The setter makes the groups it needs — setting a property on a layer
/// that has no `transform` makes one — but it does not invent a *list*,
/// and the list of effects is made by `fx.effect.add` and not by a
/// property.
#[cfg(feature = "fx")]
#[test]
fn a_path_that_names_nothing_is_refused() {
    let (mut session, layer) = a_comp();
    session
        .execute("fx.effect.add", &json!({ "layer": layer, "effect": "blur" }))
        .expect("the effect is added");
    let error = session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "effects/#5/radius", "value": 1 }))
        .expect_err("there is no fifth effect");
    assert!(error.to_string().contains("number 5"), "{error}");
}

/// The property tree is in the **payload**, not in the tree.
///
/// A layer has fifty properties and a comp has thirty layers: a thousand
/// nodes for one picture, and the node graph would be unreadable.
#[cfg(feature = "fx")]
#[test]
fn the_property_tree_is_in_the_payload() {
    let (mut session, layer) = a_comp();
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "opacity", "value": 50 }))
        .expect("the property is set");
    let node = session.document().node(layer).expect("the layer is there");
    assert_eq!(node.data["props"]["opacity"], json!(50), "the property is in the payload");
    assert!(node.children.is_empty(), "the layer holds no nodes");
}

/// The properties are counted, and the animated ones.
#[cfg(feature = "fx")]
#[test]
fn the_properties_are_counted() {
    let (mut session, layer) = a_comp();
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "opacity", "value": 50 }))
        .expect("the property is set");
    session
        .execute(
            "fx.prop.set",
            &json!({ "layer": layer, "path": "position", "value": { "value": [0, 0], "keys": [] } }),
        )
        .expect("the animated property is set");
    let plan = session.evaluate();
    let props: Vec<_> = plan.items.iter().filter(|item| item.kind == "fx.props").collect();
    assert_eq!(props.len(), 1, "one layer: {plan:?}");
    assert_eq!(props[0].value["properties"], json!(2), "opacity and position");
    assert_eq!(props[0].value["animated"], json!(1), "the one with keyframes on it");
}
