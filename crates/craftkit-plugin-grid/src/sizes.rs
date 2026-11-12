//! The **sizes** of a sheet.
//!
//! A sheet's rows have a **height** and its columns have a **width**,
//! and they are facts of the sheet: the tree's row height and column
//! width are stored with the sheet and are the same in every view.
//!
//! The **units** are the tree's own:
//!
//! - a row's height is in **points**, because the tree's dialogue asks
//!   for points — a row is fifteen points high by default;
//! - a column's width is in **characters**, because the tree's dialogue
//!   asks for characters — a column is eight and a half characters
//!   wide by default.
//!
//! And a size of **nothing** is refused: a row of no height is a
//! **hidden** row in the tree, and hiding a row is a thing of its own
//! that this tree has not got yet — so a size that means hiding is
//! refused rather than stored as a size.
//!
//! # What the port found
//!
//! - **The tree's row height may be given in pixels or in points and
//!   its column width in pixels or in characters.** The tree's
//!   commands take either and work out which one was meant. Here a
//!   size is in the unit the tree's dialogue uses, and the unit is in
//!   the command's name: the height is points and the width is
//!   characters.
//! - **The tree's size applies to a range of rows or columns at once**
//!   — `2:5`, `2:2,4:5` — and its dialogue asks for all of them. Here
//!   a size is for **one** row or **one** column, and a size for many
//!   is many sizes: one command and one row each.
//! - **The tree's autofit measures a font and this one counts.** The
//!   tree may measure the text of a cell and make the row or the
//!   column the size of the text, and that is a **measurement** of a
//!   font. Here the text is counted — the width of a column is the
//!   width of its longest line, and the height of a row is the height
//!   of its lines (the module `autofit`).

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Text};

/// The **size** of a row or a column: a number of points or of
/// characters.
fn size_of(value: Number, unit: &str, id: &str) -> Result<f64, EngineError> {
    let size = value.0;
    if !size.is_finite() || size < 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a size of {unit} is a number that is there: {size}"),
        });
    }
    if size == 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "a size of no {unit} is a hidden row or column, and the hiding of one is not here"
            ),
        });
    }
    Ok(size)
}

commands! {
    /// The **height** of a row, in points.
    grid.row.height {
        area: "grid",
        layer: "grid",
        icon: "rows",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// Which row, counting from one.

            row: Number,
            /// How high it is, in points.

            height: Number,
        },
        run: {

            let id = "grid.row.height".to_string();
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
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let row = self.row.0;
            if !row.is_finite() || row < 1.0 || row.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a row is counted from one: {row} is not one"),
                });
            }
            let row = row as u64;
            let height = size_of(self.height, "points", &id)?;
            let mut rows = node.data.get("rows").cloned().unwrap_or(json!({}));
            if !rows.is_object() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the rows of the sheet `{sheet}` are not a group"),
                });
            }
            rows[row.to_string()] = json!(height);
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["rows"] = rows;
            }
            Ok(Outcome::changed(format!("the row {row} is {height} points high")).with_data(json!({
                "sheet": sheet,
                "row": row,
                "height": height,
            })))

        }
    }
    /// The **width** of a column, in characters.
    grid.column.width {
        area: "grid",
        layer: "grid",
        icon: "columns",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// Which column: `A`, `B`, `AA`.

            column: Text,
            /// How wide it is, in characters.

            width: Number,
        },
        run: {

            let id = "grid.column.width".to_string();
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
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            let column = self.column.0.trim().to_string();
            if column.is_empty() || !column.chars().all(|character| character.is_ascii_alphabetic()) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
                });
            }
            let column = column.to_uppercase();
            let width = size_of(self.width, "characters", &id)?;
            let mut columns = node.data.get("columns").cloned().unwrap_or(json!({}));
            if !columns.is_object() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the columns of the sheet `{sheet}` are not a group"),
                });
            }
            columns[&column] = json!(width);
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["columns"] = columns;
            }
            Ok(Outcome::changed(format!("the column {column} is {width} characters wide")).with_data(json!({
                "sheet": sheet,
                "column": column,
                "width": width,
            })))

        }
    }
}

/// The **width** of a column, in characters, when it has one.
///
/// A column with no width has no width: the tree's default width is a
/// fact of the tree's dialogue and of its renderer, and this tree
/// writes the text of a cell and not a picture of it.
pub(crate) fn column_width(node: &craftkit_sdk::Node, col: u32) -> Option<f64> {
    let name = crate::formula::CellRef::column_text(col);
    node.data.get("columns")?.get(&name)?.as_f64()
}
