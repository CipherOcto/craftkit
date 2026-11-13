//! The **full stop** of a paragraph.
//!
//! The tree's AutoCorrect turns two spaces into a full stop: a
//! paragraph that ends with two spaces after a word ends with a full
//! stop and one space instead — the writer pressed the key twice by
//! accident, and the second space says so.
//!
//! # The rules
//!
//! - The text must **end** with two spaces, and the word before them
//!   must end with a **letter or a digit** or with a **closing mark**:
//!   `)`, `]`, `}`, `"`, `'`, `”`, `’`, `»` or `›`.
//! - The **first** of the two spaces becomes the full stop, and the
//!   second stays: "word  " becomes "word. ".
//! - The text does **not change length**, and so the **runs** do not
//!   move: a space and a full stop are one character each.
//! - A paragraph that holds a **revision** is not touched.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

/// The marks that may come **before** the two spaces.
const CLOSERS: &str = ")]}\"'\u{201D}\u{2019}\u{00BB}\u{203A}";

/// The text with its **full stop**, and whether it has one.
fn stopped(text: &str) -> Option<String> {
    let characters: Vec<char> = text.chars().collect();
    if characters.len() < 3 {
        return None;
    }
    let last = characters[characters.len() - 1];
    let before = characters[characters.len() - 2];
    if last != ' ' || before != ' ' {
        return None;
    }
    let word = characters[characters.len() - 3];
    if !(word.is_alphanumeric() || CLOSERS.contains(word)) {
        return None;
    }
    let mut out: String = characters[..characters.len() - 2].iter().collect();
    out.push('.');
    out.push(' ');
    Some(out)
}

commands! {
    /// **Make** the **full stop** of a paragraph.
    ///
    /// The two spaces at the end of the text become a full stop and a
    /// space, and the **runs** do not move.
    prose.block.stops {
        area: "prose",
        layer: "prose",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The paragraph.
            block: NodeRef,
        },
        run: {
            let id = "prose.block.stops".to_string();
            let block = self.block.0;
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{block}`"),
                });
            };
            if node.kind != "prose.block.paragraph" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{block}` is a `{}` and not a paragraph", node.kind),
                });
            }
            // A paragraph with a **revision** in it is not touched.
            if let Some(runs) = node.data.get("runs").and_then(|runs| runs.as_array()) {
                for run in runs {
                    if run.get("revision").is_some() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the paragraph `{block}` holds a revision"),
                        });
                    }
                }
            }
            let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
            let Some(stopped) = stopped(&text) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the paragraph `{block}` does not end with two spaces after a word"),
                });
            };
            let Some(node) = session.document_mut()?.node_mut(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{block}`"),
                });
            };
            node.data["text"] = json!(stopped.clone());
            Ok(Outcome::changed(format!("the paragraph `{block}` has its full stop"))
                .with_data(json!({ "block": block, "text": stopped })))

        }
    }
}
