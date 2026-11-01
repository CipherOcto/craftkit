//! The **objects** of a PDF: the values a file is made of.
//!
//! A PDF file is made of **objects**, and each one is a value: a
//! number, a name, a string, an array, a dictionary, a boolean, null,
//! or a **reference** to another object. This module reads those
//! values, and it reads the **object stream**: the stream that holds
//! a file's objects in one place, so that they may be compressed
//! together.
//!
//! # The object stream
//!
//! An object stream's dictionary says how many objects it holds — the
//! entry `/N` — and where the first one's data begins — the entry
//! `/First`. The **header** is that many pairs of integers: the number
//! of an object and the offset of its data, counted from `/First`.
//!
//! # What it does not do
//!
//! - **There is no writer.** A file may be read and not written.
//! - **The references are not resolved.** A reference says which
//!   object it names; finding that object is the file's business.

use craftkit_sdk::serde_json::{Map, Value as Json};

/// Why an object could not be **read**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not what an object is.
    Malformed(String),
}

/// An **object** of an object stream: its number and its value.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// The object's number in the file.
    pub number: u64,
    /// What the object says.
    pub value: Json,
}

/// Read the **objects** of an object stream.
///
/// The stream's bytes are the ones the file holds — decoded already —
/// and the count and the first offset come from the stream's own
/// dictionary.
pub fn object_stream(bytes: &[u8], count: usize, first: usize) -> Result<Vec<Object>, Error> {
    let mut at = 0_usize;
    let mut objects: Vec<Object> = Vec::with_capacity(count.min(1024));
    for _ in 0..count {
        let (number, after) = integer(bytes, at)?;
        let (offset, after) = integer(bytes, after)?;
        let number = u64::try_from(number)
            .map_err(|_| Error::Malformed("an object's number is not a whole number".to_string()))?;
        let offset = usize::try_from(offset)
            .map_err(|_| Error::Malformed("an object's offset is not a whole number".to_string()))?;
        let start = first
            .checked_add(offset)
            .ok_or_else(|| Error::Malformed("an object's offset is out of the stream".to_string()))?;
        let (value, _) = value(bytes, start)?;
        objects.push(Object { number, value });
        at = after;
    }
    Ok(objects)
}

/// The **value** at a place, and where it ended.
fn value(bytes: &[u8], at: usize) -> Result<(Json, usize), Error> {
    let at = skip_space(bytes, at);
    let Some(byte) = bytes.get(at).copied() else {
        return Err(Error::Malformed("the object ends where it begins".to_string()));
    };
    match byte {
        b'/' => {
            let (name, after) = name(bytes, at)?;
            Ok((Json::String(name), after))
        }
        b'(' => {
            let (text, after) = literal_string(bytes, at)?;
            Ok((Json::String(text), after))
        }
        b'<' => {
            if bytes.get(at + 1) == Some(&b'<') {
                let (dict, after) = dictionary(bytes, at)?;
                Ok((dict, after))
            } else {
                let (text, after) = hex_string(bytes, at)?;
                Ok((Json::String(text), after))
            }
        }
        b'[' => {
            let (list, after) = array(bytes, at)?;
            Ok((list, after))
        }
        b']' | b'}' | b')' => Err(Error::Malformed("a value with nothing before it".to_string())),
        b'+' | b'-' | b'.' | b'0'..=b'9' => number_or_reference(bytes, at),
        _ => {
            let (word, after) = keyword(bytes, at)?;
            match word.as_str() {
                "true" => Ok((Json::Bool(true), after)),
                "false" => Ok((Json::Bool(false), after)),
                "null" => Ok((Json::Null, after)),
                other => Err(Error::Malformed(format!("the word `{other}` is not a value"))),
            }
        }
    }
}

/// A **number**, or a **reference** to another object.
///
/// A reference is two numbers and the letter `R`, and it is the one
/// thing that is not a value: it says which object is meant.
fn number_or_reference(bytes: &[u8], at: usize) -> Result<(Json, usize), Error> {
    let (first, after_first) = number(bytes, at)?;
    let first = first.as_f64().ok_or_else(|| Error::Malformed("a number is not a number".to_string()))?;
    // A **reference**: a second number and the letter `R`.
    let probe = skip_space(bytes, after_first);
    if let Ok((second, after_second)) = number(bytes, probe)
        && let Some(second) = second.as_f64()
    {
        let after = skip_space(bytes, after_second);
        if bytes.get(after) == Some(&b'R') {
            let mut map = Map::new();
            map.insert("ref".to_string(), Json::Array(vec![number_value(first)?, number_value(second)?]));
            return Ok((Json::Object(map), after + 1));
        }
    }
    Ok((number_value(first)?, after_first))
}

/// A **number**.
fn number(bytes: &[u8], at: usize) -> Result<(Json, usize), Error> {
    let start = at;
    let mut at = at;
    if matches!(bytes.get(at), Some(b'+') | Some(b'-')) {
        at += 1;
    }
    let mut seen_digit = false;
    let mut seen_dot = false;
    while let Some(byte) = bytes.get(at).copied() {
        match byte {
            b'0'..=b'9' => {
                seen_digit = true;
                at += 1;
            }
            b'.' if !seen_dot => {
                seen_dot = true;
                at += 1;
            }
            _ => break,
        }
    }
    if !seen_digit {
        return Err(Error::Malformed("a number has no digits".to_string()));
    }
    let text = String::from_utf8_lossy(&bytes[start..at]).trim().to_string();
    let Ok(number) = text.parse::<f64>() else {
        return Err(Error::Malformed(format!("the number `{text}` is not a number")));
    };
    if !number.is_finite() {
        return Err(Error::Malformed(format!("the number `{text}` is not finite")));
    }
    Ok((number_value(number)?, at))
}

/// A **number** as the caller gets it.
fn number_value(number: f64) -> Result<Json, Error> {
    let Some(value) = craftkit_sdk::serde_json::Number::from_f64(number) else {
        return Err(Error::Malformed("a number is not finite".to_string()));
    };
    Ok(Json::Number(value))
}

/// A **whole number**, for the header of an object stream.
fn integer(bytes: &[u8], at: usize) -> Result<(i64, usize), Error> {
    let (value, after) = number(bytes, skip_space(bytes, at))?;
    let number = value.as_f64().ok_or_else(|| Error::Malformed("a number is not a number".to_string()))?;
    if number.fract() != 0.0 {
        return Err(Error::Malformed("a whole number is wanted".to_string()));
    }
    Ok((number as i64, after))
}

/// A **name**: `/Name`.
fn name(bytes: &[u8], at: usize) -> Result<(String, usize), Error> {
    let mut at = at + 1;
    let mut name: Vec<u8> = Vec::new();
    while let Some(byte) = bytes.get(at).copied() {
        if is_space(byte) || is_delimiter(byte) {
            break;
        }
        at += 1;
        if byte == b'#' {
            let high = bytes.get(at).copied();
            let low = bytes.get(at + 1).copied();
            match (high, low) {
                (Some(high), Some(low)) => match (hex(high), hex(low)) {
                    (Some(high), Some(low)) => {
                        name.push(high * 16 + low);
                        at += 2;
                    }
                    _ => return Err(Error::Malformed("a name has a bad escape".to_string())),
                },
                _ => return Err(Error::Malformed("a name has a bad escape".to_string())),
            }
        } else {
            name.push(byte);
        }
    }
    Ok((format!("/{}", String::from_utf8_lossy(&name)), at))
}

/// A **literal string**: `(text)`.
fn literal_string(bytes: &[u8], at: usize) -> Result<(String, usize), Error> {
    let mut at = at + 1;
    let mut text: Vec<u8> = Vec::new();
    let mut depth = 1_usize;
    while let Some(byte) = bytes.get(at).copied() {
        at += 1;
        match byte {
            b'\\' => {
                let Some(next) = bytes.get(at).copied() else {
                    return Err(Error::Malformed("a string ends in an escape".to_string()));
                };
                at += 1;
                match next {
                    b'n' => text.push(b'\n'),
                    b'r' => text.push(b'\r'),
                    b't' => text.push(b'\t'),
                    b'b' => text.push(0x08),
                    b'f' => text.push(0x0c),
                    b'(' => text.push(b'('),
                    b')' => text.push(b')'),
                    b'\\' => text.push(b'\\'),
                    b'0'..=b'7' => {
                        let mut value = u32::from(next - b'0');
                        for _ in 0..2 {
                            let Some(digit) = bytes.get(at).copied() else { break };
                            if !(b'0'..=b'7').contains(&digit) {
                                break;
                            }
                            value = value * 8 + u32::from(digit - b'0');
                            at += 1;
                        }
                        text.push((value & 0xff) as u8);
                    }
                    other => text.push(other),
                }
            }
            b'(' => {
                depth += 1;
                text.push(byte);
            }
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((text.iter().map(|byte| *byte as char).collect(), at));
                }
                text.push(byte);
            }
            other => text.push(other),
        }
    }
    Err(Error::Malformed("a string is never closed".to_string()))
}

/// A **hex string**: `<48656C6C6F>`.
fn hex_string(bytes: &[u8], at: usize) -> Result<(String, usize), Error> {
    let mut at = at + 1;
    let mut digits: Vec<u8> = Vec::new();
    while let Some(byte) = bytes.get(at).copied() {
        at += 1;
        if byte == b'>' {
            break;
        }
        if is_space(byte) {
            continue;
        }
        let Some(value) = hex(byte) else {
            return Err(Error::Malformed("a hex string has a bad digit".to_string()));
        };
        digits.push(value);
    }
    if digits.len() % 2 == 1 {
        digits.push(0);
    }
    let text: String = digits.chunks(2).map(|pair| pair[0] * 16 + pair[1]).map(|byte| byte as char).collect();
    Ok((text, at))
}

/// A **dictionary**: `<< /Key value >>`.
///
/// The dictionary is a list of **pairs**, as the kit's own dictionaries
/// are: a JSON object does not keep the order of its keys.
fn dictionary(bytes: &[u8], at: usize) -> Result<(Json, usize), Error> {
    let mut at = at + 2;
    let mut pairs: Vec<Json> = Vec::new();
    loop {
        at = skip_space(bytes, at);
        if bytes.get(at) == Some(&b'>') && bytes.get(at + 1) == Some(&b'>') {
            return Ok((Json::Array(pairs), at + 2));
        }
        if at >= bytes.len() {
            return Err(Error::Malformed("a dictionary is never closed".to_string()));
        }
        let (key, after) = name(bytes, at)?;
        let (value, after) = value(bytes, after)?;
        pairs.push(Json::Array(vec![Json::String(key), value]));
        at = after;
    }
}

/// The **pairs** of a dictionary, and where the dictionary ended.
///
/// The pairs are the dictionary's keys and values, in the order the
/// file had them.
pub fn dictionary_of(bytes: &[u8], at: usize) -> Result<(Vec<(String, Json)>, usize), Error> {
    let (value, after) = dictionary(bytes, skip_space(bytes, at))?;
    let mut pairs: Vec<(String, Json)> = Vec::new();
    if let Some(list) = value.as_array() {
        for pair in list {
            let Some(pair) = pair.as_array() else { continue };
            let Some(key) = pair.first().and_then(|key| key.as_str()) else { continue };
            let Some(value) = pair.get(1) else { continue };
            pairs.push((key.to_string(), value.clone()));
        }
    }
    Ok((pairs, after))
}

/// An **array**: `[1 2 3]`.
fn array(bytes: &[u8], at: usize) -> Result<(Json, usize), Error> {
    let mut at = at + 1;
    let mut values: Vec<Json> = Vec::new();
    loop {
        at = skip_space(bytes, at);
        match bytes.get(at) {
            Some(b']') => return Ok((Json::Array(values), at + 1)),
            None => return Err(Error::Malformed("an array is never closed".to_string())),
            _ => {}
        }
        let (value, after) = value(bytes, at)?;
        values.push(value);
        at = after;
    }
}

/// A **word**: `true`, `false` or `null`.
fn keyword(bytes: &[u8], at: usize) -> Result<(String, usize), Error> {
    let start = at;
    let mut at = at;
    while let Some(byte) = bytes.get(at).copied() {
        if is_space(byte) || is_delimiter(byte) {
            break;
        }
        at += 1;
    }
    if at == start {
        return Err(Error::Malformed("the bytes say nothing".to_string()));
    }
    Ok((String::from_utf8_lossy(&bytes[start..at]).to_string(), at))
}

/// The **space** before a value.
fn skip_space(bytes: &[u8], at: usize) -> usize {
    let mut at = at;
    while let Some(byte) = bytes.get(at).copied() {
        if is_space(byte) {
            at += 1;
        } else {
            break;
        }
    }
    at
}

/// Whether a byte is **space**.
fn is_space(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | b'\r' | 0x0c | 0x00)
}

/// Whether a byte **ends** a token.
fn is_delimiter(byte: u8) -> bool {
    matches!(byte, b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%')
}

/// The **value** of a hex digit.
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}
