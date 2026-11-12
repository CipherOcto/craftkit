//! The **information** of a document.
//!
//! A PDF file has a **trailer**, and the trailer may name an
//! **information dictionary**: the object that says who wrote the file,
//! what it is called and when it was made. It is the `/Info` of the
//! format.
//!
//! Here the information dictionary is an **ordinary object** — a node
//! of the kind `pdf.object` — and the document says which object it is
//! with a node of its own. That is the honest port: the format says
//! the information is an object, and a thing that is an object is an
//! object here too.
//!
//! # What the port found
//!
//! - **The tree's information is a table of its own**: the properties
//!   of a document — its title, its author, its subject, its keywords,
//!   its creator — are fields with names, and a document that has one
//!   of them says so. Here the information is a **dictionary of
//!   entries**, and the entries are the object's own: the tree's
//!   names are the PDF's names, and the PDF's names are open.
//! - **The tree's trailer writes the information under a name of its
//!   own** (`DOCPROPS` in its own file format). Here the trailer
//!   writes `/Info`, which is what a PDF file says.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome};

/// The **information node** of a document: the node that says which
/// object is the information dictionary.
fn the_information_node(document: &Document) -> Option<&Node> {
    document.walk().find(|node| node.kind == "pdf.info")
}

/// The **information** of a document: the object that says who wrote
/// it.
///
/// The information is found in one of two ways:
///
/// - a node of the kind `pdf.info` **names** the object — that is how
///   a document made in this tree says which object it is, and the
///   name is the **node** of the object;
/// - the **trailer** names it with `/Info`, which is what a file says,
///   and the name is the **number** of the object.
///
/// A document that says neither has no information.
pub fn the_information(document: &Document) -> Option<&Node> {
    if let Some(info) = the_information_node(document) {
        let id = info.data.get("object").and_then(|id| id.as_u64())?;
        return document.node(id);
    }
    let value = crate::trailer::entry_of(document, "Info")?;
    let number = crate::number_of(&value)?;
    document.walk().find(|node| node.data.get("object").and_then(|held| held.as_u64()) == Some(number))
}

/// The **text** an entry of the information holds.
pub fn text_of_entry(document: &Document, key: &str) -> Option<String> {
    let entries = crate::entries_of(&the_information(document)?.data);
    let (_, value) = entries.into_iter().find(|(name, _)| name == key)?;
    crate::fields::text_of(&value)
}

commands! {
    /// Say which object is the file's **information**.
    pdf.info.set {
        area: "pdf",
        layer: "pdf",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The object that is the information dictionary.

            object: NodeRef,
        },
        run: {

            let id = "pdf.info.set".to_string();
            let object = self.object.0;
            let Some(node) = session.document().node(object) else {
                return Err(EngineError::Failed { id, message: format!("there is no node `{object}`") });
            };
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{object}` is a `{}` and not an object", node.kind),
                });
            }
            let number = node.data.get("object").and_then(|number| number.as_u64()).ok_or_else(|| {
                EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{object}` has no object number"),
                }
            })?;
            // There is **one** information dictionary in a file, and a
            // second one would be a second answer to one question.
            if let Some(already) = the_information_node(session.document()) {
                let already_object = already.data.get("object").and_then(|object| object.as_u64()).unwrap_or(0);
                if already_object != object {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("the file already has the information of the node `{already_object}`"),
                    });
                }
                return Ok(Outcome::changed(format!("the object `{number}` is the information of the file"))
                    .with_data(json!({ "object": object, "number": number })));
            }
            let info = session.document_mut()?.add_node_with_under(
                None,
                "the information".to_string(),
                "pdf.info",
                None,
                json!({ "object": object }),
            );
            Ok(Outcome::changed(format!("the object `{number}` is the information of the file"))
                .with_data(json!({ "id": info, "object": object, "number": number })))

        }
    }
    /// **Read** the **information** of a PDF file.
    ///
    /// A query: it does not change the document. A file that has no
    /// information dictionary has no information, and the answer says so.
    pdf.info.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The entry to read.
            ///
            /// Leave it out and everything the information says is read.

            key: Option<craftkit_sdk::Text>,
        },
        run: {

            let id = "pdf.info.read".to_string();
            let document = session.document();
            let Some(information) = crate::information::the_information(document) else {
                return Ok(Outcome::new("the file has no information").with_data(json!({
                    "has_information": false,
                    "entries": Vec::<Json>::new(),
                })));
            };
            let entries = crate::entries_of(&information.data);
            let listed: Vec<Json> = match &self.key {
                None => entries.iter().map(|(key, value)| json!([key.clone(), value.clone()])).collect(),
                Some(key) => {
                    let key = key.0.trim();
                    let one: Vec<Json> = entries
                        .iter()
                        .filter(|(name, _)| name == key)
                        .map(|(name, value)| json!([name.clone(), value.clone()]))
                        .collect();
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the information has no entry `{key}`"),
                        });
                    }
                    one
                }
            };
            let mut data = json!({
                "has_information": true,
                "object": information.data.get("object").cloned().unwrap_or(Json::Null),
            });
            if let Some(map) = data.as_object_mut() {
                for key in
                    ["Title", "Author", "Subject", "Keywords", "Creator", "Producer", "CreationDate", "ModDate"]
                {
                    let value = crate::information::text_of_entry(document, key)
                        .map(|value| json!(value))
                        .unwrap_or(Json::Null);
                    map.insert(key.to_string(), value);
                }
            }
            if let Some(map) = data.as_object_mut() {
                map.insert("entries".to_string(), json!(listed));
            }
            Ok(Outcome::new("the file has information").with_data(data))

        }
    }
}
