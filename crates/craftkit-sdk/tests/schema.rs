//! The SDK's one JSON Schema generator.
//!
//! The mapping moved **here** from the MCP door, and this is the test that
//! says it still says what it said: the shape of a command's parameters is
//! the same as it was before the move.

use craftkit_sdk::schema::to_json_schema;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Doc, ParamDoc};

/// A command with nothing to take.
#[test]
fn a_command_with_no_parameters_has_an_empty_schema() {
    let schema = to_json_schema(&[]);
    assert_eq!(
        schema,
        json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false,
        })
    );
}

/// Every kind of parameter says what it takes.
#[test]
fn every_kind_of_parameter_says_what_it_takes() {
    let schema = to_json_schema(&[
        ParamDoc::required("count", Doc::Number, "How many."),
        ParamDoc::required("name", Doc::Text, "What it is called."),
        ParamDoc::required("on", Doc::Bool, "Whether it is on."),
        ParamDoc::required("node", Doc::NodeRef, "Which node."),
        ParamDoc::required("kind", Doc::NodeKind, "Which kind."),
        ParamDoc::required("raw", Doc::Json, "Any value."),
    ]);
    let properties = schema["properties"].as_object().expect("the schema has properties");
    assert_eq!(properties["count"]["type"], json!("number"));
    assert_eq!(properties["name"]["type"], json!("string"));
    assert_eq!(properties["on"]["type"], json!("boolean"));
    assert_eq!(properties["node"]["type"], json!("integer"));
    assert_eq!(properties["node"]["minimum"], json!(0));
    assert_eq!(properties["kind"]["type"], json!("string"));
    // The door out: any value at all — no type is said.
    assert!(properties["raw"].get("type").is_none(), "no type is claimed");
    assert_eq!(properties["raw"]["description"], json!("Any value."));
}

/// A required parameter is required, and an optional one is not.
#[test]
fn the_required_list_holds_the_required_parameters() {
    let schema = to_json_schema(&[
        ParamDoc::required("name", Doc::Text, "What it is called."),
        ParamDoc::optional("muted", Doc::Bool, "Whether it is muted."),
    ]);
    assert_eq!(schema["required"], json!(["name"]));
}

/// The help is the description, and an empty help says nothing.
#[test]
fn the_help_is_the_description() {
    let schema = to_json_schema(&[
        ParamDoc::required("name", Doc::Text, "What it is called."),
        ParamDoc::required("other", Doc::Text, "   "),
    ]);
    let properties = schema["properties"].as_object().expect("the schema has properties");
    assert_eq!(properties["name"]["description"], json!("What it is called."));
    assert!(properties["other"].get("description").is_none());
}

/// A parameter with a second spelling offers both.
#[test]
fn a_parameter_with_a_second_spelling_offers_both() {
    let schema = to_json_schema(&[ParamDoc::either("node", Doc::NodeRef, "Which node.", "node_name")]);
    let properties = schema["properties"].as_object().expect("the schema has properties");
    assert!(properties.contains_key("node"));
    assert!(properties.contains_key("node_name"));
    // Neither half is required on its own: the pair is.
    assert!(schema.get("required").is_none());
}

/// A plugin's own kind says its name and its unit.
#[test]
fn a_plugins_kind_carries_its_name_and_its_unit() {
    let schema = to_json_schema(&[ParamDoc::required("length", Doc::Number, "How long.")
        .with_type_name(Some("length"))
        .with_unit(Some("mm"))]);
    let properties = schema["properties"].as_object().expect("the schema has properties");
    assert_eq!(properties["length"]["title"], json!("length"));
    assert_eq!(properties["length"]["unit"], json!("mm"));
    // The shape is unchanged: a length is a number, and the schema says
    // number.
    assert_eq!(properties["length"]["type"], json!("number"));
}
