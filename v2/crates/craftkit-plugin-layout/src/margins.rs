//! The **margin** of a page.
//!
//! A page may have **margins**: the room between the edge of the page
//! and the text that is set on it. The margin is a fact of the
//! **page** — four numbers, the room at the **top**, at the
//! **bottom**, at the **left** and at the **right** — and the **margin
//! box** of the page is **derived**: it is the box inside the margins,
//! and it is the box the text is set in. So the box cannot be out of
//! date, and the margin box of a page with no margins is the box of
//! the page.
//!
//! The rules:
//!
//! - a number that is **not a number**, or that is **negative**, is
//!   refused: the margin is room, and there is no room that is less
//!   than nothing;
//! - margins that leave **no room** for the page are refused — the
//!   left and the right must be less than the width together, and the
//!   top and the bottom less than the height together;
//! - a side that is **not said** keeps the room it had.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

/// The **margins** of a page: the room at each side of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Margin {
    /// The room at the top of the page.
    pub top: f64,
    /// The room at the bottom of the page.
    pub bottom: f64,
    /// The room at the left of the page.
    pub left: f64,
    /// The room at the right of the page.
    pub right: f64,
}

impl Margin {
    /// The margins of a page that has none: no room at all.
    pub fn none() -> Margin {
        Margin { top: 0.0, bottom: 0.0, left: 0.0, right: 0.0 }
    }

    /// The margins, as they are kept.
    pub fn to_json(self) -> Json {
        json!({
            "top": self.top,
            "bottom": self.bottom,
            "left": self.left,
            "right": self.right,
        })
    }

    /// The **margin box** of a page of this size: the box inside the
    /// margins.
    ///
    /// It is `None` when the margins leave no room.
    pub fn box_of(self, width: f64, height: f64) -> Option<(f64, f64, f64, f64)> {
        let inner_width = width - self.left - self.right;
        let inner_height = height - self.top - self.bottom;
        if inner_width <= 0.0 || inner_height <= 0.0 {
            return None;
        }
        Some((self.left, self.top, inner_width, inner_height))
    }
}

/// The **margins** of a page, if it has them.
pub fn margin_of(page: &Node) -> Margin {
    let Some(margin) = page.data.get("margin") else { return Margin::none() };
    let side = |name: &str| {
        margin
            .get(name)
            .and_then(|value| value.as_f64())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or(0.0)
    };
    Margin { top: side("top"), bottom: side("bottom"), left: side("left"), right: side("right") }
}

/// The **margin box** of a page, if it has one.
pub fn margin_box_of(page: &Node) -> Option<(f64, f64, f64, f64)> {
    let width = page.data.get("width")?.as_f64()?;
    let height = page.data.get("height")?.as_f64()?;
    margin_of(page).box_of(width, height)
}

/// **Set** the margins of a page.
///
/// A side that is not said keeps the room it had.
#[derive(Command)]
#[command(id = "layout.page.margin", area = "layout", layer = "layout", icon = "rect", mutates, undoable)]
pub struct SetMargin {
    /// The page.
    pub page: NodeRef,
    /// The room at the top of the page.
    pub top: Option<Number>,
    /// The room at the bottom of the page.
    pub bottom: Option<Number>,
    /// The room at the left of the page.
    pub left: Option<Number>,
    /// The room at the right of the page.
    pub right: Option<Number>,
}

impl SetMargin {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.page.margin".to_string();
        let page = self.page.0;
        let Some(node) = session.document().node(page).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no page `{page}`"),
            });
        };
        if node.kind != "layout.page" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{page}` is a `{}` and not a page", node.kind),
            });
        }
        let mut margin = margin_of(&node);
        for (side, given) in
            [("top", &self.top), ("bottom", &self.bottom), ("left", &self.left), ("right", &self.right)]
        {
            let Some(given) = given else { continue };
            let value = given.0;
            if !value.is_finite() || value < 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "{value} is not the room of a margin: the room is a number and not a negative one"
                    ),
                });
            }
            match side {
                "top" => margin.top = value,
                "bottom" => margin.bottom = value,
                "left" => margin.left = value,
                _ => margin.right = value,
            }
        }
        // The margins must leave **room** for the page: a margin box
        // that is not a box is not a box.
        let width = node.data.get("width").and_then(|width| width.as_f64()).unwrap_or(0.0);
        let height = node.data.get("height").and_then(|height| height.as_f64()).unwrap_or(0.0);
        if margin.box_of(width, height).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the margins leave no room for the page: the page is {width} by {height}"),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(page) {
            node.data["margin"] = margin.to_json();
        }
        Ok(Outcome::changed(format!("the page `{page}` has margins"))
            .with_data(json!({ "page": page, "margin": margin.to_json() })))
    }
}
