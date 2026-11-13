//! The **quotes** of a paragraph.
//!
//! A straight quote is a mark of the keyboard: it is the same whether
//! it opens a saying or closes it. A **smart quote** is the mark of
//! the book: “ and ” and ‘ and ’.
//!
//! # The rules
//!
//! - The **character before** the quote decides which one it is. It
//!   is an **opening** quote when there is nothing before it, when
//!   what is before it is a **space** or any other space, or when it
//!   is an **opening mark**: `(`, `[`, `{`, `<`, `—`, `–`, `«`, `‹`,
//!   `„` or `‚`.
//! - Two marks are the tree's own and are here as they are: a `"`
//!   after a `‘` is an **opening** quote, and a `'` after a `“` is an
//!   **opening** quote.
//! - A quote that is **already smart** is left alone: only the
//!   straight ones are changed.
//! - The **runs** of the paragraph do not move: a straight quote and
//!   a smart one are one character each, so the text keeps its
//!   length and every run covers what it covered.
//! - A paragraph with a **revision** in it is not touched: the tree
//!   refuses to rewrite a tracked change, and so does this tree.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

/// The marks that **open** a saying.
const OPENERS: &str = "([{<—–«‹„‚";

/// Whether the quote is an **opening** one, and what it becomes.
fn smart_of(before: Option<char>, character: char) -> char {
    let opening = before.is_none_or(|before| {
        before.is_whitespace()
            || OPENERS.contains(before)
            || (character == '"' && before == '‘')
            || (character == '\'' && before == '“')
    });
    match character {
        '"' => {
            if opening {
                '“'
            } else {
                '”'
            }
        }
        '\'' => {
            if opening {
                '‘'
            } else {
                '’'
            }
        }
        other => other,
    }
}

/// The text with its **straight quotes** made smart.
fn smart(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut before: Option<char> = None;
    for character in text.chars() {
        let smart = match character {
            '"' | '\'' => smart_of(before, character),
            other => other,
        };
        out.push(smart);
        before = Some(character);
    }
    out
}

commands! {
    /// **Make** the **quotes** of a paragraph smart.
    ///
    /// The straight quotes become the smart ones, and the paragraph's
    /// **runs** do not move: a quote is one character before and after.
    prose.block.quotes {
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
            let id = "prose.block.quotes".to_string();
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
            let smartened = smart(&text);
            let changed = text.chars().zip(smartened.chars()).filter(|(before, after)| before != after).count();
            if smartened == text {
                return Ok(Outcome::new(format!("the paragraph `{block}` has no straight quotes"))
                    .with_data(json!({ "block": block, "text": text, "changed": 0 })));
            }
            let Some(node) = session.document_mut()?.node_mut(block) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{block}`"),
                });
            };
            node.data["text"] = json!(smartened.clone());
            Ok(Outcome::changed(format!("the paragraph `{block}` has its quotes made smart"))
                .with_data(json!({ "block": block, "text": smartened, "changed": changed })))

        }
    }
}
