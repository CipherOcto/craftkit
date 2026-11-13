//! The **note** on a cell.
//!
//! A cell may carry a **note**: a piece of text that says something
//! about the cell and is not the cell's value. It is the tree's
//! *note* — the yellow box in the corner of a cell — and it is not the
//! tree's *comment*, which is a thread of replies (see the module's
//! note on what the port did not settle).
//!
//! A note belongs to the **sheet** and names a cell, and that is where
//! it is kept: the tree keeps its comments in a map of the sheet and
//! not in the cell, because a cell in the tree is a value and a
//! formula and a style, and a note is none of those three.
//!
//! So a note **survives** the cell being written to — the value of a
//! cell may change and the note stays — and it goes only when the note
//! is taken away or when the **whole cell** is cleared, which is the
//! tree's rule: the tree's *clear all* takes the comment of the cell
//! with it, and its *clear contents* and *clear formats* do not.

use std::collections::BTreeMap;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// A **note**: the text of it, and who wrote it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Note {
    /// The text of the note.
    pub text: String,
    /// Who wrote it, when that is known.
    pub author: Option<String>,
}

/// The **notes** of a sheet: by the address of the cell they are on.
pub fn notes_of(node: &Node) -> BTreeMap<String, Note> {
    let mut notes = BTreeMap::new();
    let Some(map) = node.data.get("notes").and_then(|notes| notes.as_object()) else {
        return notes;
    };
    for (address, note) in map {
        let Some(text) = note.get("text").and_then(|text| text.as_str()) else { continue };
        let author = note.get("author").and_then(|author| author.as_str()).map(|author| author.to_string());
        notes.insert(address.to_string(), Note { text: text.to_string(), author });
    }
    notes
}

/// The notes of a sheet, as they are kept.
pub fn notes_into(notes: &BTreeMap<String, Note>) -> Json {
    let mut map = craftkit_sdk::serde_json::Map::new();
    for (address, note) in notes {
        let mut one = craftkit_sdk::serde_json::Map::new();
        one.insert("text".to_string(), Json::String(note.text.clone()));
        if let Some(author) = &note.author {
            one.insert("author".to_string(), Json::String(author.clone()));
        }
        map.insert(address.clone(), Json::Object(one));
    }
    Json::Object(map)
}

/// The sheet a command is about, or the error that says there is no
/// such sheet.
pub(crate) fn sheet_of(session: &Session, sheet: u64, id: &str) -> Result<String, EngineError> {
    let Some(node) = session.document().node(sheet) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
        });
    }
    Ok(node.name.clone())
}

commands! {
    /// Put a **note** on a cell.
    ///
    /// A note that is already on the cell is **edited**: the text is the
    /// new text and the author is kept unless a new one is given.
    grid.cell.note.set {
        area: "grid",
        layer: "grid",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
            /// The text of the note.
            text: Text,
            /// Who wrote it.
            author: Option<Text>,
        },
        run: {
            let id = "grid.cell.note.set".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let text = self.text.0.clone();
            if text.is_empty() {
                return Err(EngineError::Failed { id: id.clone(), message: "a note holds text".to_string() });
            }
            sheet_of(session, sheet, &id)?;
            let author = self.author.as_ref().map(|author| author.0.clone()).filter(|author| !author.is_empty());
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut notes = notes_of(node);
            let address = cell.to_text();
            let author = author.or_else(|| notes.get(&address).and_then(|note| note.author.clone()));
            notes.insert(address.clone(), Note { text, author: author.clone() });
            node.data["notes"] = notes_into(&notes);
            // A **note replaces a comment**: the tree's own rule.
            let mut comments = crate::comments::comments_of(node);
            let took_comment = comments.remove(&address).is_some();
            node.data["comments"] = crate::comments::comments_into(&comments);
            let who = match &author {
                Some(author) => format!(" by {author}"),
                None => String::new(),
            };
            Ok(Outcome::changed(format!("wrote a note on {address}{who}")).with_data(json!({
                "sheet": sheet,
                "cell": address,
                "text": notes.get(&address).map(|note| note.text.clone()),
                "author": author,
                "took_comment": took_comment,
            })))

        }
    }

    /// The **note** on a cell.
    grid.cell.note.get {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
        },
        run: {
            let id = "grid.cell.note.get".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            sheet_of(session, sheet, &id)?;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let notes = notes_of(node);
            let address = cell.to_text();
            let note = notes.get(&address);
            let text = note.map(|note| note.text.clone());
            Ok(Outcome::new(match &text {
                Some(_) => format!("{address} holds a note"),
                None => format!("{address} holds no note"),
            })
            .with_data(json!({
                "sheet": sheet,
                "cell": address,
                "note": match note {
                    Some(note) => json!({
                        "text": note.text,
                        "author": note.author,
                    }),
                    None => Json::Null,
                },
            })))

        }
    }

    /// Take the **note** away from a cell.
    ///
    /// A cell that holds no note is not refused: there is nothing to take
    /// away, and the answer says so.
    grid.cell.note.delete {
        area: "grid",
        layer: "grid",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
        },
        run: {
            let id = "grid.cell.note.delete".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            sheet_of(session, sheet, &id)?;
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut notes = notes_of(node);
            let address = cell.to_text();
            let was_there = notes.remove(&address).is_some();
            node.data["notes"] = notes_into(&notes);
            let outcome = if was_there {
                Outcome::changed(format!("took the note off {address}"))
            } else {
                Outcome::new(format!("{address} held no note"))
            };
            Ok(outcome.with_data(json!({
                "sheet": sheet,
                "cell": address,
                "was_there": was_there,
            })))

        }
    }
}
