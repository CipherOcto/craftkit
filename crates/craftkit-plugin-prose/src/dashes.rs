//! The **dashes** of a paragraph.
//!
//! Two hyphens are a mark of the keyboard; a **dash** is the mark of
//! the book: an *en dash* — `–` — between two things, and an *em
//! dash* — `—` — inside a word.
//!
//! # The rules
//!
//! - A **pair of hyphens** is a dash. A run of three or more hyphens
//!   is not: those are the lines a border is made of, and the tree
//!   leaves them alone.
//! - A pair of hyphens that has **no space before it** is an **em
//!   dash**: `word--word` becomes `word—word`.
//! - A pair of hyphens that has a **space before it** and a **space
//!   after it** is an **en dash**: `word -- word` becomes
//!   `word – word`.
//! - A pair of hyphens at the **end** of the text is left alone: there
//!   is nothing after it and it says nothing.
//! - The **runs** of the paragraph are rewritten with the text, because
//!   two hyphens become one dash and the text is one character
//!   shorter. A run that covers no words after the change is dropped.
//! - A paragraph with a **revision** in it is not touched.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// A **dash** to make: where its pair of hyphens is, and whether it is
/// an en dash or an em dash.
struct Dash {
    /// The character the pair of hyphens begins at.
    at: usize,
    /// Whether it is an en dash.
    en: bool,
}

/// The **dashes** of a text: the pairs of hyphens that are dashes.
fn dashes_of(text: &str) -> Vec<Dash> {
    let characters: Vec<char> = text.chars().collect();
    let mut dashes = Vec::new();
    let mut at = 0usize;
    while at < characters.len() {
        if characters[at] != '-' {
            at += 1;
            continue;
        }
        // The **run** of hyphens.
        let mut end = at;
        while end < characters.len() && characters[end] == '-' {
            end += 1;
        }
        // A run of three or more is a line, not a dash, and a pair at
        // the end of the text says nothing.
        if end - at == 2 && end < characters.len() {
            let before = if at == 0 { None } else { Some(characters[at - 1]) };
            let after = characters[end];
            let spaced = before.map(|before| before.is_whitespace()).unwrap_or(true);
            if !spaced {
                dashes.push(Dash { at, en: false });
            } else if after.is_whitespace() {
                dashes.push(Dash { at, en: true });
            }
        }
        at = end;
    }
    dashes
}

/// The text with its **dashes** made, and the **runs** with it.
fn rewritten(text: &str, runs: &[Json]) -> (String, Vec<Json>) {
    let dashes = dashes_of(text);
    let characters: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut at = 0usize;
    for dash in &dashes {
        for character in &characters[at..dash.at] {
            out.push(*character);
        }
        out.push(if dash.en { '–' } else { '—' });
        at = dash.at + 2;
    }
    for character in &characters[at.min(characters.len())..] {
        out.push(*character);
    }
    // The **runs**: every boundary moves back by the number of the
    // dashes that are before it.
    let moved = |offset: usize| -> usize { offset - dashes.iter().filter(|dash| dash.at < offset).count() };
    let mut new_runs = Vec::new();
    let mut start = 0usize;
    for run in runs {
        let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
        let end = start + length;
        let new_start = moved(start);
        let new_end = moved(end);
        let mut run = run.clone();
        run["len"] = json!(new_end.saturating_sub(new_start));
        if new_end > new_start {
            new_runs.push(run);
        }
        start = end;
    }
    (out, new_runs)
}

/// **Make** the **dashes** of a paragraph typographic.
///
/// The pairs of hyphens become en dashes and em dashes, and the
/// **runs** are rewritten with the text.
#[derive(Command)]
#[command(id = "prose.block.dashes", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetDashes {
    /// The paragraph.
    pub block: NodeRef,
}

impl SetDashes {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.dashes".to_string();
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
        let (text, runs) = rewritten(&text, &runs);
        let changed = text.chars().filter(|character| *character == '–' || *character == '—').count();
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        node.data["text"] = json!(text.clone());
        node.data["runs"] = Json::Array(runs.clone());
        Ok(Outcome::changed(format!("the paragraph `{block}` has its dashes made"))
            .with_data(json!({ "block": block, "text": text, "dashes": changed })))
    }
}
