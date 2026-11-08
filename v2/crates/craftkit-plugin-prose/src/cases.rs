//! The **case** of the text of a paragraph.
//!
//! The tree changes the case of the text five ways, and they are the
//! tree's own rules:
//!
//! - **upper**: every character is uppercased;
//! - **lower**: every character is lowercased, and the Greek capital
//!   sigma takes its final form at the end of a word — `ΣΟΦΟΣ` becomes
//!   `σοφος` and not `σοφοσ`;
//! - **toggle**: each character has its case flipped;
//! - **title**: the first letter of every word is capitalised;
//! - **sentence**: the first letter of the paragraph and the first
//!   letter after a full stop, a question mark or an exclamation mark
//!   are capitalised — and a full stop between two digits is a
//!   **decimal point**, not the end of a sentence.
//!
//! # The rules
//!
//! - The text may **change length**: `ß` uppercased is `SS`, and the
//!   **runs** move with it — the tree's own rule, the tree's own
//!   mapping.
//! - A paragraph that holds a **revision** is not touched.
//! - A **mode** that is not one of the tree's five is refused by name.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// The **modes** the tree changes the case by.
pub const MODES: [&str; 5] = ["upper", "lower", "toggle", "title", "sentence"];

/// The **case** of each character of the text, in order.
///
/// The tree's own rule: the Greek capital sigma takes its final form
/// at the end of a word, and a full stop between two digits is a
/// decimal point and not the end of a sentence.
pub fn converted(text: &str, mode: &str) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let mut out: Vec<String> = Vec::with_capacity(characters.len());
    // The **lower** form: the Greek capital sigma is the final form
    // when it ends a word.
    let lower = |at: usize, character: char| -> String {
        let before =
            at.checked_sub(1).and_then(|at| characters.get(at)).is_some_and(|before| before.is_alphabetic());
        let after = characters.get(at + 1).is_some_and(|after| after.is_alphabetic());
        if character == 'Σ' && before && !after {
            return "ς".to_string();
        }
        character.to_lowercase().collect()
    };
    // The **upper** form.
    let upper = |character: char| -> String { character.to_uppercase().collect() };
    // The **title** form: the first letter of every word.
    let mut up = mode == "title";
    // The **sentence** form: the first letter of the paragraph and the
    // first letter after a full stop.
    let mut sentence = true;
    for (at, &character) in characters.iter().enumerate() {
        let new = match mode {
            "upper" => upper(character),
            "lower" => lower(at, character),
            "toggle" => {
                if character.is_uppercase() {
                    lower(at, character)
                } else {
                    upper(character)
                }
            }
            "title" => {
                let new = if up && character.is_alphabetic() {
                    up = false;
                    upper(character)
                } else {
                    lower(at, character)
                };
                if character.is_whitespace() {
                    up = true;
                }
                new
            }
            _ => {
                let new = if sentence && character.is_alphabetic() {
                    sentence = false;
                    upper(character)
                } else {
                    lower(at, character)
                };
                let is_decimal = character == '.'
                    && at.checked_sub(1).and_then(|at| characters.get(at)).is_some_and(char::is_ascii_digit)
                    && characters.get(at + 1).is_some_and(char::is_ascii_digit);
                if matches!(character, '!' | '?') || (character == '.' && !is_decimal) {
                    sentence = true;
                }
                new
            }
        };
        out.push(new);
    }
    out
}

/// The **text** with its case changed, and the **runs** that move with
/// it.
pub fn recased(
    text: &str,
    runs: &[craftkit_sdk::serde_json::Value],
    mode: &str,
) -> (String, Vec<craftkit_sdk::serde_json::Value>) {
    let parts = converted(text, mode);
    let mut out = String::with_capacity(text.len());
    for part in &parts {
        out.push_str(part);
    }
    // The **runs**: every run keeps the characters it covers, and its
    // length is the length of what those characters became.
    let lengths: Vec<usize> = parts.iter().map(|part| part.chars().count()).collect();
    let mut new_runs: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
    let mut start = 0usize;
    for run in runs {
        let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
        let end = start + length;
        let new_length: usize = lengths.iter().skip(start).take(end.saturating_sub(start)).sum();
        let mut run = run.clone();
        run["len"] = json!(new_length);
        if new_length > 0 {
            new_runs.push(run);
        }
        start = end;
    }
    (out, new_runs)
}

/// **Change** the **case** of the text of a paragraph.
///
/// The text is changed in the way the tree changes it, and the **runs**
/// move with the text.
#[derive(Command)]
#[command(id = "prose.block.case", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetCase {
    /// The paragraph.
    pub block: NodeRef,
    /// The way the case is changed: `upper`, `lower`, `toggle`, `title`
    /// or `sentence`.
    pub mode: craftkit_sdk::Text,
}

impl SetCase {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.case".to_string();
        let block = self.block.0;
        let mode = self.mode.0.trim().to_lowercase();
        if !MODES.contains(&mode.as_str()) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the mode `{mode}` is not one of the tree's"),
            });
        }
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
        let runs = node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default();
        for run in &runs {
            if run.get("revision").is_some() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the paragraph `{block}` holds a revision"),
                });
            }
        }
        let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
        let (text, runs) = recased(&text, &runs, &mode);
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        node.data["text"] = json!(text.clone());
        node.data["runs"] = craftkit_sdk::serde_json::Value::Array(runs.clone());
        Ok(Outcome::changed(format!("the paragraph `{block}` has its case changed"))
            .with_data(json!({ "block": block, "mode": mode, "text": text })))
    }
}
