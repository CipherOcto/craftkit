//! The **signatures** of a file.
//!
//! A file may be **signed**: a person signs it, and the file carries
//! the signature — the bytes of the signature, the range of the file
//! they cover, and who signed it and when.
//!
//! A signature is a **field** of the kind `Sig`, and its `/V` is the
//! **signature dictionary**: the `/ByteRange` it covers, the
//! `/Contents` that hold the signature itself, the `/Name` of the
//! signer, the `/M` of the time and the `/SubFilter` of the way it
//! was made.
//!
//! The rules:
//!
//! - a file with **no signature field** has no signatures, and the
//!   answer says so;
//! - a signature field with **no value** is not signed yet, and the
//!   answer says that;
//! - the bytes of a signature are **not checked**: there is no key
//!   here and no way to check them, and an answer that said a
//!   signature is good would be a guess.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, Outcome};

/// One **signature** of a file: what the file says about it.
#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    /// The name of the field that holds the signature.
    pub field: Option<String>,
    /// The **byte range** the signature covers: the offsets, in
    /// pairs.
    pub byte_range: Vec<i64>,
    /// The **name** of the signer.
    pub name: Option<String>,
    /// The **time** the file was signed at.
    pub time: Option<String>,
    /// The **sub-filter**: the way the signature was made.
    pub sub_filter: Option<String>,
    /// Whether the signature holds bytes.
    pub signed: bool,
}

/// The **entries** of a node's dictionary.
fn entries_of(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **text** a value holds: the file's own spelling of a text.
fn text_of(value: &Json) -> Option<String> {
    match value {
        Json::String(text) => Some(text.clone()),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str()).map(|text| text.to_string()),
        _ => None,
    }
}

/// The **numbers** a value holds.
fn numbers_of(value: &Json) -> Vec<i64> {
    value.as_array().map(|list| list.iter().filter_map(crate::whole_number_of).collect()).unwrap_or_default()
}

/// The **signature** a field holds, if it is signed.
fn signature_of(document: &Document, node: &Node) -> Signature {
    let entries = entries_of(node);
    let field = entry(&entries, "T").and_then(text_of);
    // The **value** of the field: the signature dictionary, which may
    // be named by a reference.
    let dictionary = match entry(&entries, "V") {
        None => Vec::new(),
        Some(value) => match crate::preflight::node_of(document, value) {
            Some(node) => entries_of(node),
            None => Vec::new(),
        },
    };
    let byte_range = entry(&dictionary, "ByteRange").map(numbers_of).unwrap_or_default();
    let name = entry(&dictionary, "Name").and_then(text_of);
    let time = entry(&dictionary, "M").and_then(text_of);
    let sub_filter = entry(&dictionary, "SubFilter").and_then(|value| value.as_str().map(str::to_string));
    let signed = entry(&dictionary, "Contents").is_some();
    Signature { field, byte_range, name, time, sub_filter, signed }
}

/// The **signatures** of a document: the fields of the kind `Sig`.
pub fn signatures_of(document: &Document) -> Vec<Signature> {
    let mut out: Vec<Signature> = Vec::new();
    for node in document.walk() {
        if node.kind != "pdf.object" && node.kind != "pdf.stream" {
            continue;
        }
        let entries = entries_of(node);
        let is_signature = entry(&entries, "FT").and_then(|kind| kind.as_str()) == Some("Sig");
        if !is_signature {
            continue;
        }
        out.push(signature_of(document, node));
    }
    out
}

commands! {
    /// **Read** the **signatures** of a PDF file.
    ///
    /// A query: it does not change the document. The bytes of a signature
    /// are **not checked**: there is no key here, and an answer that said
    /// a signature is good would be a guess.
    pdf.signatures.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The signature to read.
            ///
            /// Leave it out and every signature is read.
            field: Option<craftkit_sdk::Text>,
        },
        run: {
            let id = "pdf.signatures.read".to_string();
            let document = session.document();
            let all = signatures_of(document);
            let listed: Vec<Json> = match &self.field {
                None => all.iter().map(signature_json).collect(),
                Some(field) => {
                    let wanted = field.0.trim();
                    let one: Vec<Json> = all
                        .iter()
                        .filter(|signature| signature.field.as_deref() == Some(wanted))
                        .map(signature_json)
                        .collect();
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the file has no signature called `{wanted}`"),
                        });
                    }
                    one
                }
            };
            let count = listed.len();
            Ok(Outcome::new(format!("the file has {count} signature(s)")).with_data(json!({
                "signatures": listed,
                "count": count,
                "verified": false,
            })))

        }
    }
}

/// A **signature**, as the answer gives it.
fn signature_json(signature: &Signature) -> Json {
    json!({
        "field": signature.field.clone().map(|field| json!(field)).unwrap_or(Json::Null),
        "byte_range": signature.byte_range.clone(),
        "name": signature.name.clone().map(|name| json!(name)).unwrap_or(Json::Null),
        "time": signature.time.clone().map(|time| json!(time)).unwrap_or(Json::Null),
        "sub_filter": signature.sub_filter.clone().map(|filter| json!(filter)).unwrap_or(Json::Null),
        "signed": signature.signed,
    })
}
