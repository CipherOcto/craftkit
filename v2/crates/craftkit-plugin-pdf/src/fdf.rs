//! The **FDF file** of the data of a form: the values of the fields, in
//! the syntax of PDF.
//!
//! The tree's own file: the version line of the FDF, one object, and a
//! trailer that names it as the root.
//!
//! # The rules
//!
//! - The file is a file of **PDF syntax**: the header, the object, the
//!   trailer and the end.
//! - The **fields** of the form are written in the `/Fields` array, and
//!   each one is a dictionary with its **name** (`/T`) and its **value**
//!   (`/V`).
//! - The **value** of a **button** is a **name** — the state it holds,
//!   or `Off` when it holds none — and the value of every other field
//!   is a **string**.
//! - A **push button** is left out: it holds nothing, and the tree
//!   leaves it out too.
//!
//! # What it does not settle
//!
//! - The **comments** of the tree's FDF are not here: this carries the
//!   values of the fields and not the annotations.
//! - The **appearances** of the fields are not carried: the file holds
//!   the values and nothing else.
//! - The file is read back here, but only its **values**: the comments
//!   of the tree's FDF are not read.

use craftkit_sdk::serde_json::{Value as Json, json};

use crate::fields::{Field, FieldKind};

/// The **flag** of a **push button**: the sixteenth bit.
const PUSH_BUTTON: u32 = 1 << 16;

/// A **text** of a file, as the file writes it.
fn text(value: &str) -> String {
    format!("({})", crate::writer::escape_text(value))
}

/// The **value** of a field, as the file writes it.
///
/// The tree's own rule: a **button** holds a **state**, and a state is
/// a name; every other field holds a **string**.
fn value_of_field(field: &Field) -> String {
    match field.kind {
        FieldKind::Button => {
            let value = field.value.clone().unwrap_or_else(|| "Off".to_string());
            format!("/{}", crate::writer::escape_name(&value))
        }
        _ => text(field.value.as_deref().unwrap_or("")),
    }
}

/// Whether the field is a **push button**: it holds nothing.
fn is_push_button(field: &Field) -> bool {
    field.kind == FieldKind::Button && (field.flags as u32) & PUSH_BUTTON != 0
}

/// The **FDF file** of the data of the fields, and how many fields it
/// holds.
///
/// The tree's own file: the header of the FDF, one object with the
/// dictionary of the data in it, and the trailer that names the object
/// as the root.
pub fn fdf_of(fields: &[Field], file: &str) -> (Vec<u8>, usize) {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"%FDF-1.2\n%\xE2\xE3\xCF\xD3\n1 0 obj\n");
    out.extend_from_slice(b"<< /FDF << /Fields [");
    let mut count = 0usize;
    for field in fields {
        if is_push_button(field) {
            continue;
        }
        out.extend_from_slice(b"<< /T ");
        out.extend_from_slice(text(&field.name).as_bytes());
        out.extend_from_slice(b" /V ");
        out.extend_from_slice(value_of_field(field).as_bytes());
        out.extend_from_slice(b" >> ");
        count += 1;
    }
    out.extend_from_slice(b"] ");
    out.extend_from_slice(b"/F ");
    out.extend_from_slice(text(file).as_bytes());
    out.extend_from_slice(b" >> >>\nendobj\ntrailer\n<< /Root 1 0 R >>\n%%EOF\n");
    (out, count)
}

/// A **token** of a file of PDF syntax.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// A **word**: a number, a keyword, or the name of an object.
    Word(String),
    /// A **name**: `/Name`.
    Name(String),
    /// A **text**: a string in `( )` or in `< >`.
    Text(String),
    /// A **number**.
    Number(f64),
    /// The `<<` of a dictionary.
    DictOpen,
    /// The `>>` of a dictionary.
    DictClose,
    /// The `[` of an array.
    ArrayOpen,
    /// The `]` of an array.
    ArrayClose,
}

/// The **characters** that end a word.
fn is_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'\t' | b'\r' | b'\n' | b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

/// The **tokens** of a file of PDF syntax.
fn tokens_of(bytes: &[u8]) -> Result<Vec<Token>, String> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut at = 0usize;
    while at < bytes.len() {
        let byte = bytes[at];
        match byte {
            b' ' | b'\t' | b'\r' | b'\n' => at += 1,
            b'%' => {
                // A **comment**: the rest of the line.
                while at < bytes.len() && bytes[at] != b'\n' {
                    at += 1;
                }
            }
            b'(' => {
                let (text, next) = literal_string(bytes, at)?;
                tokens.push(Token::Text(text));
                at = next;
            }
            b'<' => {
                if bytes.get(at + 1) == Some(&b'<') {
                    tokens.push(Token::DictOpen);
                    at += 2;
                } else {
                    let (text, next) = hex_string(bytes, at)?;
                    tokens.push(Token::Text(text));
                    at = next;
                }
            }
            b'>' => {
                if bytes.get(at + 1) == Some(&b'>') {
                    tokens.push(Token::DictClose);
                    at += 2;
                } else {
                    return Err("a `>` that is not a `>>`".to_string());
                }
            }
            b'[' => {
                tokens.push(Token::ArrayOpen);
                at += 1;
            }
            b']' => {
                tokens.push(Token::ArrayClose);
                at += 1;
            }
            b'/' => {
                let mut name = String::new();
                at += 1;
                while at < bytes.len() && !is_delimiter(bytes[at]) {
                    // A `#` and two digits are the **escape** of a name:
                    // the byte they say is the byte of the name.
                    if bytes[at] == b'#' {
                        let pair = bytes.get(at + 1..at + 3).and_then(|pair| {
                            std::str::from_utf8(pair).ok().and_then(|pair| u8::from_str_radix(pair, 16).ok())
                        });
                        let Some(byte) = pair else {
                            return Err("a name holds an escape that is not one".to_string());
                        };
                        name.push(byte as char);
                        at += 3;
                        continue;
                    }
                    name.push(bytes[at] as char);
                    at += 1;
                }
                tokens.push(Token::Name(name));
            }
            _ => {
                let mut word = String::new();
                while at < bytes.len() && !is_delimiter(bytes[at]) {
                    word.push(bytes[at] as char);
                    at += 1;
                }
                if word.is_empty() {
                    return Err(format!(
                        "the file holds a character that may not be in it: {}",
                        byte as char
                    ));
                }
                match word.parse::<f64>() {
                    Ok(number) if number.is_finite() => tokens.push(Token::Number(number)),
                    _ => tokens.push(Token::Word(word)),
                }
            }
        }
    }
    Ok(tokens)
}

/// A **string** in `( )`, with the escapes of the syntax.
fn literal_string(bytes: &[u8], at: usize) -> Result<(String, usize), String> {
    let mut out = String::new();
    let mut depth = 1usize;
    let mut cursor = at + 1;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        match byte {
            b'\\' => {
                let Some(next) = bytes.get(cursor + 1) else {
                    return Err("a string ends in the middle of an escape".to_string());
                };
                match next {
                    b'n' => out.push('\n'),
                    b'r' => out.push('\r'),
                    b't' => out.push('\t'),
                    b'b' => out.push('\u{8}'),
                    b'f' => out.push('\u{c}'),
                    b'(' => out.push('('),
                    b')' => out.push(')'),
                    b'\\' => out.push('\\'),
                    b'\n' => {}
                    b'\r' => {
                        cursor += 1;
                        if bytes.get(cursor + 1) == Some(&b'\n') {
                            cursor += 1;
                        }
                    }
                    b'0'..=b'7' => {
                        let mut value = 0u32;
                        let mut digits = 0usize;
                        while digits < 3 {
                            let Some(digit) = bytes.get(cursor + 1 + digits) else { break };
                            let Some(digit) = (*digit as char).to_digit(8) else { break };
                            value = value * 8 + digit;
                            digits += 1;
                        }
                        cursor += digits;
                        out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
                    }
                    other => out.push(*other as char),
                }
                cursor += 2;
            }
            b'(' => {
                depth += 1;
                out.push('(');
                cursor += 1;
            }
            b')' => {
                depth -= 1;
                cursor += 1;
                if depth == 0 {
                    return Ok((out, cursor));
                }
                out.push(')');
            }
            other => {
                out.push(other as char);
                cursor += 1;
            }
        }
    }
    Err("a string is not closed".to_string())
}

/// A **string** in `< >`: the bytes of it, read as Latin-1.
fn hex_string(bytes: &[u8], at: usize) -> Result<(String, usize), String> {
    let mut digits = String::new();
    let mut cursor = at + 1;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'>' {
            if !digits.len().is_multiple_of(2) {
                digits.push('0');
            }
            let mut out = String::new();
            let raw = digits.as_bytes();
            let mut index = 0usize;
            while index + 1 < raw.len() {
                let pair = std::str::from_utf8(&raw[index..index + 2]).unwrap_or("00");
                let value = u8::from_str_radix(pair, 16).unwrap_or(0);
                out.push(value as char);
                index += 2;
            }
            return Ok((out, cursor + 1));
        }
        if byte.is_ascii_hexdigit() {
            digits.push(byte as char);
        }
        cursor += 1;
    }
    Err("a string is not closed".to_string())
}

/// A **reader** of the tokens of a file.
struct Reader {
    tokens: Vec<Token>,
    at: usize,
}

impl Reader {
    /// The **token** that is next, and it is not taken.
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.at)
    }

    /// The **token** that is next, and it is taken.
    fn take(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.at).cloned();
        if token.is_some() {
            self.at += 1;
        }
        token
    }

    /// The **value** that is next.
    fn value(&mut self) -> Result<Json, String> {
        match self.peek() {
            None => Err("the file ends in the middle of a value".to_string()),
            Some(Token::DictOpen) => {
                self.take();
                self.dictionary()
            }
            Some(Token::ArrayOpen) => {
                self.take();
                self.array()
            }
            Some(Token::Name(name)) => {
                let name = name.clone();
                self.take();
                // A **name** is a plain string: this is how the writer
                // of the files of PDF spells one.
                Ok(Json::String(name))
            }
            Some(Token::Text(text)) => {
                let text = text.clone();
                self.take();
                Ok(json!({ "string": text }))
            }
            Some(Token::Number(number)) => {
                let number = *number;
                self.take();
                let Some(value) = craftkit_sdk::serde_json::Number::from_f64(number) else {
                    return Err("a number of the file is not finite".to_string());
                };
                Ok(Json::Number(value))
            }
            Some(Token::Word(word)) => {
                let word = word.clone();
                self.take();
                match word.as_str() {
                    "true" => Ok(Json::Bool(true)),
                    "false" => Ok(Json::Bool(false)),
                    "null" => Ok(Json::Null),
                    other => Err(format!("`{other}` is not a value")),
                }
            }
            Some(other) => Err(format!("{other:?} is not a value")),
        }
    }

    /// The **entries** of a dictionary.
    fn dictionary(&mut self) -> Result<Json, String> {
        let mut entries: Vec<Json> = Vec::new();
        loop {
            match self.peek() {
                Some(Token::DictClose) => {
                    self.take();
                    break;
                }
                None => return Err("the file ends in the middle of a dictionary".to_string()),
                _ => {}
            }
            let key = match self.take() {
                Some(Token::Name(name)) => name,
                Some(other) => return Err(format!("{other:?} is not the name of an entry")),
                None => return Err("the file ends in the middle of a dictionary".to_string()),
            };
            let value = self.value()?;
            entries.push(json!([key, value]));
        }
        Ok(json!({ "entries": entries }))
    }

    /// The **values** of an array.
    fn array(&mut self) -> Result<Json, String> {
        let mut values: Vec<Json> = Vec::new();
        loop {
            match self.peek() {
                Some(Token::ArrayClose) => {
                    self.take();
                    break;
                }
                None => return Err("the file ends in the middle of an array".to_string()),
                _ => {}
            }
            values.push(self.value()?);
        }
        Ok(Json::Array(values))
    }
}

/// The **entries** of a dictionary, as the pairs of it.
fn entries_of(value: &Json) -> Vec<(String, Json)> {
    value
        .get("entries")
        .and_then(|entries| entries.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    let key = pair.first()?.as_str()?.to_string();
                    let value = pair.get(1)?.clone();
                    Some((key, value))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **value** under a key of a dictionary.
fn value_of(value: &Json, key: &str) -> Option<Json> {
    entries_of(value).into_iter().find(|(held, _)| held == key).map(|(_, value)| value)
}

/// The **text** of a value: a string or a name.
fn text_of(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(Json::as_str).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **values** of an FDF file: the name of each field and the value
/// it holds.
///
/// The tree's own file: the values are in the `/Fields` array of the
/// `/FDF` dictionary, and each one has its **name** (`/T`) and its
/// **value** (`/V`).
pub fn values_of(bytes: &[u8]) -> Result<Vec<(String, Option<String>)>, String> {
    let tokens = tokens_of(bytes)?;
    // The **object**: the words before it are the header of the file and
    // the number of the object.
    let after = tokens
        .iter()
        .position(|token| matches!(token, Token::Word(word) if word == "obj"))
        .map(|at| at + 1)
        .unwrap_or(0);
    let mut reader = Reader { tokens, at: after };
    let value = reader.value()?;
    let fdf = value_of(&value, "FDF").ok_or("the file has no FDF dictionary in it")?;
    let fields = value_of(&fdf, "Fields").ok_or("the file has no fields in it")?;
    let Some(list) = fields.as_array() else {
        return Err("the fields of the file are a list".to_string());
    };
    let mut out: Vec<(String, Option<String>)> = Vec::new();
    for field in list {
        let name = value_of(field, "T")
            .and_then(|value| text_of(&value))
            .ok_or("a field of the file has no name")?;
        let value = match value_of(field, "V") {
            None => None,
            Some(value) => match value {
                Json::Array(values) => values.first().and_then(text_of),
                other => text_of(&other),
            },
        };
        out.push((name, value));
    }
    Ok(out)
}
