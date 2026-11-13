//! The **fields** of a block.
//!
//! A field is a piece of a paragraph that says something on its own:
//! the date, the title of the document, the name of the author, the
//! number of a page. The paragraph carries the **code** — the
//! instruction, such as `DATE` or `TITLE` — and the words the field
//! **shows**.
//!
//! # The rules
//!
//! - **A field is an interval over the text of its block.** It begins
//!   at one character and ends at another, and the words between them
//!   are its **result**. The result is **derived**: it is worked out
//!   every time from the text, and it is never written down, so it
//!   cannot be out of date.
//! - **A field has a name**, and the name is **where it begins**. Two
//!   fields cannot begin at the same character, and a field that is
//!   named is the one that begins there.
//! - **Two fields cannot cover the same words.** A field that covers
//!   words another field covers is refused, and the block is left as
//!   it was.
//! - **The code may not be empty**, and it is at most **4096**
//!   characters long. A code that is longer is refused.
//! - **A field is locked against change.** A locked field may not be
//!   changed and may not be unlinked.
//! - **Unlinking a field leaves its words alone.** The words a field
//!   covers were ordinary words all along, and they stay ordinary
//!   words; what is taken away is the field itself.
//!
//! # What the port found
//!
//! - **The tree keeps the result beside the code, and this tree
//!   derives it.** The tree's field carries the instruction *and* a
//!   cached result, and the cached result is what the field shows
//!   until the fields are updated. Here the words a field covers are
//!   its result, and they are worked out every time — so a field
//!   cannot show a result that its text does not say, and there is
//!   nothing to update. That is the same rule as the **runs** of a
//!   paragraph (§ the fourth instance of "derived values are
//!   derived"), and here the derived thing is the result of a field.
//! - **The tree's field is an object in the text, and this tree's
//!   field is an interval.** The tree's field stands in the text as
//!   one placeholder character, and the result is *beside* it. Here
//!   the field covers the words themselves. The consequence is
//!   visible when a field is unlinked: the tree rewrites the text and
//!   puts the result in the field's place, and here the text does not
//!   change at all — the words were already there.
//! - **The tree's lock keeps a result across an update, and this
//!   tree has no update.** So the lock is said here as a **refusal**:
//!   a locked field may not be changed and may not be unlinked. It is
//!   the same promise with nothing behind it but the refusal.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The longest **code** a field may have.
pub const MAX_CODE: usize = 4096;

/// The **text** of a block.
fn text_of(node: &Node) -> String {
    node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string()
}

/// The **fields** of a block, as they are kept.
pub fn fields_of(node: &Node) -> Vec<Json> {
    node.data.get("fields").and_then(|fields| fields.as_array()).cloned().unwrap_or_default()
}

/// The **words** a field covers.
///
/// A field that runs past the end of the text covers what is there and
/// stops — the tree's rule for a run, and the rule for a field here.
fn result_of(text: &str, field: &Json) -> String {
    let characters: Vec<char> = text.chars().collect();
    let start = field.get("start").and_then(|start| start.as_u64()).unwrap_or(0) as usize;
    let end = field.get("end").and_then(|end| end.as_u64()).unwrap_or(0) as usize;
    let start = start.min(characters.len());
    let end = end.min(characters.len()).max(start);
    characters[start..end].iter().collect()
}

/// The **fields** of a block, each with the **result** it covers.
pub fn fields_with_result(node: &Node) -> Json {
    let text = text_of(node);
    Json::Array(fields_of(node).iter().map(|field| with_result(&text, field)).collect())
}

/// A field with the **result** it covers.
fn with_result(text: &str, field: &Json) -> Json {
    let mut field = field.clone();
    field["result"] = json!(result_of(text, &field));
    field
}

/// The **block** a field belongs to.
fn the_block(session: &Session, command: &str, block: u64) -> Result<(), EngineError> {
    let Some(node) = session.document().node(block) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no block `{block}`"),
        });
    };
    if !node.kind.starts_with("prose.block.") {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{block}` is a `{}` and not a block", node.kind),
        });
    }
    Ok(())
}

/// A number that says **where** in a text, in characters.
fn offset_of(value: f64, command: &str, what: &str) -> Result<usize, EngineError> {
    if !value.is_finite() {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("{what} is a number that is there: {value}"),
        });
    }
    if value < 0.0 || value.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("{what} is a whole number of characters, and {value} is not"),
        });
    }
    Ok(value as usize)
}

/// The **code** a field says.
fn code_of(code: &str, command: &str) -> Result<String, EngineError> {
    let code = code.trim();
    if code.is_empty() {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "a field says what it is, and this one says nothing".to_string(),
        });
    }
    if code.chars().count() > MAX_CODE {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("a code is at most {MAX_CODE} characters long"),
        });
    }
    Ok(code.to_string())
}

/// The field that **begins** at a place.
fn field_at(fields: &[Json], start: usize) -> Option<Json> {
    fields
        .iter()
        .find(|field| field.get("start").and_then(|start| start.as_u64()).unwrap_or(u64::MAX) == start as u64)
        .cloned()
}

commands! {
    /// Put a **field** on a block.
    ///
    /// The field covers the words between `start` and `end`, and it says
    /// what it is with its code.
    prose.block.field.add {
        area: "prose",
        layer: "prose",
        icon: "link",
        mutates,
        undoable,
        params: {
            /// The block.
            block: NodeRef,
            /// Where the field begins, in characters.
            start: Number,
            /// Where the field ends, in characters.
            end: Number,
            /// The instruction of the field.
            code: Text,
            /// Whether the field is locked against change.
            locked: Option<Bool>,
        },
        run: {
            let id = "prose.block.field.add".to_string();
            let block = self.block.0;
            the_block(session, &id, block)?;
            let code = code_of(&self.code.0, &id)?;
            let start = offset_of(self.start.0, &id, "the start of a field")?;
            let end = offset_of(self.end.0, &id, "the end of a field")?;
            if start >= end {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a field covers at least one character, and {start} to {end} covers none"),
                });
            }
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            let text = text_of(node);
            let characters = text.chars().count();
            if end > characters {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the text of the block has {characters} characters, and the field ends at {end}"
                    ),
                });
            }
            // Two fields may not cover the same words.
            let fields = fields_of(node);
            for field in &fields {
                let other_start = field.get("start").and_then(|start| start.as_u64()).unwrap_or(0) as usize;
                let other_end = field.get("end").and_then(|end| end.as_u64()).unwrap_or(0) as usize;
                if start < other_end && other_start < end {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the field covers words another field covers: the field at {other_start}"
                        ),
                    });
                }
            }
            let field = json!({
                "start": start,
                "end": end,
                "code": code,
                "locked": self.locked.map(|locked| locked.0).unwrap_or(false),
            });
            // The **fields** are kept in the order they begin in.
            let mut fields = fields;
            fields.push(field.clone());
            fields.sort_by_key(|field| field.get("start").and_then(|start| start.as_u64()).unwrap_or(0));
            let Some(node) = session.document_mut()?.node_mut(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            node.data["fields"] = Json::Array(fields);
            Ok(Outcome::changed(format!("the block `{block}` has a field at {start}"))
                .with_data(json!({ "block": block, "field": with_result(&text, &field) })))

        }
    }

    /// **Change** a **field** of a block.
    ///
    /// The field is named by **where it begins**, and a locked field is
    /// not changed at all.
    prose.block.field.set {
        area: "prose",
        layer: "prose",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The block.
            block: NodeRef,
            /// The field: the character it begins at.
            field: Number,
            /// The instruction of the field.
            code: Option<Text>,
            /// Whether the field is locked against change.
            locked: Option<Bool>,
        },
        run: {
            let id = "prose.block.field.set".to_string();
            let block = self.block.0;
            the_block(session, &id, block)?;
            if self.code.is_none() && self.locked.is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a field is changed or it is not".to_string(),
                });
            }
            let start = offset_of(self.field.0, &id, "the name of a field")?;
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            let mut fields = fields_of(node);
            let Some(mut field) = field_at(&fields, start) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no field at {start} in the block `{block}`"),
                });
            };
            if field.get("locked").and_then(|locked| locked.as_bool()).unwrap_or(false) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the field at {start} is locked"),
                });
            }
            if let Some(code) = &self.code {
                field["code"] = json!(code_of(&code.0, &id)?);
            }
            if let Some(locked) = &self.locked {
                field["locked"] = json!(locked.0);
            }
            let text = text_of(node);
            if let Some(held) = fields.iter_mut().find(|held| {
                held.get("start").and_then(|start| start.as_u64()).unwrap_or(u64::MAX) == start as u64
            }) {
                *held = field.clone();
            }
            let Some(node) = session.document_mut()?.node_mut(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            node.data["fields"] = Json::Array(fields);
            Ok(Outcome::changed(format!("the field at {start} of the block `{block}` is changed"))
                .with_data(json!({ "block": block, "field": with_result(&text, &field) })))

        }
    }

    /// **Read** the **fields** of a block, each with the **result** it
    /// covers.
    prose.block.field.read {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The block.
            block: NodeRef,
        },
        run: {
            let id = "prose.block.field.read".to_string();
            let block = self.block.0;
            the_block(session, &id, block)?;
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            let fields = fields_with_result(node);
            Ok(Outcome::new(format!(
                "the block `{block}` has {} fields",
                fields.as_array().map(|f| f.len()).unwrap_or(0)
            ))
            .with_data(json!({ "block": block, "fields": fields })))

        }
    }

    /// **Unlink** a **field** of a block.
    ///
    /// The field is taken away and its **words stay**: they were ordinary
    /// words all along. A locked field is not unlinked.
    prose.block.field.unlink {
        area: "prose",
        layer: "prose",
        icon: "trash",
        mutates,
        undoable,
        params: {
            /// The block.
            block: NodeRef,
            /// The field: the character it begins at.
            field: Number,
        },
        run: {
            let id = "prose.block.field.unlink".to_string();
            let block = self.block.0;
            the_block(session, &id, block)?;
            let start = offset_of(self.field.0, &id, "the name of a field")?;
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            let mut fields = fields_of(node);
            let Some(field) = field_at(&fields, start) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no field at {start} in the block `{block}`"),
                });
            };
            if field.get("locked").and_then(|locked| locked.as_bool()).unwrap_or(false) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the field at {start} is locked"),
                });
            }
            fields.retain(|held| {
                held.get("start").and_then(|start| start.as_u64()).unwrap_or(u64::MAX) != start as u64
            });
            let Some(node) = session.document_mut()?.node_mut(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no block `{block}`"),
                });
            };
            node.data["fields"] = Json::Array(fields);
            Ok(Outcome::changed(format!("the field at {start} of the block `{block}` is unlinked"))
                .with_data(json!({ "block": block, "field": field })))

        }
    }
}
