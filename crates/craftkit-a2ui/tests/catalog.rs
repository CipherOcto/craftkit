//! The **a2ui catalogue** — the projection of the catalogue entries.

use craftkit_a2ui::serde_json::json;
use craftkit_a2ui::{catalog, defs, schema};

/// The names of the table's properties, as text.
fn names_of(table: &[&[u8]]) -> Vec<String> {
    table.iter().map(|name| String::from_utf8_lossy(name).to_string()).collect()
}

/// An entry, in the generic view's shape.
fn entry(name: &str, id: &str, schema: craftkit_a2ui::serde_json::Value) -> craftkit_a2ui::serde_json::Value {
    json!({
        "id": id,
        "name": name,
        "label": name,
        "layer": "a2ui",
        "doc": "One line.",
        "schema": schema,
    })
}

/// The catalogue holds the components, by their **a2ui** names.
#[test]
fn the_catalogue_holds_the_components_by_their_a2ui_names() {
    let catalogue = catalog::catalogue(&[
        entry("Text", "a2ui.text", json!({ "type": "object" })),
        entry("Card", "a2ui.card", json!({ "type": "object" })),
    ]);
    let components = catalogue["components"].as_object().expect("there are components");
    assert_eq!(components.len(), 2);
    assert!(components.contains_key("Text"), "by the a2ui name");
    assert!(!components.contains_key("a2ui.text"), "and not by the craftkit id");
}

/// The catalogue carries **one** `catalogId`, and it is the layer's own.
#[test]
fn the_catalogue_carries_one_catalog_id() {
    let catalogue = catalog::catalogue(&[]);
    assert_eq!(catalogue["catalogId"], json!(catalog::CATALOG_ID));
    assert_eq!(catalogue["protocolVersion"], json!(catalog::PROTOCOL_VERSION));
}

/// The **schema text's** property set is the **table's**.
///
/// The layer's two artefacts cannot drift, and this is the test that says
/// so — it is **this** crate's test, because the layer is where both
/// live.
#[test]
fn the_schema_texts_property_set_is_the_tables() {
    for (component, table) in [
        ("Text", defs::TEXT),
        ("Image", defs::IMAGE),
        ("Icon", defs::ICON),
        ("Row", defs::ROW),
        ("Column", defs::COLUMN),
        ("List", defs::LIST),
        ("Card", defs::CARD),
        ("Divider", defs::DIVIDER),
        ("Button", defs::BUTTON),
        ("CheckBox", defs::CHECK_BOX),
        ("TextField", defs::TEXT_FIELD),
    ] {
        let names = names_of(table);
        let published = schema::published(
            component,
            &names.iter().map(|name| name.as_str()).collect::<Vec<&str>>(),
            &vec![None; names.len()],
        );
        let properties = schema::properties_of(&published);
        for name in &names {
            assert!(properties.contains(name), "{component} is missing `{name}`");
        }
        // The **exempt** entries are not the table's, and the table does
        // not hold them.
        for exempt in defs::EXEMPT {
            let exempt = String::from_utf8_lossy(exempt).to_string();
            assert!(!names.contains(&exempt), "{component} must not hold `{exempt}`");
        }
    }
}

/// The catalogue's `$defs` holds **fourteen** entries: the thirteen
/// re-derived definitions and the catalogue's own `anyComponent`.
#[test]
fn the_defs_hold_fourteen_entries() {
    let entries = vec![entry(
        "Text",
        "a2ui.text",
        schema::published("Text", &["text", "variant", "weight"], &[Some("DynamicString"), None, None]),
    )];
    let catalogue = catalog::catalogue(&entries);
    let defs = catalogue["$defs"].as_object().expect("there are definitions");
    assert_eq!(defs.len(), 14, "thirteen definitions and the catalogue's own");
    assert!(defs.contains_key("anyComponent"));
    assert!(defs.contains_key("anyFunction"));
    // And the definitions the layer removed are not there.
    assert!(!defs.contains_key("CheckRule"), "the check rule is gone");
    assert!(!defs.contains_key("IndexSystemFunction"), "and the reserved function");
    assert!(!defs.contains_key("DynamicStringList"), "and the unused list");
    assert!(!defs.contains_key("DynamicNumber"), "and the unused number");
}

/// The **`anyComponent`** is a `oneOf` over the catalogue's own components.
#[test]
fn any_component_is_a_one_of_over_the_catalogue() {
    let entries = vec![entry("Text", "a2ui.text", json!({})), entry("Card", "a2ui.card", json!({}))];
    let catalogue = catalog::catalogue(&entries);
    let defs = catalogue["$defs"].as_object().expect("there are definitions");
    let branches = defs["anyComponent"]["oneOf"].as_array().expect("it is a oneOf");
    assert_eq!(branches.len(), 2);
    assert_eq!(branches[0], json!({ "$ref": "#/components/Text" }));
    assert_eq!(branches[1], json!({ "$ref": "#/components/Card" }));
    assert_eq!(defs["anyComponent"]["discriminator"], json!({ "propertyName": "component" }));
}

/// Every `$ref` in the published catalogue is **local**.
#[test]
fn every_ref_is_local() {
    let entries = vec![entry(
        "Text",
        "a2ui.text",
        schema::published("Text", &["text", "variant", "weight"], &[Some("DynamicString"), None, None]),
    )];
    let catalogue = catalog::catalogue(&entries);
    let text = catalogue.to_string();
    assert!(!text.contains("common_types.json"), "no file is named: {text}");
    assert!(!text.contains(".json#"), "and no ref points at a file");
}
