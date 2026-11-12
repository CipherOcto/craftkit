//! The **parent** of a page.
//!
//! A page may have a **parent**: the page it takes what it has not got
//! of its own from. That is the tree's **master page** — a page that
//! carries the margins and the columns, and that the pages of a
//! document follow.
//!
//! So the margins of a page are **derived**:
//!
//! - a page that has margins of its own keeps them;
//! - a page that has none takes the margins of the nearest parent
//!   that has some;
//! - and the same is true of the **columns**.
//!
//! And the rules:
//!
//! - the parent must be a **page**, and the page may not be its own
//!   parent;
//! - the parent may not be a **descendant** of the page: a page that
//!   is its own ancestor is a loop, and a loop has no end;
//! - the chain of parents is walked until it ends, and a chain that
//!   loops is stopped — a document that says something impossible
//!   must not hang the engine.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Session};

use crate::columns::Columns;
use crate::margins::Margin;

/// The **parent** of a page, if it has one.
pub fn parent_of(page: &Node) -> Option<u64> {
    page.data.get("parent").and_then(|parent| parent.as_u64())
}

/// The **chain** of parents of a page: its parent, its parent's
/// parent, and so on.
///
/// The chain stops at a page that has no parent, and at a page that
/// has been in the chain already — a loop has no end, and a walk that
/// never ends is a walk that never finishes.
pub fn parents_of(document: &Document, page: u64) -> Vec<u64> {
    let mut chain: Vec<u64> = Vec::new();
    let mut at = page;
    while let Some(node) = document.node(at) {
        let Some(parent) = parent_of(node) else { break };
        if chain.contains(&parent) {
            break;
        }
        chain.push(parent);
        at = parent;
    }
    chain
}

/// The **margins** of a page: its own if it has them, and the margins
/// of the nearest parent that has some otherwise.
pub fn margins_of(document: &Document, page: u64) -> Margin {
    let Some(node) = document.node(page) else { return Margin::none() };
    if node.data.get("margin").is_some() {
        return crate::margins::margin_of(node);
    }
    for parent in parents_of(document, page) {
        let Some(node) = document.node(parent) else { continue };
        if node.data.get("margin").is_some() {
            return crate::margins::margin_of(node);
        }
    }
    Margin::none()
}

/// The **columns** of a page: its own if it has them, and the columns
/// of the nearest parent that has some otherwise.
pub fn columns_of(document: &Document, page: u64) -> Columns {
    let Some(node) = document.node(page) else { return Columns::one() };
    if node.data.get("columns").is_some() {
        return crate::columns::columns_of(node);
    }
    for parent in parents_of(document, page) {
        let Some(node) = document.node(parent) else { continue };
        if node.data.get("columns").is_some() {
            return crate::columns::columns_of(node);
        }
    }
    Columns::one()
}

/// The **margin box** of a page: the box inside the margins it has,
/// whether they are its own or its parent's.
pub fn margin_box_of(document: &Document, page: u64) -> Option<(f64, f64, f64, f64)> {
    let node = document.node(page)?;
    let width = node.data.get("width")?.as_f64()?;
    let height = node.data.get("height")?.as_f64()?;
    margins_of(document, page).box_of(width, height)
}

/// The **column boxes** of a page: the boxes of the margin box,
/// divided into columns.
pub fn column_boxes_of(document: &Document, page: u64) -> Option<Vec<(f64, f64, f64, f64)>> {
    let margin_box = margin_box_of(document, page)?;
    columns_of(document, page).boxes_of(margin_box)
}

/// The **page** a node is, if it is one.
fn page_of(session: &Session, id: u64, command: &str) -> Result<Node, EngineError> {
    let Some(node) = session.document().node(id).cloned() else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no node `{id}`"),
        });
    };
    if node.kind != "layout.page" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{id}` is a `{}` and not a page", node.kind),
        });
    }
    Ok(node)
}

/// **Give** a page a parent: the page it takes what it has not got of
/// its own from.
#[derive(Command)]
#[command(id = "layout.page.parent", area = "layout", layer = "layout", icon = "page", mutates, undoable)]
pub struct SetParent {
    /// The page.
    pub page: NodeRef,
    /// The page it follows.
    pub parent: NodeRef,
}

impl SetParent {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.page.parent".to_string();
        let page = self.page.0;
        let parent = self.parent.0;
        page_of(session, page, &id)?;
        let parent_node = page_of(session, parent, &id)?;
        if page == parent {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the page `{page}` may not be its own parent"),
            });
        }
        // The parent may not be a **descendant** of the page: a page
        // that is its own ancestor is a loop.
        if crate::parents::parents_of(session.document(), parent).contains(&page) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the page `{parent}` follows the page `{page}`, and that is a loop"),
            });
        }
        let name = parent_node.name.clone();
        if let Some(node) = session.document_mut()?.node_mut(page) {
            node.data["parent"] = json!(parent);
        }
        Ok(Outcome::changed(format!("the page `{page}` follows the page `{name}`"))
            .with_data(json!({ "page": page, "parent": parent })))
    }
}

/// Take the **parent** of a page away.
#[derive(Command)]
#[command(
    id = "layout.page.parent.clear",
    area = "layout",
    layer = "layout",
    icon = "page",
    mutates,
    undoable
)]
pub struct ClearParent {
    /// The page.
    pub page: NodeRef,
}

impl ClearParent {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.page.parent.clear".to_string();
        let page = self.page.0;
        let node = page_of(session, page, &id)?;
        if crate::parents::parent_of(&node).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the page `{page}` follows no page"),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(page) {
            node.data["parent"] = json!(null);
        }
        Ok(Outcome::changed(format!("the page `{page}` follows no page now"))
            .with_data(json!({ "page": page })))
    }
}
