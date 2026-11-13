//! The **pages** of a spread.
//!
//! The pages of a spread stand in a row, and they may be **deleted**
//! and **moved**.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A document needs **at least one page**: the last page of a
//!   document may not be deleted.
//! - The **items** that stand beyond the page that is deleted move
//!   left by the width of that page, because the pages close up.
//! - A page is **moved** to another place among the pages of its
//!   spread, and a place that is not there is refused by name.
//!
//! # What it does not do
//!
//! - **The items of a page do not go with it.** The tree's items are
//!   on a page, and the tree's page takes them with it; here the
//!   items are in the spread, and they stay.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

/// The **pages** of a spread, in the order they stand in.
fn pages_of(node: &Node) -> Vec<u64> {
    node.children.iter().filter(|child| child.kind == "layout.page").map(|child| child.id).collect()
}

/// The **width** of a page, as the page says it.
fn width_of(node: &Node) -> f64 {
    node.data.get("width").and_then(|width| width.as_f64()).filter(|width| width.is_finite()).unwrap_or(0.0)
}

/// The **page** a command is about, and the spread it is in.
fn page_of(session: &Session, command: &str, page: u64) -> Result<u64, EngineError> {
    let Some(node) = session.document().node(page) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no page `{page}`"),
        });
    };
    if node.kind != "layout.page" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{page}` is a `{}` and not a page", node.kind),
        });
    }
    Ok(session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == page))
        .map(|node| node.id)
        .unwrap_or(page))
}

/// The **place** a page stands at in its spread: the width of the
/// pages before it.
fn place_of(spread: &Node, page: u64) -> f64 {
    let mut place = 0.0;
    for child in spread.children.iter() {
        if child.id == page {
            break;
        }
        if child.kind == "layout.page" {
            place += width_of(child);
        }
    }
    place
}

commands! {
    /// **Delete** a page.
    ///
    /// The pages close up: the items that stand beyond the page that is
    /// deleted move left by its width. And a document needs at least one
    /// page.
    layout.page.delete {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The page.
            page: NodeRef,
        },
        run: {
            let command = "layout.page.delete";
            let page = self.page.0;
            let spread = page_of(session, command, page)?;
            // The **pages** of the document: a document needs at least one.
            let pages: usize = session.document().walk().filter(|node| node.kind == "layout.page").count();
            if pages < 2 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a document needs at least one page".to_string(),
                });
            }
            let Some(spread_node) = session.document().node(spread).cloned() else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no spread `{spread}`"),
                });
            };
            // The **place** of the page and its **width**: the items that
            // stand beyond it move left by that width.
            let place = place_of(&spread_node, page);
            let width = spread_node
                .children
                .iter()
                .find(|child| child.id == page)
                .map(|child| width_of(child))
                .unwrap_or(0.0);
            let beyond = place + width;
            let mut moved = Vec::new();
            if let Some(node) = session.document_mut()?.node_mut(spread) {
                for child in node.children.iter_mut() {
                    if !child.kind.starts_with("layout.item.") {
                        continue;
                    }
                    let Some(rect) = child.data.get("rect").and_then(|rect| rect.as_object()).cloned() else {
                        continue;
                    };
                    let Some(x) = rect.get("x").and_then(|x| x.as_f64()) else { continue };
                    if x < beyond {
                        continue;
                    }
                    let mut rect = rect;
                    rect.insert("x".to_string(), Json::from(x - width));
                    let child = std::sync::Arc::make_mut(child);
                    child.data["rect"] = Json::Object(rect);
                    moved.push(child.id);
                }
            }
            // The **page** is gone.
            session.document_mut()?.remove_node(page);
            Ok(Outcome::changed(format!("the page `{page}` is gone")).with_data(json!({
                "page": page,
                "spread": spread,
                "items": moved,
            })))

        }
    }

    /// **Move** a page to another place among the pages of its spread.
    ///
    /// The place is counted in the order of the pages, and a place that is
    /// not there is refused by name.
    layout.page.move {
        area: "layout",
        layer: "layout",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The page.
            page: NodeRef,
            /// The place it goes: the first of the pages is 0.
            to: Number,
        },
        run: {
            let command = "layout.page.move";
            let page = self.page.0;
            let spread = page_of(session, command, page)?;
            let Some(spread_node) = session.document().node(spread).cloned() else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no spread `{spread}`"),
                });
            };
            let pages = pages_of(&spread_node);
            let to = self.to.0;
            if !to.is_finite() || to.fract() != 0.0 || to < 0.0 || to >= pages.len() as f64 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the place is a number from 0 to {}, and {to} is not one", pages.len()),
                });
            }
            let to = to as usize;
            let Some(index) = pages.iter().position(|held| *held == page) else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the page `{page}` is not in the spread `{spread}`"),
                });
            };
            // The **pages** in their new order, and the children of the
            // spread with them.
            let mut order = pages.clone();
            let moved = order.remove(index);
            order.insert(to.min(order.len()), moved);
            let mut children = spread_node.children.clone();
            let slots: Vec<usize> = children
                .iter()
                .enumerate()
                .filter(|(_, child)| child.kind == "layout.page")
                .map(|(index, _)| index)
                .collect();
            for (at, slot) in slots.iter().enumerate() {
                let Some(next) = order.get(at) else { continue };
                let Some(node) = session.document().node(*next).cloned() else { continue };
                children[*slot] = std::sync::Arc::new(node);
            }
            if let Some(node) = session.document_mut()?.node_mut(spread) {
                node.children = children;
            }
            Ok(Outcome::changed(format!("the page `{page}` is at {to}")).with_data(json!({
                "page": page,
                "spread": spread,
                "to": to,
                "pages": order,
            })))

        }
    }
}
