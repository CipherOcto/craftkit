//! The **number** of an object.
//!
//! An indirect object is named by its **number** in the file, and every
//! reference to it is that number. The number is therefore a fact with
//! two faces: it is the object's own, and it is in the dictionaries of
//! every object that points at it.
//!
//! So a number may not be **taken twice** — two objects with one number
//! are a file that names one object in two ways, and the file cannot
//! say which one it means — and a number may be **changed**, and when
//! it is, every reference to the object follows it. A reference that
//! was left behind would be a reference to an object that is not
//! there.
//!
//! # What it does not do
//!
//! - **The generation is not touched.** A reference names a number and
//!   a generation, and this arc moves the number and keeps the
//!   generation, because a generation says which version of an object
//!   is meant and this arc does not make a new version of anything.
//! - **The number of the root may be changed** like any other: the
//!   root is the object whose type is `Catalog`, and the file is
//!   written with the number the object has.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

use crate::{entries_of, pairs_of, the_number_of};

commands! {
    /// The **number** of an object.
    ///
    /// The number is changed and every reference to the object follows it:
    /// a reference that was left behind would name an object that is not
    /// there.
    ///
    /// A number that is **another object's** is refused, and so is a number
    /// that is not a whole number, and so is a number the object already
    /// has.
    pdf.object.number {
        area: "pdf",
        layer: "pdf",
        icon: "hash",
        mutates,
        undoable,
        params: {
            /// The object.
            object: NodeRef,
            /// The number it has in the file.
            number: Number,
        },
        run: {
            let id = "pdf.object.number".to_string();
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
            let own = entries_of(&node.data);
            let Some(old) = the_number_of(node) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the object `{object}` has no object number"),
                });
            };
            let number = self.number.0;
            if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "an object number is a whole number".to_string(),
                });
            }
            let number = number as u64;
            if number == 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "object numbers start at one".to_string(),
                });
            }
            if number == old {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the object `{object}` already has the number `{number}`"),
                });
            }
            if let Some(other) = holder_of(session, number, Some(object)) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the number `{number}` is the number of the object `{other}`"),
                });
            }
            // The **references** to it, in the dictionaries of the objects
            // that hold them.
            let holders: Vec<(u64, Vec<(String, serde_json::Value)>)> = session
                .document()
                .walk()
                .filter(|node| node.id != object && (node.kind == "pdf.object" || node.kind == "pdf.stream"))
                .filter_map(|node| {
                    let entries = entries_of(&node.data);
                    let kept = renumbered(&entries, old, number);
                    if kept == entries { None } else { Some((node.id, kept)) }
                })
                .collect();
            let mut moved: Vec<serde_json::Value> = Vec::new();
            for (holder, kept) in holders {
                let data = serde_json::json!({
                    "object": session
                        .document()
                        .node(holder)
                        .and_then(|node| node.data.get("object").cloned())
                        .unwrap_or(serde_json::Value::Null),
                    "entries": pairs_of(&kept),
                });
                if let Some(node) = session.document_mut()?.node_mut(holder) {
                    node.data = data;
                }
                moved.push(serde_json::json!({ "object": holder }));
            }
            let data = serde_json::json!({
                "object": number,
                "entries": own
                    .iter()
                    .map(|(key, value)| serde_json::json!([key, value]))
                    .collect::<Vec<_>>(),
            });
            if let Some(node) = session.document_mut()?.node_mut(object) {
                node.data = data;
                node.name = format!("object {number}");
            }
            Ok(Outcome::changed(format!("the object `{object}` has the number `{number}`"))
                .with_data(serde_json::json!({ "id": object, "object": number, "was": old, "moved": moved })))

        }
    }
}

/// The object that has a number, if one has it.
///
/// The object itself may be left out, for the object that is being
/// given the number is not in the way.
pub(crate) fn holder_of(session: &Session, number: u64, except: Option<u64>) -> Option<u64> {
    session.document().walk().find_map(|node| {
        if Some(node.id) == except {
            return None;
        }
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return None;
        }
        if the_number_of(node) == Some(number) { Some(node.id) } else { None }
    })
}

/// The entries of a dictionary, with the references to one object number
/// moved to another.
///
/// A value that names the number is changed, and a list of them is
/// changed one element at a time.
fn renumbered(
    entries: &[(String, serde_json::Value)],
    old: u64,
    new: u64,
) -> Vec<(String, serde_json::Value)> {
    entries.iter().map(|(key, value)| (key.clone(), renumbered_value(value, old, new))).collect()
}

/// A value, with the references to one object number moved to another.
fn renumbered_value(value: &serde_json::Value, old: u64, new: u64) -> serde_json::Value {
    if let Some(number) = named(value) {
        if number == old {
            let generation = value
                .get("ref")
                .and_then(|pair| pair.as_array())
                .and_then(|pair| pair.get(1))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            return serde_json::json!({ "ref": [new, generation] });
        }
        return value.clone();
    }
    if crate::objects::is_dictionary(value) {
        let entries = renumbered(&crate::objects::pairs_of(value), old, new);
        return serde_json::json!({ "entries": pairs_of(&entries) });
    }
    if let Some(list) = value.as_array() {
        return serde_json::Value::Array(list.iter().map(|item| renumbered_value(item, old, new)).collect());
    }
    value.clone()
}

/// The object number a value names, if it names one.
fn named(value: &serde_json::Value) -> Option<u64> {
    crate::number_of(value)
}
