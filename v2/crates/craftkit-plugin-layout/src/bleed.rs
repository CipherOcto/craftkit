//! The **bleed** of a document.
//!
//! A document may have a **bleed**: the room the paper has beyond the
//! page, so that the ink that runs off the edge of the page has
//! somewhere to go. The bleed is a fact of the **document** — every
//! page of it is bled the same way — and it is four numbers: the room
//! at the **top**, at the **bottom**, at the **left** and at the
//! **right**.
//!
//! The **bleed box** of a page is **derived**: it is the box of the
//! page grown by the bleed, and it is the box the paper takes when it
//! is cut. So a page's bleed box cannot be out of date, and the box
//! of a document with no bleed is the box of the page.
//!
//! The rules:
//!
//! - a number that is **not a number**, or that is **negative**, is
//!   refused: the bleed is room, and there is no room that is less
//!   than nothing;
//! - a side that is **not said** keeps the room it had, and a
//!   document with no bleed has no room at all.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Session};

/// The **bleed** of a document: the room at each side of the page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bleed {
    /// The room at the top of the page.
    pub top: f64,
    /// The room at the bottom of the page.
    pub bottom: f64,
    /// The room at the left of the page.
    pub left: f64,
    /// The room at the right of the page.
    pub right: f64,
}

impl Bleed {
    /// The bleed of a document that has none: no room at all.
    pub fn none() -> Bleed {
        Bleed { top: 0.0, bottom: 0.0, left: 0.0, right: 0.0 }
    }

    /// The bleed, as it is kept.
    pub fn to_json(self) -> Json {
        json!({
            "top": self.top,
            "bottom": self.bottom,
            "left": self.left,
            "right": self.right,
        })
    }

    /// The **bleed box** of a page of this size: the box of the page
    /// grown by the bleed.
    pub fn box_of(self, width: f64, height: f64) -> (f64, f64, f64, f64) {
        (-self.left, -self.top, width + self.left + self.right, height + self.top + self.bottom)
    }
}

/// The **bleed** of a document, if it has one.
pub fn bleed_of(document: &Node) -> Bleed {
    let Some(bleed) = document.data.get("bleed") else { return Bleed::none() };
    let side = |name: &str| {
        bleed
            .get(name)
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(0.0)
    };
    Bleed { top: side("top"), bottom: side("bottom"), left: side("left"), right: side("right") }
}

/// The **document** a page is in, if it is in one.
pub fn document_of_page<'a>(document: &'a Document, page: &Node) -> Option<&'a Node> {
    document.walk().find(|node| {
        node.kind == "layout.document"
            && node.children.iter().any(|spread| {
                spread.kind == "layout.spread" && spread.children.iter().any(|child| child.id == page.id)
            })
    })
}

/// The **bleed box** of a page: the box of the page grown by the bleed
/// of the document it is in.
///
/// It is `None` when the page has no size to bleed.
pub fn bleed_box_of(document: &Document, page: &Node) -> Option<(f64, f64, f64, f64)> {
    let width = page.data.get("width")?.as_f64()?;
    let height = page.data.get("height")?.as_f64()?;
    let bleed = document_of_page(document, page).map(bleed_of).unwrap_or_else(Bleed::none);
    Some(bleed.box_of(width, height))
}

/// **Set** the bleed of a document.
///
/// A side that is not said keeps the room it had.
#[derive(Command)]
#[command(id = "layout.document.bleed", area = "layout", layer = "layout", icon = "rect", mutates, undoable)]
pub struct SetBleed {
    /// The document.
    pub document: NodeRef,
    /// The room at the top of the page.
    pub top: Option<Number>,
    /// The room at the bottom of the page.
    pub bottom: Option<Number>,
    /// The room at the left of the page.
    pub left: Option<Number>,
    /// The room at the right of the page.
    pub right: Option<Number>,
}

impl SetBleed {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.document.bleed".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        if node.kind != "layout.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{document}` is a `{}` and not a document", node.kind),
            });
        }
        let mut bleed = bleed_of(node);
        for (side, given) in
            [("top", &self.top), ("bottom", &self.bottom), ("left", &self.left), ("right", &self.right)]
        {
            let Some(given) = given else { continue };
            let value = given.0;
            if !value.is_finite() || value < 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "{value} is not the room of a bleed: the room is a number and not a negative one"
                    ),
                });
            }
            match side {
                "top" => bleed.top = value,
                "bottom" => bleed.bottom = value,
                "left" => bleed.left = value,
                _ => bleed.right = value,
            }
        }
        if let Some(node) = session.document_mut()?.node_mut(document) {
            node.data["bleed"] = bleed.to_json();
        }
        Ok(Outcome::changed(format!("the document `{document}` has a bleed of {bleed:?}"))
            .with_data(json!({ "document": document, "bleed": bleed.to_json() })))
    }
}
