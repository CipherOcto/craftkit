//! The **writer**: the document, written out as a PDF file.
//!
//! The reader reads a file into the document; the writer writes the
//! document out as a file. The two are the same questions asked in the
//! other order: where each object is, and what it says.
//!
//! # What is written
//!
//! - The **objects**, one after another, each with its number and the
//!   word `obj`;
//! - the **cross-reference table** that says where they are;
//! - the **trailer**, which says how many objects there are and which
//!   one is the root.
//!
//! # What it does not do
//!
//! - **There is no encryption and no object stream.** The file is a
//!   plain file: the classic table, the objects as they are.
//! - **A name and a string are told apart by the payload.** A name is
//!   a plain string and a **text** string is `{"string": …}`, so that
//!   the writer knows which is which.

use craftkit_sdk::serde_json::Value as Json;

/// Why a document could not be **written**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The document says something that is not in a file.
    Malformed(String),
}

/// One **object** of the document, as the writer writes it.
#[derive(Debug, Clone, PartialEq)]
pub struct Object {
    /// The object's number in the file.
    pub number: u64,
    /// Its dictionary: the pairs, in order.
    pub entries: Vec<(String, Json)>,
    /// The bytes of its stream, if it has one.
    pub stream: Option<Vec<u8>>,
}

/// Write the **objects** as a file.
///
/// The root is the object that is the file's **catalog** — the one the
/// trailer names — and the file is not a file without it.
pub fn write(objects: &[Object], root: u64) -> Result<Vec<u8>, Error> {
    if objects.iter().all(|object| object.number != root) {
        return Err(Error::Malformed(format!("the file has no object {root} to be its root")));
    }
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
    // The **objects**, and where each one is.
    let mut offsets: Vec<(u64, usize)> = Vec::with_capacity(objects.len());
    let mut ordered: Vec<&Object> = objects.iter().collect();
    ordered.sort_by_key(|object| object.number);
    for object in &ordered {
        offsets.push((object.number, file.len()));
        file.extend_from_slice(format!("{} 0 obj\n", object.number).as_bytes());
        let mut entries = object.entries.clone();
        if object.stream.is_some() {
            // The **length** of the stream is in its dictionary.
            let length = object.stream.as_ref().map(|stream| stream.len()).unwrap_or(0);
            entries.retain(|(key, _)| key != "Length");
            entries.push(("Length".to_string(), Json::Number((length as u64).into())));
        }
        file.extend_from_slice(dictionary(&entries).as_bytes());
        file.push(b'\n');
        if let Some(stream) = &object.stream {
            file.extend_from_slice(b"stream\n");
            file.extend_from_slice(stream);
            file.push(b'\n');
            file.extend_from_slice(b"endstream");
        }
        file.extend_from_slice(b"\nendobj\n");
    }
    // The **table**: where each object is.
    let table_at = file.len();
    file.extend_from_slice(b"xref\n");
    for (first, count) in runs(&offsets) {
        file.extend_from_slice(format!("{first} {count}\n").as_bytes());
        for (number, offset) in &offsets {
            if *number < first || *number >= first + count {
                continue;
            }
            file.extend_from_slice(format!("{offset:010} 00000 n\r\n").as_bytes());
        }
    }
    file.extend_from_slice(
        format!("trailer\n<< /Size {} /Root {root} 0 R >>\nstartxref\n", objects.len()).as_bytes(),
    );
    file.extend_from_slice(table_at.to_string().as_bytes());
    file.extend_from_slice(b"\n%%EOF\n");
    Ok(file)
}

/// The **runs** of consecutive object numbers in the table.
///
/// The table is a list of subsections, and a subsection is a first
/// object and a count, so the numbers that are not consecutive are in
/// subsections of their own.
fn runs(offsets: &[(u64, usize)]) -> Vec<(u64, u64)> {
    let mut runs: Vec<(u64, u64)> = Vec::new();
    for (number, _) in offsets {
        match runs.last_mut() {
            Some((first, count)) if *first + *count == *number => *count += 1,
            _ => runs.push((*number, 1)),
        }
    }
    runs
}

/// A **dictionary**, as the file writes it.
fn dictionary(entries: &[(String, Json)]) -> String {
    let mut out = String::from("<<");
    for (key, value) in entries {
        out.push_str(&format!(" /{} {}", escape_name(key), value_of(value)));
    }
    out.push_str(" >>");
    out
}

/// A **value**, as the file writes it.
fn value_of(value: &Json) -> String {
    match value {
        Json::Null => "null".to_string(),
        Json::Bool(value) => value.to_string(),
        Json::Number(number) => {
            if let Some(number) = number.as_u64() {
                number.to_string()
            } else {
                number.to_string()
            }
        }
        Json::String(text) => {
            // A **name**: the payload's spelling of one.
            format!("/{}", escape_name(text))
        }
        Json::Array(list) => {
            let parts: Vec<String> = list.iter().map(value_of).collect();
            format!("[{}]", parts.join(" "))
        }
        Json::Object(map) => {
            if let Some(Json::String(text)) = map.get("string") {
                // A **text** string: the payload says so.
                return format!("({})", escape_text(text));
            }
            if let Some(Json::Array(pair)) = map.get("ref") {
                let number = pair.first().and_then(|number| number.as_f64()).unwrap_or(0.0);
                let generation = pair.get(1).and_then(|number| number.as_f64()).unwrap_or(0.0);
                return format!("{} {} R", number as u64, generation as u64);
            }
            // A **dictionary**: the payload holds one as a list of pairs
            // under the name `entries`, and the file holds one between
            // two angle brackets.
            if crate::objects::is_dictionary(value) {
                return dictionary(&crate::objects::pairs_of(value));
            }
            let entries: Vec<(String, Json)> =
                map.iter().map(|(key, value)| (key.clone(), value.clone())).collect();
            dictionary(&entries)
        }
    }
}

/// A **name**, with the characters that may not be in one escaped.
fn escape_name(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b' '..=b'!' | b'#'..=b'[' | b']'..=b'~' => out.push(byte as char),
            other => out.push_str(&format!("#{other:02X}")),
        }
    }
    out
}

/// A **text**, with the characters that may not be in a string escaped.
fn escape_text(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'(' => out.push_str("\\("),
            b')' => out.push_str("\\)"),
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            0x20..=0x7e => out.push(byte as char),
            other => out.push_str(&format!("\\{other:03o}")),
        }
    }
    out
}
