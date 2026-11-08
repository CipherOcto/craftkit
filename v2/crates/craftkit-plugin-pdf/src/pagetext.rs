//! The **text** of a page.
//!
//! A page's content is a stream of operators, and the operators that
//! **show text** are four of them:
//!
//! - `Tj` shows a string;
//! - `TJ` shows an array of strings, with numbers between them that
//!   move the pen;
//! - `'` shows a string and moves to the next line;
//! - `"` shows a string and moves to the next line.
//!
//! This module reads the text those operators show, in the order the
//! stream shows it.
//!
//! # The rules
//!
//! - **Text is shown inside a text object.** The operators `BT` and
//!   `ET` open and close one, and a string shown outside one is not
//!   text — it is ignored.
//! - **A number in a `TJ` array is a space.** A number below `-100` —
//!   a move of more than a tenth of an em — is the space between two
//!   words, and the answer puts one there.
//! - **A string is bytes.** What the bytes mean depends on the font
//!   that is in use: the bytes of a string are the **codes** of the
//!   font, and a font that carries a **map** (§575) says what each code
//!   stands for. A string shown in such a font is read through that
//!   map, and the answer says it was read as `unicode`. A string shown
//!   in a font with no map is read as the bytes it is: a string that
//!   begins with a byte-order mark is read as UTF-16 — big-endian or
//!   little-endian, as the mark says — and any other string is read as
//!   Latin-1, the byte's own value as a character.
//! - **The font in use is the one the file names.** The operator `Tf`
//!   says which font the text after it is shown in, and the piece of
//!   text says which font that was.
//!
//! # What it does not do
//!
//! - **The place of the text is worked out.** The text matrix of the
//!   text, turned by the **transform of the page** — the `cm`
//!   operator, the `q` and `Q` that keep it, and the page's own
//!   `/Matrix` — is the place the text is at. The tree's own rule, and
//!   the tree does not lay out a page: what is not worked out is the
//!   **width** of the text, which is the font's own business.
//! - **The encoding of a font is not read.** A font with no map has no
//!   Unicode here, and the tree falls back to the encoding.
//! - **The resources of the parent are not read.** A page that takes
//!   its fonts from the `/Pages` node above it is read as a page with
//!   no fonts.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.page.text";

/// The number below which a number in a `TJ` array is a **space**.
///
/// A number in the array moves the pen by that many thousandths of a
/// unit of text space, and a move of more than a tenth of an em is the
/// gap between two words.
const SPACE_BELOW: f64 = -100.0;

/// One **piece** of text: what an operator showed.
#[derive(Debug, Clone, PartialEq)]
pub struct Piece {
    /// The text itself.
    pub text: String,
    /// The operator that showed it.
    pub operator: String,
    /// How the bytes were read: `unicode`, `utf-16be`, `utf-16le` or
    /// `latin-1`.
    pub encoding: String,
    /// Whether the operator moved to the next line after it.
    pub break_after: bool,
    /// Where the text **begins**: the origin of the text matrix.
    pub x: Option<f64>,
    /// Where the text **begins**: the origin of the text matrix.
    pub y: Option<f64>,
    /// How **wide** the text is, along its own line, in the page's
    /// own space.
    pub width: Option<f64>,
    /// The **box** the text takes: from the descent of the font to its
    /// ascent, in the page's own space. The four numbers are the left,
    /// the bottom, the right and the top of it.
    pub bounds: Option<[f64; 4]>,
    /// The **operation** that showed the text: the place it holds in
    /// the content stream.
    pub operation: usize,
    /// How far the text moves the pen, in **glyphs of an em**: the
    /// width of the text without the size of the font.
    pub advance: Option<f64>,
    /// The **render mode** the text was shown in: nothing when the
    /// text is filled, which is the default, and the mode itself when
    /// it is not.
    pub mode: Option<u32>,
    /// The **font** the text is shown in: the name the page gives it.
    pub font: Option<String>,
}

impl Piece {
    /// The piece, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "text": self.text.clone(),
            "operator": self.operator.clone(),
            "encoding": self.encoding.clone(),
            "line_break": self.break_after,
            "x": self.x.map(|x| json!(x)).unwrap_or(Json::Null),
            "y": self.y.map(|y| json!(y)).unwrap_or(Json::Null),
            "width": self.width.map(|width| json!(width)).unwrap_or(Json::Null),
            "box": self.bounds.map(|box_| json!(box_)).unwrap_or(Json::Null),
            "operation": json!(self.operation),
            "advance": self.advance.map(|advance| json!(advance)).unwrap_or(Json::Null),
            "mode": self.mode.map(|mode| json!(mode)).unwrap_or(Json::Null),
            "font": self.font.clone().map(|font| json!(font)).unwrap_or(Json::Null),
        })
    }
}

/// The **text** of a page: its pieces, and the text they make.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PageText {
    /// The pieces, in the order the stream showed them.
    pub pieces: Vec<Piece>,
    /// The **streams** the text came from.
    pub streams: usize,
}

impl PageText {
    /// The text of the page, as one string.
    ///
    /// The pieces follow one another, and a piece whose operator moved
    /// to the next line is followed by a line break.
    pub fn joined(&self) -> String {
        let mut out = String::new();
        for piece in &self.pieces {
            out.push_str(&piece.text);
            if piece.break_after {
                out.push('\n');
            }
        }
        out
    }
}

/// The **text of a page that is not seen**.
///
/// The tree's own rules:
/// - a text drawn in the **invisible** mode is not seen: the modes
///   three and seven — the mode that paints nothing and the mode that
///   only clips — are the modes of the tree;
/// - a text that is **wholly off the page** is not seen either;
/// - a text with **no size** is seen where it is: a box with no width
///   and no height is on the page when it begins there.
#[derive(Command)]
#[command(id = "pdf.page.text.hidden", area = "pdf", layer = "pdf", icon = "list")]
pub struct HiddenText {
    /// The page whose text is read.
    pub page: NodeRef,
}

impl HiddenText {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let page = self.page.0;
        if session.document().node(page).is_none() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{page}`"),
            });
        }
        // The **box** of the page: the media box, and the page's own
        // if it has none.
        let boxes = crate::boxes::boxes_of(session, page);
        let Some(area) = crate::boxes::rect_of(&boxes[0]) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the page `{page}` has no box"),
            });
        };
        let text = text_of_page(session.document(), page)?;
        let mut hidden: Vec<Json> = Vec::new();
        for piece in &text.pieces {
            let invisible = matches!(piece.mode, Some(3) | Some(7));
            let off_the_page = match piece.bounds {
                Some(bounds) => {
                    let degenerate = bounds[2] - bounds[0] < 0.01 && bounds[3] - bounds[1] < 0.01;
                    !degenerate && !overlaps(area, bounds)
                }
                None => false,
            };
            if !invisible && !off_the_page {
                continue;
            }
            hidden.push(json!({
                "text": piece.text.clone(),
                "operator": piece.operator.clone(),
                "mode": piece.mode.map(|mode| json!(mode)).unwrap_or(Json::Null),
                "x": piece.x.map(|x| json!(x)).unwrap_or(Json::Null),
                "y": piece.y.map(|y| json!(y)).unwrap_or(Json::Null),
                "box": piece.bounds.map(|bounds| json!(bounds)).unwrap_or(Json::Null),
                "why": if invisible { "invisible" } else { "off the page" },
            }));
        }
        let count = hidden.len();
        Ok(Outcome::new(format!("{count} piece(s) of text are not seen")).with_data(json!({
            "page": page,
            "pieces": hidden,
            "count": count,
        })))
    }
}

/// Whether two rectangles **overlap**: the tree's own rule.
fn overlaps(a: [f64; 4], b: [f64; 4]) -> bool {
    a[0].max(b[0]) < a[2].min(b[2]) && a[1].max(b[1]) < a[3].min(b[3])
}

/// The **text** of a page.
///
/// The page is a node of the document, and its content is in its
/// `/Contents`: a stream, or a list of streams. A page with no content
/// has no text, and the answer says so rather than failing.
pub fn text_of_page(document: &Document, page: u64) -> Result<PageText, EngineError> {
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
    let entries = crate::entries_of(&node.data);
    let is_a_page = entries.iter().any(|(key, value)| key == "Type" && value.as_str() == Some("Page"));
    if !is_a_page {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the node `{page}` is not a page"),
        });
    }
    let Some(contents) = entries.iter().find(|(key, _)| key == "Contents").map(|(_, value)| value.clone())
    else {
        // A page with no content has no text. That is not a failure:
        // it is a page that says nothing.
        return Ok(PageText::default());
    };
    let listed: Vec<Json> = match contents {
        Json::Array(list) => list,
        one => vec![one],
    };
    let mut found = PageText::default();
    // The content of a page is the content of its streams, one after
    // the other: the font in use in the first is the font in use in
    // the second.
    let mut operations: Vec<crate::content::Operation> = Vec::new();
    for value in listed {
        let Some(stream) = crate::preflight::node_of(document, &value) else { continue };
        if stream.kind != "pdf.stream" {
            continue;
        }
        found.streams += 1;
        let decoded = crate::decoded_bytes(stream, None, None, false, COMMAND)?;
        let read = crate::content::operations(&decoded.bytes).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the content of the page cannot be read: {error:?}"),
        })?;
        operations.extend(read);
    }
    found.pieces = pieces_of_operations(document, page, &operations);
    Ok(found)
}

/// The **pieces** of the text of a page, for a set of operations.
///
/// The page says which fonts it has and which transform it begins
/// with, and the operations say what the text is.
pub(crate) fn pieces_of_operations(
    document: &Document,
    page: u64,
    operations: &[crate::content::Operation],
) -> Vec<Piece> {
    let maps = crate::pagefonts::maps_of(document, page);
    let widths: Vec<(String, crate::widths::Widths)> = crate::pagefonts::fonts_of(document, page)
        .into_iter()
        .filter_map(|(name, font)| crate::widths::widths_of(document, font).map(|widths| (name, widths)))
        .collect();
    let metrics: Vec<(String, crate::metrics::Metrics)> = crate::pagefonts::fonts_of(document, page)
        .into_iter()
        .map(|(name, font)| (name, crate::metrics::metrics_of(document, font)))
        .collect();
    let entries = document.node(page).map(|node| crate::entries_of(&node.data)).unwrap_or_default();
    let last = operations.len().saturating_sub(1);
    pieces_with(&maps, operations, 0, last, matrix_of(document, &entries), &widths, &metrics)
}

/// The **transform** a page begins with: its own `/Matrix`.
///
/// The tree's own rule: the page's matrix is the transform the page
/// begins with, and a page that says nothing begins with the one that
/// changes nothing.
fn matrix_of(document: &Document, entries: &[(String, Json)]) -> crate::content::Matrix {
    let Some((_, value)) = entries.iter().find(|(key, _)| key == "Matrix") else {
        return crate::content::Matrix::IDENTITY;
    };
    let numbers: Vec<f64> = match value {
        Json::Array(list) => list.iter().filter_map(|number| number.as_f64()).collect(),
        other => match crate::preflight::node_of(document, other) {
            Some(node) => node
                .data
                .as_array()
                .map(|list| list.iter().filter_map(|number| number.as_f64()).collect())
                .unwrap_or_default(),
            None => Vec::new(),
        },
    };
    crate::content::Matrix::from_numbers(&numbers).unwrap_or(crate::content::Matrix::IDENTITY)
}

/// The **operations** of a page: the content of every stream it has,
/// in the order the page holds them.
pub(crate) fn operations_of_page(
    document: &Document,
    page: u64,
) -> Result<Vec<crate::content::Operation>, EngineError> {
    let Some(node) = document.node(page) else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("there is no node `{page}`"),
        });
    };
    let entries = crate::entries_of(&node.data);
    let Some(contents) = entries.iter().find(|(key, _)| key == "Contents").map(|(_, value)| value.clone())
    else {
        // A page with no content has no operations. That is not a
        // failure: it is a page that says nothing.
        return Ok(Vec::new());
    };
    let listed: Vec<Json> = match contents {
        Json::Array(list) => list,
        one => vec![one],
    };
    let mut operations: Vec<crate::content::Operation> = Vec::new();
    for value in listed {
        let Some(stream) = crate::preflight::node_of(document, &value) else { continue };
        if stream.kind != "pdf.stream" {
            continue;
        }
        let decoded = crate::decoded_bytes(stream, None, None, false, COMMAND)?;
        let found = crate::content::operations(&decoded.bytes).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the content of the page cannot be read: {error:?}"),
        })?;
        operations.extend(found);
    }
    Ok(operations)
}

/// The **place** of the text: the matrices that say where it is.
///
/// The **text matrix** says where the text is, and the **line matrix**
/// says where the line is; the text of a piece begins at the origin of
/// the text matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Place {
    /// The text matrix: `a b c d e f`.
    text: [f64; 6],
    /// The line matrix: `a b c d e f`.
    line: [f64; 6],
    /// The **leading**: how far a line is from the one before it.
    leading: f64,
    /// The **rise**: how far the text is above the line it is on.
    rise: f64,
    /// The **size** of the font in use.
    size: f64,
    /// The **scale** of the text: how wide it is, in hundredths.
    scale: f64,
    /// The **character spacing**: how far each character is from the
    /// one before it.
    spacing: f64,
    /// The **word spacing**: how far a space is from where it would
    /// be.
    word_spacing: f64,
}

impl Place {
    /// A place where nothing has been said yet.
    const NONE: Place = Place {
        text: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        line: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        leading: 0.0,
        rise: 0.0,
        size: 0.0,
        scale: 1.0,
        spacing: 0.0,
        word_spacing: 0.0,
    };

    /// The origin of the text: where the text **begins**, and how
    /// wide it is.
    ///
    /// The **rendering matrix** is the tree's own: the size and the
    /// scale of the font, the rise, the text matrix, and then the
    /// transform of the page.
    fn origin(&self, ctm: &crate::content::Matrix, glyph: Option<f64>) -> (f64, f64, Option<f64>) {
        let rendering = crate::content::Matrix([self.size * self.scale, 0.0, 0.0, self.size, 0.0, self.rise])
            .then(&crate::content::Matrix(self.text))
            .then(ctm);
        let [a, b, .., e, f] = rendering.0;
        // The **width**: how far the text goes along its own line.
        let width = glyph.map(|glyph| ((a * glyph).powi(2) + (b * glyph).powi(2)).sqrt());
        (e, f, width)
    }

    /// The **box** the text takes: from the descent of the font to
    /// its ascent.
    ///
    /// The tree's own rule: the rectangle the glyphs take in the text
    /// space, and the rendering matrix turns it into the page's own
    /// space.
    fn box_of(
        &self,
        ctm: &crate::content::Matrix,
        glyph: Option<f64>,
        metrics: &crate::metrics::Metrics,
    ) -> Option<[f64; 4]> {
        let glyph = glyph?;
        let rendering = crate::content::Matrix([self.size * self.scale, 0.0, 0.0, self.size, 0.0, self.rise])
            .then(&crate::content::Matrix(self.text))
            .then(ctm);
        Some(rendering.bbox([0.0, metrics.descent, glyph, metrics.ascent]))
    }

    /// Move the **text** by a distance, and leave the line where it
    /// is.
    ///
    /// The tree's own rule: the text moves on, and the line it is on
    /// stays where it is.
    fn advance_by(&mut self, dx: f64) {
        let text = self.text;
        self.text = [text[0], text[1], text[2], text[3], dx * text[0] + text[4], dx * text[1] + text[5]];
    }

    /// Move the **line** by a distance, and put the text on it.
    fn move_to(&mut self, dx: f64, dy: f64) {
        let line = self.line;
        self.line = [
            line[0],
            line[1],
            line[2],
            line[3],
            dx * line[0] + dy * line[2] + line[4],
            dx * line[1] + dy * line[3] + line[5],
        ];
        self.text = self.line;
    }
}

/// The **advance** of the bytes of a string: how far the text goes
/// along its own line.
///
/// The tree's own rule: the width of each code, the size of the
/// font, the character spacing, and the word spacing of a space. The
/// answer is `None` when the font says no width: a width that is
/// guessed is a width that is wrong.
fn advance_of(
    widths: Option<&crate::widths::Widths>,
    bytes: &[u8],
    size: f64,
    spacing: f64,
    word_spacing: f64,
) -> Option<(f64, f64)> {
    let widths = widths?;
    // The advance in the **text space**, which is how far the text
    // moves on, and the advance in the **glyph space**, which is how
    // wide the text is: the glyph space is the text space without
    // the size of the font.
    let mut text = 0.0;
    let mut glyph = 0.0;
    for code in widths.codes(bytes) {
        let width = widths.width(code)?;
        let extra = if code == 32 { word_spacing } else { 0.0 };
        text += width / 1000.0 * size + spacing + extra;
        glyph += width / 1000.0;
        if size > 0.0 {
            glyph += (spacing + extra) / size;
        }
    }
    Some((text, glyph))
}

/// The **numbers** an operation was given.
fn numbers_of(operation: &crate::content::Operation) -> Vec<f64> {
    operation.operands.iter().filter_map(|operand| operand.as_f64()).collect()
}

/// The **font** an operation puts in use: the name the file gives it.
fn font_named(operation: &crate::content::Operation) -> Option<String> {
    let Json::String(name) = operation.operands.first()? else { return None };
    Some(name.strip_prefix('/').unwrap_or(name).to_string())
}

/// The **map** of a font, by the name the page gives it.
fn map_of<'a>(maps: &'a [(String, crate::tounicode::Map)], name: &str) -> Option<&'a crate::tounicode::Map> {
    maps.iter().find(|(held, _)| held == name).map(|(_, map)| map)
}

/// The **pieces** of text a set of operations shows.
///
/// The pieces between `from` and `to` are the ones that are returned,
/// but the whole of the operations is read: the font in use is the one
/// the file says at the time, and a font named before the range is
/// still in use inside it.
pub(crate) fn pieces_of(
    maps: &[(String, crate::tounicode::Map)],
    operations: &[crate::content::Operation],
    from: usize,
    to: usize,
) -> Vec<Piece> {
    pieces_with(maps, operations, from, to, crate::content::Matrix::IDENTITY, &[], &[])
}

/// The **pieces** of text, with the **transform of the page** the page
/// begins with.
pub(crate) fn pieces_with(
    maps: &[(String, crate::tounicode::Map)],
    operations: &[crate::content::Operation],
    from: usize,
    to: usize,
    start: crate::content::Matrix,
    widths: &[(String, crate::widths::Widths)],
    metrics: &[(String, crate::metrics::Metrics)],
) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    // Text is shown inside a **text object**.
    let mut in_text = false;
    // The **place** of the text.
    let mut place = Place::NONE;
    let mut placed = false;
    // The **transform of the page**: the `q` and `Q` keep it, and the
    // `cm` changes it.
    let mut ctm = start;
    let mut stack: Vec<(crate::content::Matrix, u32)> = Vec::new();
    // The **font** in use, and the map that reads its codes.
    let mut font: Option<String> = None;
    // The **widths** of the font in use: how wide its codes are.
    let mut font_widths: Option<&crate::widths::Widths> = None;
    // The **metrics** of the font in use: how tall its glyphs are.
    let mut font_metrics = crate::metrics::Metrics::FALLBACK;
    // The **render mode** of the text: how it is drawn. It is a part
    // of the state of the page, and the `q` and `Q` keep it.
    let mut render: u32 = 0;
    for (index, operation) in operations.iter().enumerate() {
        if index > to {
            break;
        }
        let within = index >= from;
        match operation.operator.as_str() {
            "BT" => {
                in_text = true;
                place = Place::NONE;
                placed = false;
            }
            "ET" => in_text = false,
            "Tf" => {
                font = font_named(operation);
                font_widths =
                    widths.iter().find(|(name, _)| Some(name) == font.as_ref()).map(|(_, widths)| widths);
                font_metrics = metrics
                    .iter()
                    .find(|(name, _)| Some(name) == font.as_ref())
                    .map(|(_, metrics)| *metrics)
                    .unwrap_or(crate::metrics::Metrics::FALLBACK);
                let numbers = numbers_of(operation);
                if let Some(size) = numbers.first() {
                    place.size = *size;
                }
            }
            "Tz" => {
                let numbers = numbers_of(operation);
                if let Some(scale) = numbers.first() {
                    place.scale = scale / 100.0;
                }
            }
            "Tc" => {
                let numbers = numbers_of(operation);
                if let Some(spacing) = numbers.first() {
                    place.spacing = *spacing;
                }
            }
            "Tw" => {
                let numbers = numbers_of(operation);
                if let Some(spacing) = numbers.first() {
                    place.word_spacing = *spacing;
                }
            }
            "q" => {
                stack.push((ctm, render));
            }
            "Q" => {
                if let Some((saved, mode)) = stack.pop() {
                    ctm = saved;
                    render = mode;
                }
            }
            "Tr" => {
                let numbers = numbers_of(operation);
                if let Some(mode) = numbers.first()
                    && mode.is_finite()
                    && *mode >= 0.0
                    && *mode <= 7.0
                {
                    render = *mode as u32;
                }
            }
            "cm" => {
                let numbers = numbers_of(operation);
                if let Some(matrix) = crate::content::Matrix::from_numbers(&numbers) {
                    ctm = matrix.then(&ctm);
                }
            }
            "Tm" => {
                let numbers = numbers_of(operation);
                if numbers.len() == 6 {
                    place.text = [numbers[0], numbers[1], numbers[2], numbers[3], numbers[4], numbers[5]];
                    place.line = place.text;
                    placed = true;
                }
            }
            "Td" | "TD" => {
                let numbers = numbers_of(operation);
                if numbers.len() == 2 {
                    if operation.operator == "TD" {
                        place.leading = -numbers[1];
                    }
                    place.move_to(numbers[0], numbers[1]);
                    placed = true;
                }
            }
            "T*" => {
                place.move_to(0.0, -place.leading);
                placed = true;
            }
            "TL" => {
                let numbers = numbers_of(operation);
                if let Some(leading) = numbers.first() {
                    place.leading = *leading;
                }
            }
            "Ts" => {
                let numbers = numbers_of(operation);
                if let Some(rise) = numbers.first() {
                    place.rise = *rise;
                }
            }
            "Tj" => {
                if !in_text {
                    continue;
                }
                let Some(bytes) = operation.operands.last().and_then(bytes_of) else { continue };
                let (text, encoding) = read_through(map_of(maps, font.as_deref().unwrap_or("")), &bytes);
                // The pen moves on by the text, whether the piece is
                // inside the range or not: the text before the range
                // moves the pen as well.
                let advance = advance_of(font_widths, &bytes, place.size, place.spacing, place.word_spacing);
                let (x, y, width) = place.origin(&ctm, advance.map(|(_, glyph)| glyph));
                let box_ = place.box_of(&ctm, advance.map(|(_, glyph)| glyph), &font_metrics);
                if let Some((advance, _)) = advance {
                    place.advance_by(advance * place.scale);
                }
                if !within {
                    continue;
                }
                pieces.push(Piece {
                    text,
                    operator: "Tj".to_string(),
                    encoding,
                    break_after: false,
                    x: placed.then_some(x),
                    y: placed.then_some(y),
                    width: placed.then_some(width).flatten(),
                    bounds: placed.then_some(box_).flatten(),
                    operation: index,
                    advance: advance.map(|(_, glyph)| glyph),
                    mode: (render != 0).then_some(render),
                    font: font.clone(),
                });
            }
            "TJ" => {
                if !in_text {
                    continue;
                }
                let Some(array) = operation.operands.last().and_then(|value| value.as_array()) else {
                    continue;
                };
                let (text, encoding) = shown(array, map_of(maps, font.as_deref().unwrap_or("")));
                // The **array** moves the pen: a number of -v moves
                // it by v thousandths of the size, and a string shows
                // text and moves it on by the width of the text.
                let mut glyph_advance = 0.0;
                let mut em_advance = 0.0;
                let beginning = place;
                for item in array {
                    if let Json::String(_) = item {
                        let Some(bytes) = bytes_of(item) else { continue };
                        let Some((width, glyph)) =
                            advance_of(font_widths, &bytes, place.size, place.spacing, place.word_spacing)
                        else {
                            continue;
                        };
                        glyph_advance += glyph;
                        em_advance += glyph;
                        place.advance_by(width * place.scale);
                        continue;
                    }
                    if let Some(number) = item.as_f64() {
                        let move_by = -number / 1000.0 * place.size;
                        glyph_advance += -number / 1000.0;
                        em_advance += -number / 1000.0;
                        place.advance_by(move_by * place.scale);
                    }
                }
                let (x, y, width) = beginning.origin(&ctm, Some(glyph_advance));
                let box_ = beginning.box_of(&ctm, Some(glyph_advance), &font_metrics);
                // An array with no text in it still moves the pen, and
                // so does an array outside the range.
                if text.is_empty() || !within {
                    continue;
                }
                pieces.push(Piece {
                    text,
                    operator: "TJ".to_string(),
                    encoding,
                    break_after: false,
                    x: placed.then_some(x),
                    y: placed.then_some(y),
                    width: placed.then_some(width).flatten(),
                    bounds: placed.then_some(box_).flatten(),
                    operation: index,
                    advance: Some(em_advance),
                    mode: (render != 0).then_some(render),
                    font: font.clone(),
                });
            }
            "'" | "\"" => {
                if !in_text {
                    continue;
                }
                let Some(bytes) = operation.operands.last().and_then(bytes_of) else { continue };
                let (text, encoding) = read_through(map_of(maps, font.as_deref().unwrap_or("")), &bytes);
                let advance = advance_of(font_widths, &bytes, place.size, place.spacing, place.word_spacing);
                let (x, y, width) = place.origin(&ctm, advance.map(|(_, glyph)| glyph));
                let box_ = place.box_of(&ctm, advance.map(|(_, glyph)| glyph), &font_metrics);
                if let Some((advance, _)) = advance {
                    place.advance_by(advance * place.scale);
                }
                if !within {
                    continue;
                }
                pieces.push(Piece {
                    text,
                    operator: operation.operator.clone(),
                    encoding,
                    break_after: true,
                    x: placed.then_some(x),
                    y: placed.then_some(y),
                    width: placed.then_some(width).flatten(),
                    bounds: placed.then_some(box_).flatten(),
                    operation: index,
                    advance: advance.map(|(_, glyph)| glyph),
                    mode: (render != 0).then_some(render),
                    font: font.clone(),
                });
            }
            _ => {}
        }
    }
    pieces
}

/// The text an array of a `TJ` operator shows.
///
/// The strings are shown one after the other, and a number below
/// `-100` is the space between two words.
fn shown(array: &[Json], map: Option<&crate::tounicode::Map>) -> (String, String) {
    let mut text = String::new();
    let mut encoding = "latin-1".to_string();
    for item in array {
        match item {
            _ if bytes_of(item).is_some() => {
                let Some(bytes) = bytes_of(item) else { continue };
                let (piece, kind) = read_through(map, &bytes);
                if kind != "latin-1" {
                    encoding = kind;
                }
                text.push_str(&piece);
            }
            Json::Number(number) if number.as_f64().is_some_and(|value| value < SPACE_BELOW) => {
                text.push(' ');
            }
            _ => {}
        }
    }
    (text, encoding)
}

/// The **bytes** of a string of a content stream.
///
/// The bytes of a PDF string are the string's characters, each one
/// below 256, and what they mean depends on the font.
fn bytes_of(value: &Json) -> Option<Vec<u8>> {
    crate::crypt::bytes_of(value)
}

/// The **text** of a string's bytes, and how they were read.
///
/// A string shown in a font that carries a map is read **through** that
/// map; any other string is read as the bytes it is.
fn read_through(map: Option<&crate::tounicode::Map>, bytes: &[u8]) -> (String, String) {
    match map {
        Some(map) => (text_through(map, bytes), "unicode".to_string()),
        None => decode_bytes(bytes),
    }
}

/// The **text** of a string's bytes, and how they were read.
///
/// The bytes of a PDF string are bytes, and a string that begins with a
/// byte-order mark is UTF-16; any other string is Latin-1.
pub(crate) fn decode_bytes(bytes: &[u8]) -> (String, String) {
    if bytes.len() >= 2 && bytes[0] == 0xfe && bytes[1] == 0xff {
        let units: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|pair| u16::from_be_bytes([pair[0], *pair.get(1).unwrap_or(&0)]))
            .collect();
        return (String::from_utf16_lossy(&units), "utf-16be".to_string());
    }
    if bytes.len() >= 2 && bytes[0] == 0xff && bytes[1] == 0xfe {
        let units: Vec<u16> = bytes[2..]
            .chunks(2)
            .map(|pair| u16::from_le_bytes([pair[0], *pair.get(1).unwrap_or(&0)]))
            .collect();
        return (String::from_utf16_lossy(&units), "utf-16le".to_string());
    }
    (bytes.iter().map(|byte| *byte as char).collect(), "latin-1".to_string())
}

/// The **text** of a string's bytes, read through the map of the
/// font.
///
/// The bytes of a string are the **codes** of the font, and the map
/// says what each code stands for. A code that is not in the map is
/// read as the byte it is: a code the map does not know is a code
/// this tree cannot say, and the byte is the truth there is.
fn text_through(map: &crate::tounicode::Map, bytes: &[u8]) -> String {
    let length = if map.code_length == 0 { 1 } else { map.code_length };
    let mut out = String::new();
    let mut at = 0usize;
    while at < bytes.len() {
        let code = if length == 2 {
            let high = u16::from(bytes[at]);
            let low = u16::from(*bytes.get(at + 1).unwrap_or(&0));
            high * 256 + low
        } else {
            u16::from(bytes[at])
        };
        match map.text_of(code) {
            Some(text) => out.push_str(text),
            None => out.push(char::from(bytes[at])),
        }
        at += length;
    }
    out
}

/// **Read** the **text** of a page.
///
/// A query: it does not change the document. A page with no content has
/// no text, and the answer says so.
#[derive(Command)]
#[command(id = "pdf.page.text", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadPageText {
    /// The page whose text is read.
    pub page: NodeRef,
}

impl ReadPageText {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let page = self.page.0;
        let found = text_of_page(session.document(), page)?;
        let text = found.joined();
        let pieces = found.pieces.iter().map(|piece| piece.to_json()).collect::<Vec<Json>>();
        Ok(Outcome::new(format!("the page `{page}` shows {} pieces of text", found.pieces.len())).with_data(
            json!({
                "page": page,
                "streams": found.streams,
                "pieces": pieces,
                "text": text,
            }),
        ))
    }
}
