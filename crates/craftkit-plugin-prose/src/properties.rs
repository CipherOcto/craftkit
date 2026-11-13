//! The **properties** of a document.
//!
//! A document says more than its title: who wrote it, what it is
//! about, the words a search looks for, what a reader may say about
//! it, and the **custom** properties that a workflow of a firm puts
//! in it — the project, the client, the number of the ticket.
//!
//! # The rules
//!
//! - The properties are **facts of the document**, and they are kept
//!   in its payload: they are few, they hold nothing, and they are
//!   the document's own.
//! - A property that is **not named** keeps what it was, and a
//!   property that is named with an **empty** value **takes it away**:
//!   a document that has no author and a document whose author was
//!   taken away are the same document.
//! - A **custom** property is a **name** and a **value**. The name is
//!   at most **255** characters long and it may not be empty, and the
//!   value is at most **one mebibyte**. A name is matched **without
//!   regard to case**, as the tree's is, and a name that is already
//!   there keeps the spelling it has.
//! - A custom property is taken away by a **null** and by an **empty**
//!   value; the tree takes it away by a null alone.
//! - The **title** is not one of these: it has its own command
//!   (`prose.document.title`).
//! - A node that is **not a document** may not have properties.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Text};

/// The longest **name** a custom property may have.
pub const MAX_NAME: usize = 255;

/// The longest **value** a property may have, in bytes.
pub const MAX_VALUE: usize = 1024 * 1024;

/// The properties a document **says** of itself, besides the title.
pub const BUILT_INS: [&str; 7] =
    ["author", "subject", "keywords", "comments", "category", "last_modified_by", "revision"];

/// The **properties** of a document, as they are kept.
pub fn properties_of(node: &Node) -> Json {
    node.data
        .get("properties")
        .and_then(|properties| properties.as_object())
        .cloned()
        .map(Json::Object)
        .unwrap_or_else(|| Json::Object(craftkit_sdk::serde_json::Map::new()))
}

/// The **custom** properties of a document, as they are kept.
pub fn custom_of(node: &Node) -> Json {
    node.data
        .get("custom_properties")
        .and_then(|custom| custom.as_object())
        .cloned()
        .map(Json::Object)
        .unwrap_or_else(|| Json::Object(craftkit_sdk::serde_json::Map::new()))
}

/// A **value** of a property: a text, and a text that is not empty.
fn value_of(value: &str, id: &str, what: &str) -> Result<Option<String>, EngineError> {
    if value.len() > MAX_VALUE {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("{what} is at most {MAX_VALUE} bytes long"),
        });
    }
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(value.to_string()))
}

/// The **name** of a custom property.
fn name_of(name: &str, id: &str) -> Result<String, EngineError> {
    let name = name.trim();
    if name.is_empty() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a custom property has a name".to_string(),
        });
    }
    if name.chars().count() > MAX_NAME {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a name is at most {MAX_NAME} characters long"),
        });
    }
    Ok(name.to_string())
}

/// The **key** a name is kept under: the spelling it has, when it is
/// already there.
fn the_key(custom: &Json, name: &str) -> String {
    let Some(map) = custom.as_object() else { return name.to_string() };
    map.keys().find(|held| held.eq_ignore_ascii_case(name)).cloned().unwrap_or_else(|| name.to_string())
}

commands! {
    /// **Say** what a document is.
    prose.document.properties {
        area: "prose",
        layer: "prose",
        icon: "tag",
        mutates,
        undoable,
        params: {
            /// The document.
            document: NodeRef,
            /// Who wrote it.
            ///
            /// An empty author takes the author away.
            author: Option<Text>,
            /// What it is about.
            subject: Option<Text>,
            /// The words a search looks for.
            keywords: Option<Text>,
            /// What a reader may say about it.
            comments: Option<Text>,
            /// The category it is in.
            category: Option<Text>,
            /// Who touched it last.
            ///
            /// An empty name takes the name away.
            last_modified_by: Option<Text>,
            /// How many times it has been revised.
            ///
            /// A whole number, and a revision of zero takes the revision away.
            revision: Option<Number>,
            /// The **custom** properties: a list of pairs, each a name and a
            /// value.
            ///
            /// A null or an empty value takes the property away.
            custom: Option<Json>,
        },
        run: {
            let id = "prose.document.properties".to_string();
            let document = self.document.0;
            let Some(node) = session.document().node(document).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{document}`"),
                });
            };
            if node.kind != "prose.document" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{document}` is a `{}` and not a document", node.kind),
                });
            }
            // The **built-in** properties that are named, and the values
            // they are given.
            let mut properties = properties_of(&node);
            let given = [
                ("author", self.author.as_ref().map(|value| value.0.as_str())),
                ("subject", self.subject.as_ref().map(|value| value.0.as_str())),
                ("keywords", self.keywords.as_ref().map(|value| value.0.as_str())),
                ("comments", self.comments.as_ref().map(|value| value.0.as_str())),
                ("category", self.category.as_ref().map(|value| value.0.as_str())),
                ("last_modified_by", self.last_modified_by.as_ref().map(|value| value.0.as_str())),
            ];
            for (name, value) in given {
                let Some(value) = value else { continue };
                match value_of(value, &id, "a property")? {
                    Some(value) => properties[name] = json!(value),
                    None => {
                        if let Some(map) = properties.as_object_mut() {
                            map.remove(name);
                        }
                    }
                }
            }
            // The **revision** is a whole number, and a revision of zero
            // is no revision at all: the tree's rule, that nothing means
            // nothing.
            if let Some(revision) = self.revision {
                let number = revision.0;
                if !number.is_finite() || number.fract() != 0.0 || number < 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("a revision is a whole number, and {number} is not"),
                    });
                }
                if number == 0.0 {
                    if let Some(map) = properties.as_object_mut() {
                        map.remove("revision");
                    }
                } else {
                    properties["revision"] = json!(number as u64);
                }
            }
            // The **custom** properties that are named.
            let mut custom = custom_of(&node);
            if let Some(given) = &self.custom {
                let Some(list) = given.as_object() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the custom properties are a list of pairs".to_string(),
                    });
                };
                // The **removals** first: a name that is only a different
                // spelling is taken away before it is set again.
                for (name, value) in list {
                    if value.is_null() {
                        let key = the_key(&custom, name.trim());
                        if let Some(map) = custom.as_object_mut() {
                            map.remove(&key);
                        }
                    }
                }
                for (name, value) in list {
                    if value.is_null() {
                        continue;
                    }
                    let Some(value) = value.as_str() else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the custom property `{name}` is a text or nothing"),
                        });
                    };
                    let name = name_of(name, &id)?;
                    let key = the_key(&custom, &name);
                    match value_of(value, &id, "a custom property")? {
                        Some(value) => custom[key] = json!(value),
                        None => {
                            if let Some(map) = custom.as_object_mut() {
                                map.remove(&key);
                            }
                        }
                    }
                }
            }
            // The **properties** are kept, and a document with none of
            // them has none.
            let Some(node) = session.document_mut()?.node_mut(document) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{document}`"),
                });
            };
            if properties.as_object().map(|map| map.is_empty()).unwrap_or(true) {
                if let Some(map) = node.data.as_object_mut() {
                    map.remove("properties");
                }
            } else {
                node.data["properties"] = properties.clone();
            }
            if custom.as_object().map(|map| map.is_empty()).unwrap_or(true) {
                if let Some(map) = node.data.as_object_mut() {
                    map.remove("custom_properties");
                }
            } else {
                node.data["custom_properties"] = custom.clone();
            }
            let mut properties = properties_of(node);
            if let Some(map) = properties.as_object_mut() {
                for name in BUILT_INS {
                    if !map.contains_key(name) {
                        map.insert(name.to_string(), Json::Null);
                    }
                }
            }
            Ok(Outcome::changed(format!("the document `{document}` has its properties")).with_data(json!({
                "document": document,
                "properties": properties,
                "custom": custom,
            })))

        }
    }
}
