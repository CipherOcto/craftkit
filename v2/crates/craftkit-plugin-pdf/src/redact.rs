//! The **text under a region is removed**.
//!
//! This is the other half of the tree's redaction: §638 says what a
//! region covers, and this takes it away. The text that is under the
//! region is gone from the content of the page, and the rest of the
//! line keeps its place — the tree's own rule.
//!
//! # The rules
//!
//! - **The removed text leaves a space of its own.** The tree's own
//!   rule: the text is replaced by a number that moves the pen back by
//!   the width of what was taken, so the rest of the line stays where
//!   it was.
//! - **A line that moves on keeps moving on**: the tree's own rule, the
//!   `T*` of the operator that showed it is written out first.
//! - **Nothing is written that could not be verified.** The text that
//!   is left under the region is read again after the change, and if
//!   any of it is still there the whole thing is refused and the file
//!   is left as it was.
//! - **A page whose content is filtered is refused**: its bytes are
//!   not the content, and writing them back would change what the
//!   file says.
//! - **A page with more than one content stream is refused**: the text
//!   of the page is read from all of them together, and one file
//!   cannot hold them all.
//!
//! # What it does not do
//!
//! - **The mark is not kept.** The region is not in the document
//!   afterwards: what the file says is what was written, and the
//!   region is gone with the command.
//! - **The images and the paths** under the region are not taken away:
//!   the tree's redaction takes them too, and that is not here.
//! - **The glyphs are not counted one by one**: the tree works glyph
//!   by glyph, and this tree works by the piece.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.redact.text";

/// The **number** that moves the pen back by a width: the width of
/// the text in thousandths of an em.
const THOUSANDTHS: f64 = 1000.0;

/// The **operators** that take the place of a removed piece of text.
///
/// The tree's own rule: a number that moves the pen back by the width
/// of what was taken, so that the rest of the line keeps its place.
fn removal_of(
    operation: &crate::content::Operation,
    piece: &crate::pagetext::Piece,
) -> Vec<crate::content::Operation> {
    let mut out: Vec<crate::content::Operation> = Vec::new();
    let advance = piece.advance.unwrap_or(0.0);
    // The number that moves the pen back: a number of a PDF is a
    // number, and a number that is not a number is not written.
    let number = match craftkit_sdk::serde_json::Number::from_f64(-advance * THOUSANDTHS) {
        Some(number) => Json::Number(number),
        None => Json::Number(craftkit_sdk::serde_json::Number::from(0)),
    };
    match operation.operator.as_str() {
        // The tree's own rule: the line moves on before the text that
        // is left.
        "\"" => {
            let spacing = operation.operands.get(1).cloned().unwrap_or(Json::Null);
            let word = operation.operands.first().cloned().unwrap_or(Json::Null);
            out.push(crate::content::Operation {
                operator: "Tw".to_string(),
                operands: vec![word],
                inline: None,
            });
            out.push(crate::content::Operation {
                operator: "Tc".to_string(),
                operands: vec![spacing],
                inline: None,
            });
            out.push(crate::content::Operation {
                operator: "T*".to_string(),
                operands: Vec::new(),
                inline: None,
            });
        }
        "'" => out.push(crate::content::Operation {
            operator: "T*".to_string(),
            operands: Vec::new(),
            inline: None,
        }),
        _ => {}
    }
    out.push(crate::content::Operation {
        operator: "TJ".to_string(),
        operands: vec![Json::Array(vec![number])],
        inline: None,
    });
    out
}

/// The **operations** with the text under a region taken away.
///
/// The answer is the operations and how many pieces were taken.
pub fn remove_under(
    operations: &[crate::content::Operation],
    pieces: &[crate::pagetext::Piece],
    region: [f64; 4],
) -> (Vec<crate::content::Operation>, usize) {
    let mut out: Vec<crate::content::Operation> = Vec::new();
    let mut removed = 0usize;
    for (index, operation) in operations.iter().enumerate() {
        let piece = pieces.iter().find(|piece| piece.operation == index);
        match piece {
            Some(piece)
                if piece.bounds.is_some()
                    && crate::region::under(region, piece.bounds.unwrap_or([0.0; 4]), origin_of(piece)) =>
            {
                removed += 1;
                out.extend(removal_of(operation, piece));
            }
            _ => out.push(operation.clone()),
        }
    }
    (out, removed)
}

/// The **origin** of a piece: where its text begins.
fn origin_of(piece: &crate::pagetext::Piece) -> (f64, f64) {
    (piece.x.unwrap_or_default(), piece.y.unwrap_or_default())
}

/// The **content stream** of a page, when it has exactly one.
fn stream_of(document: &Document, page: u64) -> Option<u64> {
    let node = document.node(page)?;
    let entries = crate::entries_of(&node.data);
    let contents = entries.iter().find(|(key, _)| key == "Contents").map(|(_, value)| value.clone())?;
    let listed = match contents {
        Json::Array(list) => list,
        one => vec![one],
    };
    if listed.len() != 1 {
        return None;
    }
    crate::preflight::node_of(document, &listed[0]).map(|node| node.id)
}

/// The **text under a region is removed**.
///
/// A change: the content of the page is written again without the
/// text the region covers.
#[derive(Command)]
#[command(id = "pdf.redact.text", area = "pdf", layer = "pdf", icon = "remove", mutates, undoable)]
pub struct RedactText {
    /// The page whose text is removed.
    pub page: NodeRef,
    /// The region: four numbers, the left, the bottom, the right and
    /// the top of it.
    pub rect: Json,
}

impl RedactText {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let page = self.page.0;
        let document = session.document();
        let Some(node) = document.node(page) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{page}`"),
            });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the node `{page}` is a `{}` and not an object", node.kind),
            });
        }
        let Some(region) = crate::boxes::rect_of(&self.rect) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the region is not a rectangle of four numbers".to_string(),
            });
        };
        let Some(stream) = stream_of(document, page) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the page `{page}` does not have exactly one content stream"),
            });
        };
        let Some(stream_node) = document.node(stream) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{stream}`"),
            });
        };
        if let Some(names) = crate::filters::filter_of(&crate::entries_of(&stream_node.data)) {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the stream is filtered by {names:?}, and its bytes are not the content"),
            });
        }
        let Some(path) = stream_node.stream.clone() else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the stream `{stream}` has no file"),
            });
        };
        let limits = crate::filters::Limits::default();
        let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), limits.max_input as u64)
            .map_err(|message| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the stream's bytes cannot be read: {message}"),
        })?;
        let operations = crate::content::operations(&bytes).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the content cannot be read: {error:?}"),
        })?;
        // The **pieces** of the page: the text and where it is.
        let pieces = crate::pagetext::pieces_of_operations(document, page, &operations);
        let (changed, removed) = remove_under(&operations, &pieces, region);
        if removed == 0 {
            return Ok(Outcome::new(format!("nothing of the page `{page}` is under the region"))
                .with_data(json!({ "page": page, "region": json!(region), "removed": 0 })));
        }
        // The **verification**: the text that is left is read again, and
        // if any of it is still under the region nothing is written.
        let left = crate::pagetext::pieces_of_operations(document, page, &changed);
        let residue = left
            .iter()
            .filter(|piece| {
                // A piece with nothing but spaces in it is the gap the
                // removal leaves, and a gap is not text.
                if piece.text.trim().is_empty() {
                    return false;
                }
                piece.bounds.is_some()
                    && crate::region::under(region, piece.bounds.unwrap_or([0.0; 4]), origin_of(piece))
            })
            .count();
        if residue > 0 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "{residue} piece(s) of text are still under the region, so nothing was changed"
                ),
            });
        }
        let written = crate::content::write_operations(&changed);
        std::fs::write(&path, &written).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the content cannot be written: {error}"),
        })?;
        Ok(Outcome::changed(format!("{removed} piece(s) of text removed")).with_data(json!({
            "page": page,
            "region": json!(region),
            "removed": removed,
            "bytes": written.len(),
        })))
    }
}
