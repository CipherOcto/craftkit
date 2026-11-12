//! The **optional content** of a file: the layers a file shows.
//!
//! A file may hold **layers** — parts of the content that are shown
//! and parts that are hidden, and a viewer shows the list and lets a
//! reader turn the layers on and off. They live in the catalog's
//! `/OCProperties`, and the layers themselves are **optional content
//! groups**: each one has a name and a place in the list.
//!
//! The **default configuration** says which of the layers are on when
//! the file is opened: the `/ON` list and the `/OFF` list, and the
//! `/BaseState` — which is what a layer that is in neither list
//! follows.
//!
//! The rules:
//!
//! - a file that has no optional content has **no layers**, and the
//!   answer says so;
//! - a layer that says how it is to be shown (`/Usage`) is shown the
//!   way it says;
//! - a layer that is in neither list follows the **base state**, and a
//!   base state of `UseNone` shows nothing;
//! - the **order** of the layers is not read: the order is the order
//!   the list is in, and the tree does not make a tree of it;
//! - an **intent** that is a dictionary is not read: the intent of a
//!   layer may be a name or a dictionary of names, and a dictionary
//!   says nothing this tree reads.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.layers.read";

/// One **layer**: an optional content group.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    /// The name of the layer, as the file says it.
    pub name: String,
    /// The **intent** of the layer, when it is a name.
    pub intent: Option<String>,
    /// Whether the layer is **shown** when the file is opened.
    pub visible: bool,
}

impl Layer {
    /// The layer, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "name": self.name.clone(),
            "intent": self.intent.clone().map(|intent| json!(intent)).unwrap_or(Json::Null),
            "visible": self.visible,
        })
    }
}

/// The **entries** of a value: the pairs of a dictionary, whether it
/// is written where it is used or named by a reference.
fn entries_of_value(document: &Document, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(node) = crate::preflight::node_of(document, value) {
        return Some(crate::entries_of(&node.data));
    }
    let list = value.get("entries")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let pair = entry.as_array()?;
                let key = pair.first()?.as_str()?.to_string();
                Some((key, pair.get(1)?.clone()))
            })
            .collect(),
    )
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **text** a value holds: the file's own spelling of a text.
fn text_of(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **layers** of a document, and whether each one is shown.
pub fn layers_of(document: &Document) -> Vec<Layer> {
    let Some(catalog) = crate::preflight::the_catalog(document) else { return Vec::new() };
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(properties) = entry(&catalog_entries, "OCProperties") else { return Vec::new() };
    let Some(properties_entries) = entries_of_value(document, properties) else { return Vec::new() };
    let Some(groups) = entry(&properties_entries, "OCGs").and_then(|groups| groups.as_array()) else {
        return Vec::new();
    };
    // The **default configuration**: what is on, what is off, and what
    // a layer that is in neither list follows.
    let configuration =
        entry(&properties_entries, "D").and_then(|default| entries_of_value(document, default));
    let base = configuration
        .as_ref()
        .and_then(|entries| entry(entries, "BaseState"))
        .and_then(|state| state.as_str())
        .unwrap_or("ON")
        .to_string();
    let on = numbers_of(configuration.as_ref().and_then(|entries| entry(entries, "ON")));
    let off = numbers_of(configuration.as_ref().and_then(|entries| entry(entries, "OFF")));
    let mut out: Vec<Layer> = Vec::new();
    for group in groups {
        let Some(node) = crate::preflight::node_of(document, group) else { continue };
        let entries = crate::entries_of(&node.data);
        let name = entry(&entries, "Name").and_then(text_of).unwrap_or_default();
        let intent = entry(&entries, "Intent").and_then(|intent| intent.as_str()).map(String::from);
        let number = node
            .data
            .get("object")
            .and_then(crate::whole_number_of)
            .and_then(|number| u64::try_from(number).ok());
        let visible = match usage_of(document, &entries) {
            Some(state) => state == "ON",
            None => {
                if let Some(number) = number {
                    if off.contains(&number) {
                        false
                    } else if on.contains(&number) {
                        true
                    } else {
                        base == "ON"
                    }
                } else {
                    base == "ON"
                }
            }
        };
        out.push(Layer { name, intent, visible });
    }
    out
}

/// The **state** a layer says it is shown in, when it says one.
///
/// The usage of a layer may say how it is to be shown for the view, the
/// for the print and the for the edit, and the first of the three
/// that says something is the one that is read.
fn usage_of(document: &Document, entries: &[(String, Json)]) -> Option<String> {
    let usage = entry(entries, "Usage")?;
    let usage = entries_of_value(document, usage)?;
    for kind in ["View", "Print", "Edit"] {
        let Some(view) = entry(&usage, kind).and_then(|view| entries_of_value(document, view)) else {
            continue;
        };
        if let Some(state) = entry(&view, "ViewState").and_then(|state| state.as_str()) {
            return Some(state.to_string());
        }
    }
    None
}

/// The **numbers** a list of references names.
fn numbers_of(value: Option<&Json>) -> Vec<u64> {
    let Some(list) = value.and_then(|value| value.as_array()) else { return Vec::new() };
    list.iter().filter_map(crate::number_of).collect()
}

commands! {
    /// **Read** the **layers** of a PDF file.
    ///
    /// A query: it does not change the document. A file with no optional
    /// content has no layers, and the answer says so.
    pdf.layers.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The layer to read.
            ///
            /// Leave it out and every layer is read.

            name: Option<craftkit_sdk::Text>,
        },
        run: {

            let id = COMMAND.to_string();
            let all = layers_of(session.document());
            let listed: Vec<Json> = match &self.name {
                None => all.iter().map(|layer| layer.to_json()).collect(),
                Some(name) => {
                    let wanted = name.0.trim();
                    let one: Vec<Json> =
                        all.iter().filter(|layer| layer.name == wanted).map(|layer| layer.to_json()).collect();
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the file has no layer called `{wanted}`"),
                        });
                    }
                    one
                }
            };
            let count = listed.len();
            Ok(Outcome::new(format!("the file has {count} layer(s)")).with_data(json!({
                "layers": listed,
                "count": count,
            })))

        }
    }
}
