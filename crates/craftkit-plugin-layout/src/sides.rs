//! The **side** of a page.
//!
//! A page may be a **left-hand** page or a **right-hand** page: the
//! two faces of a spread, and the left of them is the one whose spine
//! is on its right.
//!
//! The side is what makes the tree's **inside** and **outside**
//! margins mean something: the inside margin is the room at the
//! **spine**, and the outside margin is the room at the other edge —
//! and on a left-hand page the two swap sides.
//!
//! So the inside and the outside margins of a page are **derived**:
//!
//! - a page that is **single**, or that is a **right-hand** page, has
//!   its inside margin at its left and its outside margin at its
//!   right;
//! - a **left-hand** page has its inside margin at its right, because
//!   the spine is on that side.
//!
//! And the rules:
//!
//! - a side that is **not one** of the three is refused: a page that
//!   is neither single nor left nor right is a page of no side;
//! - a node that is not a page may not have a side.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Text};

/// The **side** of a page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The page is on its own: it has no facing page.
    Single,
    /// The page is the **left** face of a spread.
    Left,
    /// The page is the **right** face of a spread.
    Right,
}

impl Side {
    /// The side a name says, if it says one.
    pub fn parse(name: &str) -> Option<Side> {
        match name.trim().to_ascii_lowercase().as_str() {
            "single" => Some(Side::Single),
            "left" => Some(Side::Left),
            "right" => Some(Side::Right),
            _ => None,
        }
    }

    /// The name of the side.
    pub fn name(self) -> &'static str {
        match self {
            Side::Single => "single",
            Side::Left => "left",
            Side::Right => "right",
        }
    }
}

/// The **side** of a page: a page that says nothing is **single**.
pub fn side_of(page: &Node) -> Side {
    page.data.get("side").and_then(|side| side.as_str()).and_then(Side::parse).unwrap_or(Side::Single)
}

/// The **inside** and the **outside** margins of a page: the room at
/// the spine and the room at the other edge.
pub fn inside_outside(page: &Node) -> (f64, f64) {
    let margin = crate::margins::margin_of(page);
    match side_of(page) {
        Side::Left => (margin.right, margin.left),
        Side::Single | Side::Right => (margin.left, margin.right),
    }
}

commands! {
    /// **Say** which side of its spread a page is on.
    layout.page.side {
        area: "layout",
        layer: "layout",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The page.
            page: NodeRef,
            /// The side it is on: `single`, `left` or `right`.
            side: Text,
        },
        run: {
            let id = "layout.page.side".to_string();
            let page = self.page.0;
            let Some(node) = session.document().node(page).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{page}`"),
                });
            };
            if node.kind != "layout.page" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{page}` is a `{}` and not a page", node.kind),
                });
            }
            let name = self.side.0.trim();
            let Some(side) = Side::parse(name) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{name}` is not a side: a side is `single`, `left` or `right`"),
                });
            };
            if let Some(node) = session.document_mut()?.node_mut(page) {
                node.data["side"] = json!(side.name());
            }
            Ok(Outcome::changed(format!("the page `{page}` is a {} page", side.name()))
                .with_data(json!({ "page": page, "side": side.name() })))

        }
    }
}
