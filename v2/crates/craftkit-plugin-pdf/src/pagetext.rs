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
//! - **The place of the text is not worked out.** A piece of text is
//!   where it was shown, and the tree does not lay out a page.
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
    let maps = crate::pagefonts::maps_of(document, page);
    let last = operations.len().saturating_sub(1);
    found.pieces = pieces_of(&maps, &operations, 0, last);
    Ok(found)
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
}

impl Place {
    /// A place where nothing has been said yet.
    const NONE: Place = Place {
        text: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        line: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
        leading: 0.0,
        rise: 0.0,
    };

    /// The origin of the text: where the text **begins**.
    fn origin(&self) -> (f64, f64) {
        (self.text[4], self.text[5] + self.rise)
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
    let mut pieces: Vec<Piece> = Vec::new();
    // Text is shown inside a **text object**.
    let mut in_text = false;
    // The **place** of the text.
    let mut place = Place::NONE;
    let mut placed = false;
    // The **font** in use, and the map that reads its codes.
    let mut font: Option<String> = None;
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
                if !within {
                    continue;
                }
                let (x, y) = place.origin();
                pieces.push(Piece {
                    text,
                    operator: "Tj".to_string(),
                    encoding,
                    break_after: false,
                    x: placed.then_some(x),
                    y: placed.then_some(y),
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
                if text.is_empty() || !within {
                    continue;
                }
                let (x, y) = place.origin();
                pieces.push(Piece {
                    text,
                    operator: "TJ".to_string(),
                    encoding,
                    break_after: false,
                    x: placed.then_some(x),
                    y: placed.then_some(y),
                    font: font.clone(),
                });
            }
            "'" | "\"" => {
                if !in_text {
                    continue;
                }
                let Some(bytes) = operation.operands.last().and_then(bytes_of) else { continue };
                let (text, encoding) = read_through(map_of(maps, font.as_deref().unwrap_or("")), &bytes);
                if !within {
                    continue;
                }
                let (x, y) = place.origin();
                pieces.push(Piece {
                    text,
                    operator: operation.operator.clone(),
                    encoding,
                    break_after: true,
                    x: placed.then_some(x),
                    y: placed.then_some(y),
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
            Json::String(_) => {
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
    let Json::String(text) = value else { return None };
    Some(text.chars().map(|character| character as u32 as u8).collect())
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
