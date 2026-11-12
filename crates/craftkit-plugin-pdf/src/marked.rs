//! The **marked content** of a tagged file: the text under each
//! element of its structure tree.
//!
//! A tagged file says what its content **is** — a heading, a
//! paragraph, a figure — in its structure tree, and it says where each
//! of those is on the page in its **marked content**: the content
//! between a `BDC` and the `EMC` that closes it, with a number that
//! says which one it is (an `MCID`).
//!
//! So the text of a heading is found by looking for the marked content
//! that carries the heading's number and reading the text under it.
//!
//! The rules:
//!
//! - a file that is **not tagged** has no marked content, and the
//!   answer says so;
//! - an element that says **no page** has no text: the content cannot
//!   be looked for without knowing where to look;
//! - a page may say its marked content by a **number** of its own or
//!   by the **name** of a list of properties, and both are read;
//! - the walk is **bounded**: a structure tree deeper than the tree
//!   reads is not read further.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, Outcome};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.marked.read";

/// The most **deep** a structure tree is read.
const MAX_DEPTH: usize = 64;

/// One **marked** element: what it is, and the text under it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Marked {
    /// What the element **is**: its type, as the file says it.
    pub kind: String,
    /// The **title** of the element.
    pub title: Option<String>,
    /// The **alternative text** of the element.
    pub alt: Option<String>,
    /// The **language** of the element.
    pub language: Option<String>,
    /// The **page** the element is on.
    pub page: Option<u64>,
    /// The **text** under the element.
    pub text: String,
}

impl Marked {
    /// The element, as the answer gives it.
    fn to_json(&self) -> Json {
        json!({
            "kind": self.kind.clone(),
            "title": self.title.clone().map(|title| json!(title)).unwrap_or(Json::Null),
            "alt": self.alt.clone().map(|alt| json!(alt)).unwrap_or(Json::Null),
            "language": self.language.clone().map(|language| json!(language)).unwrap_or(Json::Null),
            "page": self.page.map(|page| json!(page)).unwrap_or(Json::Null),
            "text": self.text.clone(),
        })
    }
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

/// The **node** an object number names.
fn node_of_number(document: &Document, number: u64) -> Option<&Node> {
    document.walk().find(|node| node.data.get("object").and_then(|found| found.as_u64()) == Some(number))
}

/// The **marked content** of a tagged document: every element of the
/// structure tree, with the text under it.
pub fn marked_of(document: &Document) -> Vec<Marked> {
    let mut out: Vec<Marked> = Vec::new();
    let Some(catalog) = crate::preflight::the_catalog(document) else { return out };
    let catalog_entries = entries_of(catalog);
    let Some(root) =
        entry(&catalog_entries, "StructTreeRoot").and_then(|root| crate::preflight::node_of(document, root))
    else {
        return out;
    };
    let root_entries = entries_of(root);
    // The root of the tree may be an element itself, and the element
    // that is the document is below it.
    if entry(&root_entries, "S").is_some() {
        walk(document, root, None, 0, &mut out);
        return out;
    }
    let Some(children) = entry(&root_entries, "K") else { return out };
    let listed: Vec<Json> = match children {
        Json::Array(list) => list.clone(),
        one => vec![one.clone()],
    };
    for child in listed {
        let Some(node) = crate::preflight::node_of(document, &child) else { continue };
        walk(document, node, None, 0, &mut out);
    }
    out
}

/// Walk the **structure tree**, and read the text under each element.
fn walk(document: &Document, node: &Node, page: Option<u64>, depth: usize, out: &mut Vec<Marked>) {
    if depth >= MAX_DEPTH {
        return;
    }
    let entries = entries_of(node);
    let Some(kind) = entry(&entries, "S").and_then(|kind| kind.as_str()) else { return };
    // The **page** the element is on: the one it says, or the one it
    // inherits.
    let page = entry(&entries, "Pg")
        .and_then(crate::number_of)
        .and_then(|number| node_of_number(document, number))
        .map(|node| node.id)
        .or(page);
    let mut marked = Marked {
        kind: kind.to_string(),
        title: entry(&entries, "T").and_then(text_of),
        alt: entry(&entries, "Alt").and_then(text_of),
        language: entry(&entries, "Lang").and_then(text_of),
        page,
        text: String::new(),
    };
    // The **text** of the element: the marked content it points at.
    if let Some(content) = entry(&entries, "K") {
        marked.text = text_of_content(document, content, page);
    }
    // The element comes **before** the elements below it: a tree is
    // read from its root down.
    out.push(marked);
    // The **elements** below it.
    if let Some(content) = entry(&entries, "K") {
        children_of(document, content, page, depth, out);
    }
}

/// The **text** under the marked content an element points at.
fn text_of_content(document: &Document, value: &Json, page: Option<u64>) -> String {
    let mut text = String::new();
    if let Json::Array(list) = value {
        for item in list {
            text.push_str(&text_of_content(document, item, page));
        }
        return text;
    }
    // A **number**: the marked content it names, on the page.
    if let Some(number) = value.as_f64() {
        if let Some(page) = page
            && let Some(found) = text_under(document, page, number as i64)
        {
            text.push_str(&found);
        }
        return text;
    }
    // A **dictionary**: either the marked content of an element, or
    // the element's own content.
    if let Some(entries) = dictionary_of(value) {
        if let Some(id) = entry(&entries, "MCID").and_then(|id| id.as_f64()) {
            if let Some(page) = page
                && let Some(found) = text_under(document, page, id as i64)
            {
                text.push_str(&found);
            }
            return text;
        }
        if let Some(content) = entry(&entries, "K") {
            text.push_str(&text_of_content(document, content, page));
        }
    }
    text
}

/// The **elements** below an element: the tree goes on down.
fn children_of(document: &Document, value: &Json, page: Option<u64>, depth: usize, out: &mut Vec<Marked>) {
    if let Json::Array(list) = value {
        for item in list {
            children_of(document, item, page, depth, out);
        }
        return;
    }
    // A **reference**: the element it names.
    if let Some(node) = crate::preflight::node_of(document, value) {
        let entries = entries_of(node);
        if entry(&entries, "S").is_some() {
            walk(document, node, page, depth + 1, out);
        }
        return;
    }
    // A **dictionary**: the content of an element.
    if let Some(entries) = dictionary_of(value)
        && let Some(content) = entry(&entries, "K")
    {
        children_of(document, content, page, depth, out);
    }
}

/// The **pairs** of a dictionary that is a value.
///
/// A dictionary in the payload is a list of pairs under the name
/// `entries`, and a dictionary in a content stream is the list of
/// pairs itself. Both are read.
fn dictionary_of(value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(list) = value.get("entries").and_then(|entries| entries.as_array()) {
        return Some(
            list.iter()
                .filter_map(|entry| {
                    let pair = entry.as_array()?;
                    let key = pair.first()?.as_str()?.to_string();
                    Some((key, pair.get(1)?.clone()))
                })
                .collect(),
        );
    }
    let list = value.as_array()?;
    let mut entries: Vec<(String, Json)> = Vec::with_capacity(list.len());
    for entry in list {
        let pair = entry.as_array()?;
        if pair.len() != 2 {
            return None;
        }
        let key = pair.first()?.as_str()?.to_string();
        entries.push((key, pair.get(1)?.clone()));
    }
    Some(entries)
}

/// The **number** of a marked content: the `MCID` of its properties.
///
/// The properties are either written where they are used — a
/// dictionary with an `/MCID` in it — or they are the **name** of a
/// list of properties, and the list is in the page's resources.
fn mcid_of(operation: &crate::content::Operation, properties: Option<&[(String, Json)]>) -> Option<i64> {
    let value = operation.operands.last()?;
    // The properties written where they are used.
    if let Some(entries) = dictionary_of(value) {
        if let Some(id) = entry(&entries, "/MCID").and_then(|id| id.as_f64()) {
            return Some(id as i64);
        }
        if let Some(id) = entry(&entries, "MCID").and_then(|id| id.as_f64()) {
            return Some(id as i64);
        }
        return None;
    }
    // The properties are the **name** of a list.
    let Json::String(name) = value else { return None };
    let name = name.strip_prefix('/')?;
    let properties = properties?;
    let listed = entry(properties, name)?;
    let entries = dictionary_of(listed)?;
    entry(&entries, "/MCID")
        .or_else(|| entry(&entries, "MCID"))
        .and_then(|id| id.as_f64())
        .map(|id| id as i64)
}

/// The **lists of properties** of a page: the names the content
/// stream may use.
fn properties_of(document: &Document, page: u64) -> Option<Vec<(String, Json)>> {
    let node = document.node(page)?;
    let entries = entries_of(node);
    let resources = entry(&entries, "Resources")?;
    let resources = match crate::preflight::node_of(document, resources) {
        Some(node) => entries_of(node),
        None => dictionary_of(resources)?,
    };
    let properties = entry(&resources, "Properties")?;
    match crate::preflight::node_of(document, properties) {
        Some(node) => Some(entries_of(node)),
        None => dictionary_of(properties),
    }
}

/// The **text** under a marked content of a page.
fn text_under(document: &Document, page: u64, mcid: i64) -> Option<String> {
    let operations = crate::pagetext::operations_of_page(document, page).ok()?;
    let properties = properties_of(document, page);
    // The marked content is between a `BDC` and the `EMC` that closes
    // it, and the depth says which `EMC` is the one.
    let mut depth = 0usize;
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;
    for (at, operation) in operations.iter().enumerate() {
        match operation.operator.as_str() {
            "BDC" | "BMC" => {
                if depth == 0 && start.is_none() && mcid_of(operation, properties.as_deref()) == Some(mcid) {
                    start = Some(at);
                }
                depth += 1;
            }
            "EMC" => {
                depth = depth.saturating_sub(1);
                if depth == 0 && start.is_some() && end.is_none() {
                    end = Some(at);
                }
            }
            _ => {}
        }
    }
    let (start, end) = match (start, end) {
        (Some(start), Some(end)) => (start, end),
        _ => return None,
    };
    let maps = crate::pagefonts::maps_of(document, page);
    let pieces = crate::pagetext::pieces_of(&maps, &operations, start, end);
    let mut out = String::new();
    for piece in pieces {
        out.push_str(&piece.text);
        if piece.break_after {
            out.push('\n');
        }
    }
    Some(out)
}

commands! {
    /// **Read** the **marked content** of a PDF file: the text under every
    /// element of the structure tree.
    ///
    /// A query: it does not change the document. A file that is not tagged
    /// has no marked content, and the answer says so.
    pdf.marked.read {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The element to read.
            ///
            /// Leave it out and every element is read.

            kind: Option<craftkit_sdk::Text>,
        },
        run: {

            let id = COMMAND.to_string();
            let all = marked_of(session.document());
            let listed: Vec<Json> = match &self.kind {
                None => all.iter().map(|marked| marked.to_json()).collect(),
                Some(kind) => {
                    let wanted = kind.0.trim();
                    let one: Vec<Json> = all
                        .iter()
                        .filter(|marked| marked.kind == wanted)
                        .map(|marked| marked.to_json())
                        .collect();
                    if one.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the file has no element of the kind `{wanted}`"),
                        });
                    }
                    one
                }
            };
            let count = listed.len();
            Ok(Outcome::new(format!("the file has {count} marked element(s)")).with_data(json!({
                "marked": listed,
                "count": count,
            })))

        }
    }
}
