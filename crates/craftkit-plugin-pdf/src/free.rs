//! The **free numbers** of a file.
//!
//! An object number that is not in use is **free**. A file says which
//! numbers are free: the cross-reference table has a row for each of
//! them, and the row says the number is free and the **generation** it
//! gets if it is used again.
//!
//! The tree's rule, and it is the finding: a number that is taken away
//! is **not** given back. It is freed, the generation is **bumped**, and
//! the next number is past it — so a reference from an older revision
//! of the file, which says `5 0 R`, does not find a new object that
//! happens to have the number five. The tree's `next_num` only ever
//! grows, and a freed number is a number something had.
//!
//! The free numbers are a fact of the **file**, and the file is the
//! document: so they are in the document's own payload, under the
//! battery's name.
//!
//! # What it does not do
//!
//! - **The generation of a live object is not here.** Every object of
//!   this tree is written with the generation zero, so a number that is
//!   freed gets the generation one: the one after the generation it
//!   had.

use craftkit_sdk::Document;
use craftkit_sdk::serde_json;

/// The name the PDF battery keeps the file's own facts under.
pub(crate) const FILE_KEY: &str = "pdf";

/// One **free number**: a number that is not in use, and the
/// generation it gets if it is used again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Free {
    /// The number that is not in use.
    pub number: u64,
    /// The generation the number gets if it is used again.
    pub generation: u64,
}

/// The **free numbers** of the document.
///
/// The list is sorted and holds each number once: a number that is
/// free is free, and there is no order to it that means anything.
pub(crate) fn the_free(document: &Document) -> Vec<Free> {
    let Some(list) =
        document.data.get(FILE_KEY).and_then(|file| file.get("free")).and_then(|free| free.as_array())
    else {
        return Vec::new();
    };
    let mut free: Vec<Free> = Vec::with_capacity(list.len());
    for entry in list {
        let Some(number) = entry.get("object").and_then(|number| number.as_u64()) else { continue };
        let generation = entry.get("generation").and_then(|generation| generation.as_u64()).unwrap_or(1);
        free.push(Free { number, generation });
    }
    free.sort_by_key(|entry| entry.number);
    free.dedup_by_key(|entry| entry.number);
    free
}

/// **Put** the free numbers into the document.
///
/// The rest of the battery's own facts are kept: the payload is the
/// battery's, and taking the free numbers out of it would take away
/// something the caller did not ask about.
pub(crate) fn set_the_free(document: &mut Document, free: &[Free]) {
    let mut data = match document.data.as_object() {
        Some(object) => object.clone(),
        None => serde_json::Map::new(),
    };
    let list: Vec<serde_json::Value> = free
        .iter()
        .map(|entry| serde_json::json!({ "object": entry.number, "generation": entry.generation }))
        .collect();
    let mut file = match data.get(FILE_KEY).and_then(|file| file.as_object()) {
        Some(object) => object.clone(),
        None => serde_json::Map::new(),
    };
    file.insert("free".to_string(), serde_json::Value::Array(list));
    data.insert(FILE_KEY.to_string(), serde_json::Value::Object(file));
    document.data = serde_json::Value::Object(data);
}

/// **Free** a number: it is not in use any more, and it gets the
/// generation after the one it had.
///
/// The number is freed once: a number that is free already is freed
/// again with the same generation, because a number that is not in use
/// has no generation of its own to bump.
pub(crate) fn free_number(document: &mut Document, number: u64, generation: u64) {
    let mut free = the_free(document);
    free.retain(|entry| entry.number != number);
    free.push(Free { number, generation: generation.saturating_add(1) });
    free.sort_by_key(|entry| entry.number);
    set_the_free(document, &free);
}

/// The **next number**: a number nothing has.
///
/// A number that is free is still a number something had, so the next
/// number is past it — and the tree's rule is that the next number
/// only ever grows.
pub(crate) fn the_next_number(document: &Document, after: u64) -> u64 {
    let mut highest = after;
    for node in document.walk() {
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            continue;
        }
        if let Some(number) = node.data.get("object").and_then(|number| number.as_u64()) {
            highest = highest.max(number);
        }
    }
    for entry in the_free(document) {
        highest = highest.max(entry.number);
    }
    highest + 1
}
