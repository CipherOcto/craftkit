//! The **expression** of a property.
//!
//! An expression says what the value of a property is at every frame.
//! The tree's expressions are JavaScript on a JavaScript engine; here
//! an expression is a small arithmetic language of this tree's own,
//! with the tree's names where they mean the same thing.

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

/// A **layer** with one property on it, and the layer's id.
fn a_property(session: &mut Session, path: &str) -> u64 {
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": path, "value": { "keys": [] } }))
        .expect("the property is animated");
    layer
}

/// A property with one **keyframe**, and the layer's id.
fn a_keyed_property(session: &mut Session, path: &str, frame: u64, value: f64) -> u64 {
    let layer = a_property(session, path);
    session
        .execute("fx.keyframe.set", &json!({ "layer": layer, "path": path, "frame": frame, "value": value }))
        .expect("the keyframe is set");
    layer
}

/// The **value** of a property at a frame.
fn value_at(session: &mut Session, layer: u64, path: &str, frame: u64) -> f64 {
    let outcome = session
        .execute("fx.prop.at", &json!({ "layer": layer, "path": path, "frame": frame }))
        .expect("the value is read");
    outcome.data["value"].as_f64().expect("the value is a number")
}

/// An expression says what the value is.
#[test]
fn an_expression_says_what_the_value_is() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 5.0);
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "value * 2" }),
        )
        .expect("the expression is put");
    assert_eq!(value_at(&mut session, layer, "opacity", 0), 10.0, "the value is twice itself");
}

/// The name `value` is the property's **own** value.
#[test]
fn value_is_the_property_own_value() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 5.0);
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "value + 1" }),
        )
        .expect("the expression is put");
    assert_eq!(value_at(&mut session, layer, "opacity", 0), 6.0, "the value is one more");
}

/// The name `time` is the **frame**.
#[test]
fn time_is_the_frame() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    session
        .execute("fx.prop.expression", &json!({ "layer": layer, "path": "opacity", "expression": "time" }))
        .expect("the expression is put");
    assert_eq!(value_at(&mut session, layer, "opacity", 7), 7.0, "the time is the frame");
}

/// The operators: the multiplication is done before the addition.
#[test]
fn the_operators_have_their_order() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "1 + 2 * 3" }),
        )
        .expect("the expression is put");
    assert_eq!(value_at(&mut session, layer, "opacity", 0), 7.0, "the multiplication comes first");
}

/// The power is to the right, and it is right-associative.
#[test]
fn the_power_is_to_the_right() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "2 ^ 3 ^ 2" }),
        )
        .expect("the expression is put");
    assert_eq!(value_at(&mut session, layer, "opacity", 0), 512.0, "two to the three to the two");
}

/// The **linear** interpolation is the tree's: the time from one to
/// the other, and the values.
#[test]
fn linear_is_the_trees_linear() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "linear(time, 0, 10, 0, 100)" }),
        )
        .expect("the expression is put");
    assert_eq!(value_at(&mut session, layer, "opacity", 5), 50.0, "halfway is half");
}

/// The **ease** is the tree's smoothstep.
#[test]
fn ease_is_the_trees_smoothstep() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "ease(0.25, 0, 1)" }),
        )
        .expect("the expression is put");
    assert!((value_at(&mut session, layer, "opacity", 0) - 0.15625).abs() < 1e-9, "the curve is the tree's");
}

/// The **eases** in and out are the tree's too.
#[test]
fn the_eases_in_and_out_are_the_trees() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "easeIn(0.5, 0, 1)" }),
        )
        .expect("the expression is put");
    assert!((value_at(&mut session, layer, "opacity", 0) - 0.375).abs() < 1e-9, "ease in is the tree's");
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "easeOut(0.5, 0, 1)" }),
        )
        .expect("the expression is put");
    assert!((value_at(&mut session, layer, "opacity", 0) - 0.625).abs() < 1e-9, "and ease out is the tree's");
}

/// The **functions**: the tree's, and they mean here what they mean
/// there.
#[test]
fn the_functions_are_the_trees() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    for (expression, expected) in [
        ("clamp(5, 0, 1)", 1.0),
        ("min(3, 4)", 3.0),
        ("max(3, 4)", 4.0),
        ("abs(-3)", 3.0),
        ("round(2.5)", 3.0),
        ("floor(2.7)", 2.0),
        ("ceil(2.1)", 3.0),
        ("sqrt(9)", 3.0),
        ("sin(0)", 0.0),
        ("cos(0)", 1.0),
    ] {
        session
            .execute(
                "fx.prop.expression",
                &json!({ "layer": layer, "path": "opacity", "expression": expression }),
            )
            .expect("the expression is put");
        let value = value_at(&mut session, layer, "opacity", 0);
        assert!((value - expected).abs() < 1e-9, "{expression} is {expected} and not {value}");
    }
}

/// An expression that does **not parse** is refused.
#[test]
fn an_expression_that_does_not_parse_is_refused() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    let refused = session
        .execute("fx.prop.expression", &json!({ "layer": layer, "path": "opacity", "expression": "1 +" }));
    assert!(refused.is_err(), "the expression ends too early");
}

/// A function this tree does **not** have is refused: the tree's
/// JavaScript is not here.
#[test]
fn a_function_that_is_not_here_is_refused() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    let refused = session.execute(
        "fx.prop.expression",
        &json!({ "layer": layer, "path": "opacity", "expression": "wiggle(1, 2)" }),
    );
    assert!(refused.is_err(), "there is no wiggle here");
}

/// A function with the **wrong number of arguments** is refused.
#[test]
fn a_function_with_the_wrong_arguments_is_refused() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 0.0);
    let refused = session.execute(
        "fx.prop.expression",
        &json!({ "layer": layer, "path": "opacity", "expression": "abs(1, 2)" }),
    );
    assert!(refused.is_err(), "abs takes one argument");
}

/// An expression on a property that is **not animated** is refused.
#[test]
fn an_expression_on_a_property_that_is_not_animated_is_refused() {
    let mut session = session();
    let outcome = session
        .execute("fx.comp.add", &json!({ "width": 1920, "height": 1080, "duration": 100 }))
        .expect("the comp is added");
    let comp = outcome.data["id"].as_u64().expect("the comp has an id");
    let outcome = session
        .execute("fx.layer.add", &json!({ "comp": comp, "name": "the layer" }))
        .expect("the layer is added");
    let layer = outcome.data["id"].as_u64().expect("the layer has an id");
    session
        .execute("fx.prop.set", &json!({ "layer": layer, "path": "opacity", "value": 0.5 }))
        .expect("the property is set");
    let refused = session
        .execute("fx.prop.expression", &json!({ "layer": layer, "path": "opacity", "expression": "value" }));
    assert!(refused.is_err(), "an expression needs keyframes");
}

/// The expression is **taken away**, and the keyframes say the value
/// again.
#[test]
fn the_expression_is_taken_away() {
    let mut session = session();
    let layer = a_keyed_property(&mut session, "opacity", 0, 5.0);
    session
        .execute(
            "fx.prop.expression",
            &json!({ "layer": layer, "path": "opacity", "expression": "value * 2" }),
        )
        .expect("the expression is put");
    assert_eq!(value_at(&mut session, layer, "opacity", 0), 10.0, "the expression says the value");
    session
        .execute("fx.prop.expression.delete", &json!({ "layer": layer, "path": "opacity" }))
        .expect("the expression is taken away");
    assert_eq!(value_at(&mut session, layer, "opacity", 0), 5.0, "and the keyframe says it again");
}
