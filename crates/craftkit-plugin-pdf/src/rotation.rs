//! The **rotation** of a page.
//!
//! A page may be turned: `/Rotate` says by how many degrees, and a
//! positive number turns the page **clockwise**. The degrees are a
//! multiple of ninety, and the number is **taken back into the
//! circle**: a page turned four times is a page not turned at all.
//!
//! The tree's finding is that the rotation is **inherited** — it is
//! among the keys a page may leave out and take from its ancestor —
//! and that a page that is turned by an amount that is not a multiple
//! of ninety is turned by **the multiple of ninety in it**: the tree's
//! own rule truncates the degrees. Here the amount is **refused**,
//! because a page turned by forty-five degrees is not a page that is
//! turned.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{self, Value};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The **entries** of an object, as the document holds them.
fn entries_of(payload: &Value) -> Vec<(String, Value)> {
    let Some(list) = payload.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            let value = pair.get(1)?.clone();
            Some((key, value))
        })
        .collect()
}

/// The **entries** of an object, into the document.
fn entries_into(payload: &Value, entries: &[(String, Value)]) -> Value {
    serde_json::json!({
        "object": payload.get("object").cloned().unwrap_or(Value::Null),
        "entries": entries
            .iter()
            .map(|(key, value)| serde_json::json!([key, value]))
            .collect::<Vec<_>>(),
    })
}

/// The **parent** of a page.
fn parent_of(session: &Session, id: u64) -> Option<u64> {
    session
        .document()
        .relations
        .iter()
        .find(|relation| {
            relation.kind == "ref"
                && relation.from == id
                && relation
                    .payload
                    .get("key")
                    .and_then(|key| key.as_str())
                    .map(|key| key == "Parent")
                    .unwrap_or(false)
        })
        .map(|relation| relation.to)
}

/// The **rotation** of a page, in degrees, as the page has it: its
/// own, or the one it inherits from an ancestor, or none at all.
pub fn rotation_of(session: &Session, id: u64) -> i64 {
    if let Some(node) = session.document().node(id)
        && let Some((_, value)) = entries_of(&node.data).iter().find(|(name, _)| name == "Rotate")
        && let Some(degrees) = crate::whole_number_of(value)
    {
        return degrees.rem_euclid(360);
    }
    let mut ancestor = parent_of(session, id);
    let mut steps = 0_usize;
    while let Some(current) = ancestor {
        steps += 1;
        if steps > 64 {
            break;
        }
        let Some(current_node) = session.document().node(current) else { break };
        if let Some((_, value)) = entries_of(&current_node.data).iter().find(|(name, _)| name == "Rotate")
            && let Some(degrees) = crate::whole_number_of(value)
        {
            return degrees.rem_euclid(360);
        }
        ancestor = parent_of(session, current);
    }
    0
}

commands! {
    /// Turn a **page**.
    ///
    /// The degrees are a **multiple of ninety**, and an amount that is not
    /// is refused: the tree's command truncates the degrees to the
    /// multiple of ninety in them, and a page turned by forty-five degrees
    /// is not a page that is turned.
    ///
    /// The new rotation is **taken back into the circle**: a page turned
    /// four times is a page not turned at all.
    pdf.page.rotate {
        area: "pdf",
        layer: "pdf",
        icon: "rotate",
        mutates,
        undoable,
        params: {
            /// The page.
            page: NodeRef,
            /// The degrees, clockwise. A multiple of ninety.
            degrees: Number,
        },
        run: {
            let id = "pdf.page.rotate".to_string();
            let page = self.page.0;
            let degrees = self.degrees.0;
            if !degrees.is_finite() || degrees.fract() != 0.0 || degrees % 90.0 != 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "a page is turned by a multiple of ninety degrees".to_string(),
                });
            }
            let Some(node) = session.document().node(page) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{page}`"),
                });
            };
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{page}` is a `{}` and not an object", node.kind),
                });
            }
            let current = rotation_of(session, page);
            let new = (current + degrees as i64).rem_euclid(360);
            let Some(node) = session.document_mut()?.node_mut(page) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{page}`"),
                });
            };
            let mut entries = entries_of(&node.data);
            match entries.iter_mut().find(|(name, _)| name == "Rotate") {
                Some(entry) => entry.1 = serde_json::json!(new),
                None => entries.push(("Rotate".to_string(), serde_json::json!(new))),
            }
            node.data = entries_into(&node.data, &entries);
            Ok(Outcome::changed(format!("the page `{page}` is turned to {new} degrees"))
                .with_data(serde_json::json!({ "id": page, "rotate": new })))

        }
    }

    /// The **rotation** of a page, as it stands.
    ///
    /// The number the page has, or the one it inherits, and the circle it
    /// is in.
    pdf.page.rotation {
        area: "pdf",
        layer: "pdf",
        icon: "rotate",
        params: {
            /// The page.
            page: NodeRef,
        },
        run: {
            let id = "pdf.page.rotation".to_string();
            let page = self.page.0;
            let Some(node) = session.document().node(page) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{page}`"),
                });
            };
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{page}` is a `{}` and not an object", node.kind),
                });
            }
            let own = entries_of(&node.data)
                .into_iter()
                .find(|(name, _)| name == "Rotate")
                .and_then(|(_, value)| crate::whole_number_of(&value));
            let degrees = rotation_of(session, page);
            Ok(Outcome::new(format!("the page `{page}` is turned {degrees} degrees"))
                .with_data(serde_json::json!({ "id": page, "rotate": degrees, "own": own })))

        }
    }
}
