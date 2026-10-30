//! The typed readers.
//!
//! A parameter is read by a **type**, and these are the types.

use craftkit_sdk::{Bool, Either, NodeRef, Number, ParamKind, Params, Text};
use serde_json::json;

fn params(value: &serde_json::Value) -> Params<'_> {
    Params::new(value)
}

#[test]
fn a_number_is_read() {
    let value = json!({ "distance": 12.5 });
    let distance = Number::read(&params(&value), "distance").expect("the number is read");
    assert_eq!(distance.0, 12.5);
}

#[test]
fn a_number_that_is_not_a_number_is_refused() {
    let value = json!({ "distance": "far" });
    assert!(Number::read(&params(&value), "distance").is_err());
}

#[test]
fn a_value_that_is_not_a_number_at_all_is_refused() {
    let value = json!({ "distance": null });
    assert!(Number::read(&params(&value), "distance").is_err());
}

#[test]
fn a_parameter_that_is_not_there_is_refused() {
    let value = json!({});
    let error = Text::read(&params(&value), "name").expect_err("the parameter is missing");
    assert!(error.message.contains("required"), "{error:?}");
}

#[test]
fn an_optional_parameter_that_is_not_there_is_none() {
    let value = json!({});
    let name = Option::<Text>::read(&params(&value), "name").expect("the parameter is optional");
    assert_eq!(name, None);
}

#[test]
fn an_optional_parameter_that_is_there_is_read() {
    let value = json!({ "name": "filter" });
    let name = Option::<Text>::read(&params(&value), "name").expect("the parameter is read");
    assert_eq!(name, Some(Text("filter".to_string())));
}

#[test]
fn an_optional_parameter_is_not_required() {
    // A constant fact about the types, checked at compile time.
    const _: () = assert!(!<Option<Text> as ParamKind>::REQUIRED);
    const _: () = assert!(<Text as ParamKind>::REQUIRED);
}

#[test]
fn the_first_spelling_of_a_pair_is_read() {
    let value = json!({ "node": 7 });
    let node = Either::<NodeRef, Text>::read_either(&params(&value), "node", "node_name")
        .expect("the parameter is read");
    assert_eq!(node.left(), Some(NodeRef(7)));
}

#[test]
fn the_second_spelling_of_a_pair_is_read() {
    let value = json!({ "node_name": "filter" });
    let node = Either::<NodeRef, Text>::read_either(&params(&value), "node", "node_name")
        .expect("the parameter is read");
    assert_eq!(node.right(), Some(Text("filter".to_string())));
}

#[test]
fn a_pair_with_neither_spelling_is_refused() {
    let value = json!({});
    let error = Either::<NodeRef, Text>::read_either(&params(&value), "node", "node_name")
        .expect_err("the pair is missing");
    assert!(error.message.contains("node_name"), "{error:?}");
}

#[test]
fn an_unknown_key_is_refused() {
    let value = json!({ "name": "filter", "colour": "red" });
    let error = params(&value).deny_unknown(&["name"]).expect_err("the key is unknown");
    assert!(error.message.contains("colour"), "{error:?}");
}

#[test]
fn a_known_key_is_accepted() {
    let value = json!({ "name": "filter" });
    assert!(params(&value).deny_unknown(&["name"]).is_ok());
}

#[test]
fn a_call_with_no_parameters_at_all_is_accepted() {
    let value = json!({});
    assert!(params(&value).deny_unknown(&["name"]).is_ok());
}

#[test]
fn a_boolean_is_read() {
    let value = json!({ "muted": true });
    let muted = Bool::read(&params(&value), "muted").expect("the flag is read");
    assert!(muted.0);
}

/// A **text** parameter is bounded: the core owns this kind, so the
/// core owns its bound.
#[test]
fn a_text_that_is_too_long_is_refused() {
    let value = json!({ "name": "x".repeat(70_000) });
    let error = Text::read(&params(&value), "name").expect_err("that text is too long");
    assert!(error.message.contains("longer"), "{error:?}");
}

/// And a text that fits is read.
#[test]
fn a_text_that_fits_is_read() {
    let value = json!({ "name": "x".repeat(70) });
    let name = Text::read(&params(&value), "name").expect("the text is read");
    assert_eq!(name.0.len(), 70);
}

/// The **raw door** is bounded too: a value may be of any shape, and it
/// may not be of any size.
#[test]
fn a_raw_value_with_too_many_values_is_refused() {
    let items: Vec<i64> = (0..1_000_001).collect();
    let value = json!({ "raw": items });
    let error = serde_json::Value::read(&params(&value), "raw").expect_err("that value is too large");
    assert!(error.message.contains("larger"), "{error:?}");
}

/// And a raw value that fits is read.
#[test]
fn a_raw_value_that_fits_is_read() {
    let items: Vec<i64> = (0..10).collect();
    let value = json!({ "raw": items });
    let raw = serde_json::Value::read(&params(&value), "raw").expect("the value is read");
    assert_eq!(raw.as_array().map(|items| items.len()), Some(10));
}

/// A raw value that is nested too deeply is refused, so the walk that
/// counts it cannot be made to run out of stack.
#[test]
fn a_raw_value_that_is_too_deep_is_refused() {
    let mut value = json!(1);
    for _ in 0..80 {
        value = json!([value]);
    }
    let error =
        serde_json::Value::read(&params(&json!({ "raw": value })), "raw").expect_err("that is too deep");
    assert!(error.message.contains("larger"), "{error:?}");
}
