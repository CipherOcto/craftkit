//! The **view** of a sheet.
//!
//! A sheet is not only its cells: it says whether the gridlines are
//! drawn, whether the headings are, whether the zeros are shown, and
//! whether it runs right to left. Those are **facts of the sheet**, and
//! the tree keeps them with the sheet and saves them with the file.
//!
//!
//! And a sheet says whether the **outline symbols** are drawn — the
//! marks beside the rows and the columns that are in an outline — and
//! whether the **formulas** are shown instead of the values they
//! stand for. Those are the tree's own flags of a sheet.
//!
//! And a sheet may have **frozen panes**: the rows above a cell and
//! the columns to the left of it stay put while the sheet scrolls.
//! The tree's rule is the cell's own: freezing at `B2` freezes the one
//! row above it and the one column to its left, and freezing at `A1`
//! freezes nothing at all — the tree's command puts no freeze where
//! the cell is the first one.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Text};

/// The **view** of a sheet: what is drawn of it, and what is not.
#[derive(Debug, Clone, PartialEq)]
pub struct SheetView {
    /// Whether the **gridlines** are drawn.
    pub gridlines: bool,
    /// Whether the **headings** — the row numbers and the column
    /// letters — are drawn.
    pub headings: bool,
    /// Whether the **zeros** are shown: a cell that holds zero shows
    /// an empty cell when the zeros are not shown.
    pub zeros: bool,
    /// Whether the sheet runs **right to left**.
    pub right_to_left: bool,
    /// Whether the **outline symbols** are drawn: the marks beside the
    /// rows and the columns that are in an outline.
    pub outline_symbols: bool,
    /// Whether the **formulas** are shown: a cell that holds a formula
    /// shows the formula and not its value.
    pub show_formulas: bool,
    /// The **frozen panes**: the rows above and the columns to the
    /// left of a cell.
    ///
    /// `None` is no freeze, and it is not a freeze at the first cell:
    /// the tree's command puts no freeze where the cell is the first
    /// one, and the two are the same thing here.
    pub freeze: Option<(u32, u32)>,
}

impl Default for SheetView {
    fn default() -> Self {
        SheetView {
            gridlines: true,
            headings: true,
            zeros: true,
            right_to_left: false,
            outline_symbols: true,
            show_formulas: false,
            freeze: None,
        }
    }
}

impl SheetView {
    /// The view as it is written.
    pub fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        json!({
            "gridlines": self.gridlines,
            "headings": self.headings,
            "zeros": self.zeros,
            "right_to_left": self.right_to_left,
            "outline_symbols": self.outline_symbols,
            "show_formulas": self.show_formulas,
            "freeze": match self.freeze {
                Some((rows, cols)) => json!([rows, cols]),
                None => json!(null),
            },
        })
    }

    /// The view as it is read.
    pub fn from_json(value: &craftkit_sdk::serde_json::Value) -> SheetView {
        let mut view = SheetView::default();
        if let Some(flag) = value.get("gridlines").and_then(|flag| flag.as_bool()) {
            view.gridlines = flag;
        }
        if let Some(flag) = value.get("headings").and_then(|flag| flag.as_bool()) {
            view.headings = flag;
        }
        if let Some(flag) = value.get("zeros").and_then(|flag| flag.as_bool()) {
            view.zeros = flag;
        }
        if let Some(flag) = value.get("right_to_left").and_then(|flag| flag.as_bool()) {
            view.right_to_left = flag;
        }
        if let Some(flag) = value.get("outline_symbols").and_then(|flag| flag.as_bool()) {
            view.outline_symbols = flag;
        }
        if let Some(flag) = value.get("show_formulas").and_then(|flag| flag.as_bool()) {
            view.show_formulas = flag;
        }
        if let Some(freeze) = value.get("freeze").and_then(|freeze| freeze.as_array()) {
            let rows = freeze.first().and_then(|rows| rows.as_u64()).unwrap_or(0) as u32;
            let cols = freeze.get(1).and_then(|cols| cols.as_u64()).unwrap_or(0) as u32;
            view.freeze = if rows == 0 && cols == 0 { None } else { Some((rows, cols)) };
        }
        view
    }
}

/// The **view** of a sheet, as the document holds it.
pub fn view_of(node: &craftkit_sdk::Node) -> SheetView {
    match node.data.get("view") {
        Some(value) => SheetView::from_json(value),
        None => SheetView::default(),
    }
}

commands! {
    /// Put the **view** of a sheet.
    ///
    /// A flag that is not named keeps its value, and a flag named with no
    /// value is **toggled**: the tree's command toggles a flag it is
    /// asked for without a value, and a flag asked for with a value is
    /// set.
    grid.sheet.view {
        area: "grid",
        layer: "grid",
        icon: "eye",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// Whether the gridlines are drawn.

            gridlines: Option<Bool>,
            /// Whether the headings are drawn.

            headings: Option<Bool>,
            /// Whether the zeros are shown.

            zeros: Option<Bool>,
            /// Whether the sheet runs right to left.

            right_to_left: Option<Bool>,
            /// Whether the **outline symbols** are drawn.

            outline_symbols: Option<Bool>,
            /// Whether the **formulas** are shown: a cell that holds a formula
            /// shows the formula and not its value.

            show_formulas: Option<Bool>,
        },
        run: {

            let id = "grid.sheet.view".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let mut view = view_of(node);
            if let Some(flag) = self.gridlines {
                view.gridlines = flag.0;
            }
            if let Some(flag) = self.headings {
                view.headings = flag.0;
            }
            if let Some(flag) = self.zeros {
                view.zeros = flag.0;
            }
            if let Some(flag) = self.right_to_left {
                view.right_to_left = flag.0;
            }
            if let Some(flag) = self.outline_symbols {
                view.outline_symbols = flag.0;
            }
            if let Some(flag) = self.show_formulas {
                view.show_formulas = flag.0;
            }
            let data = view.to_json();
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            node.data["view"] = data;
            Ok(Outcome::changed(format!("the view of the sheet `{sheet}` is set"))
                .with_data(json!({ "sheet": sheet, "view": view.to_json() })))

        }
    }
    /// **Freeze** the panes of a sheet at a cell.
    ///
    /// The rows above the cell and the columns to its left are frozen, and
    /// a cell that is the first one freezes nothing: the tree's command
    /// puts no freeze where the cell is `A1`, and a freeze of no rows and
    /// no columns is no freeze.
    grid.sheet.freeze {
        area: "grid",
        layer: "grid",
        icon: "lock",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The cell the panes freeze at: `B2` freezes the row above it and
            /// the column to its left.

            cell: Text,
        },
        run: {

            let id = "grid.sheet.freeze".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let mut view = view_of(node);
            let freeze = if cell.row == 0 && cell.col == 0 { None } else { Some((cell.row, cell.col)) };
            view.freeze = freeze;
            let data = view.to_json();
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            node.data["view"] = data;
            Ok(Outcome::changed(format!("the panes of the sheet `{sheet}` are frozen")).with_data(
                json!({ "sheet": sheet, "freeze": match freeze {
                    Some((rows, cols)) => json!([rows, cols]),
                    None => json!(null),
                } }),
            ))

        }
    }
    /// **Unfreeze** the panes of a sheet.
    ///
    /// A sheet with no frozen panes scrolls as a whole, and that is the
    /// tree's own.
    grid.sheet.freeze.delete {
        area: "grid",
        layer: "grid",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
        },
        run: {

            let id = "grid.sheet.freeze.delete".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let mut view = view_of(node);
            view.freeze = None;
            let data = view.to_json();
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            node.data["view"] = data;
            Ok(Outcome::changed(format!("the panes of the sheet `{sheet}` are not frozen"))
                .with_data(json!({ "sheet": sheet })))

        }
    }
    /// The **zoom** of a sheet, as a percentage.
    ///
    /// The tree's zoom runs from ten to four hundred, and a zoom outside
    /// that range is **refused**: the tree's command clamps it, and a
    /// zoom asked for as a thousand per cent is not a zoom.
    grid.sheet.zoom {
        area: "grid",
        layer: "grid",
        icon: "zoom",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The zoom, as a percentage: from ten to four hundred.

            percent: Number,
        },
        run: {

            let id = "grid.sheet.zoom".to_string();
            let sheet = self.sheet.0;
            let percent = self.percent.0;
            if !percent.is_finite() || !(10.0..=400.0).contains(&percent) {
                return Err(EngineError::Failed {
                    id,
                    message: "a zoom is a percentage from ten to four hundred".to_string(),
                });
            }
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            node.data["zoom"] = json!(percent);
            Ok(Outcome::changed(format!("the sheet `{sheet}` is at {percent} per cent"))
                .with_data(json!({ "sheet": sheet, "zoom": percent })))

        }
    }
}
