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

/// Whether a box is **inside** another: the tree's own rule.
fn covered_by(region: [f64; 4], bounds: [f64; 4]) -> bool {
    const SLACK: f64 = 0.01;
    bounds[0] >= region[0] - SLACK
        && bounds[1] >= region[1] - SLACK
        && bounds[2] <= region[2] + SLACK
        && bounds[3] <= region[3] + SLACK
}

/// The **points** a path operator adds, the tree's own rule.
fn points_of(operation: &crate::content::Operation) -> Vec<(f64, f64)> {
    match operation.operator.as_str() {
        "re" => {
            let numbers: Vec<f64> = operation.operands.iter().filter_map(|value| value.as_f64()).collect();
            if numbers.len() < 4 {
                return Vec::new();
            }
            let (x, y, width, height) = (numbers[0], numbers[1], numbers[2], numbers[3]);
            vec![(x, y), (x + width, y), (x, y + height), (x + width, y + height)]
        }
        "h" | "W" | "W*" => Vec::new(),
        _ => {
            let numbers: Vec<f64> = operation.operands.iter().filter_map(|value| value.as_f64()).collect();
            numbers.chunks(2).filter(|pair| pair.len() == 2).map(|pair| (pair[0], pair[1])).collect()
        }
    }
}

/// Whether an operator **builds** a path.
fn builds_a_path(operator: &str) -> bool {
    matches!(operator, "m" | "l" | "c" | "v" | "y" | "h" | "re" | "W" | "W*")
}

/// Whether an operator **paints** a path.
fn paints_a_path(operator: &str) -> bool {
    matches!(operator, "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n")
}

/// What the walk found and what it took away.
#[derive(Debug, Default)]
pub struct Drawings {
    /// The operations that are left.
    pub operations: Vec<crate::content::Operation>,
    /// How many **inline images** were taken away.
    pub images: usize,
    /// How many **paths** were taken away.
    pub paths: usize,
}

/// The **drawings** of a page with the ones under a region taken away.
///
/// The tree's own rules:
/// - an **inline image** is drawn in the square that changes nothing,
///   and the transform of the page places it: if the region hits that
///   square the image is gone;
/// - a **path** is a box of its own points, and if the region covers
///   that box the path is gone; a path that only hits the region is
///   left alone — the tree clips it, and that is not here;
/// - a path that **clips** keeps its clip and paints nothing.
pub fn drawings_under(operations: &[crate::content::Operation], region: [f64; 4]) -> Drawings {
    let mut out = Drawings::default();
    // The **transform of the page**: the `q` and `Q` keep it, and the
    // `cm` changes it.
    let mut ctm = crate::content::Matrix::IDENTITY;
    let mut stack: Vec<crate::content::Matrix> = Vec::new();
    // The **path** being built, its box, and whether it clips.
    let mut path: Vec<crate::content::Operation> = Vec::new();
    let mut path_box: Option<[f64; 4]> = None;
    let mut clips = false;
    for operation in operations {
        match operation.operator.as_str() {
            "q" => {
                stack.push(ctm);
                out.operations.push(operation.clone());
                continue;
            }
            "Q" => {
                if let Some(saved) = stack.pop() {
                    ctm = saved;
                }
                out.operations.push(operation.clone());
                continue;
            }
            "cm" => {
                let numbers: Vec<f64> =
                    operation.operands.iter().filter_map(|value| value.as_f64()).collect();
                if let Some(matrix) = crate::content::Matrix::from_numbers(&numbers) {
                    ctm = matrix.then(&ctm);
                }
                out.operations.push(operation.clone());
                continue;
            }
            "BI" => {
                // The **inline image**: the square that changes
                // nothing, drawn by the transform of the page.
                let bounds = ctm.bbox([0.0, 0.0, 1.0, 1.0]);
                if crate::region::under(region, bounds, (bounds[0], bounds[1])) {
                    out.images += 1;
                    continue;
                }
                out.operations.push(operation.clone());
                continue;
            }
            _ => {}
        }
        if builds_a_path(&operation.operator) {
            for (x, y) in points_of(operation) {
                let (u, v) = ctm.apply(x, y);
                path_box = Some(match path_box {
                    None => [u, v, u, v],
                    Some(box_) => [box_[0].min(u), box_[1].min(v), box_[2].max(u), box_[3].max(v)],
                });
            }
            clips |= matches!(operation.operator.as_str(), "W" | "W*");
            path.push(operation.clone());
            continue;
        }
        if paints_a_path(&operation.operator) {
            let body = std::mem::take(&mut path);
            let bounds = path_box.take();
            let was_clipping = std::mem::replace(&mut clips, false);
            let covered =
                operation.operator != "n" && bounds.map(|bounds| covered_by(region, bounds)).unwrap_or(false);
            if !covered {
                out.operations.extend(body);
                out.operations.push(operation.clone());
                continue;
            }
            out.paths += 1;
            if was_clipping {
                // The clip is kept, and the path paints nothing.
                out.operations.extend(body);
                out.operations.push(crate::content::Operation {
                    operator: "n".to_string(),
                    operands: Vec::new(),
                    inline: None,
                });
            }
            continue;
        }
        out.operations.push(operation.clone());
    }
    // A path that was never painted is still a path.
    out.operations.extend(path);
    out
}

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

/// The **content** of a page: its stream, its operations, and the file
/// they are written to.
struct Content {
    /// The stream the content is in.
    stream: u64,
    /// The file the stream's bytes are in.
    path: String,
    /// The operations of the stream.
    operations: Vec<crate::content::Operation>,
}

/// The **content** of a page, when it has exactly one stream and that
/// stream says what it is in.
///
/// The tree's own rules: a page with more than one content stream is
/// refused, and so is a stream that is filtered — its bytes are not
/// the content.
fn content_of(document: &Document, page: u64) -> Result<Content, EngineError> {
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
    Ok(Content { stream, path, operations })
}

/// Write the **operations** back to the file of a stream.
fn write_operations(path: &str, operations: &[crate::content::Operation]) -> Result<usize, EngineError> {
    let written = crate::content::write_operations(operations);
    std::fs::write(path, &written).map_err(|error| EngineError::Failed {
        id: COMMAND.to_string(),
        message: format!("the content cannot be written: {error}"),
    })?;
    Ok(written.len())
}

/// The **drawings under a region are taken away**.
///
/// A change: the inline images and the paths the region covers are
/// gone from the content of the page.
#[derive(Command)]
#[command(id = "pdf.redact.drawings", area = "pdf", layer = "pdf", icon = "remove", mutates, undoable)]
pub struct RedactDrawings {
    /// The page whose drawings are taken away.
    pub page: NodeRef,
    /// The region: four numbers, the left, the bottom, the right and
    /// the top of it.
    pub rect: Json,
}

impl RedactDrawings {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let page = self.page.0;
        let document = session.document();
        if document.node(page).is_none() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{page}`"),
            });
        }
        let Some(region) = crate::boxes::rect_of(&self.rect) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the region is not a rectangle of four numbers".to_string(),
            });
        };
        let content = content_of(document, page)?;
        let drawings = drawings_under(&content.operations, region);
        if drawings.images == 0 && drawings.paths == 0 {
            return Ok(Outcome::new(format!("nothing of the page `{page}` is under the region"))
                .with_data(json!({ "page": page, "region": json!(region), "images": 0, "paths": 0 })));
        }
        let bytes = write_operations(&content.path, &drawings.operations)?;
        Ok(Outcome::changed(format!("{} image(s) and {} path(s) removed", drawings.images, drawings.paths))
            .with_data(json!({
                "page": page,
                "stream": content.stream,
                "region": json!(region),
                "images": drawings.images,
                "paths": drawings.paths,
                "bytes": bytes,
            })))
    }
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
        let content = content_of(document, page)?;
        // The **pieces** of the page: the text and where it is.
        let pieces = crate::pagetext::pieces_of_operations(document, page, &content.operations);
        let (changed, removed) = remove_under(&content.operations, &pieces, region);
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
        let bytes = write_operations(&content.path, &changed)?;
        Ok(Outcome::changed(format!("{removed} piece(s) of text removed")).with_data(json!({
            "page": page,
            "stream": content.stream,
            "region": json!(region),
            "removed": removed,
            "bytes": bytes,
        })))
    }
}
