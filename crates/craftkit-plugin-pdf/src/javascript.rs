//! The **JavaScript** of a file.
//!
//! A file may hold **scripts**: little programs a reader's program
//! runs when the file is opened. They live in the catalog's `/Names`
//! dictionary, under `/JavaScript`, and they are a **name tree**: a
//! list of a name and the script it holds.
//!
//! The rules:
//!
//! - a file that holds **no script** has none, and the answer says
//!   so;
//! - a script is **read and not run**: this tree reads what a file
//!   says, and it does not run what a file says, and a program in a
//!   file is a program and not a fact;
//! - the **kids** of the name tree are read, up to eight levels deep:
//!   a file with many scripts splits its name tree;
//! - a script that is **not a stream** is skipped, and the answer says
//!   what it skipped.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.scripts.read";

/// The most **deep** a name tree is read.
const MAX_DEPTH: usize = 8;

/// One **script** of a file.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Script {
    /// The name of the script, as the file says it.
    pub name: String,
    /// The **text** of the script.
    pub text: String,
}

impl Script {
    /// The script, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "name": self.name.clone(),
            "text": self.text.clone(),
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

/// The **scripts** of a document, and the things it holds that are not
/// scripts.
pub fn scripts_of(document: &Document) -> (Vec<Script>, Vec<String>) {
    let mut scripts: Vec<Script> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let Some(catalog) = crate::preflight::the_catalog(document) else { return (scripts, skipped) };
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(names) = entry(&catalog_entries, "Names") else { return (scripts, skipped) };
    let Some(names_entries) = entries_of_value(document, names) else { return (scripts, skipped) };
    let Some(javascript) = entry(&names_entries, "JavaScript") else { return (scripts, skipped) };
    let Some(tree) = entries_of_value(document, javascript) else { return (scripts, skipped) };
    walk(document, &tree, 0, &mut scripts, &mut skipped);
    (scripts, skipped)
}

/// Walk a **name tree**, and read the scripts under it.
fn walk(
    document: &Document,
    tree: &[(String, Json)],
    depth: usize,
    scripts: &mut Vec<Script>,
    skipped: &mut Vec<String>,
) {
    if depth >= MAX_DEPTH {
        return;
    }
    // The **names** of this part of the tree: a name and the script it
    // holds.
    if let Some(list) = entry(tree, "Names").and_then(|names| names.as_array()) {
        let mut at = 0usize;
        while at + 1 < list.len() {
            let name = text_of(&list[at]).unwrap_or_default();
            let value = &list[at + 1];
            match crate::preflight::node_of(document, value) {
                Some(node) if node.kind == "pdf.stream" => {
                    let text = crate::decoded_bytes(node, None, None, false, COMMAND)
                        .map(|decoded| String::from_utf8_lossy(&decoded.bytes).into_owned())
                        .unwrap_or_default();
                    scripts.push(Script { name, text });
                }
                Some(_) => skipped.push(format!("{name}: it is not a stream")),
                None => skipped.push(format!("{name}: it is not a stream")),
            }
            at += 2;
        }
    }
    // The **kids**: the tree is split, and the scripts are in the
    // parts.
    if let Some(kids) = entry(tree, "Kids").and_then(|kids| kids.as_array()) {
        for kid in kids {
            let Some(kid_entries) = entries_of_value(document, kid) else { continue };
            walk(document, &kid_entries, depth + 1, scripts, skipped);
        }
    }
}

commands! {
    /// **Read** the **JavaScript** of a PDF file.
    ///
    /// A query: it does not change the document. The scripts are read and
    /// not run: a program in a file is a program and not a fact.
    pdf.scripts.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The script to read.
            ///
            /// Leave it out and every script is read.
            name: Option<craftkit_sdk::Text>,
        },
        run: {
            let id = COMMAND.to_string();
            let (all, skipped) = scripts_of(session.document());
            let listed: Vec<Json> = match &self.name {
                None => all.iter().map(|script| script.to_json()).collect(),
                Some(name) => {
                    let wanted = name.0.trim();
                    let one: Vec<Json> = all
                        .iter()
                        .filter(|script| script.name == wanted)
                        .map(|script| script.to_json())
                        .collect();
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the file has no script called `{wanted}`"),
                        });
                    }
                    one
                }
            };
            let count = listed.len();
            Ok(Outcome::new(format!("the file holds {count} script(s)")).with_data(json!({
                "scripts": listed,
                "count": count,
                "skipped": skipped,
            })))

        }
    }
}
