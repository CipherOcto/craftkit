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
//! - **The pixels of an image are cleared**, as the tree clears them:
//!   an image that is only partly covered keeps its place and the
//!   covered part of it is black. An image this tree cannot read is
//!   taken away whole, which is the tree's own fail-closed rule.
//! - **The images of a form are not cleared**: the pixels of an image
//!   that a form draws are not cleared, and the image is left alone.
//! - **The glyphs are not counted one by one**: the tree works glyph
//!   by glyph, and this tree works by the piece.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

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
    /// How many **images** were taken away: the inline ones and the
    /// images of the page itself.
    pub images: usize,
    /// How many **paths** were taken away.
    pub paths: usize,
    /// How many **shadings** were clipped.
    pub shadings: usize,
}

/// An **operation** with numbers as its operands.
fn op_of(operator: &str, numbers: &[f64]) -> crate::content::Operation {
    crate::content::Operation {
        operator: operator.to_string(),
        operands: numbers.iter().map(|number| json!(number)).collect(),
        inline: None,
    }
}

/// The **clip** that cuts a region out of a shading: the tree's own
/// rule.
///
/// A shading fills the clip it is in, so the region is cut out of it:
/// the box of the page and the region are drawn as one path, and the
/// **even-odd** rule leaves the region out of it. The path is written
/// in the space the shading is painted in, so the transform of the page
/// is undone first.
fn clip_out(region: [f64; 4], ctm: &crate::content::Matrix) -> Option<Vec<crate::content::Operation>> {
    let inverse = ctm.invert()?;
    // The **box of the page**, and a little more than it: the tree's
    // own box.
    const PAGE: f64 = 1_000_000.0;
    let big = [-PAGE - 10.0, -PAGE - 10.0, PAGE + 10.0, PAGE + 10.0];
    let outer = [(big[0], big[1]), (big[2], big[1]), (big[2], big[3]), (big[0], big[3])];
    let inner =
        [(region[0], region[1]), (region[0], region[3]), (region[2], region[3]), (region[2], region[1])];
    let mut ops: Vec<crate::content::Operation> = Vec::new();
    for poly in [outer, inner] {
        for (at, (x, y)) in poly.iter().enumerate() {
            let (u, v) = inverse.apply(*x, *y);
            ops.push(op_of(if at == 0 { "m" } else { "l" }, &[u, v]));
        }
        ops.push(op_of("h", &[]));
    }
    ops.push(op_of("W*", &[]));
    ops.push(op_of("n", &[]));
    Some(ops)
}

/// The **XObjects** of a page: the name each has and the node of it.
///
/// The tree's own rule: the XObjects are in the resources of the
/// page, and a resource may be written where it is used or named by a
/// reference.
fn xobjects_of(document: &Document, page: u64) -> Vec<(String, u64)> {
    let Some(node) = document.node(page) else { return Vec::new() };
    let entries = crate::entries_of(&node.data);
    let Some(resources) = entries.iter().find(|(key, _)| key == "Resources").map(|(_, value)| value.clone())
    else {
        return Vec::new();
    };
    let Some(resources) = entries_of_value(document, &resources) else { return Vec::new() };
    let Some(xobjects) = resources.iter().find(|(key, _)| key == "XObject").map(|(_, value)| value.clone())
    else {
        return Vec::new();
    };
    let Some(xobjects) = entries_of_value(document, &xobjects) else { return Vec::new() };
    let mut out: Vec<(String, u64)> = Vec::new();
    for (name, value) in xobjects {
        let Some(number) = crate::number_of(&value) else { continue };
        let Some(node) = document.walk().find(|node| crate::the_number_of(node) == Some(number)) else {
            continue;
        };
        out.push((name, node.id));
    }
    out
}

/// The **entries** of a value: the pairs of a dictionary, whether it is
/// written where it is used or named by a reference.
fn entries_of_value(document: &Document, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(node) = crate::preflight::node_of(document, value) {
        return Some(crate::entries_of(&node.data));
    }
    let list = value.get("entries")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let pair = entry.as_array()?;
                let key = pair.first()?.as_str()?.to_string();
                Some((key, pair.get(1)?.clone()))
            })
            .collect(),
    )
}

/// Whether an XObject is an **image**: the tree's own rule, the
/// `/Subtype` of the object.
fn is_an_image(document: &Document, xobject: u64) -> bool {
    document
        .node(xobject)
        .map(|node| {
            crate::entries_of(&node.data)
                .iter()
                .any(|(key, value)| key == "Subtype" && value.as_str() == Some("Image"))
        })
        .unwrap_or(false)
}

/// A **form** whose content was rewritten: the name it had, the
/// dictionary it carries, and the bytes of its new content.
type Rewritten = (String, Vec<(String, Json)>, Vec<u8>);

/// An **image** whose pixels were cleared: the name it had, the bytes
/// of its samples, and the dictionary it carries.
type Fresh = (String, Vec<u8>, Vec<(String, Json)>);

/// The **images** a page draws, and where each of them is drawn.
///
/// The tree's own rule: the `q` and the `Q` keep the transform and the
/// `cm` changes it, and an image is drawn in the square that changes
/// nothing, so the transform of the page places it.
fn images_drawn(
    operations: &[crate::content::Operation],
    images: &[(String, u64)],
    document: &Document,
) -> Vec<(String, u64, crate::content::Matrix)> {
    let mut out: Vec<(String, u64, crate::content::Matrix)> = Vec::new();
    let mut ctm = crate::content::Matrix::IDENTITY;
    let mut stack: Vec<crate::content::Matrix> = Vec::new();
    for operation in operations {
        match operation.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => {
                if let Some(saved) = stack.pop() {
                    ctm = saved;
                }
            }
            "cm" => {
                let numbers: Vec<f64> =
                    operation.operands.iter().filter_map(|value| value.as_f64()).collect();
                if let Some(matrix) = crate::content::Matrix::from_numbers(&numbers) {
                    ctm = matrix.then(&ctm);
                }
            }
            "Do" => {
                let name =
                    operation.operands.first().and_then(|operand| operand.as_str()).unwrap_or_default();
                let name = name.strip_prefix('/').unwrap_or(name).to_string();
                let drawn = images.iter().find(|(held, _)| *held == name).map(|(_, id)| *id);
                if drawn.is_some_and(|id| is_an_image(document, id)) {
                    out.push((name, drawn.unwrap_or(0), ctm));
                }
            }
            _ => {}
        }
    }
    out
}

/// The **forms** a page draws, and where each of them is drawn.
///
/// The tree's own rule: a form is drawn in its own space, and its own
/// matrix places it on the page.
fn forms_drawn(
    operations: &[crate::content::Operation],
    images: &[(String, u64)],
    document: &Document,
) -> Vec<(String, u64, crate::content::Matrix)> {
    let mut out: Vec<(String, u64, crate::content::Matrix)> = Vec::new();
    let mut ctm = crate::content::Matrix::IDENTITY;
    let mut stack: Vec<crate::content::Matrix> = Vec::new();
    for operation in operations {
        match operation.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => {
                if let Some(saved) = stack.pop() {
                    ctm = saved;
                }
            }
            "cm" => {
                let numbers: Vec<f64> =
                    operation.operands.iter().filter_map(|value| value.as_f64()).collect();
                if let Some(matrix) = crate::content::Matrix::from_numbers(&numbers) {
                    ctm = matrix.then(&ctm);
                }
            }
            "Do" => {
                let name =
                    operation.operands.first().and_then(|operand| operand.as_str()).unwrap_or_default();
                let name = name.strip_prefix('/').unwrap_or(name).to_string();
                let drawn = images.iter().find(|(held, _)| *held == name).map(|(_, id)| *id);
                let Some(id) = drawn else { continue };
                if !is_a_form(document, id) {
                    continue;
                }
                // The form's **own matrix** places it on the page.
                let own = document
                    .node(id)
                    .map(|node| crate::entries_of(&node.data))
                    .and_then(|entries| {
                        entries.iter().find(|(key, _)| key == "Matrix").map(|(_, value)| value.clone())
                    })
                    .and_then(|value| {
                        value.as_array().map(|list| {
                            list.iter().filter_map(|number| number.as_f64()).collect::<Vec<f64>>()
                        })
                    });
                let placed =
                    match own.as_ref().and_then(|numbers| crate::content::Matrix::from_numbers(numbers)) {
                        Some(matrix) => matrix.then(&ctm),
                        None => ctm,
                    };
                out.push((name, id, placed));
            }
            _ => {}
        }
    }
    out
}

/// Whether an XObject is a **form**: the tree's own rule, the
/// `/Subtype` of the object.
fn is_a_form(document: &Document, xobject: u64) -> bool {
    document
        .node(xobject)
        .map(|node| {
            crate::entries_of(&node.data)
                .iter()
                .any(|(key, value)| key == "Subtype" && value.as_str() == Some("Form"))
        })
        .unwrap_or(false)
}

/// Whether a region **covers** a box: the tree's own rule, with a
/// slack of a hundredth of a point.
fn covers(region: [f64; 4], bounds: [f64; 4]) -> bool {
    const SLACK: f64 = 0.01;
    bounds[0] >= region[0] - SLACK
        && bounds[1] >= region[1] - SLACK
        && bounds[2] <= region[2] + SLACK
        && bounds[3] <= region[3] + SLACK
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
/// - an **image** of the page is drawn in the square that changes
///   nothing, and the transform of the page places it: an image the
///   region hits is gone — the tree's own rule, and the tree's own
///   rule for an image it cannot read is the same;
/// - a path that **clips** keeps its clip and paints nothing.
pub fn drawings_under(
    operations: &[crate::content::Operation],
    region: [f64; 4],
    doomed: &[String],
    renamed: &[(String, String)],
    start: crate::content::Matrix,
) -> Drawings {
    let mut out = Drawings::default();
    // The **transform of the page**: the `q` and `Q` keep it, and the
    // `cm` changes it. A form starts at the transform that places it,
    // for its own content is in its own space.
    let mut ctm = start;
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
            "Do" => {
                // The **image** the operator draws. The tree's own
                // rules: an image the region **covers** is gone, and an
                // image the region only **hits** has its covered pixels
                // cleared — and it is then drawn under a new name, for
                // the object the pages share keeps the name it had.
                let name =
                    operation.operands.first().and_then(|operand| operand.as_str()).unwrap_or_default();
                let name = name.strip_prefix('/').unwrap_or(name).to_string();
                if doomed.contains(&name) {
                    out.images += 1;
                    continue;
                }
                let mut operation = operation.clone();
                if let Some((_, new_name)) = renamed.iter().find(|(old, _)| *old == name)
                    && let Some(operand) = operation.operands.first_mut()
                {
                    *operand = json!(new_name);
                }
                out.operations.push(operation);
                continue;
            }
            "sh" => {
                // The **shading** fills the clip it is in, so the
                // region is cut out of it: the tree's own rule.
                if let Some(clip) = clip_out(region, &ctm) {
                    out.operations.push(op_of("q", &[]));
                    out.operations.extend(clip);
                    out.operations.push(operation.clone());
                    out.operations.push(op_of("Q", &[]));
                    out.shadings += 1;
                    continue;
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

/// The **operations** of a stream: the content it holds.
///
/// The tree's own rule: the bytes of a stream that is filtered are
/// undone before they are read.
fn operations_of(node: &Node) -> Result<Vec<crate::content::Operation>, EngineError> {
    let decoded = crate::decoded_bytes(node, None, None, false, COMMAND)?;
    crate::content::operations(&decoded.bytes).map_err(|error| EngineError::Failed {
        id: COMMAND.to_string(),
        message: format!("the content cannot be read: {error:?}"),
    })
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
        let images = xobjects_of(document, page);
        // The **images** the page draws, and the ones the region takes
        // away: the tree's own rules — an image the region **covers**
        // is gone, and an image it only **hits** has its covered pixels
        // cleared, unless the kit cannot read it, and then it is gone
        // as well.
        let drawn = images_drawn(&content.operations, &images, document);
        let mut doomed: Vec<String> = Vec::new();
        let mut fresh: Vec<Fresh> = Vec::new();
        for (name, id, ctm) in &drawn {
            let bounds = ctm.bbox([0.0, 0.0, 1.0, 1.0]);
            if !overlaps(region, bounds) {
                continue;
            }
            if covers(region, bounds) {
                doomed.push(name.clone());
                continue;
            }
            let Some(node) = document.node(*id) else {
                doomed.push(name.clone());
                continue;
            };
            match crate::image::clear(document, node, ctm, &[region]) {
                // The **pixels** of the image are cleared: it is written
                // again, as a new object.
                Some(Some(bytes)) => {
                    fresh.push((name.clone(), bytes, crate::image::entries_of_an_image(node)))
                }
                // No pixel of it is covered: it is left as it is.
                Some(None) => {}
                // The kit cannot read it: it is taken away whole.
                None => doomed.push(name.clone()),
            }
        }
        // The **forms** the region hits: their content is rewritten, and
        // they are written again as new objects — the tree's own rule,
        // and the reason is the same as the one of the images.
        let forms = forms_drawn(&content.operations, &images, document);
        let mut doomed_forms: Vec<String> = Vec::new();
        let mut rewritten: Vec<Rewritten> = Vec::new();
        for (name, id, placed) in &forms {
            // The **box** of the form: its own BBox, placed on the page.
            let box_ = document
                .node(*id)
                .map(|node| crate::entries_of(&node.data))
                .and_then(|entries| {
                    entries.iter().find(|(key, _)| key == "BBox").map(|(_, value)| value.clone())
                })
                .and_then(|value| {
                    value
                        .as_array()
                        .map(|list| list.iter().filter_map(|number| number.as_f64()).collect::<Vec<f64>>())
                })
                .filter(|corners| corners.len() == 4)
                .map(|corners| placed.bbox([corners[0], corners[1], corners[2], corners[3]]));
            if let Some(box_) = box_
                && !overlaps(region, box_)
            {
                continue;
            }
            let Some(node) = document.node(*id) else {
                doomed_forms.push(name.clone());
                continue;
            };
            // The **content** of the form: a form whose content cannot
            // be read is taken away whole.
            let Ok(operations) = operations_of(node) else {
                doomed_forms.push(name.clone());
                continue;
            };
            let drawings = drawings_under(&operations, region, &[], &[], *placed);
            if drawings.images == 0 && drawings.paths == 0 && drawings.shadings == 0 {
                continue;
            }
            let bytes = crate::content::write_operations(&drawings.operations);
            // The **dictionary** of the form, without the ones that say
            // how its bytes are encoded.
            let entries: Vec<(String, Json)> = crate::entries_of(&node.data)
                .into_iter()
                .filter(|(key, _)| key != "Filter" && key != "Length" && key != "DecodeParms")
                .collect();
            rewritten.push((name.clone(), entries, bytes));
        }
        // The **new images**: a new object for each of them, and a new
        // name for it, so that the object the pages share is untouched.
        let directory =
            std::path::Path::new(&content.path).parent().map(|parent| parent.to_string_lossy().to_string());
        let mut renamed: Vec<(String, String)> = Vec::new();
        let mut cleared = 0usize;
        let mut forms_written = 0usize;
        for (name, bytes, entries) in &fresh {
            let number = crate::free::the_next_number(session.document(), 0);
            let Some(directory) = directory.clone() else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the stream of the page has no folder, and the image has nowhere to go"
                        .to_string(),
                });
            };
            let path = format!("{directory}/{number}.bin");
            std::fs::write(&path, bytes).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the image cannot be written: {error}"),
            })?;
            let data = json!({
                "object": number,
                "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
            });
            let stream =
                session.document_mut()?.add_node_with(format!("object {number}"), "pdf.stream", None, data);
            if let Some(node) = session.document_mut()?.node_mut(stream) {
                node.stream = Some(path);
            }
            let new_name =
                fresh_name(&images, &renamed.iter().map(|(_, name)| name.clone()).collect::<Vec<_>>());
            point_at(session, page, &new_name, number)?;
            renamed.push((name.clone(), new_name));
            cleared += 1;
        }
        // The **new forms**: the same rule as the images — a new object
        // with the new content, and a new name for it.
        for (name, entries, bytes) in &rewritten {
            let number = crate::free::the_next_number(session.document(), 0);
            let Some(directory) = directory.clone() else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: "the stream of the page has no folder, and the form has nowhere to go"
                        .to_string(),
                });
            };
            let path = format!("{directory}/{number}.bin");
            std::fs::write(&path, bytes).map_err(|error| EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the form cannot be written: {error}"),
            })?;
            let data = json!({
                "object": number,
                "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
            });
            let stream =
                session.document_mut()?.add_node_with(format!("object {number}"), "pdf.stream", None, data);
            if let Some(node) = session.document_mut()?.node_mut(stream) {
                node.stream = Some(path);
            }
            let new_name =
                fresh_name(&images, &renamed.iter().map(|(_, name)| name.clone()).collect::<Vec<_>>());
            point_at(session, page, &new_name, number)?;
            renamed.push((name.clone(), new_name));
            forms_written += 1;
        }
        let mut gone = doomed.clone();
        gone.extend(doomed_forms.iter().cloned());
        let drawings =
            drawings_under(&content.operations, region, &gone, &renamed, crate::content::Matrix::IDENTITY);
        if drawings.images == 0
            && drawings.paths == 0
            && drawings.shadings == 0
            && cleared == 0
            && forms_written == 0
        {
            return Ok(Outcome::new(format!("nothing of the page `{page}` is under the region")).with_data(
                json!({
                    "page": page,
                    "region": json!(region),
                    "images": 0,
                    "cleared": 0,
                    "forms": 0,
                    "paths": 0,
                    "shadings": 0,
                }),
            ));
        }
        let bytes = write_operations(&content.path, &drawings.operations)?;
        Ok(Outcome::changed(format!(
            "{} image(s) removed, {} image(s) cleared, {} form(s) rewritten, {} path(s) removed, and {} shading(s) clipped",
            drawings.images, cleared, forms_written, drawings.paths, drawings.shadings
        ))
        .with_data(json!({
            "page": page,
            "stream": content.stream,
            "region": json!(region),
            "images": drawings.images,
            "cleared": cleared,
            "forms": forms_written,
            "paths": drawings.paths,
            "shadings": drawings.shadings,
            "bytes": bytes,
        })))
    }
}

/// A **name** for a new image that no other image has.
///
/// The tree's own rule: the name of a redacted image is its own, so
/// that the object the pages share keeps the name it had.
fn fresh_name(images: &[(String, u64)], taken: &[String]) -> String {
    let mut index = taken.len() + 1;
    loop {
        let name = format!("PCRedacted{index}");
        if !images.iter().any(|(held, _)| *held == name) && !taken.contains(&name) {
            return name;
        }
        index += 1;
    }
}

/// **Point** the page at a new image.
///
/// The name of the image is a new one, and it is put in the resources
/// of the page: the object the pages share is untouched.
fn point_at(session: &mut Session, page: u64, name: &str, number: u64) -> Result<(), EngineError> {
    let mut data = session.document().node(page).map(|node| node.data.clone()).unwrap_or(Json::Null);
    let value = json!({ "ref": [number, 0] });
    if !add_to_dictionary(session.document_mut()?, &mut data, &["Resources", "XObject"], name, value) {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the resources of the page `{page}` cannot be written"),
        });
    }
    if let Some(node) = session.document_mut()?.node_mut(page) {
        node.data = data;
    }
    Ok(())
}

/// Set the **value** of a key in a dictionary that is named by a
/// reference.
///
/// The dictionary may be written where it is used or named by a
/// reference, and the tree's own rule is the same for both: the key
/// holds the value, and the dictionary is written back where it was.
fn add_to_dictionary(
    document: &mut Document,
    data: &mut Json,
    path: &[&str],
    key: &str,
    value: Json,
) -> bool {
    let Some(entries) = data.get_mut("entries").and_then(|entries| entries.as_array_mut()) else {
        return false;
    };
    let Some((first, rest)) = path.split_first() else {
        // The last step: the key holds the value.
        match entries.iter_mut().find(|pair| pair.get(0).and_then(|name| name.as_str()) == Some(key)) {
            Some(pair) => match pair.get_mut(1) {
                Some(slot) => {
                    *slot = value;
                    return true;
                }
                None => return false,
            },
            None => {
                entries.push(json!([key, value]));
                return true;
            }
        }
    };
    let Some(slot) = entries
        .iter_mut()
        .find(|pair| pair.get(0).and_then(|name| name.as_str()) == Some(*first))
        .and_then(|pair| pair.get_mut(1))
    else {
        return false;
    };
    if rest.is_empty() {
        *slot = value;
        return true;
    }
    // The value is a **reference**: the node holds the dictionary.
    if let Some(number) = crate::number_of(slot) {
        let id = document.walk().find(|node| crate::the_number_of(node) == Some(number)).map(|node| node.id);
        let Some(id) = id else { return false };
        let mut held = document.node(id).map(|node| node.data.clone()).unwrap_or(Json::Null);
        if !add_to_dictionary(document, &mut held, rest, key, value) {
            return false;
        }
        if let Some(node) = document.node_mut(id) {
            node.data = held;
        }
        return true;
    }
    // The value is written where it is used.
    add_to_dictionary(document, slot, rest, key, value)
}

/// Whether two rectangles **overlap**: the tree's own rule.
///
/// The tree's rule for the annotations is not the rule of the text
/// (§638): an annotation is under a region when the two overlap at
/// all, and the text is under it when more than a fifth of it is.
pub(crate) fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0].max(b[0]) < a[2].min(b[2]) && a[1].max(b[1]) < a[3].min(b[3])
}

/// The **annotations of a page**: the node of each and its rectangle.
fn annotations_of(document: &Document, page: u64) -> Vec<(u64, Option<[f64; 4]>, String)> {
    let mut out: Vec<(u64, Option<[f64; 4]>, String)> = Vec::new();
    for relation in document.relations.iter().filter(|relation| {
        relation.kind == "ref"
            && relation.from == page
            && relation
                .payload
                .get("key")
                .and_then(|key| key.as_str())
                .map(|key| key == "Annots")
                .unwrap_or(false)
    }) {
        let Some(node) = document.node(relation.to) else { continue };
        let entries = crate::entries_of(&node.data);
        let subtype = entries
            .iter()
            .find(|(key, _)| key == "Subtype")
            .and_then(|(_, value)| value.as_str())
            .unwrap_or_default()
            .to_string();
        let rectangle =
            entries.iter().find(|(key, _)| key == "Rect").and_then(|(_, value)| crate::boxes::rect_of(value));
        out.push((node.id, rectangle, subtype));
    }
    out
}

/// The **annotations under a region are taken away**.
///
/// A change: the annotations the region overlaps are gone from the
/// document, and the pop-ups that belong to them go with them.
#[derive(Command)]
#[command(id = "pdf.redact.annotations", area = "pdf", layer = "pdf", icon = "remove", mutates, undoable)]
pub struct RedactAnnotations {
    /// The page whose annotations are taken away.
    pub page: NodeRef,
    /// The region: four numbers, the left, the bottom, the right and
    /// the top of it.
    pub rect: Json,
}

impl RedactAnnotations {
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
        // The **annotations** the region overlaps. A pop-up is never
        // taken away by the region: it goes with the annotation it
        // belongs to.
        let annotations = annotations_of(document, page);
        let mut doomed: Vec<u64> = Vec::new();
        for (id, rectangle, subtype) in &annotations {
            if subtype == "Popup" {
                continue;
            }
            let Some(rectangle) = rectangle else { continue };
            if !overlaps(region, *rectangle) {
                continue;
            }
            doomed.push(*id);
        }
        // The **pop-ups** of the annotations that are gone.
        for (id, _rectangle, subtype) in &annotations {
            if subtype != "Popup" || doomed.contains(id) {
                continue;
            }
            let parent = document.node(*id).map(|node| crate::entries_of(&node.data)).and_then(|entries| {
                entries.iter().find(|(key, _)| key == "Parent").map(|(_, value)| value.clone())
            });
            let Some(parent) = parent else { continue };
            let Some(number) = crate::number_of(&parent) else { continue };
            let owner =
                document.walk().find(|node| crate::the_number_of(node) == Some(number)).map(|node| node.id);
            if let Some(owner) = owner
                && doomed.contains(&owner)
            {
                doomed.push(*id);
            }
        }
        if doomed.is_empty() {
            return Ok(Outcome::new(format!("nothing of the page `{page}` is under the region"))
                .with_data(json!({ "page": page, "region": json!(region), "removed": 0 })));
        }
        let mut removed: Vec<u64> = Vec::new();
        for id in &doomed {
            if session.document_mut()?.remove_node(*id) {
                removed.push(*id);
            }
        }
        Ok(Outcome::changed(format!("{} annotation(s) removed", removed.len())).with_data(json!({
            "page": page,
            "region": json!(region),
            "removed": removed.len(),
            "annotations": removed,
        })))
    }
}

/// The **transform** of a content stream at its end.
///
/// The `q` and the `Q` keep it, and the `cm` changes it: the tree's
/// own rule, and the rule of the reader of the content.
fn transform_at_end(operations: &[crate::content::Operation]) -> crate::content::Matrix {
    let mut ctm = crate::content::Matrix::IDENTITY;
    let mut stack: Vec<crate::content::Matrix> = Vec::new();
    for operation in operations {
        match operation.operator.as_str() {
            "q" => stack.push(ctm),
            "Q" => {
                if let Some(saved) = stack.pop() {
                    ctm = saved;
                }
            }
            "cm" => {
                let numbers: Vec<f64> =
                    operation.operands.iter().filter_map(|value| value.as_f64()).collect();
                if let Some(matrix) = crate::content::Matrix::from_numbers(&numbers) {
                    ctm = matrix.then(&ctm);
                }
            }
            _ => {}
        }
    }
    ctm
}

/// The **colour** a box is filled with: three numbers, red, green and
/// blue, each between zero and one.
///
/// The tree's own default is black, and a colour that is not three
/// numbers is refused by name.
fn colour_of(value: &Json, id: &str) -> Result<[f64; 3], EngineError> {
    let Some(numbers) = value.as_array() else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a colour is three numbers: red, green and blue".to_string(),
        });
    };
    if numbers.len() != 3 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a colour is three numbers, and this one is {}", numbers.len()),
        });
    }
    let mut colour = [0.0f64; 3];
    for (at, number) in numbers.iter().enumerate() {
        let Some(number) = number.as_f64() else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "a colour is three numbers: red, green and blue".to_string(),
            });
        };
        if !number.is_finite() || !(0.0..=1.0).contains(&number) {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("the colour {number} is not between zero and one"),
            });
        }
        colour[at] = number;
    }
    Ok(colour)
}

/// The **box** of a redaction is drawn into the page.
///
/// The tree's own rule: the mark of a redaction is gone when it is
/// applied, and **a box in its fill colour is drawn in its place**,
/// with the overlay text of the mark on it. The tree's fill colour is
/// black, and a mark with no colour of its own has no box.
///
/// Here the box is drawn on its own: the commands that take the text,
/// the drawings and the annotations away are the tree's, and this
/// command draws what the tree draws in place of the mark.
#[derive(Command)]
#[command(id = "pdf.redact.box", area = "pdf", layer = "pdf", icon = "remove", mutates, undoable)]
pub struct RedactBox {
    /// The page the box goes on.
    pub page: NodeRef,
    /// The region: four numbers, the left, the bottom, the right and
    /// the top of it.
    pub rect: Json,
    /// The colour of the box: three numbers, red, green and blue.
    ///
    /// The tree's default is black.
    pub color: Option<Json>,
}

impl RedactBox {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.redact.box".to_string();
        let page = self.page.0;
        let document = session.document();
        let Some(node) = document.node(page) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{page}`"),
            });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{page}` is a `{}` and not an object", node.kind),
            });
        }
        let Some(region) = crate::boxes::rect_of(&self.rect) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the region is not a rectangle of four numbers".to_string(),
            });
        };
        let colour = match &self.color {
            Some(colour) => colour_of(colour, &id)?,
            None => [0.0, 0.0, 0.0],
        };
        let content = content_of(document, page)?;
        // The **box**: the region, filled with the colour, drawn into
        // the content of the page.
        let mut changed = content.operations.clone();
        // The **transform** the content ends in, and the way back from
        // the space of the region to the space the content is written
        // in: the region is in the space of the page, and the path of
        // the box is written in the space of the content.
        let end = transform_at_end(&content.operations);
        let (x, y, width, height) = match end.invert() {
            Some(back) => {
                let (left, bottom) = back.apply(region[0], region[1]);
                let (right, top) = back.apply(region[2], region[3]);
                (left, bottom, right - left, top - bottom)
            }
            None => (region[0], region[1], region[2] - region[0], region[3] - region[1]),
        };
        changed.push(op_of("q", &[]));
        changed.push(op_of("rg", &[colour[0], colour[1], colour[2]]));
        changed.push(op_of("re", &[x, y, width, height]));
        changed.push(op_of("f", &[]));
        changed.push(op_of("Q", &[]));
        let bytes = write_operations(&content.path, &changed)?;
        Ok(Outcome::changed(format!("a box was drawn on the page `{page}`")).with_data(json!({
            "page": page,
            "stream": content.stream,
            "region": json!(region),
            "color": json!(colour),
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
