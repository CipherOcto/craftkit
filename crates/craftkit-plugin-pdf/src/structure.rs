//! The **structure** of a file.
//!
//! A file may be **tagged**: its content carries a structure tree —
//! the elements the content is made of, and what each one is. A
//! heading is an `H1`, a paragraph is a `P`, a picture is a `Figure`,
//! and a screen reader walks the tree and says what the file is.
//!
//! The tree hangs from the catalog's `/StructTreeRoot`, and every
//! element says:
//!
//! - what it **is** — its `/S`, the type of the element;
//! - its **title** (`/T`) and its **alternative text** (`/Alt`);
//! - its **language** (`/Lang`);
//! - its **children** (`/K`) — the elements below it.
//!
//! The rules:
//!
//! - a file with **no structure tree** is not tagged, and the answer
//!   says so;
//! - a child that is **not an element** — a marked content number, say
//!   — is skipped: it is not a thing of the tree;
//! - the walk is **bounded**: a tree deeper than the tree reads is
//!   not read further, and the answer says how deep it went — a
//!   structure tree is written by a program and a program can make a
//!   deep one, and this tree reads a document and not a machine.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, Outcome};

/// The most **deep** a structure tree is read.
const MAX_DEPTH: usize = 64;

/// One **element** of a structure tree.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    /// What the element **is**: its type, as the file says it.
    pub kind: String,
    /// The **title** of the element.
    pub title: Option<String>,
    /// The **alternative text** of the element.
    pub alt: Option<String>,
    /// The **language** of the element.
    pub language: Option<String>,
    /// The **children** of the element.
    pub children: Vec<Element>,
}

impl Element {
    /// The element, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "kind": self.kind.clone(),
            "title": self.title.clone().map(|title| json!(title)).unwrap_or(Json::Null),
            "alt": self.alt.clone().map(|alt| json!(alt)).unwrap_or(Json::Null),
            "language": self.language.clone().map(|language| json!(language)).unwrap_or(Json::Null),
            "children": self.children.iter().map(|child| child.to_json()).collect::<Vec<Json>>(),
        })
    }
}

/// The **entries** of a node's dictionary.
fn entries_of(node: &craftkit_sdk::Node) -> Vec<(String, Json)> {
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

/// The **element** a node holds.
fn element_of(document: &Document, node: &craftkit_sdk::Node, depth: usize) -> Option<Element> {
    let entries = entries_of(node);
    // An element says what it **is**.
    let kind = entry(&entries, "S").and_then(|kind| kind.as_str())?;
    let children = match entry(&entries, "K") {
        Some(children) => children_of(document, children, depth),
        None => Vec::new(),
    };
    Some(Element {
        kind: kind.to_string(),
        title: entry(&entries, "T").and_then(text_of),
        alt: entry(&entries, "Alt").and_then(text_of),
        language: entry(&entries, "Lang").and_then(text_of),
        children,
    })
}

/// The **children** of an element: the elements below it.
fn children_of(document: &Document, value: &Json, depth: usize) -> Vec<Element> {
    if depth >= MAX_DEPTH {
        return Vec::new();
    }
    let listed: Vec<Json> = match value {
        Json::Array(list) => list.clone(),
        one => vec![one.clone()],
    };
    let mut out: Vec<Element> = Vec::new();
    for child in listed {
        // A child that is **not a reference** is a marked content
        // number, and it is not a thing of the tree.
        let Some(node) = crate::preflight::node_of(document, &child) else { continue };
        let Some(element) = element_of(document, node, depth + 1) else { continue };
        out.push(element);
    }
    out
}

/// The **structure tree** of a document: the root and what is below
/// it.
///
/// The tree hangs from the catalog's `/StructTreeRoot`, and that node
/// is the **root of the tree** and not an element of it: the element
/// that is the document is below it, and it is that element this
/// reading gives.
///
/// A document that is not tagged has no structure tree, and the answer
/// is nothing.
pub fn structure_of(document: &Document) -> Option<Element> {
    let catalog = crate::preflight::the_catalog(document)?;
    let catalog_entries = entries_of(catalog);
    let root = entry(&catalog_entries, "StructTreeRoot")?;
    let root = crate::preflight::node_of(document, root)?;
    let entries = entries_of(root);
    // The root of the tree may be an element itself.
    if entry(&entries, "S").is_some() {
        return element_of(document, root, 1);
    }
    let children = entry(&entries, "K")?;
    let listed: Vec<Json> = match children {
        Json::Array(list) => list.clone(),
        one => vec![one.clone()],
    };
    for child in listed {
        let Some(node) = crate::preflight::node_of(document, &child) else { continue };
        if let Some(element) = element_of(document, node, 1) {
            return Some(element);
        }
    }
    None
}

commands! {
    /// **Read** the **structure** of a PDF file.
    ///
    /// A query: it does not change the document. A file that is not
    /// tagged has no structure, and the answer says so.
    pdf.structure.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
        },
        run: {

            let document = session.document();
            let Some(root) = structure_of(document) else {
                return Ok(Outcome::new("the file is not tagged").with_data(json!({
                    "tagged": false,
                    "structure": Json::Null,
                })));
            };
            let tree = root.to_json();
            Ok(Outcome::new("the file is tagged").with_data(json!({
                "tagged": true,
                "structure": tree,
            })))

        }
    }
}
