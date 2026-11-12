//! The **embedded file** of a document: the tree's own file that a file
//! carries.
//!
//! The tree's own: a file may carry files of its own, and a viewer
//! shows them in a list of attachments. They live in the catalog's
//! `/Names` dictionary, under `/EmbeddedFiles`, and they are a **name
//! tree**: a list of a name and the file that carries it.
//!
//! # The rules
//!
//! - The **name** of the file is the name the file specification
//!   carries — the **Unicode** one (`/UF`) and, failing that, the
//!   plain one (`/F`) — when the caller says none, and it must not be
//!   empty: the tree's own rule, and a file nobody can find is not
//!   there.
//! - The **file specification** is an object of the document that says
//!   it is one (`/Type /Filespec`), and anything else is **refused**.
//! - The **tree of names** is the catalog's `/Names` under
//!   `/EmbeddedFiles`, and it holds a list of a name and the file that
//!   carries it: the tree's own shape.
//! - A name that is **already there** names the file again, and does
//!   not make a second entry: the tree's own rule.
//!
//! # What it does not settle
//!
//! - The **bytes** of the file are not here: they are the file
//!   specification's own business, and the command that makes an
//!   attachment carries them.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.embeddedfile.add";

/// The **object number** of a node of the document.
fn the_number_of_node(session: &Session, id: u64) -> u64 {
    session.document().node(id).and_then(crate::the_number_of).unwrap_or(0)
}

/// The **entries** of the dictionary of a node.
fn entries_of_node(node: &craftkit_sdk::Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **entries** of a dictionary that a value holds: whether it is
/// written where it is used or named by a reference.
fn entries_of_value(session: &Session, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(node) = crate::preflight::node_of(session.document(), value) {
        return Some(entries_of_node(node));
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

commands! {
    /// **Name** a **file specification** in the document's tree of embedded
    /// files.
    pdf.embeddedfile.add {
        area: "pdf",
        layer: "pdf",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The file specification the document carries.

            specification: NodeRef,
            /// The name the file is listed under.
            ///
            /// Leave it out and the name of the file specification is used.

            name: Option<Text>,
        },
        run: {

            // The **file specification**: an object of the document that
            // says it is one.
            let Some(specification) = session.document().node(self.specification.0) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("there is no node `{}`", self.specification.0),
                });
            };
            let entries = entries_of_node(specification);
            if !entries.iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Filespec")) {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the node `{}` is not a file specification", self.specification.0),
                });
            }
            let specification_number =
                crate::the_number_of(specification).ok_or_else(|| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the node `{}` has no object number", self.specification.0),
                })?;
            // The **name** of the file: the file's own name when the
            // caller says none, and it must not be empty: the tree's own
            // rule.
            let name = match &self.name {
                Some(name) => name.0.trim().to_string(),
                None => {
                    let unicode = entry(&entries, "UF").and_then(crate::fields::text_of);
                    let plain = entry(&entries, "F").and_then(crate::fields::text_of);
                    match (unicode, plain) {
                        (Some(unicode), _) if !unicode.trim().is_empty() => unicode.trim().to_string(),
                        (_, Some(plain)) if !plain.trim().is_empty() => plain.trim().to_string(),
                        _ => {
                            return Err(EngineError::Failed {
                                id: COMMAND.to_string(),
                                message: "the file needs a name".to_string(),
                            });
                        }
                    }
                }
            };
            if name.is_empty() {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the file needs a name".to_string(),
                });
            }
            // The **catalog**: the object that says the document is one.
            let Some(catalog) = crate::preflight::the_catalog(session.document()) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the document has no catalog".to_string(),
                });
            };
            let catalog_id = catalog.id;
            // The **names** of the document: the dictionary the catalog
            // points at.
            let catalog_entries = entries_of_node(catalog);
            let names = match entry(&catalog_entries, "Names") {
                Some(value) => {
                    if let Some(node) = crate::preflight::node_of(session.document(), value) {
                        (node.id, entries_of_node(node))
                    } else if value.get("entries").is_some() {
                        // The **names** are written where they are used: an
                        // object of their own is made of them, so that the
                        // document's edges can say what the catalog names.
                        let number = crate::free::the_next_number(session.document(), 0);
                        let data = json!({ "object": number, "entries": crate::pairs_of(&entries_of_value(session, value).unwrap_or_default()) });
                        let id = session.document_mut()?.add_node_with(
                            format!("object {number}"),
                            "pdf.object",
                            None,
                            data,
                        );
                        (id, entries_of_value(session, value).unwrap_or_default())
                    } else {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the names of the document are not a dictionary".to_string(),
                        });
                    }
                }
                None => {
                    let number = crate::free::the_next_number(session.document(), 0);
                    let data = json!({ "object": number, "entries": [] });
                    let id = session.document_mut()?.add_node_with(
                        format!("object {number}"),
                        "pdf.object",
                        None,
                        data,
                    );
                    (id, Vec::new())
                }
            };
            let (names_id, names_entries) = names;
            let names_number =
                crate::the_number_of(session.document().node(names_id).ok_or_else(|| EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the names of the document are not there".to_string(),
                })?)
                .unwrap_or(0);
            // The **tree of embedded files**: the names dictionary's own
            // child.
            let embedded = match entry(&names_entries, "EmbeddedFiles") {
                Some(value) => {
                    if let Some(node) = crate::preflight::node_of(session.document(), value) {
                        (node.id, entries_of_node(node))
                    } else if value.get("entries").is_some() {
                        let number = crate::free::the_next_number(session.document(), 0);
                        let data = json!({ "object": number, "entries": crate::pairs_of(&entries_of_value(session, value).unwrap_or_default()) });
                        let id = session.document_mut()?.add_node_with(
                            format!("object {number}"),
                            "pdf.object",
                            None,
                            data,
                        );
                        (id, entries_of_value(session, value).unwrap_or_default())
                    } else {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: "the tree of embedded files is not a dictionary".to_string(),
                        });
                    }
                }
                None => {
                    let number = crate::free::the_next_number(session.document(), 0);
                    let data = json!({ "object": number, "entries": [] });
                    let id = session.document_mut()?.add_node_with(
                        format!("object {number}"),
                        "pdf.object",
                        None,
                        data,
                    );
                    (id, Vec::new())
                }
            };
            let (embedded_id, embedded_entries) = embedded;
            // The **list of names**: a name and the file that carries it.
            let mut list: Vec<Json> =
                entry(&embedded_entries, "Names").and_then(|value| value.as_array().cloned()).unwrap_or_default();
            let pair = json!({ "string": name.clone() });
            let reference = json!({ "ref": [specification_number, 0] });
            // A name that is **already there** names the file again: the
            // tree's own rule.
            let mut found = false;
            let mut at = 0usize;
            while at + 1 < list.len() {
                let same = crate::fields::text_of(&list[at]).map(|text| text == name).unwrap_or(false);
                if same {
                    list[at + 1] = reference.clone();
                    found = true;
                    break;
                }
                at += 2;
            }
            if !found {
                list.push(pair);
                list.push(reference);
            }
            // The **tree of embedded files** holds the list.
            let mut entries = embedded_entries;
            entries.retain(|(key, _)| key != "Names");
            entries.push(("Names".to_string(), Json::Array(list)));
            let data = json!({ "object": the_number_of_node(session, embedded_id), "entries": crate::pairs_of(&entries) });
            if let Some(node) = session.document_mut()?.node_mut(embedded_id) {
                node.data = data;
            }
            // The **names** of the document hold the tree of embedded files.
            let mut names_entries = names_entries;
            names_entries.retain(|(key, _)| key != "EmbeddedFiles");
            names_entries.push((
                "EmbeddedFiles".to_string(),
                json!({ "ref": [the_number_of_node(session, embedded_id), 0] }),
            ));
            let data = json!({ "object": names_number, "entries": crate::pairs_of(&names_entries) });
            if let Some(node) = session.document_mut()?.node_mut(names_id) {
                node.data = data;
            }
            // And the **catalog** holds the names of the document.
            let catalog_entries = crate::entries_of(
                &session
                    .document()
                    .node(catalog_id)
                    .ok_or_else(|| EngineError::Failed {
                        id: COMMAND.to_string(),
                        message: "the document has no catalog".to_string(),
                    })?
                    .data,
            );
            let mut catalog_entries = catalog_entries;
            catalog_entries.retain(|(key, _)| key != "Names");
            catalog_entries.push(("Names".to_string(), json!({ "ref": [names_number, 0] })));
            let data = json!({
                "object": the_number_of_node(session, catalog_id),
                "entries": crate::pairs_of(&catalog_entries),
            });
            if let Some(node) = session.document_mut()?.node_mut(catalog_id) {
                node.data = data;
            }
            // The **edges**: the catalog names the names, the names name
            // the tree of embedded files, and the tree names the file.
            for (from, to, key) in [
                (catalog_id, names_id, "Names"),
                (names_id, embedded_id, "EmbeddedFiles"),
                (embedded_id, self.specification.0, "Names"),
            ] {
                let already = session.document().relations_from(from, Some("ref")).iter().any(|relation| {
                    relation.to == to && relation.payload.get("key").and_then(|key| key.as_str()) == Some(key)
                });
                if !already {
                    let relation = craftkit_sdk::Relation::with_payload(
                        from,
                        to,
                        "ref",
                        json!({ "key": key, "generation": 0 }),
                    );
                    if !session.document_mut()?.add_relation(relation) {
                        return Err(EngineError::Failed {
                            id: COMMAND.to_string(),
                            message: format!("the object `{from}` already refers to `{to}` under `{key}`"),
                        });
                    }
                }
            }
            Ok(Outcome::changed(format!("named the `{name}` in the document")).with_data(json!({
                "name": name,
                "specification": self.specification.0,
                "specification_object": specification_number,
                "names": names_id,
                "embedded": embedded_id,
            })))

        }
    }
}
