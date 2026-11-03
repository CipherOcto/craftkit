//! The **loader**: a PDF file, read into the document.
//!
//! A file is an **index** and the objects it points at. The loader
//! reads the index, reads each object, and puts the objects into the
//! document: an object with no stream is a **node** of the kind
//! `pdf.object`, and one with a stream is a node of the kind
//! `pdf.stream` whose bytes are in a **file of their own**, beside the
//! document.
//!
//! # The two passes
//!
//! A reference may point **forward**: an object may refer to one that
//! is written later in the file. So the graph is built in two passes —
//! the objects first, and the references after — and that is a
//! plugin's business, and the price of the core's rule.
//!
//! # The free numbers
//!
//! The numbers the file says are **free** are read as well: they are
//! numbers something had, and the file says which they are and the
//! generation each of them gets if it is used again.

use craftkit_sdk::serde_json::Value as Json;

use crate::free::Free;
use crate::objects::{self, Error as ObjectsError};
use crate::xref::{self, Error as XrefError};

/// Why a file could not be **loaded**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The bytes are not what a file is.
    Malformed(String),
    /// The kit does not know how to read this.
    Unsupported(String),
}

/// One **object** of a file, as the loader read it.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    /// The object's number in the file.
    pub number: u64,
    /// Its generation.
    pub generation: u64,
    /// Its dictionary: the pairs, in order.
    pub entries: Vec<(String, Json)>,
    /// The bytes of its stream, if it has one.
    pub stream: Option<Vec<u8>>,
    /// The **references** it holds: the key of the dictionary they are
    /// under, and the object each one names.
    pub references: Vec<Reference>,
}

/// A **reference** an object holds.
#[derive(Debug, Clone, PartialEq)]
pub struct Reference {
    /// The key of the dictionary the reference is under.
    pub key: String,
    /// The object it names.
    pub number: u64,
    /// Its generation.
    pub generation: u64,
}

/// A **file** that has been read: its objects, and the numbers that
/// are **free**.
#[derive(Debug, Clone, PartialEq)]
pub struct File {
    /// The objects of the file.
    pub objects: Vec<Loaded>,
    /// The numbers that are not in use, and the generation each of
    /// them gets if it is used again.
    pub free: Vec<Free>,
}

/// Read the **objects** of a file.
///
/// The bytes are the file's own, and the objects are read in the order
/// the file's index has them. The numbers the file says are **free**
/// are read too: they are numbers something had, and the file says so.
pub fn load(bytes: &[u8]) -> Result<File, Error> {
    let table = xref::read(bytes).map_err(from_xref)?;
    let mut loaded: Vec<Loaded> = Vec::with_capacity(table.entries.len());
    let mut free: Vec<Free> = Vec::new();
    for entry in &table.entries {
        if entry.kind == xref::Kind::Free {
            // A free row of the table says the generation the number
            // gets if it is used again.
            if entry.number > 0 {
                free.push(Free { number: entry.number, generation: entry.generation });
            }
            continue;
        }
        if entry.kind != xref::Kind::InFile {
            continue;
        }
        let at = usize::try_from(entry.offset)
            .map_err(|_| Error::Malformed("an object's place is not a place in the file".to_string()))?;
        if at >= bytes.len() {
            return Err(Error::Malformed("an object is past the end of the file".to_string()));
        }
        let (object, _) = objects::indirect(bytes, at).map_err(from_objects)?;
        let mut entries: Vec<(String, Json)> = Vec::new();
        for (key, value) in objects::pairs_of(&object.value) {
            // The **kit's** spelling of a key: no slash in front.
            let key = key.strip_prefix('/').unwrap_or(key.as_str()).to_string();
            entries.push((key, value));
        }
        let mut references: Vec<Reference> = Vec::new();
        for (key, value) in &entries {
            references_of(&key.clone(), value, &mut references);
        }
        let stream = match object.stream {
            Some((start, length)) => {
                let data = bytes
                    .get(start..start + length)
                    .ok_or_else(|| Error::Malformed("a stream is past the end of the file".to_string()))?;
                Some(data.to_vec())
            }
            None => None,
        };
        loaded.push(Loaded {
            number: object.number,
            generation: object.generation,
            entries,
            stream,
            references,
        });
    }
    Ok(File { objects: loaded, free })
}

/// The **references** a value holds.
fn references_of(key: &str, value: &Json, references: &mut Vec<Reference>) {
    match value {
        Json::Object(map) => {
            if let Some(Json::Array(pair)) = map.get("ref") {
                let number = pair.first().and_then(|number| number.as_f64());
                let generation = pair.get(1).and_then(|number| number.as_f64());
                if let (Some(number), Some(generation)) = (number, generation) {
                    references.push(Reference {
                        key: key.to_string(),
                        number: number as u64,
                        generation: generation as u64,
                    });
                }
                return;
            }
            for (_name, value) in map {
                references_of(key, value, references);
            }
        }
        Json::Array(list) => {
            for value in list {
                references_of(key, value, references);
            }
        }
        _ => {}
    }
}

/// The error of the **objects** module, as the loader's own.
fn from_objects(error: ObjectsError) -> Error {
    match error {
        ObjectsError::Malformed(message) => Error::Malformed(message),
    }
}

/// The error of the **table** module, as the loader's own.
fn from_xref(error: XrefError) -> Error {
    match error {
        XrefError::Malformed(message) => Error::Malformed(message),
        XrefError::Unsupported(message) => Error::Unsupported(message),
    }
}
