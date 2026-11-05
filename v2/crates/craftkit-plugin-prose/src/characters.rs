//! The **character** of a run.
//!
//! A run is an **interval** over the text of a block, and it says how
//! the text it covers is written: bold, italic, underlined, in what
//! colour and in what size.
//!
//! Here the character of a range of text is **set in one place**: the
//! command is given a block, a **start** and a **length** — counted in
//! characters, not in bytes — and the runs are **split** so that the
//! range lies in whole runs, and the character is put on those runs
//! and on no others.
//!
//! The rules that hold it together:
//!
//! - the range must be **inside the text**: a range that reaches past
//!   the end is **refused** and not cut short;
//! - the command must say **something to change**: a command that
//!   changes nothing is refused;
//! - a **colour** is the kit's colour, `#rrggbb`, and a text that is
//!   not one is refused;
//! - a **size** is a number that is finite and above zero, and one
//!   that is not is refused;
//! - a **tracked change** is not formatted: a run that carries a
//!   revision is left as it is, and the command is refused rather than
//!   quietly changing half of it.
//!
//! # What the port found
//!
//! - **The tree has a command for each of the characters.** The tree's
//!   `format.bold`, `format.italic`, `format.underline`,
//!   `format.color` and `format.size` are five commands over the
//!   **selection**. Here they are one command over a **range**, because
//!   the thing is the same thing: the character of a piece of text.
//! - **The tree's selection is a place in the text and this tree's is
//!   a number.** The tree's commands act on what is selected; here the
//!   selection of the session is the tree's own, and the range of this
//!   command is **two numbers**: where the text starts and how long it
//!   is.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session, Text};

#[derive(Debug, Clone)]
/// A **piece** of the text: a stretch of it, and the run that covers
/// it, if there is one.
struct Piece {
    /// Where it begins.
    start: usize,
    /// Where it ends.
    end: usize,
    /// The run that covers it, if there is one.
    run: Option<Json>,
}

impl Piece {
    /// How long it is.
    fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }
}

/// The **pieces** of a text: the runs it has, and the text that no run
/// covers.
fn pieces_of(text_length: usize, runs: &[Json]) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    let mut at = 0usize;
    for run in runs {
        let length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
        let end = at.saturating_add(length).min(text_length);
        if end > at {
            pieces.push(Piece { start: at, end, run: Some(run.clone()) });
        }
        at = end;
    }
    if at < text_length {
        pieces.push(Piece { start: at, end: text_length, run: None });
    }
    pieces
}

/// The **pieces**, split so that the range lies in whole pieces.
fn split_at(pieces: Vec<Piece>, at: usize) -> Vec<Piece> {
    let mut out: Vec<Piece> = Vec::new();
    for piece in pieces {
        if at > piece.start && at < piece.end {
            let mut first = piece.clone();
            first.end = at;
            let mut second = piece;
            second.start = at;
            out.push(first);
            out.push(second);
            continue;
        }
        out.push(piece);
    }
    out
}

/// The **runs** of a block, with the character put on the pieces that
/// are inside the range.
fn formatted(runs: &[Json], text_length: usize, start: usize, end: usize, character: &Json) -> Vec<Json> {
    let mut pieces = pieces_of(text_length, runs);
    pieces = split_at(pieces, start);
    pieces = split_at(pieces, end);
    let mut out: Vec<Json> = Vec::new();
    for piece in pieces {
        let inside = piece.start >= start && piece.end <= end;
        let mut run = match piece.run.clone() {
            Some(run) => run,
            None => Json::Object(craftkit_sdk::serde_json::Map::new()),
        };
        if let Some(map) = run.as_object_mut() {
            map.insert("len".to_string(), Json::from(piece.len() as u64));
            if inside && let Some(character) = character.as_object() {
                for (key, value) in character {
                    if value.is_null() {
                        map.remove(key);
                        continue;
                    }
                    map.insert(key.clone(), value.clone());
                }
            }
        }
        out.push(run);
    }
    out
}

/// The **character** of a piece of text.
#[derive(Command)]
#[command(id = "prose.run.format", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct FormatRun {
    /// The block the text is in.
    pub block: NodeRef,
    /// Where the text begins, counted in characters.
    pub start: Number,
    /// How long the text is, counted in characters.
    pub length: Number,
    /// Whether the text is **bold**.
    pub bold: Option<Bool>,
    /// Whether the text is **italic**.
    pub italic: Option<Bool>,
    /// Whether the text is **underlined**.
    pub underline: Option<Bool>,
    /// The **colour** of the text, as `#rrggbb`.
    pub colour: Option<Text>,
    /// The **size** of the text.
    pub size: Option<Number>,
}

impl FormatRun {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.run.format".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        if !node.kind.starts_with("prose.block.") {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not a block", node.kind),
            });
        }
        // The **character** that is being put on: what the command says,
        // and nothing else.
        let mut character = craftkit_sdk::serde_json::Map::new();
        if let Some(bold) = &self.bold {
            character.insert("bold".to_string(), Json::Bool(bold.0));
        }
        if let Some(italic) = &self.italic {
            character.insert("italic".to_string(), Json::Bool(italic.0));
        }
        if let Some(underline) = &self.underline {
            character.insert("underline".to_string(), Json::Bool(underline.0));
        }
        if let Some(colour) = &self.colour {
            let text = colour.0.trim().to_string();
            if let Err(why) = craftkit_sdk::Colour::from_hex(&text) {
                return Err(EngineError::Failed { id: id.clone(), message: why });
            }
            character.insert("colour".to_string(), Json::String(text));
        }
        if let Some(size) = &self.size {
            let size = size.0;
            if !size.is_finite() || size <= 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{size} is not a size: a size is a number above zero"),
                });
            }
            character.insert("size".to_string(), Json::from(size));
        }
        if character.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the command says nothing to change".to_string(),
            });
        }
        // The **range**: inside the text, and not empty.
        let text = node.data.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string();
        let length = text.chars().count();
        let start = self.start.0;
        if !start.is_finite() || start < 0.0 || start.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "{start} is not a place in the text: a place is a whole number of characters"
                ),
            });
        }
        let start = start as usize;
        let span = self.length.0;
        if !span.is_finite() || span < 0.0 || span.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("{span} is not a length of text: a length is a whole number of characters"),
            });
        }
        let span = span as usize;
        if span == 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the range is empty, and an empty range changes nothing".to_string(),
            });
        }
        let end = start.saturating_add(span);
        if end > length {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the range is {span} characters from {start}, and the text has {length} characters"
                ),
            });
        }
        // The **runs** of the block.
        let runs: Vec<Json> =
            node.data.get("runs").and_then(|runs| runs.as_array()).cloned().unwrap_or_default();
        // A **tracked change** is not formatted.
        let mut at = 0usize;
        for run in &runs {
            let run_length = run.get("len").and_then(|len| len.as_u64()).unwrap_or(0) as usize;
            let run_end = at.saturating_add(run_length);
            if run.get("revision").is_some() && run_end > start && at < end {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the text is a tracked change, and a tracked change is not formatted"
                        .to_string(),
                });
            }
            at = run_end;
        }
        let new_runs = formatted(&runs, length, start, end, &Json::Object(character.clone()));
        if let Some(node) = session.document_mut()?.node_mut(block) {
            node.data["runs"] = Json::Array(new_runs);
        }
        Ok(Outcome::changed(format!("the character of {span} characters is set")).with_data(json!({
            "block": block,
            "start": start,
            "length": span,
            "character": Json::Object(character),
        })))
    }
}
