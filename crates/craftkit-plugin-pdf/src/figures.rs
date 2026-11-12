//! The **figures** of a file: the pictures in it, and what is said of
//! them.
//!
//! A tagged file has a **structure tree**, and the elements of that
//! tree say what they are: a picture is a `Figure`. A screen reader
//! reads the tree, and the **alternative text** of a figure is what it
//! says when it reaches one.
//!
//! # The rules
//!
//! - A file with **no structure tree** is not tagged, and it has **no
//!   figures**: the answer says so.
//! - What an element **is** is its `/S`, and the **role map** of the
//!   tree says what a name means: an element whose `/S` is a name the
//!   role map maps to `Figure` is a figure, whatever its own name is.
//! - A figure is **named by its node**, and a node that is **not a
//!   figure** is refused: an alternate text is the text of a figure
//!   and not of anything else.
//! - An alternate text that is **empty** **takes the text away**: the
//!   tree's own rule.
//!
//! # What it does not settle
//!
//! - The tree's **decorative** figure — a figure that is turned into
//!   an artifact and leaves the tree — is not here: it rewrites the
//!   content of a page and the parent tree with it.
//! - The tree's **box** of a figure — where it is on the page — is not
//!   here either: it is read from the content of the page, and this
//!   command reads the tree.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session, Text};

/// The most **deep** the structure tree is walked.
const MAX_DEPTH: usize = 64;

/// The **type** of the elements that are figures.
const FIGURE: &str = "Figure";

/// A **figure** of a file: an element of the structure tree that is a
/// picture.
#[derive(Debug, Clone)]
pub struct Figure {
    /// The node of the element.
    pub id: u64,
    /// The page it is on, when the tree says.
    pub page: Option<u64>,
    /// Its **alternative text**, when it has one.
    pub alt: Option<String>,
    /// The name of its type, as the file says it.
    pub kind: String,
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

/// The **entries** of a dictionary, whether it is written where it is
/// used or named by a reference.
fn dict_of(document: &Document, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(node) = crate::preflight::node_of(document, value) {
        return Some(entries_of(node));
    }
    let list = value.get("entries")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|pair| {
                let pair = pair.as_array()?;
                let key = pair.first()?.as_str()?.to_string();
                Some((key, pair.get(1)?.clone()))
            })
            .collect(),
    )
}

/// The **role map** of a structure tree: what the names mean.
fn role_map(document: &Document, root: &Node) -> Vec<(String, String)> {
    let root_entries = entries_of(root);
    let Some(map) = entry(&root_entries, "RoleMap") else { return Vec::new() };
    let Some(entries) = dict_of(document, map) else { return Vec::new() };
    entries
        .into_iter()
        .filter_map(|(key, value)| value.as_str().map(|name| (key, name.to_string())))
        .collect()
}

/// The **standard type** of an element: its own name, or what the role
/// map says its name means.
fn standard_of(kind: &str, roles: &[(String, String)]) -> String {
    roles
        .iter()
        .find(|(name, _)| name == kind)
        .map(|(_, standard)| standard.clone())
        .unwrap_or_else(|| kind.to_string())
}

/// The **type** of an element: its `/S`, which may be a name or a
/// dictionary that holds one.
fn kind_of(entries: &[(String, Json)]) -> Option<String> {
    let value = entry(entries, "S")?;
    match value {
        Json::String(name) => Some(name.clone()),
        Json::Object(map) => map.get("S").and_then(|name| name.as_str()).map(|name| name.to_string()),
        _ => None,
    }
}

/// The **figures** of a document, in the order of the tree.
///
/// The tree's own rule: the figures are the elements whose type is
/// `Figure`, and the page of a figure is the page it says or the page
/// it is under.
pub fn figures_of(document: &Document) -> Vec<Figure> {
    let Some(catalog) = crate::preflight::the_catalog(document) else { return Vec::new() };
    let catalog_entries = entries_of(catalog);
    let Some(root) = entry(&catalog_entries, "StructTreeRoot") else { return Vec::new() };
    let Some(root) = crate::preflight::node_of(document, root) else { return Vec::new() };
    let roles = role_map(document, root);
    let mut out: Vec<Figure> = Vec::new();
    walk(document, root, &roles, None, 0, &mut out);
    out
}

/// The **elements** below a node, and the figures among them.
fn walk(
    document: &Document,
    node: &Node,
    roles: &[(String, String)],
    page: Option<u64>,
    depth: usize,
    out: &mut Vec<Figure>,
) {
    if depth >= MAX_DEPTH {
        return;
    }
    let entries = entries_of(node);
    let own_page = entry(&entries, "Pg")
        .and_then(|value| crate::preflight::node_of(document, value))
        .map(|node| node.id)
        .or(page);
    let kind = kind_of(&entries).unwrap_or_default();
    if standard_of(&kind, roles) == FIGURE {
        out.push(Figure { id: node.id, page: own_page, alt: entry(&entries, "Alt").and_then(text_of), kind });
    }
    let Some(children) = entry(&entries, "K") else { return };
    let listed: Vec<Json> = match children {
        Json::Array(list) => list.clone(),
        one => vec![one.clone()],
    };
    for child in listed {
        // A child that is **not a reference** is a marked content
        // number, and it is not a thing of the tree.
        let Some(child) = crate::preflight::node_of(document, &child) else { continue };
        walk(document, child, roles, own_page, depth + 1, out);
    }
}

/// The **figures** of a file.
#[derive(Command)]
#[command(id = "pdf.figure.read", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadFigures {}

impl ReadFigures {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let document = session.document();
        let figures = figures_of(document);
        let tagged = crate::preflight::the_catalog(document)
            .map(|catalog| {
                let catalog_entries = entries_of(catalog);
                entry(&catalog_entries, "StructTreeRoot").is_some()
            })
            .unwrap_or(false);
        let figures: Vec<Json> = figures
            .iter()
            .map(|figure| {
                json!({
                    "id": figure.id,
                    "page": match figure.page {
                        Some(page) => json!(page),
                        None => Json::Null,
                    },
                    "kind": figure.kind.clone(),
                    "alt": match &figure.alt {
                        Some(alt) => json!(alt),
                        None => Json::Null,
                    },
                })
            })
            .collect();
        Ok(Outcome::new(format!("the file has {} figure(s)", figures.len())).with_data(json!({
            "tagged": tagged,
            "figures": figures,
        })))
    }
}

/// **Set** the **alternative text** of a figure.
///
/// The text that is empty **takes the text away**: the tree's own rule.
#[derive(Command)]
#[command(id = "pdf.figure.alt.set", area = "pdf", layer = "pdf", icon = "pencil", mutates, undoable)]
pub struct SetFigureAlt {
    /// The figure: the node of its element in the structure tree.
    pub figure: NodeRef,
    /// The alternative text. An empty text takes the text away.
    pub alt: Text,
}

impl SetFigureAlt {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "pdf.figure.alt.set".to_string();
        let figure = self.figure.0;
        let Some(node) = session.document().node(figure) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{figure}`"),
            });
        };
        // The node must be a **figure**: an alternate text is the text
        // of a figure and not of anything else.
        let figures = figures_of(session.document());
        if !figures.iter().any(|held| held.id == figure) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{figure}` is not a figure of the file"),
            });
        }
        let alt = self.alt.0.trim();
        let mut entries = entries_of(node);
        let key = "Alt".to_string();
        match entries.iter_mut().find(|(name, _)| name == &key) {
            Some(entry) => {
                if alt.is_empty() {
                    entries.retain(|(name, _)| name != &key);
                } else {
                    entry.1 = json!({ "string": alt });
                }
            }
            None => {
                if !alt.is_empty() {
                    entries.push((key, json!({ "string": alt })));
                }
            }
        }
        let data = json!({
            "object": node.data.get("object").cloned().unwrap_or(Json::Null),
            "entries": entries.iter().map(|(key, value)| json!([key, value])).collect::<Vec<_>>(),
        });
        let Some(node) = session.document_mut()?.node_mut(figure) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{figure}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the figure `{figure}` has its alternative text")).with_data(json!({
            "figure": figure,
            "alt": if alt.is_empty() { Json::Null } else { json!(alt) },
        })))
    }
}
