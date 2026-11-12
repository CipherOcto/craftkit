//! The **facing page** of a page.
//!
//! A page that faces another page has the two of them side by side in
//! one spread: the **left** face and the **right** face. The page
//! across from a page is its **facing page**.
//!
//! The facing page is **derived**: it is worked out from the spread
//! and the side every time it is asked for, so it cannot be out of
//! date with the document.
//!
//! The rules:
//!
//! - a page that says it is **single** has no facing page: it does
//!   not face another;
//! - a page whose spread holds **two** pages has the other one as its
//!   facing page;
//! - a spread that holds **one** page, or more than two, has no
//!   facing page in it: there is nothing to face.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome};

use crate::sides::Side;

/// The **spread** a page is in.
pub fn spread_of(document: &Document, page: u64) -> Option<u64> {
    document
        .walk()
        .filter(|node| node.kind == "layout.spread")
        .find(|spread| spread.children.iter().any(|child| child.id == page))
        .map(|spread| spread.id)
}

/// The **pages** of a spread, in the order they stand.
fn pages_of(document: &Document, spread: u64) -> Vec<u64> {
    document
        .node(spread)
        .map(|spread| {
            spread.children.iter().filter(|child| child.kind == "layout.page").map(|child| child.id).collect()
        })
        .unwrap_or_default()
}

/// The **facing page** of a page: the one across from it.
///
/// There is none when the page does not face another — when it says
/// it is single — or when its spread does not hold exactly two pages.
pub fn facing_of(document: &Document, page: u64) -> Option<u64> {
    let node = document.node(page)?;
    if node.kind != "layout.page" {
        return None;
    }
    if crate::sides::side_of(node) == Side::Single {
        return None;
    }
    let spread = spread_of(document, page)?;
    let pages = pages_of(document, spread);
    if pages.len() != 2 {
        return None;
    }
    pages.into_iter().find(|other| *other != page)
}

commands! {
    /// **Read** the facing page of a page.
    ///
    /// A query: it does not change the document.
    layout.page.facing {
        area: "layout",
        layer: "layout",
        icon: "page",
        params: {
            /// The page.

            page: NodeRef,
        },
        run: {

            let id = "layout.page.facing".to_string();
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
            let document = session.document();
            let facing = facing_of(document, page);
            let spread = spread_of(document, page);
            Ok(Outcome::new(match facing {
                Some(facing) => format!("the page `{page}` faces the page `{facing}`"),
                None => format!("the page `{page}` faces nothing"),
            })
            .with_data(json!({
                "page": page,
                "facing": facing.map(|facing| json!(facing)).unwrap_or(Json::Null),
                "spread": spread.map(|spread| json!(spread)).unwrap_or(Json::Null),
            })))

        }
    }
}
