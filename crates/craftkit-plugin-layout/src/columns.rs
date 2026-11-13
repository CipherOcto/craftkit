//! The **columns** of a page.
//!
//! A page may be set in **columns**: the text is set in two or more
//! columns side by side, with a **gutter** of room between them. The
//! columns are a fact of the **page** — a number of them and the
//! width of the gutter — and the **column boxes** are **derived**: the
//! margin box of the page divided into equal columns with the gutter
//! between them. So the boxes cannot be out of date, and they follow
//! the margins when the margins change.
//!
//! The rules:
//!
//! - a number of columns that is **not a whole number above one** is
//!   refused, and so is a number beyond the tree's own **two hundred
//!   and sixteen**;
//! - a **gutter** that is not a number, or that is negative, is
//!   refused;
//! - a gutter so wide that the columns **do not fit** in the margin
//!   box is refused: there is no column left to set text in.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome};

/// The **most** columns a page may have: the tree's own.
const MAX_COLUMNS: u32 = 216;

/// The **columns** of a page: how many there are and the gutter
/// between them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Columns {
    /// How many columns there are.
    pub count: u32,
    /// The room between them.
    pub gutter: f64,
}

impl Columns {
    /// The columns of a page that says nothing: one column, and no
    /// gutter to speak of.
    pub fn one() -> Columns {
        Columns { count: 1, gutter: 0.0 }
    }

    /// The columns, as they are kept.
    pub fn to_json(self) -> Json {
        json!({ "count": self.count, "gutter": self.gutter })
    }

    /// The **column boxes** of a margin box: the box divided into
    /// equal columns with the gutter between them.
    ///
    /// It is `None` when the gutter is too wide for the columns to
    /// fit.
    pub fn boxes_of(self, box_of: (f64, f64, f64, f64)) -> Option<Vec<(f64, f64, f64, f64)>> {
        let (x, y, width, height) = box_of;
        let count = self.count.max(1) as f64;
        let column_width = (width - self.gutter * (count - 1.0)) / count;
        if column_width <= 0.0 {
            return None;
        }
        Some(
            (0..self.count)
                .map(|index| {
                    let left = x + index as f64 * (column_width + self.gutter);
                    (left, y, column_width, height)
                })
                .collect(),
        )
    }
}

/// The **columns** of a page, if it has them.
pub fn columns_of(page: &Node) -> Columns {
    let Some(columns) = page.data.get("columns") else { return Columns::one() };
    let count = columns.get("count").and_then(|count| count.as_u64()).unwrap_or(1);
    let count = count.clamp(1, MAX_COLUMNS as u64) as u32;
    let gutter = columns
        .get("gutter")
        .and_then(|gutter| gutter.as_f64())
        .filter(|gutter| gutter.is_finite() && *gutter >= 0.0)
        .unwrap_or(0.0);
    Columns { count, gutter }
}

commands! {
    /// **Set** the columns of a page.
    layout.page.columns {
        area: "layout",
        layer: "layout",
        icon: "rect",
        mutates,
        undoable,
        params: {
            /// The page.
            page: NodeRef,
            /// How many columns there are.
            count: Number,
            /// The room between them. Without it, the columns have no gutter.
            gutter: Option<Number>,
        },
        run: {
            let id = "layout.page.columns".to_string();
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
            let count = self.count.0;
            if !count.is_finite() || count < 1.0 || count.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{count} is not a number of columns: the columns are counted from one"),
                });
            }
            if count > MAX_COLUMNS as f64 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a page may have at most {MAX_COLUMNS} columns"),
                });
            }
            let gutter = self.gutter.map(|gutter| gutter.0).unwrap_or(0.0);
            if !gutter.is_finite() || gutter < 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{gutter} is not a gutter: the gutter is a number and not a negative one"),
                });
            }
            let columns = Columns { count: count as u32, gutter };
            // The columns must **fit**: there is no column left to set
            // text in when the gutter is wider than the room.
            if let Some(margin_box) = crate::margins::margin_box_of(&node)
                && columns.boxes_of(margin_box).is_none()
            {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the columns do not fit in the margin box with a gutter of {gutter}"),
                });
            }
            if let Some(node) = session.document_mut()?.node_mut(page) {
                node.data["columns"] = columns.to_json();
            }
            Ok(Outcome::changed(format!("the page `{page}` is set in {} column(s)", columns.count))
                .with_data(json!({ "page": page, "columns": columns.to_json() })))

        }
    }
}
