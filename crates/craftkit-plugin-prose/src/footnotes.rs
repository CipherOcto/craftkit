//! The **footnotes** of a document.
//!
//! A footnote is a note that belongs to the foot of the page, and the
//! text that points at it is a **reference** — a run that names the
//! footnote.
//!
//! The **number** of a footnote is **derived**: it is where the
//! footnote is referenced in the document, and the first reference of
//! a footnote gives it its number. So a footnote that is referenced
//! again is not renumbered, and a footnote that is not referenced has
//! no number at all.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Text};

/// The most characters a footnote may have.
pub const MAX_TEXT: usize = 32_767;

/// The **numbers** of the notes of a document, by the note.
///
/// They are **derived**: the document is walked in order and the first
/// reference of a note gives it its number. The **footnotes** and the
/// **endnotes** are counted apart, as the tree counts them apart.
pub fn numbers(document: &Document) -> std::collections::BTreeMap<u64, u32> {
    let mut numbers: std::collections::BTreeMap<u64, u32> = std::collections::BTreeMap::new();
    let mut next_footnote = 1u32;
    let mut next_endnote = 1u32;
    for node in document.walk().filter(|node| node.kind == "prose.document") {
        for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
            let Some(runs) = block.data.get("runs").and_then(|runs| runs.as_array()) else { continue };
            for run in runs {
                // The **footnotes** and the **endnotes** are counted
                // apart, as the tree counts them apart.
                for (key, next) in [("footnote", &mut next_footnote), ("endnote", &mut next_endnote)] {
                    let Some(note) = run.get(key).and_then(|note| note.as_u64()) else { continue };
                    if numbers.contains_key(&note) {
                        continue;
                    }
                    numbers.insert(note, *next);
                    *next = next.saturating_add(1);
                }
            }
        }
    }
    numbers
}

commands! {
    /// Put a **footnote** in a document.
    ///
    /// The footnote is a **node**: it is few, and a paragraph may point at
    /// it.
    prose.footnote.add {
        area: "prose",
        layer: "prose",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The document it goes in.

            document: NodeRef,
            /// What it says.

            text: Text,
        },
        run: {

            let id = "prose.footnote.add".to_string();
            let document = self.document.0;
            let Some(node) = session.document().node(document) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no document `{document}`"),
                });
            };
            if node.kind != "prose.document" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{document}` is a `{}` and not a document", node.kind),
                });
            }
            let text = self.text.0.clone();
            if text.chars().count() > MAX_TEXT {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a footnote is at most {MAX_TEXT} characters long"),
                });
            }
            let footnote = session.document_mut()?.add_node_with_under(
                Some(document),
                "a footnote".to_string(),
                "prose.footnote",
                None,
                json!({ "text": text.clone() }),
            );
            Ok(Outcome::changed(format!("the document `{document}` has a footnote"))
                .with_data(json!({ "id": footnote, "document": document, "text": text })))

        }
    }
    /// **Read** the footnotes of a block.
    ///
    /// The answer is every run that points at a footnote, with the
    /// **number** of the footnote — derived — and what it says.
    prose.block.footnotes {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The block.

            block: NodeRef,
        },
        run: {

            let id = "prose.block.footnotes".to_string();
            let block = self.block.0;
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed { id, message: format!("there is no block `{block}`") });
            };
            if !node.kind.starts_with("prose.block.") {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{block}` is a `{}` and not a block", node.kind),
                });
            }
            let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
            let characters: Vec<char> = text.chars().collect();
            let document = session.document();
            let numbers = numbers(document);
            let mut footnotes: Vec<Json> = Vec::new();
            let mut at = 0usize;
            let runs = node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default();
            for (index, run) in runs.iter().enumerate() {
                let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
                let end = at.saturating_add(length).min(characters.len());
                let covered: String = characters[at.min(characters.len())..end].iter().collect();
                at = end;
                let Some(footnote) = run.get("footnote").and_then(|footnote| footnote.as_u64()) else { continue };
                let said =
                    document.node(footnote).and_then(|node| node.data.get("text").cloned()).unwrap_or(Json::Null);
                let number = numbers.get(&footnote).copied().map(Json::from).unwrap_or(Json::Null);
                footnotes.push(json!({
                    "run": index,
                    "footnote": footnote,
                    "number": number,
                    "text": said,
                    "reference": covered,
                }));
            }
            Ok(Outcome::new(format!("the block `{block}` has {} footnote(s)", footnotes.len())).with_data(
                json!({
                    "block": block,
                    "footnotes": footnotes,
                }),
            ))

        }
    }
}
