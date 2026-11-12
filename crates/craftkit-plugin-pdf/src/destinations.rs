//! The **named destinations** of a file.
//!
//! A file may give its pages **names**: a viewer shows them in a list
//! of destinations, and a link may point at one of them by name. The
//! names live in the catalog's `/Names` dictionary, under `/Dests`,
//! and they are a **name tree**: a list of a name and what it points
//! at.
//!
//! The rules:
//!
//! - a file that names no destination has **no named destinations**,
//!   and the answer says so;
//! - a destination that points at a **page** gives the page it points
//!   at — the number of the object, which is the number the file
//!   uses;
//! - a destination that points at **something else** — a place inside
//!   a page, say — is reported with **no page**, because this tree
//!   does not work out where inside a page a destination is;
//! - the **kids** of a name tree are not read: a tree with kids is a
//!   tree of trees, and the names of the kids are not read here.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome};

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

/// The **page** a destination points at: the number of the object.
fn page_of(document: &Document, value: &Json) -> Option<u64> {
    // The destination is either an array — the page and where in it —
    // or a dictionary with the array under `/D`.
    let array: Vec<Json> = match value.as_array() {
        Some(array) => array.clone(),
        None => {
            let entries = entries_of_value(document, value)?;
            entry(&entries, "D")?.as_array()?.clone()
        }
    };
    let first = array.first()?;
    crate::number_of(first)
}

/// The **named destinations** of a document: the name and the page.
pub fn destinations_of(document: &Document) -> Vec<(String, Option<u64>)> {
    let Some(catalog) = crate::preflight::the_catalog(document) else { return Vec::new() };
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(names) = entry(&catalog_entries, "Names") else { return Vec::new() };
    let Some(names_entries) = entries_of_value(document, names) else { return Vec::new() };
    let Some(dests) = entry(&names_entries, "Dests") else { return Vec::new() };
    let Some(dests_entries) = entries_of_value(document, dests) else { return Vec::new() };
    let Some(list) = entry(&dests_entries, "Names").and_then(|names| names.as_array()) else {
        return Vec::new();
    };
    let mut out: Vec<(String, Option<u64>)> = Vec::new();
    // The list is a name and what it points at, a name and what it
    // points at.
    let mut at = 0usize;
    while at + 1 < list.len() {
        let Some(name) = text_of(&list[at]) else {
            at += 2;
            continue;
        };
        let value = &list[at + 1];
        out.push((name, page_of(document, value)));
        at += 2;
    }
    out
}

commands! {
    /// **Read** the **named destinations** of a PDF file.
    ///
    /// A query: it does not change the document. A file that names no
    /// destination has none, and the answer says so.
    pdf.dests.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The entry to read.
            ///
            /// Leave it out and every destination is read.

            name: Option<craftkit_sdk::Text>,
        },
        run: {

            let id = "pdf.dests.read".to_string();
            let document = session.document();
            let all = destinations_of(document);
            let listed: Vec<Json> = match &self.name {
                None => all.iter().map(|(name, page)| json!({ "name": name, "page": page })).collect(),
                Some(name) => {
                    let wanted = name.0.trim();
                    let one: Vec<Json> = all
                        .iter()
                        .filter(|(held, _)| held == wanted)
                        .map(|(name, page)| json!({ "name": name, "page": page }))
                        .collect();
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the file has no destination called `{wanted}`"),
                        });
                    }
                    one
                }
            };
            let count = listed.len();
            Ok(Outcome::new(format!("the file has {count} named destination(s)")).with_data(json!({
                "destinations": listed,
                "count": count,
            })))

        }
    }
}
