//! The **form** of a PDF file.
//!
//! A file with fields in it has an **interactive form**: the catalog
//! names an **AcroForm** dictionary, and that dictionary says:
//!
//! - the **fields** the form has — the objects it lists;
//! - whether the **appearances** of the fields are to be made by the
//!   reader (`NeedAppearances`), which is what a file says when it has
//!   no appearance streams of its own;
//! - the **default appearance** (`DA`) — the way a field's text looks
//!   when the field says nothing;
//! - the **default resources** (`DR`) — the fonts and the other things
//!   the fields use;
//! - the **signature flags** (`SigFlags`) — what the file says about
//!   the signatures it holds;
//! - the **calculation order** (`CO`) — the fields that are
//!   calculated, and the order they are calculated in.
//!
//! The rules:
//!
//! - a file that names no form **has no form**, and the answer says
//!   so;
//! - the form may be written where it is used, or named by a
//!   reference, and both are read the same way;
//! - the fields and the calculation order are **lists of objects**,
//!   and the numbers are the numbers the file holds.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, Outcome, Text};

/// The **form** of a file: what its dictionary says.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    /// The objects of the fields the form lists.
    pub fields: Vec<u64>,
    /// Whether the reader is to make the appearances of the fields.
    pub need_appearances: bool,
    /// The default appearance of a field.
    pub default_appearance: Option<String>,
    /// The default resources of the form: the pairs they hold.
    pub default_resources: Vec<(String, Json)>,
    /// The signature flags of the form.
    pub signature_flags: Option<i64>,
    /// The fields that are calculated, in the order they are
    /// calculated in.
    pub calculation_order: Vec<u64>,
}

/// The **entries** of a node's dictionary.
fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
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

/// The **dictionary** a value holds: the pairs of an inline
/// dictionary.
fn dictionary_of(value: &Json) -> Option<Vec<(String, Json)>> {
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

/// The **node** an object number names.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| node.data.get("object").and_then(|found| found.as_u64()) == Some(number))
}

/// The **object numbers** a list of references holds.
fn numbers_of(value: &Json) -> Vec<u64> {
    let Some(list) = value.as_array() else { return Vec::new() };
    list.iter().filter_map(crate::number_of).collect()
}

/// The **form** of a document: what the catalog names.
pub fn form_of(document: &Document) -> Option<Form> {
    let catalog = crate::preflight::the_catalog(document)?;
    let entries = entries_of(catalog);
    let value = entry(&entries, "AcroForm")?;
    // The form may be named by a reference, or written where it is
    // used.
    let dictionary = match crate::number_of(value) {
        Some(number) => {
            let node = node_of_number(document, number)?;
            entries_of(node)
        }
        None => dictionary_of(value)?,
    };
    let fields = entry(&dictionary, "Fields").map(numbers_of).unwrap_or_default();
    let need_appearances =
        entry(&dictionary, "NeedAppearances").and_then(|flag| flag.as_bool()).unwrap_or(false);
    let default_appearance = entry(&dictionary, "DA").and_then(text_of);
    let default_resources = entry(&dictionary, "DR").and_then(dictionary_of).unwrap_or_default();
    let signature_flags = entry(&dictionary, "SigFlags").and_then(crate::whole_number_of);
    let calculation_order = entry(&dictionary, "CO").map(numbers_of).unwrap_or_default();
    Some(Form {
        fields,
        need_appearances,
        default_appearance,
        default_resources,
        signature_flags,
        calculation_order,
    })
}

commands! {
    /// **Read** the **form** of a PDF file.
    ///
    /// A query: it does not change the document. A file that names no form
    /// has no form, and the answer says so.
    pdf.form.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The entry to read.
            ///
            /// Leave it out and everything the form says is read.
            key: Option<Text>,
        },
        run: {
            let id = "pdf.form.read".to_string();
            let document = session.document();
            let Some(form) = form_of(document) else {
                return Ok(Outcome::new("the file has no form").with_data(json!({
                    "form": Json::Null,
                    "has_form": false,
                })));
            };
            let listed: Vec<Json> = match &self.key {
                None => Vec::new(),
                Some(key) => {
                    let key = key.0.trim();
                    let mut one: Vec<Json> = Vec::new();
                    if key == "Fields" {
                        one.push(json!(["Fields", json!(form.fields.clone())]));
                    }
                    if key == "NeedAppearances" {
                        one.push(json!(["NeedAppearances", json!(form.need_appearances)]));
                    }
                    if key == "DA" {
                        one.push(json!([
                            "DA",
                            form.default_appearance.clone().map(|value| json!(value)).unwrap_or(Json::Null)
                        ]));
                    }
                    if key == "SigFlags" {
                        one.push(json!([
                            "SigFlags",
                            form.signature_flags.map(|flags| json!(flags)).unwrap_or(Json::Null)
                        ]));
                    }
                    if key == "CO" {
                        one.push(json!(["CO", json!(form.calculation_order.clone())]));
                    }
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the form has no entry `{key}`"),
                        });
                    }
                    one
                }
            };
            Ok(Outcome::new(format!("the form has {} field(s)", form.fields.len())).with_data(json!({
                "has_form": true,
                "fields": form.fields.clone(),
                "field_count": form.fields.len(),
                "need_appearances": form.need_appearances,
                "default_appearance": form.default_appearance.clone().map(|value| json!(value)).unwrap_or(Json::Null),
                "default_resources": form.default_resources.iter().map(|(key, value)| json!([key.clone(), value.clone()])).collect::<Vec<Json>>(),
                "signature_flags": form.signature_flags.map(|flags| json!(flags)).unwrap_or(Json::Null),
                "calculation_order": form.calculation_order.clone(),
                "entries": listed,
            })))

        }
    }
}
