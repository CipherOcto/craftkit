//! The **Unicode map** of a font: the characters its codes stand for.
//!
//! A font's **codes** are the bytes it uses for its characters, and
//! they are not the characters: a font may use one code for `A` and
//! another for `a`, and the codes of a font that is not a Latin one
//! say nothing at all about the letters.
//!
//! A font may therefore carry a **map**: a `ToUnicode` CMap that says
//! what each of its codes stands for. The map is a stream of text —
//! the CMap the specification describes — and it holds two kinds of
//! line:
//!
//! - a **character**: a code and the text it stands for;
//! - a **range**: a first code, a last code, and the text they stand
//!   for — either one text for all of them, or a list of texts.
//!
//! The rules:
//!
//! - a font with **no map** has no Unicode, and the answer says so;
//! - the text of a code is read as **UTF-16**, the way a CMap writes
//!   it, and a code that stands for two characters stands for two
//!   characters;
//! - a **range** with one text gives that text to every code in it,
//!   and a range with a list gives each code its own;
//! - a line the CMap does not have is **skipped**, and the answer says
//!   what it skipped;
//! - the **CID** maps — the codes of a font that is not a Unicode
//!   font — are not read: they stand for glyphs and not for
//!   characters.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome, Session};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.font.unicode";

/// One **entry** of a map: a code and the text it stands for.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The code of the font.
    pub code: u16,
    /// The text the code stands for.
    pub text: String,
}

impl Entry {
    /// The entry, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "code": self.code,
            "text": self.text.clone(),
        })
    }
}

/// The **hex digits** of a CMap line, as a number.
///
/// The digits are the code, written as bytes: a code of one byte is
/// two digits and a code of two bytes is four.
fn code_of(text: &str) -> Option<u16> {
    let digits: String = text.chars().filter(|character| character.is_ascii_hexdigit()).collect();
    if digits.is_empty() || !digits.len().is_multiple_of(2) {
        return None;
    }
    u16::from_str_radix(&digits, 16).ok()
}

/// The **text** a CMap line holds, read as UTF-16.
///
/// The text of a CMap is written as bytes, and those bytes are the
/// characters of the text, each one two bytes and in the order of the
/// big end.
fn text_of(text: &str) -> Option<String> {
    let digits: String = text.chars().filter(|character| character.is_ascii_hexdigit()).collect();
    if digits.is_empty() || !digits.len().is_multiple_of(4) {
        return None;
    }
    let mut units: Vec<u16> = Vec::with_capacity(digits.len() / 4);
    for at in (0..digits.len()).step_by(4) {
        let unit = u16::from_str_radix(&digits[at..at + 4], 16).ok()?;
        units.push(unit);
    }
    Some(String::from_utf16_lossy(&units))
}

/// The **tokens** of a CMap line: a word, or a list in brackets.
///
/// A list is one token: the texts of a range with a list are between
/// brackets, and a list split at the spaces would be a list of one.
fn tokens_of(body: &str) -> Vec<String> {
    let mut tokens: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut in_list = false;
    for character in body.chars() {
        match character {
            '[' => {
                in_list = true;
                current.push(character);
            }
            ']' => {
                in_list = false;
                current.push(character);
                tokens.push(std::mem::take(&mut current));
            }
            other if other.is_whitespace() && !in_list => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            other => current.push(other),
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// The **map** of a font: its entries and the length of its codes.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Map {
    /// The entries of the map.
    pub entries: Vec<Entry>,
    /// How many **bytes** a code of the font is: one for a simple font
    /// and two for a font of composite fonts.
    pub code_length: usize,
}

impl Map {
    /// The **text** a code stands for.
    pub fn text_of(&self, code: u16) -> Option<&str> {
        self.entries.iter().find(|entry| entry.code == code).map(|entry| entry.text.as_str())
    }
}

/// The **entries** of a CMap, and the lines it holds that this tree
/// does not read.
pub fn map_of(bytes: &[u8]) -> (Map, Vec<String>) {
    let text = String::from_utf8_lossy(bytes);
    let mut entries: Vec<Entry> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    let mut code_length = 0usize;
    // The **code space**: the bytes a code of the font is.
    if let Some(found) = text.find("begincodespacerange") {
        let start = found + "begincodespacerange".len();
        if let Some(end) = text[start..].find("endcodespacerange") {
            let body = &text[start..start + end];
            if let Some(token) = body.split_whitespace().next() {
                let digits: String =
                    token.chars().filter(|character| character.is_ascii_hexdigit()).collect();
                if !digits.is_empty() && digits.len().is_multiple_of(2) {
                    code_length = digits.len() / 2;
                }
            }
        }
    }
    for section in ["bfchar", "bfrange"] {
        let mut at = 0usize;
        while let Some(found) = text[at..].find(&format!("begin{section}")) {
            let start = at + found + format!("begin{section}").len();
            let Some(end) = text[start..].find(&format!("end{section}")) else { break };
            let body = &text[start..start + end];
            at = start + end;
            let parts: Vec<String> = tokens_of(body);
            if section == "bfchar" {
                let mut index = 0usize;
                while index + 1 < parts.len() {
                    let (Some(code), Some(text)) = (code_of(&parts[index]), text_of(&parts[index + 1]))
                    else {
                        skipped.push(format!("{section}: a line is not a code and a text"));
                        index += 2;
                        continue;
                    };
                    entries.push(Entry { code, text });
                    index += 2;
                }
            } else {
                let mut index = 0usize;
                while index + 2 < parts.len() {
                    let Some(first) = code_of(&parts[index]) else {
                        skipped.push(format!("{section}: a line does not begin with a code"));
                        break;
                    };
                    let Some(last) = code_of(&parts[index + 1]) else {
                        skipped.push(format!("{section}: a line has no last code"));
                        break;
                    };
                    let third = parts[index + 2].clone();
                    if third.starts_with('[') {
                        // A **list**: each code has its own text.
                        let list: Vec<&str> =
                            third.trim_matches(|c| c == '[' || c == ']').split_whitespace().collect();
                        for (at, item) in list.iter().enumerate() {
                            let code = first + at as u16;
                            let Some(text) = text_of(item) else {
                                skipped.push(format!("{section}: a text is not a text"));
                                continue;
                            };
                            entries.push(Entry { code, text });
                        }
                    } else {
                        let Some(text) = text_of(&third) else {
                            skipped.push(format!("{section}: a text is not a text"));
                            index += 3;
                            continue;
                        };
                        if last >= first {
                            for code in first..=last {
                                entries.push(Entry { code, text: text.clone() });
                            }
                        }
                    }
                    index += 3;
                }
            }
        }
    }
    entries.sort_by_key(|entry| entry.code);
    entries.dedup_by_key(|entry| entry.code);
    (Map { entries, code_length }, skipped)
}

/// The **Unicode map** of a font: the characters its codes stand for.
pub fn unicode_of(document: &Document, font: u64) -> Result<Option<Map>, EngineError> {
    let Some(node) = document.node(font) else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("there is no node `{font}`"),
        });
    };
    let entries = crate::entries_of(&node.data);
    let Some(to_unicode) = entries.iter().find(|(key, _)| key == "ToUnicode").map(|(_, value)| value.clone())
    else {
        // A font with **no map** has no Unicode.
        return Ok(None);
    };
    let Some(stream) = crate::preflight::node_of(document, &to_unicode) else {
        return Ok(None);
    };
    if stream.kind != "pdf.stream" {
        return Ok(None);
    }
    let decoded = crate::decoded_bytes(stream, None, None, false, COMMAND)?;
    let (map, _) = map_of(&decoded.bytes);
    Ok(Some(map))
}

/// **Read** the **Unicode map** of a font.
///
/// A query: it does not change the document. A font with no map has
/// no Unicode, and the answer says so.
#[derive(Command)]
#[command(id = "pdf.font.unicode", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadFontUnicode {
    /// The font to read.
    pub font: craftkit_sdk::NodeRef,
}

impl ReadFontUnicode {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let font = self.font.0;
        let Some(node) = session.document().node(font) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("there is no node `{font}`"),
            });
        };
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!("the node `{font}` is a `{}` and not an object", node.kind),
            });
        }
        let map = unicode_of(session.document(), font)?;
        let Some(map) = map else {
            return Ok(Outcome::new("the font has no Unicode map").with_data(json!({
                "font": font,
                "has_map": false,
                "entries": Vec::<Json>::new(),
                "count": 0,
            })));
        };
        let listed: Vec<Json> = map.entries.iter().map(|entry| entry.to_json()).collect();
        let count = listed.len();
        Ok(Outcome::new(format!("the font has {count} mapped code(s)")).with_data(json!({
            "font": font,
            "has_map": true,
            "entries": listed,
            "count": count,
        })))
    }
}
