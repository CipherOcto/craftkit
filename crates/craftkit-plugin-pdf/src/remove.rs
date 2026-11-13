//! **Taking away** an object of the file.
//!
//! A PDF file is a set of **indirect objects**, and a set of objects is
//! only a file when the objects that are left still refer to things
//! that are there. Taking an object away therefore cannot be only a
//! matter of dropping a node: the **references** that point at it have
//! to go with it, or the file that is written names objects that are
//! not in it.
//!
//! So this module takes the object away and takes its references away
//! with it — the **edges** of the kind `ref` die with the node (the
//! SDK does that), and the **values in the dictionaries** are taken
//! away as well, because the file is written from the dictionaries and
//! not from the edges.
//!
//! A reference is a **value** as well as an edge, and it is a value
//! that may sit alone under a key (`/Parent`) or in a **list** under a
//! key (`/Kids`). A list loses the one element that names the object
//! that is gone, and keeps the rest; a key whose list becomes empty
//! goes too, because an empty list is not what the file had.
//!
//! And the **number** of the object that is gone is **free**: it is
//! not used again, and the file says so.
//!
//! # What it does not do
//!
//! - **The root cannot go.** The file needs a root, and the root is
//!   the object whose type is `Catalog`; taking it away would leave a
//!   file with no root at all, so it is refused.
//! - **The bytes of a stream stay.** The bytes of a stream live in a
//!   file beside the document, and taking the object away does not
//!   take the file away: an orphaned file is not a broken document,
//!   but it is a file that is left behind.
//! - **What the object held stays.** An annotation of a page that is
//!   taken away is an object of its own, and it stays in the document —
//!   it is simply an object that nothing refers to any more.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

use crate::{entries_of, pairs_of, the_number_of};

commands! {
    /// Take an **indirect object** away.
    ///
    /// The object is gone and the references that named it are gone with
    /// it: the edges of the kind `ref` die with the node, and the values
    /// in the dictionaries that named it are taken away — a key that held
    /// one reference loses the key, and a key that held a list loses the
    /// one element that named it.
    ///
    /// The **root** of the file is refused: a file without a root is not a
    /// file.
    pdf.object.remove {
        area: "pdf",
        layer: "pdf",
        icon: "trash",
        mutates,
        undoable,
        params: {
            /// The object.
            object: NodeRef,
        },
        run: {
            let id = "pdf.object.remove".to_string();
            let object = self.object.0;
            let Some(node) = session.document().node(object) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{object}`"),
                });
            };
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{object}` is a `{}` and not an object", node.kind),
                });
            }
            let Some(number) = the_number_of(node) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the object `{object}` has no object number"),
                });
            };
            if entries_of(&node.data)
                .iter()
                .any(|(key, value)| key == "Type" && value.as_str() == Some("Catalog"))
            {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the object `{object}` is the root of the file, and a root cannot be taken away"
                    ),
                });
            }
            // The **references** to it, in the dictionaries of the objects
            // that hold them. They are found in the dictionaries and not in
            // the edges, because the file is written from the dictionaries.
            let holders: Vec<Holder> = session
                .document()
                .walk()
                .filter(|node| node.id != object && (node.kind == "pdf.object" || node.kind == "pdf.stream"))
                .filter_map(|node| {
                    let entries = entries_of(&node.data);
                    let kept = without(&entries, number);
                    if kept == entries {
                        return None;
                    }
                    let gone: Vec<String> = entries
                        .iter()
                        .map(|(key, _)| key.clone())
                        .filter(|key| !kept.iter().any(|(name, _)| name == key))
                        .collect();
                    Some(Holder { id: node.id, kept, gone })
                })
                .collect();
            let mut unlinked: Vec<serde_json::Value> = Vec::new();
            for holder in holders {
                for key in holder.gone {
                    unlinked.push(serde_json::json!({ "object": holder.id, "key": key }));
                }
                let data = serde_json::json!({
                    "object": session
                        .document()
                        .node(holder.id)
                        .and_then(|node| node.data.get("object").cloned())
                        .unwrap_or(serde_json::Value::Null),
                    "entries": pairs_of(&holder.kept),
                });
                if let Some(node) = session.document_mut()?.node_mut(holder.id) {
                    node.data = data;
                }
            }
            if !session.document_mut()?.remove_node(object) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{object}`"),
                });
            }
            // The **number** of the object that is gone is **free**: it is
            // not used again, and the file says so.
            crate::free::free_number(session.document_mut()?, number, 0);
            Ok(Outcome::changed(format!("the object `{object}` is gone"))
                .with_data(serde_json::json!({ "id": object, "object": number, "unlinked": unlinked })))

        }
    }
}

/// An object whose **dictionary** named the object that is gone.
struct Holder {
    /// The node that holds the dictionary.
    id: u64,
    /// The dictionary, with the references taken away.
    kept: Vec<(String, serde_json::Value)>,
    /// The keys that went.
    gone: Vec<String>,
}

/// The entries of a dictionary, with the references to an object number
/// taken away.
///
/// A value that **is** a reference to the number loses its key, and a
/// value that is a **list** loses the elements that name the number —
/// and the key too, when the list is empty afterwards.
pub(crate) fn without(
    entries: &[(String, serde_json::Value)],
    number: u64,
) -> Vec<(String, serde_json::Value)> {
    let mut kept: Vec<(String, serde_json::Value)> = Vec::with_capacity(entries.len());
    for (key, value) in entries {
        if names(value, number) {
            continue;
        }
        if let Some(list) = value.as_array() {
            let rest: Vec<serde_json::Value> =
                list.iter().filter(|item| !names(item, number)).cloned().collect();
            if rest.len() != list.len() {
                if rest.is_empty() {
                    continue;
                }
                kept.push((key.clone(), serde_json::Value::Array(rest)));
                continue;
            }
        }
        kept.push((key.clone(), value.clone()));
    }
    kept
}

/// Whether a value names an object number.
///
/// A reference is a value of the shape `{"ref": [number, generation]}`.
fn names(value: &serde_json::Value, number: u64) -> bool {
    crate::number_of(value).is_some_and(|named| named == number)
}
