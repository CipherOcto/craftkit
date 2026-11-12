//! The **autofit** of a row and of a column.
//!
//! A column may be made **as wide as its text** and a row **as tall as
//! its text**, and the size is worked out from the text the cells hold.
//!
//! # The rules
//!
//! - **The width of a column is the width of its longest line.** The
//!   text of a cell may be more than one line long, and the column is
//!   as wide as the widest of the lines.
//! - **A cell under a merge is not measured.** The tree's merged cells
//!   never autofit, and the cell that holds the merge is not measured
//!   either: the merge is the width of the column, and not its text.
//! - **A column with no text is left as it is.** There is nothing to
//!   measure, and a width that is not worked out is not a width.
//! - **The height of a row is the height of its lines.** The row is
//!   as tall as the most lines any of its cells holds, and a line is
//!   as high as the default height of a row.
//! - **The hiding of a row or a column is not touched.** In the tree a
//!   hidden row has no size at all, and the fitting reveals it; here
//!   the hiding is a fact of its own, and a row that is hidden is
//!   still a row that has a height.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The height of a **line** of text, in points: the default height of
/// a row.
const LINE_HEIGHT: f64 = 15.0;

/// The **room** a column has beside its text, in characters.
const ROOM: f64 = 1.0;

/// The **sheet** a node is, if it is one.
fn the_sheet<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no sheet `{node}`"),
        });
    };
    if found.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is not a sheet"),
        });
    }
    Ok(found)
}

/// The **column** a name says: the letters of the column.
fn column_of(name: &str, id: &str) -> Result<(String, u32), EngineError> {
    let column = name.trim().to_uppercase();
    if column.is_empty() || !column.chars().all(|character| character.is_ascii_alphabetic()) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
        });
    }
    let Some(index) = crate::CellRef::column_index(&column) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
        });
    };
    Ok((column, index))
}

/// The **row** a number says, counting from one.
fn row_of(value: Number, id: &str) -> Result<u64, EngineError> {
    let row = value.0;
    if !row.is_finite() || row < 1.0 || row.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a row is counted from one: {row} is not one"),
        });
    }
    Ok(row as u64)
}

/// The **lines** of the text of a cell: the lines it is written on.
fn lines_of(text: &str) -> Vec<&str> {
    let lines: Vec<&str> = text.lines().filter(|line| !line.trim().is_empty()).collect();
    if lines.is_empty() { vec![""] } else { lines }
}

commands! {
    /// **Make** a column **as wide as its text**.
    ///
    /// The width is worked out from the text the cells hold, and it is
    /// worked out in **characters**: the tree measures the text with a font
    /// and works out a width in points, and here the text is counted.
    grid.column.autofit {
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
        },
        run: {

            let id = "grid.column.autofit".to_string();
            let sheet = self.sheet.0;
            let node = the_sheet(session, &id, sheet)?;
            let (column, index) = column_of(&self.column.0, &id)?;
            let name = node.name.clone();
            let cells = crate::cells_of(node);
            // The **merges**: a cell under one is not measured.
            let merges = crate::merges::merges_of(node);
            // The **widest** line of the column.
            let mut widest: f64 = 0.0;
            for (cell, _entry) in cells.iter() {
                if cell.col != index {
                    continue;
                }
                if merges.iter().any(|area| area.covers(cell)) {
                    continue;
                }
                let text = crate::finding::text_of(session, sheet, &name, &cells, cell, false);
                for line in lines_of(&text) {
                    let width = line.chars().count() as f64;
                    if width > widest {
                        widest = width;
                    }
                }
            }
            if widest <= 0.0 {
                return Ok(Outcome::new(format!("the column {column} holds no text")).with_data(json!({
                    "sheet": sheet,
                    "column": column,
                    "width": Json::Null,
                    "fitted": false,
                })));
            }
            let width = widest + ROOM;
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
                "fitted": true,
            })))

        }
    }
    /// **Make** a row **as tall as its text**.
    ///
    /// The height is worked out from the lines the cells hold, and a line
    /// is as high as the default height of a row.
    grid.row.autofit {
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
        },
        run: {

            let id = "grid.row.autofit".to_string();
            let sheet = self.sheet.0;
            let node = the_sheet(session, &id, sheet)?;
            let row = row_of(self.row, &id)?;
            let name = node.name.clone();
            let cells = crate::cells_of(node);
            let merges = crate::merges::merges_of(node);
            // The **most** lines any cell of the row holds.
            let mut most: usize = 0;
            for (cell, _entry) in cells.iter() {
                if cell.row as u64 + 1 != row {
                    continue;
                }
                if merges.iter().any(|area| area.covers(cell)) {
                    continue;
                }
                let text = crate::finding::text_of(session, sheet, &name, &cells, cell, false);
                let lines = lines_of(&text).len();
                if lines > most {
                    most = lines;
                }
            }
            if most == 0 {
                return Ok(Outcome::new(format!("the row {row} holds no text")).with_data(json!({
                    "sheet": sheet,
                    "row": row,
                    "height": Json::Null,
                    "fitted": false,
                })));
            }
            let height = most as f64 * LINE_HEIGHT;
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
                "fitted": true,
            })))

        }
    }
}
