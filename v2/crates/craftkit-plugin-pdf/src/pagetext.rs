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
//!   that is in use, and this module does not read the fonts: a
//!   string that begins with a byte-order mark is read as UTF-16 —
//!   big-endian or little-endian, as the mark says — and any other
//!   string is read as Latin-1, the byte's own value as a character.
//!   The answer says which of the three it was.
//!
//! # What it does not do
//!
//! - **The place of the text is not worked out.** A piece of text is
//!   where it was shown, and the tree does not lay out a page.
//! - **The fonts are not read.** The encoding of a font, its codes and
//!   its differences are not here, so a text in a font that is not
//!   UTF-16 is read as the bytes it is.

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
    /// How the bytes were read: `utf-16be`, `utf-16le` or
    /// `latin-1`.
    pub encoding: String,
    /// Whether the operator moved to the next line after it.
    pub break_after: bool,
}

impl Piece {
    /// The piece, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "text": self.text.clone(),
            "operator": self.operator.clone(),
            "encoding": self.encoding.clone(),
            "line_break": self.break_after,
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
    for value in listed {
        let Some(stream) = crate::preflight::node_of(document, &value) else { continue };
        if stream.kind != "pdf.stream" {
            continue;
        }
        found.streams += 1;
        let decoded = crate::decoded_bytes(stream, None, None, false, COMMAND)?;
        let operations = crate::content::operations(&decoded.bytes).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the content of the page cannot be read: {error:?}"),
        })?;
        let mut pieces = pieces_of(&operations);
        found.pieces.append(&mut pieces);
    }
    Ok(found)
}

/// The **pieces** of text a set of operations shows.
fn pieces_of(operations: &[crate::content::Operation]) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::new();
    // Text is shown inside a **text object**.
    let mut in_text = false;
    for operation in operations {
        match operation.operator.as_str() {
            "BT" => in_text = true,
            "ET" => in_text = false,
            "Tj" => {
                if !in_text {
                    continue;
                }
                let Some((text, encoding)) = operation.operands.last().and_then(string_of) else { continue };
                pieces.push(Piece { text, operator: "Tj".to_string(), encoding, break_after: false });
            }
            "TJ" => {
                if !in_text {
                    continue;
                }
                let Some(array) = operation.operands.last().and_then(|value| value.as_array()) else {
                    continue;
                };
                let (text, encoding) = shown(array);
                if text.is_empty() {
                    continue;
                }
                pieces.push(Piece { text, operator: "TJ".to_string(), encoding, break_after: false });
            }
            "'" | "\"" => {
                if !in_text {
                    continue;
                }
                let Some((text, encoding)) = operation.operands.last().and_then(string_of) else { continue };
                pieces.push(Piece {
                    text,
                    operator: operation.operator.clone(),
                    encoding,
                    break_after: true,
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
fn shown(array: &[Json]) -> (String, String) {
    let mut text = String::new();
    let mut encoding = "latin-1".to_string();
    for item in array {
        match item {
            Json::String(_) => {
                let Some((piece, kind)) = string_of(item) else { continue };
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

/// A **string** of a content stream, and how its bytes were read.
///
/// The bytes of a PDF string are the string's characters, each one
/// below 256, and what they mean depends on the font. A string that
/// begins with a byte-order mark is UTF-16; any other string is
/// Latin-1.
fn string_of(value: &Json) -> Option<(String, String)> {
    let Json::String(text) = value else { return None };
    let bytes: Vec<u8> = text.chars().map(|character| character as u32 as u8).collect();
    Some(decode(&bytes))
}

/// The **text** of a string's bytes, and how they were read.
fn decode(bytes: &[u8]) -> (String, String) {
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
