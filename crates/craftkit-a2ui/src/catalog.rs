//! The **a2ui catalogue** — the projection of the catalogue entries into
//! a2ui's own `catalog.json` shape.
//!
//! # What this is
//!
//! The a2ui catalogue is the **components** — the things an a2ui agent may
//! put on a surface — and the **functions**. It is a **projection of the
//! `CatalogEntry` registry** (`Ecosystem::catalog`), which is why no
//! catalogue is ever written by hand.
//!
//! # It is not the vocabulary view
//!
//! The SDK's [`craftkit_sdk::Projection::catalog`] is the **vocabulary
//! view** — every command, tool, panel and kind. This is the **a2ui
//! catalogue** — only the components, and only the functions.
//!
//! The two are **not disjoint**: an a2ui component is **both** a craftkit
//! **kind** (`a2ui.text`) and a **catalogue entry** (its published
//! schema), so it appears in the vocabulary view *as a kind* and here *as
//! a component*. The overlap is exactly the eleven a2ui kinds.
//!
//! # The shape
//!
//! a2ui's own catalogue (`catalogs/basic/v1/catalog.json`) has:
//!
//! | the key | what it is |
//! |---|---|
//! | `catalogId` | the catalogue's own id — **minted by this layer** |
//! | `protocolVersion` | the protocol version this layer speaks |
//! | `components` | one entry per registered component, by its **a2ui** name |
//! | `functions` | the function table — the local ones and the agent-side ones |
//! | `$defs` | the layer's definitions, plus `anyComponent` and `anyFunction` |
//!
//! **The `catalogId` is craftkit's own**, minted with a domain prefix the
//! way a2ui recommends (*"It is recommended to prefix this with an
//! internet domain that you own"*, `agent_to_renderer.json:32`). It is
//! **not** a2ui's id: the layer serves its own catalogue, and a surface
//! that names a catalogue the door does not serve is refused.

use craftkit_sdk::serde_json::{Map as JsonMap, Value as Json, json};

use crate::functions;
use crate::schema;

/// The catalogue's **id**: craftkit's own.
///
/// It is minted here, with a domain prefix, and it is the **one** id the
/// door advertises and the **one** id the catalogue file carries.
pub const CATALOG_ID: &str = "https://craftkit.tools/a2ui/catalogs/basic/v1/catalog.json";

/// The **protocol version** this layer speaks.
///
/// The protocol is versioned (`v0_8` … `v1_0`), and the door accepts
/// **one** version and refuses the others — craftkit's rule, not a2ui's
/// adapter chain.
pub const PROTOCOL_VERSION: &str = "1.0";

/// The **a2ui catalogue**, from the catalogue entries.
///
/// The `entries` are the generic view of the `CatalogEntry` registry —
/// what [`craftkit_sdk::Projection::entries`] returns — and each one
/// carries its **published** schema, which is a2ui's shape.
///
/// The result is a2ui's `catalog.json` shape: the components by their
/// **a2ui** names, the functions, and the `$defs`.
pub fn catalogue(entries: &[Json]) -> Result<Json, String> {
    let mut components = JsonMap::<String, Json>::new();
    for entry in entries {
        // The **published** name is the key: an a2ui component is `Text`
        // there, and `a2ui.text` here. An entry that names nothing, or
        // that carries no schema, is **refused** — a catalogue entry
        // that says nothing is an entry that says nothing.
        let Some(name) = entry.get("name").and_then(Json::as_str) else {
            return Err("a catalogue entry has no name".to_string());
        };
        let Some(schema) = entry.get("schema") else {
            return Err(format!("the catalogue entry `{name}` has no schema"));
        };
        // A name that is **listed twice** is refused: the catalogue says
        // one thing about a component, and a second entry would say
        // another.
        if components.contains_key(name) {
            return Err(format!("the catalogue lists `{name}` twice"));
        }
        components.insert(name.to_string(), schema.clone());
    }

    let mut declared = JsonMap::<String, Json>::new();
    for function in functions::all() {
        declared.insert(function.name.to_string(), functions::declaration(&function));
    }

    let mut defs = JsonMap::<String, Json>::new();
    for name in schema::DEFINITIONS {
        if let Some(definition) = schema::definition(name) {
            defs.insert((*name).to_string(), definition);
        }
    }
    // The catalogue's own two: the `oneOf`s that span the whole
    // catalogue, which no single component's derive can know.
    defs.insert("anyComponent".to_string(), any_component(entries));
    defs.insert("anyFunction".to_string(), any_function());

    Ok(json!({
        "catalogId": CATALOG_ID,
        "protocolVersion": PROTOCOL_VERSION,
        "components": Json::Object(components),
        "functions": Json::Object(declared),
        "$defs": Json::Object(defs),
    }))
}

/// The catalogue's **`anyComponent`**: a `oneOf` over its components.
///
/// Each by its own `#/components/<Name>` reference, with the
/// `component` property as the discriminator — which is how a2ui's own
/// catalogue says "any component of this catalogue".
fn any_component(entries: &[Json]) -> Json {
    let mut branches: Vec<Json> = Vec::with_capacity(entries.len());
    for entry in entries {
        let Some(name) = entry.get("name").and_then(Json::as_str) else { continue };
        branches.push(json!({ "$ref": format!("#/components/{name}") }));
    }
    json!({
        "oneOf": branches,
        "discriminator": { "propertyName": "component" },
    })
}

/// The catalogue's **`anyFunction`**: a `oneOf` over its functions.
///
/// Each by its own `#/functions/<name>` reference — which is how a2ui's
/// own catalogue says "any function of this catalogue".
fn any_function() -> Json {
    let mut branches: Vec<Json> = Vec::new();
    for function in functions::all() {
        branches.push(json!({ "$ref": format!("#/functions/{}", function.name) }));
    }
    json!({ "oneOf": branches })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An entry, in the generic view's shape.
    fn entry(name: &str, id: &str, schema: Json) -> Json {
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
        let catalogue = catalogue(&[
            entry("Text", "a2ui.text", json!({ "type": "object" })),
            entry("Card", "a2ui.card", json!({ "type": "object" })),
        ])
        .expect("the catalogue is built");
        let components = catalogue["components"].as_object().expect("there are components");
        assert_eq!(components.len(), 2);
        assert!(components.contains_key("Text"), "by the a2ui name");
        assert!(components.contains_key("Card"));
        assert!(!components.contains_key("a2ui.text"), "and not by the craftkit id");
    }

    /// The catalogue carries **one** `catalogId`.
    #[test]
    fn the_catalogue_carries_one_catalog_id() {
        let catalogue = catalogue(&[entry("Text", "a2ui.text", json!({}))]).expect("the catalogue is built");
        assert_eq!(catalogue["catalogId"], json!(CATALOG_ID));
    }

    /// The functions map holds the local ones and the agent-side ones.
    #[test]
    fn the_functions_map_holds_both_kinds() {
        let catalogue = catalogue(&[]).expect("the catalogue is built");
        let declared = catalogue["functions"].as_object().expect("there are functions");
        assert_eq!(declared.len(), 19, "five local and fourteen agent-side");
        assert!(declared.contains_key("surfaceApply"), "a local one");
        assert!(declared.contains_key("required"), "an agent-side one");
        assert_eq!(declared["surfaceApply"]["properties"]["@call"], json!({ "const": "surfaceApply" }));
        assert_eq!(declared["required"]["returnType"], json!("validationResult"));
    }

    /// The `$defs` hold the layer's definitions and the catalogue's own
    /// two.
    #[test]
    fn the_defs_hold_the_definitions_and_the_catalogue_own_two() {
        let catalogue = catalogue(&[entry("Text", "a2ui.text", json!({}))]).expect("the catalogue is built");
        let defs = catalogue["$defs"].as_object().expect("there are definitions");
        assert!(defs.contains_key("DynamicString"), "a re-derived definition");
        assert!(defs.contains_key("DataBinding"));
        assert!(defs.contains_key("Weight"));
        assert!(defs.contains_key("anyComponent"), "and the catalogue's own");
        assert!(defs.contains_key("anyFunction"));
        // The definitions the layer **removed** are not there.
        assert!(!defs.contains_key("CheckRule"), "the check rule is gone");
        assert!(!defs.contains_key("IndexSystemFunction"), "and the reserved function");
        assert!(!defs.contains_key("DynamicStringList"), "and the unused list");
    }

    /// `anyComponent` is a `oneOf` over the catalogue's own components.
    #[test]
    fn any_component_is_a_one_of_over_the_catalogue() {
        let catalogue =
            catalogue(&[entry("Text", "a2ui.text", json!({})), entry("Card", "a2ui.card", json!({}))])
                .expect("the catalogue is built");
        let defs = catalogue["$defs"].as_object().expect("there are definitions");
        let branches = defs["anyComponent"]["oneOf"].as_array().expect("it is a oneOf");
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0], json!({ "$ref": "#/components/Text" }));
        assert_eq!(branches[1], json!({ "$ref": "#/components/Card" }));
        assert_eq!(defs["anyComponent"]["discriminator"], json!({ "propertyName": "component" }));
    }

    /// `anyFunction` is a `oneOf` over the catalogue's own functions.
    #[test]
    fn any_function_is_a_one_of_over_the_catalogue() {
        let catalogue = catalogue(&[]).expect("the catalogue is built");
        let defs = catalogue["$defs"].as_object().expect("there are definitions");
        let branches = defs["anyFunction"]["oneOf"].as_array().expect("it is a oneOf");
        assert_eq!(branches.len(), 19);
        assert_eq!(branches[0], json!({ "$ref": "#/functions/surfaceCreate" }));
    }

    /// The catalogue is **one file**: every `$ref` in it is local.
    #[test]
    fn every_ref_in_the_catalogue_is_local() {
        let catalogue = catalogue(&[entry("Text", "a2ui.text", json!({}))]).expect("the catalogue is built");
        let text = catalogue.to_string();
        assert!(!text.contains("common_types.json"), "no file is named: {text}");
        assert!(!text.contains(".json#"), "and no ref points at a file");
    }
}
