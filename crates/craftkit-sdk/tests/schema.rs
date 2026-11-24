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

/// A **text** is bounded, and the schema says by how much.
///
/// The bound is the SDK's own, and the schema **reads** it: a schema that
/// declared a copy of the number would be a second place the number
/// lives. Telling the model the bound before it proposes a value is the
/// whole difference.
#[test]
fn a_text_says_the_bound_it_has() {
    let schema = to_json_schema(&[ParamDoc::required("name", Doc::Text, "What it is called.")]);
    let properties = schema["properties"].as_object().expect("the schema has properties");
    assert_eq!(properties["name"]["maxLength"], json!(craftkit_sdk::params::MAX_TEXT_CHARACTERS));
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
    let schema =
        to_json_schema(&[ParamDoc::either("node", Doc::NodeRef, Doc::Text, "Which node.", "node_name")]);
    let properties = schema["properties"].as_object().expect("the schema has properties");
    assert!(properties.contains_key("node"));
    assert!(properties.contains_key("node_name"));
    // Neither half is required on its own: the pair is.
    assert!(schema.get("required").is_none());
}

/// The two spellings are **two kinds**, and each is published with its
/// own shape.
///
/// A node's id is a number and a node's name is a text, and a schema that
/// says a name is a number is a schema that lies.
#[test]
fn the_two_spellings_are_published_with_their_own_shapes() {
    let schema =
        to_json_schema(&[ParamDoc::either("node", Doc::NodeRef, Doc::Text, "Which node.", "node_name")]);
    let properties = schema["properties"].as_object().expect("the schema has properties");
    assert_eq!(properties["node"]["type"], json!("integer"), "the id is a number: {schema}");
    assert_eq!(properties["node_name"]["type"], json!("string"), "and the name is a text: {schema}");
    // And the name's bound is the SDK's own.
    assert_eq!(
        properties["node_name"]["maxLength"],
        json!(craftkit_sdk::params::MAX_TEXT_CHARACTERS),
        "{schema}"
    );
}

/// A pair is required **as a pair**, and the schema says so.
///
/// JSON Schema cannot say "one of these two" in a word, but it can say it
/// in a shape: an `anyOf` with a `required` in each half. A schema that
/// permits a call the door refuses is a schema that lies.
#[test]
fn a_pair_is_required_as_a_pair() {
    let schema =
        to_json_schema(&[ParamDoc::either("node", Doc::NodeRef, Doc::Text, "Which node.", "node_name")]);
    assert_eq!(schema["anyOf"], json!([{ "required": ["node"] }, { "required": ["node_name"] }]), "{schema}");
}

/// Two pairs are two `anyOf`s, and the caller has to fill in **both**.
#[test]
fn two_pairs_are_required_one_after_the_other() {
    let schema = to_json_schema(&[
        ParamDoc::either("node", Doc::NodeRef, Doc::Text, "Which node.", "node_name"),
        ParamDoc::either("kind", Doc::NodeKind, Doc::Text, "Which kind.", "kind_name"),
    ]);
    assert_eq!(
        schema["allOf"],
        json!([
            { "anyOf": [{ "required": ["node"] }, { "required": ["node_name"] }] },
            { "anyOf": [{ "required": ["kind"] }, { "required": ["kind_name"] }] },
        ]),
        "{schema}"
    );
}

/// The other spelling says what it takes, and the schema says the same.
///
/// A kind that takes a string as well — a length, which may be written
/// `25` or `25mm` — says **both** shapes, for **each** spelling it has.
#[test]
fn the_other_spelling_says_what_it_takes() {
    let schema =
        to_json_schema(&[ParamDoc::either("length", Doc::Number, Doc::Number, "How long.", "length_text")
            .with_alternative_accepts_text(true)]);
    let properties = schema["properties"].as_object().expect("the schema has properties");
    assert_eq!(properties["length"]["type"], json!("number"), "{schema}");
    assert_eq!(properties["length_text"]["type"], json!(["number", "string"]), "{schema}");
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
