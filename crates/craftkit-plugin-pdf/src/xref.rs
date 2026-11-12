//! The **cross-reference table**: the file's own index of itself.
//!
//! A PDF file does not hold its objects in order. It holds them
//! wherever they were written, and a **cross-reference table** says
//! where each one is: the object number, the byte offset and the
//! generation. The table is found from the end of the file, which says
//! where the table is.
//!
//! There are two kinds of table, and this module reads both:
//!
//! - the **classic table**, a list of twenty-byte entries under the
//!   word `xref`;
//! - the **cross-reference stream**, a stream that holds the same
//!   information in binary, and which is how a file compresses its own
//!   index.
//!
//! # What it does not do
//!
//! - **The objects are not read.** The table says where an object is;
//!   fetching it is the file's business.
//! - **The **incremental save** is not followed.** A file that has been
//!   saved again has more than one table, and the newest one wins.

use craftkit_sdk::serde_json::Value as Json;

use crate::filters;
use crate::objects;

/// The error of the **objects** module, as the table's own.
fn from_objects(error: objects::Error) -> Error {
    match error {
        objects::Error::Malformed(message) => Error::Malformed(message),
    }
}

/// Why a cross-reference table could not be **read**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not what a cross-reference table is.
    Malformed(String),
    /// The kit does not know how to read this.
    Unsupported(String),
}

/// Where an object is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The object is **in the file**, at an offset.
    InFile,
    /// The object is **free**: it was once in the file and is not now.
    Free,
    /// The object is **compressed**: it is in an object stream.
    Compressed,
}

/// One **entry** of the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The object's number in the file.
    pub number: u64,
    /// Where it is: the offset for an object in the file, and the
    /// number of the stream it is in for a compressed one.
    pub offset: u64,
    /// Its generation.
    pub generation: u64,
    /// Which of the three it is.
    pub kind: Kind,
}

/// The **table** of a file, and its **trailer**.
#[derive(Debug, Clone, PartialEq)]
pub struct Xref {
    /// The entries, in the order the table has them.
    pub entries: Vec<Entry>,
    /// The trailer's dictionary: the pairs, in order.
    pub trailer: Vec<(String, Json)>,
}

impl Xref {
    /// The entry of an **object**, if the table has one.
    #[allow(dead_code)]
    pub fn entry(&self, number: u64) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.number == number)
    }
}

/// Read the **table** of a file.
pub fn read(bytes: &[u8]) -> Result<Xref, Error> {
    let start = startxref(bytes)?;
    let at = usize::try_from(start)
        .map_err(|_| Error::Malformed("the table's place in the file is not a place in it".to_string()))?;
    if at >= bytes.len() {
        return Err(Error::Malformed("the table is past the end of the file".to_string()));
    }
    let at = skip_space(bytes, at);
    if bytes.get(at..at + 4) == Some(b"xref") {
        return classic(bytes, at + 4);
    }
    stream(bytes, at)
}

/// The **place** of the table, from the end of the file.
fn startxref(bytes: &[u8]) -> Result<u64, Error> {
    // The word is near the end, and the number is after it.
    let window = bytes.len().saturating_sub(2048);
    let mut found = None;
    let mut at = window;
    while let Some(index) = find(bytes, b"startxref", at) {
        found = Some(index);
        at = index + 1;
    }
    let Some(index) = found else {
        return Err(Error::Malformed("the file does not say where its table is".to_string()));
    };
    let rest = &bytes[index + b"startxref".len()..];
    // The number is after the space that follows the word.
    let start = rest.iter().position(|byte| !byte.is_ascii_whitespace()).unwrap_or(rest.len());
    let text: String =
        rest[start..].iter().take_while(|byte| byte.is_ascii_digit()).map(|byte| *byte as char).collect();
    let number = text
        .parse::<u64>()
        .map_err(|_| Error::Malformed("the file does not say where its table is".to_string()))?;
    Ok(number)
}

/// The **classic table**: entries of twenty bytes under the word
/// `xref`.
fn classic(bytes: &[u8], mut at: usize) -> Result<Xref, Error> {
    let mut entries: Vec<Entry> = Vec::new();
    loop {
        at = skip_space(bytes, at);
        if bytes.get(at..at + 7) == Some(b"trailer") {
            at += 7;
            let (trailer, _) = objects::dictionary_of(bytes, at).map_err(from_objects)?;
            return Ok(Xref { entries, trailer });
        }
        // A **subsection**: the first object and how many there are.
        let (first, after) = number(bytes, at)?;
        let (count, after) = number(bytes, after)?;
        let first = u64::try_from(first)
            .map_err(|_| Error::Malformed("an object's number is not a whole number".to_string()))?;
        let count = u64::try_from(count)
            .map_err(|_| Error::Malformed("a count is not a whole number".to_string()))?;
        // The **entries** follow the header, one after another.
        at = skip_space(bytes, after);
        for index in 0..count {
            let entry = bytes
                .get(at..at + 20)
                .ok_or_else(|| Error::Malformed("the table ends in the middle of an entry".to_string()))?;
            let text = String::from_utf8_lossy(&entry[..18]);
            let mut parts = text.split_whitespace();
            let offset = parts.next().unwrap_or("0");
            let generation = parts.next().unwrap_or("0");
            let kind = match entry[17] {
                b'f' => Kind::Free,
                _ => Kind::InFile,
            };
            let offset = offset
                .parse::<u64>()
                .map_err(|_| Error::Malformed("an entry's offset is not a number".to_string()))?;
            let generation = generation
                .parse::<u64>()
                .map_err(|_| Error::Malformed("an entry's generation is not a number".to_string()))?;
            entries.push(Entry { number: first + index, offset, generation, kind });
            at += 20;
        }
    }
}

/// The **cross-reference stream**: the same table, in binary.
fn stream(bytes: &[u8], at: usize) -> Result<Xref, Error> {
    // The object header: the number and the generation, and the word
    // `obj`.
    let (_number, after) = number(bytes, at)?;
    let (_generation, after) = number(bytes, after)?;
    let after = skip_space(bytes, after);
    if bytes.get(after..after + 3) != Some(b"obj") {
        return Err(Error::Malformed("the table is not an object".to_string()));
    }
    let after = skip_space(bytes, after + 3);
    let (pairs, after) = objects::dictionary_of(bytes, after).map_err(from_objects)?;
    let value_of = |name: &str| pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value.clone());
    // The **data** of the stream: the word `stream`, then the bytes.
    let mut at = skip_space(bytes, after);
    if bytes.get(at..at + 6) != Some(b"stream") {
        return Err(Error::Malformed("the table has no stream in it".to_string()));
    }
    at += 6;
    if bytes.get(at) == Some(&b'\r') {
        at += 1;
    }
    if bytes.get(at) == Some(&b'\n') {
        at += 1;
    }
    let length = value_of("Length")
        .and_then(|length| length.as_f64())
        .map(|length| length as u64)
        .ok_or_else(|| Error::Unsupported("the table's length is not a number".to_string()))?;
    let length = usize::try_from(length)
        .map_err(|_| Error::Malformed("the table's length is not a length".to_string()))?;
    let end = at
        .checked_add(length)
        .ok_or_else(|| Error::Malformed("the table's length is past the end of the file".to_string()))?;
    let data = bytes
        .get(at..end)
        .ok_or_else(|| Error::Malformed("the table's data is past the end of the file".to_string()))?;
    // The **filter**, if the table is compressed.
    let filters: Vec<String> = match value_of("Filter") {
        Some(Json::String(name)) => vec![name],
        Some(Json::Array(names)) => {
            names.iter().filter_map(|name| name.as_str().map(|name| name.to_string())).collect()
        }
        _ => Vec::new(),
    };
    let limits = filters::Limits::default();
    let data = filters::decode_named(&filters, data, &limits)
        .map_err(|error| Error::Malformed(format!("the table cannot be decoded: {error:?}")))?;
    // The **widths**: how many bytes each field takes.
    let widths: Vec<usize> = value_of("W")
        .and_then(|widths| widths.as_array().cloned())
        .map(|widths| widths.iter().filter_map(|width| width.as_f64().map(|width| width as usize)).collect())
        .ok_or_else(|| Error::Unsupported("the table does not say how wide its fields are".to_string()))?;
    if widths.len() < 3 {
        return Err(Error::Unsupported("the table's fields are not three".to_string()));
    }
    let width: usize = widths.iter().sum();
    if width == 0 {
        return Err(Error::Unsupported("the table's fields are no width at all".to_string()));
    }
    // The **subsections**: which objects the table holds.
    let index: Vec<u64> = match value_of("Index") {
        Some(Json::Array(list)) => {
            list.iter().filter_map(|number| number.as_f64().map(|number| number as u64)).collect()
        }
        _ => {
            let size = value_of("Size")
                .and_then(|size| size.as_f64())
                .map(|size| size as u64)
                .ok_or_else(|| Error::Unsupported("the table does not say how big it is".to_string()))?;
            vec![0, size]
        }
    };
    let mut entries: Vec<Entry> = Vec::new();
    let mut at = 0_usize;
    let mut pair = 0_usize;
    while pair + 1 < index.len() {
        let first = index[pair];
        let count = index[pair + 1];
        for number in first..first.saturating_add(count) {
            let field = data
                .get(at..at + width)
                .ok_or_else(|| Error::Malformed("the table's data is short".to_string()))?;
            at += width;
            let mut value = 0_u64;
            for byte in field {
                value = (value << 8) | u64::from(*byte);
            }
            // The fields are the widest first: the kind, the second
            // field and the third.
            let mut fields: Vec<u64> = Vec::with_capacity(3);
            let mut rest = value;
            for width in widths.iter().rev() {
                if *width == 0 {
                    fields.push(0);
                    continue;
                }
                let mask = if *width >= 8 { u64::MAX } else { (1_u64 << (width * 8)) - 1 };
                fields.push(rest & mask);
                rest >>= width * 8;
            }
            fields.reverse();
            let kind = match fields[0] {
                0 => Kind::Free,
                1 => Kind::InFile,
                2 => Kind::Compressed,
                other => return Err(Error::Unsupported(format!("the kind {other} is not one of the three"))),
            };
            entries.push(Entry { number, offset: fields[1], generation: fields[2], kind });
        }
        pair += 2;
    }
    Ok(Xref { entries, trailer: pairs })
}

/// A **number** in the file, and where it ended.
fn number(bytes: &[u8], at: usize) -> Result<(i64, usize), Error> {
    let mut at = skip_space(bytes, at);
    let start = at;
    if matches!(bytes.get(at), Some(b'+') | Some(b'-')) {
        at += 1;
    }
    let mut seen = false;
    while let Some(byte) = bytes.get(at).copied() {
        if byte.is_ascii_digit() {
            seen = true;
            at += 1;
        } else {
            break;
        }
    }
    if !seen {
        return Err(Error::Malformed("a number has no digits".to_string()));
    }
    let text = String::from_utf8_lossy(&bytes[start..at]).to_string();
    let number =
        text.parse::<i64>().map_err(|_| Error::Malformed(format!("the number `{text}` is not a number")))?;
    Ok((number, at))
}

/// The **space** before a thing.
fn skip_space(bytes: &[u8], at: usize) -> usize {
    let mut at = at;
    while let Some(byte) = bytes.get(at).copied() {
        if byte.is_ascii_whitespace() {
            at += 1;
        } else {
            break;
        }
    }
    at
}

/// Where a **word** is, from a place onwards.
fn find(bytes: &[u8], word: &[u8], from: usize) -> Option<usize> {
    if from >= bytes.len() {
        return None;
    }
    bytes[from..].windows(word.len()).position(|window| window == word).map(|index| index + from)
}
