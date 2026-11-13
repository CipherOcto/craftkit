//! The **notes** of a document: the footnotes and the endnotes.
//!
//! A note is a note at the foot of the page or at the end of the
//! document, and the tree converts one into the other: a footnote
//! becomes an endnote, an endnote becomes a footnote, and the two
//! swap.
//!
//! # The rules
//!
//! - The tree's own rules are the rules:
//!   - `endnote`: the **footnotes** become endnotes;
//!   - `footnote`: the **endnotes** become footnotes;
//!   - `swap`: the footnotes become endnotes and the endnotes become
//!     footnotes.
//! - A note that changes is **the same note**: it keeps its text and
//!   its place, and only its kind changes.
//! - The **reference** to a note changes with it: a run that points at
//!   a footnote that became an endnote points at an endnote.
//! - A way that is **not one of the tree's three** is refused by name.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome};

/// The **ways** the tree converts the notes.
pub const WAYS: [&str; 3] = ["endnote", "footnote", "swap"];

/// The kind a note takes when it is converted.
fn kind_of(was: &str, way: &str) -> Option<&'static str> {
    match (was, way) {
        ("prose.footnote", "endnote") | ("prose.footnote", "swap") => Some("prose.endnote"),
        ("prose.endnote", "footnote") | ("prose.endnote", "swap") => Some("prose.footnote"),
        _ => None,
    }
}

commands! {
    /// **Convert** the **notes** of a document.
    ///
    /// The footnotes and the endnotes change as the tree changes them, and
    /// the references to them change with them.
    prose.notes.convert {
        area: "prose",
        layer: "prose",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The document whose notes are converted.
            document: NodeRef,
            /// What the notes become: `endnote`, `footnote` or `swap`.
            to: craftkit_sdk::Text,
        },
        run: {
            let id = "prose.notes.convert".to_string();
            let document = self.document.0;
            let way = self.to.0.trim().to_lowercase();
            if !WAYS.contains(&way.as_str()) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the way `{way}` is not one of the tree's three"),
                });
            }
            let Some(node) = session.document().node(document) else {
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
            // The **notes** that change, and the **blocks** that may point
            // at them.
            let mut flipped: Vec<(u64, &'static str)> = Vec::new();
            let mut blocks: Vec<u64> = Vec::new();
            {
                let document_node = session.document().node(document);
                let Some(document_node) = document_node else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no node `{document}`"),
                    });
                };
                for child in document_node.children.iter() {
                    if let Some(kind) = kind_of(&child.kind, &way) {
                        flipped.push((child.id, kind));
                        continue;
                    }
                    if child.kind.starts_with("prose.block.") {
                        blocks.push(child.id);
                    }
                }
            }
            if flipped.is_empty() {
                return Ok(Outcome::new(format!("the document `{document}` has no note to convert"))
                    .with_data(json!({ "document": document, "notes": 0, "references": 0 })));
            }
            // The **notes** change their kind.
            let mut notes = 0usize;
            for (note, kind) in &flipped {
                if let Some(node) = session.document_mut()?.node_mut(*note) {
                    node.kind = kind.to_string();
                    notes += 1;
                }
            }
            // The **references** change with them.
            let mut references = 0usize;
            for block in blocks {
                let Some(node) = session.document().node(block) else { continue };
                let runs = node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default();
                let mut changed = false;
                let mut new_runs: Vec<Json> = Vec::with_capacity(runs.len());
                for run in runs {
                    let mut run = run.clone();
                    // The note this run points at, and the key it holds.
                    let pointed = run
                        .get("footnote")
                        .and_then(|held| held.as_u64())
                        .map(|held| ("footnote", held))
                        .or_else(|| {
                            run.get("endnote").and_then(|held| held.as_u64()).map(|held| ("endnote", held))
                        });
                    if let Some((was, held)) = pointed
                        && let Some((_, kind)) = flipped.iter().find(|(note, _)| *note == held)
                    {
                        let now = kind.strip_prefix("prose.").unwrap_or(was);
                        if now != was {
                            let mut map = run.as_object().cloned().unwrap_or_default();
                            map.remove(was);
                            map.insert(now.to_string(), Json::Number(held.into()));
                            run = Json::Object(map);
                            changed = true;
                        }
                    }
                    new_runs.push(run);
                }
                if changed && let Some(node) = session.document_mut()?.node_mut(block) {
                    node.data["runs"] = Json::Array(new_runs);
                    references += 1;
                }
            }
            Ok(Outcome::changed(format!("{notes} note(s) converted")).with_data(json!({
                "document": document,
                "way": way,
                "notes": notes,
                "references": references,
            })))

        }
    }
}
