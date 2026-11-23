//! The **shapes** — the layer's re-derived schemas.
//!
//! The five narrowings are visible here: a branch the layer cannot
//! execute is a branch it must not publish.

use craftkit_a2ui::schema;
use craftkit_a2ui::serde_json::json;

/// The **five** narrowings are visible in the published schema.
#[test]
fn the_five_narrowings_are_visible() {
    // One: the `Icon` enum of built-in names is gone.
    let icon = schema::published("Icon", &["name", "weight"], &[None, None]);
    let branches = icon["properties"]["name"]["oneOf"].as_array().expect("it is a oneOf");
    assert_eq!(branches.len(), 2, "the 59-name branch is gone: {branches:?}");

    // Two: the `ChildList` template branch is gone.
    let child_list = schema::definition("ChildList").expect("the definition is there");
    let branches = child_list["oneOf"].as_array().expect("it is a oneOf");
    assert_eq!(branches.len(), 1, "only the static branch is left: {branches:?}");
    assert_eq!(branches[0]["type"], json!("array"));

    // Three: the `FunctionCall`'s reserved-function branch is gone.
    let call = schema::definition("FunctionCall").expect("the definition is there");
    let text = call.to_string();
    assert!(!text.contains("IndexSystemFunction"), "the reserved function is gone: {text}");

    // Four: the `Checkable`'s `@checks` is gone.
    let checkable = schema::definition("Checkable").expect("the definition is there");
    assert!(!checkable.to_string().contains("@checks"), "the check rule is gone");

    // Five: the `TextField`'s `value` keeps only the binding branch.
    let field = schema::published(
        "TextField",
        &["label", "value", "placeholder", "variant", "weight"],
        &[Some("DynamicString"), Some("DynamicString"), Some("DynamicString"), None, None],
    );
    assert_eq!(
        field["allOf"][1]["properties"]["value"],
        json!({ "$ref": "#/$defs/DataBinding" }),
        "only the binding branch is published"
    );
}

/// The **discriminator** is stamped, and the **required** list holds it.
#[test]
fn the_discriminator_is_stamped() {
    let published = schema::published("Text", &["text"], &[Some("DynamicString")]);
    assert_eq!(published["properties"]["component"], json!({ "const": "Text" }));
    let required = published["required"].as_array().expect("there is a required list");
    assert_eq!(required[0], json!("component"));
}

/// The three **checkable** components keep their `allOf`, so that
/// `Checkable` is a definition the closure reaches.
#[test]
fn the_checkable_components_keep_their_all_of() {
    for component in ["Button", "CheckBox", "TextField"] {
        let published = schema::published(component, &["weight"], &[None]);
        let parts = published["allOf"].as_array().expect("it is an allOf");
        assert_eq!(parts.len(), 2, "{component} has two parts");
        assert_eq!(parts[0], json!({ "$ref": "#/$defs/Checkable" }));
    }
    // And the others do not.
    let published = schema::published("Text", &["text"], &[None]);
    assert!(published.get("allOf").is_none(), "Text has no allOf");
}

/// The properties of a published schema are found **wherever** they are.
#[test]
fn the_properties_are_found_wherever_they_are() {
    let published = schema::published("Button", &["child", "weight"], &[Some("Child"), None]);
    let properties = schema::properties_of(&published);
    assert!(properties.contains(&"child".to_string()), "the own part's properties");
    assert!(properties.contains(&"component".to_string()), "and the discriminator");
}

/// A prop with a **definition** is a `$ref`, and a plain prop is the
/// layer's own shape.
#[test]
fn a_prop_with_a_definition_is_a_ref() {
    let published = schema::published("Text", &["text", "variant"], &[Some("DynamicString"), None]);
    assert_eq!(published["properties"]["text"], json!({ "$ref": "#/$defs/DynamicString" }));
    assert_eq!(published["properties"]["variant"]["type"], json!("string"));
    assert_eq!(published["properties"]["variant"]["enum"], json!(["caption", "body"]));
}

/// The definitions the layer publishes are **thirteen**, and the
/// catalogue's own two are not among them.
#[test]
fn the_definitions_are_thirteen() {
    assert_eq!(schema::DEFINITIONS.len(), 13);
    // `anyFunction` is on the list, but the projection writes it.
    assert!(schema::definition("anyFunction").is_none(), "the projection writes it");
}

/// Every definition the layer publishes is **buildable**.
#[test]
fn every_definition_is_buildable() {
    for name in schema::DEFINITIONS {
        if *name == "anyFunction" {
            continue;
        }
        assert!(schema::definition(name).is_some(), "{name} is buildable");
    }
}
