//! The **trailer** of a PDF file.
//!
//! A file ends with a **trailer**: a dictionary that says where the
//! file's index is, which object is the catalog, and — when the file
//! is **encrypted** — which object holds the encryption.
//!
//! The trailer is the file's own last word, and it is not an object
//! of the file: it has no number and it is not in the index. It is
//! kept in the **document's own payload**, under the key the pdf
//! battery owns, so that a document that was loaded from a file still
//! says where its trailer came from (§513).

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome, Text};

/// The key the pdf battery keeps the **trailer** under.
const TRAILER_KEY: &str = "pdf.trailer";

/// The **trailer** of a document: the pairs it holds, in the order
/// they were written.
///
/// A document that was not loaded from a file has no trailer, and the
/// answer is an empty list: there is nothing to say.
pub fn trailer_of(document: &Document) -> Vec<(String, Json)> {
    let Some(trailer) = document.data.get(TRAILER_KEY) else { return Vec::new() };
    let Some(entries) = trailer.get("entries").and_then(|entries| entries.as_array()) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let pair = entry.as_array()?;
            let key = pair.first()?.as_str()?.to_string();
            let value = pair.get(1)?.clone();
            Some((key, value))
        })
        .collect()
}

/// The **entry** of the trailer, by its key.
pub fn entry_of(document: &Document, key: &str) -> Option<Json> {
    trailer_of(document).into_iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// Whether the document is **encrypted**: its trailer names an
/// encryption.
pub fn is_encrypted(document: &Document) -> bool {
    entry_of(document, "Encrypt").is_some()
}

commands! {
    /// **Read** the trailer of a PDF file.
    ///
    /// A query: it does not change the document.
    pdf.trailer.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The entry to read.
            ///
            /// Leave it out and every entry of the trailer is read.
            key: Option<Text>,
        },
        run: {
            let document = session.document();
            let entries = trailer_of(document);
            let (entries, count) = match &self.key {
                None => (entries.clone(), entries.len()),
                Some(key) => {
                    let one: Vec<_> = entries.iter().filter(|(name, _)| name == key.0.trim()).cloned().collect();
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: "pdf.trailer.read".to_string(),
                            message: format!("the trailer has no entry `{}`", key.0.trim()),
                        });
                    }
                    let count = one.len();
                    (one, count)
                }
            };
            let listed: Vec<Json> =
                entries.iter().map(|(key, value)| json!([key.clone(), value.clone()])).collect();
            Ok(Outcome::new(format!("the trailer holds {count} entry(ies)")).with_data(json!({
                "entries": listed,
                "encrypted": is_encrypted(document),
            })))

        }
    }
}
