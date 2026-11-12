//! The **attachments** of a file: the files a file carries.
//!
//! A file may carry files of its own — a note, a spreadsheet, another
//! PDF — and a viewer shows them in a list of attachments. They live
//! in the catalog's `/Names` dictionary, under `/EmbeddedFiles`, and
//! they are a **name tree**: a list of a name and the file that
//! carries it.
//!
//! The file that carries it is a **file specification**: it says what
//! the file is called (`/F`, and `/UF` for the name in Unicode), what
//! it is about (`/Desc`), and — under `/EF` — which object holds its
//! bytes.
//!
//! The rules:
//!
//! - a file that carries no file has **no attachments**, and the
//!   answer says so;
//! - the name is the **Unicode** one when the file gives one, and the
//!   plain one otherwise;
//! - the **size** is the one the file says — the `/Size` of the
//!   parameters — and the **bytes** are the length of the bytes
//!   themselves. They are two facts, and a file whose two do not
//!   agree is a file that says two things;
//! - the **kids** of a name tree are read, up to eight levels deep: a
//!   file with many attachments splits its name tree, and the
//!   attachments are in the kids.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Outcome};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.attachments.read";

/// The most **deep** a name tree is read.
const MAX_DEPTH: usize = 8;

/// One **attachment**: a file a file carries.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Attachment {
    /// The name of the file.
    pub name: String,
    /// What the file says the attachment is about.
    pub description: Option<String>,
    /// The kind of the file, as the file says it.
    pub mime: Option<String>,
    /// The size the file says the attachment is.
    pub size: Option<u64>,
    /// The length of the bytes of the attachment.
    pub bytes: Option<usize>,
    /// The number of the object that holds the bytes.
    pub object: Option<u64>,
    /// The node that holds the bytes.
    pub id: Option<u64>,
}

impl Attachment {
    /// The attachment, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "name": self.name.clone(),
            "description": self.description.clone().map(|text| json!(text)).unwrap_or(Json::Null),
            "mime": self.mime.clone().map(|mime| json!(mime)).unwrap_or(Json::Null),
            "size": self.size.map(|size| json!(size)).unwrap_or(Json::Null),
            "bytes": self.bytes.map(|bytes| json!(bytes)).unwrap_or(Json::Null),
            "object": self.object.map(|object| json!(object)).unwrap_or(Json::Null),
            "id": self.id.map(|id| json!(id)).unwrap_or(Json::Null),
        })
    }
}

/// The **entries** of a value: the pairs of a dictionary, whether it
/// is written where it is used or named by a reference.
fn entries_of_value(document: &Document, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(node) = crate::preflight::node_of(document, value) {
        return Some(crate::entries_of(&node.data));
    }
    let list = value.get("entries")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let pair = entry.as_array()?;
                let key = pair.first()?.as_str()?.to_string();
                Some((key, pair.get(1)?.clone()))
            })
            .collect(),
    )
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **text** a value holds: the file's own spelling of a text.
///
/// A text is bytes, and the bytes are read as they are — as UTF-16
/// when the string says so, and as Latin-1 otherwise.
fn text_of(value: &Json) -> Option<String> {
    let raw = match value {
        Json::String(text) => text.clone(),
        Json::Object(map) => map.get("string").and_then(|text| text.as_str())?.to_string(),
        _ => return None,
    };
    let bytes: Vec<u8> = raw.chars().map(|character| character as u32 as u8).collect();
    Some(crate::pagetext::decode_bytes(&bytes).0)
}

/// The **attachments** of a document.
pub fn attachments_of(document: &Document) -> Vec<Attachment> {
    let mut out: Vec<Attachment> = Vec::new();
    let Some(catalog) = crate::preflight::the_catalog(document) else { return out };
    let catalog_entries = crate::entries_of(&catalog.data);
    let Some(names) = entry(&catalog_entries, "Names") else { return out };
    let Some(names_entries) = entries_of_value(document, names) else { return out };
    let Some(embedded) = entry(&names_entries, "EmbeddedFiles") else { return out };
    let Some(tree) = entries_of_value(document, embedded) else { return out };
    walk(document, &tree, 0, &mut out);
    out
}

/// Walk a **name tree**, and read the attachments under it.
fn walk(document: &Document, tree: &[(String, Json)], depth: usize, out: &mut Vec<Attachment>) {
    if depth >= MAX_DEPTH {
        return;
    }
    // The **names** of this part of the tree: a name and the file that
    // carries it.
    if let Some(list) = entry(tree, "Names").and_then(|names| names.as_array()) {
        let mut at = 0usize;
        while at + 1 < list.len() {
            let name = text_of(&list[at]).unwrap_or_default();
            let value = &list[at + 1];
            if let Some(spec) = entries_of_value(document, value) {
                out.push(attachment_of(document, &spec, &name));
            }
            at += 2;
        }
    }
    // The **kids**: the tree is split, and the attachments are in the
    // parts.
    if let Some(kids) = entry(tree, "Kids").and_then(|kids| kids.as_array()) {
        for kid in kids {
            let Some(kid_entries) = entries_of_value(document, kid) else { continue };
            walk(document, &kid_entries, depth + 1, out);
        }
    }
}

/// One **attachment**: the file a file specification carries.
fn attachment_of(document: &Document, spec: &[(String, Json)], name: &str) -> Attachment {
    // The name: the **Unicode** one when the file gives one.
    let unicode = entry(spec, "UF").and_then(text_of);
    let plain = entry(spec, "F").and_then(text_of);
    let name = match (unicode, plain) {
        (Some(unicode), _) if !unicode.is_empty() => unicode,
        (_, Some(plain)) => plain,
        _ => name.to_string(),
    };
    let description = entry(spec, "Desc").and_then(text_of);
    // The **bytes**: the object the specification names under `/EF`.
    let mut attachment = Attachment { name, description, ..Attachment::default() };
    let Some(ef) = entry(spec, "EF").and_then(|ef| entries_of_value(document, ef)) else {
        return attachment;
    };
    let stream = entry(&ef, "UF").or_else(|| entry(&ef, "F"));
    let Some(stream) = stream else { return attachment };
    let Some(node) = crate::preflight::node_of(document, stream) else { return attachment };
    attachment.id = Some(node.id);
    attachment.object = node
        .data
        .get("object")
        .and_then(crate::whole_number_of)
        .and_then(|object| u64::try_from(object).ok());
    let entries = crate::entries_of(&node.data);
    attachment.mime = entry(&entries, "Subtype").and_then(|subtype| subtype.as_str()).map(String::from);
    // The **size** the file says, under the parameters of the file.
    if let Some(params) = entry(&entries, "Params").and_then(|params| entries_of_value(document, params)) {
        attachment.size =
            entry(&params, "Size").and_then(crate::whole_number_of).and_then(|size| u64::try_from(size).ok());
    }
    // The **bytes**: the length of what is there.
    if let Ok(decoded) = crate::decoded_bytes(node, None, None, false, COMMAND) {
        attachment.bytes = Some(decoded.bytes.len());
    }
    attachment
}

commands! {
    /// **Read** the **attachments** of a PDF file.
    ///
    /// A query: it does not change the document. A file that carries no
    /// file has none, and the answer says so.
    pdf.attachments.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The attachment to read.
            ///
            /// Leave it out and every attachment is read.

            name: Option<craftkit_sdk::Text>,
        },
        run: {

            let id = COMMAND.to_string();
            let all = attachments_of(session.document());
            let listed: Vec<Json> = match &self.name {
                None => all.iter().map(|attachment| attachment.to_json()).collect(),
                Some(name) => {
                    let wanted = name.0.trim();
                    let one: Vec<Json> = all
                        .iter()
                        .filter(|attachment| attachment.name == wanted)
                        .map(|attachment| attachment.to_json())
                        .collect();
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the file has no attachment called `{wanted}`"),
                        });
                    }
                    one
                }
            };
            let count = listed.len();
            Ok(Outcome::new(format!("the file carries {count} attachment(s)")).with_data(json!({
                "attachments": listed,
                "count": count,
            })))

        }
    }
}
