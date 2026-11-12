//! The **data table** of a sheet: the table of what the sheet would
//! say for a list of inputs.
//!
//! The tree's own: a range holds the **inputs** and the **formulas**,
//! and the **results** are written into the body of the range — the
//! tree's rule, and the results are a **snapshot**: they are values
//! and not a live formula.
//!
//! The tree's rules:
//!
//! - the range needs at least **two rows and two columns**: the corner
//!   with the formulas, the row or the column with the inputs, and the
//!   body with the results;
//! - **one** of the inputs is enough, and **both** make a table of two
//!   variables;
//! - an **input cell** may not be **inside** the range: the tree's own
//!   rule;
//! - the results are **static**: a snapshot of what the sheet said,
//!   and not a live `{=TABLE()}`.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{Area, CellRef, Value};

/// The tree's own **bound** on the cells of a data table.
const MAX_CELLS: usize = 100_000;

commands! {
    /// The **data table** of a sheet.
    ///
    /// The tree's own `data.dataTable`.
    grid.data.table {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet it is on.

            sheet: NodeRef,
            /// The range of the table: `D2:E10`. The corner holds the
            /// formulas, the first row or the first column holds the inputs,
            /// and the body is written with the results.

            range: Text,
            /// The cell the **row** of inputs is tried in: the values across
            /// the top of the table go into it.

            row_input: Option<Text>,
            /// The cell the **column** of inputs is tried in: the values down
            /// the side of the table go into it.

            column_input: Option<Text>,
        },
        run: {

            let id = "grid.data.table".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed { id, message: format!("there is no sheet `{sheet}`") });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let name = node.name.clone();
            // The **range** of the table.
            let Some(area) = crate::conditional::area_of(&self.range.0) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("`{}` is not a range of cells", self.range.0.trim()),
                });
            };
            if area.start.row >= area.end.row || area.start.col >= area.end.col {
                return Err(EngineError::Failed {
                    id,
                    message: "the table needs at least two rows and two columns".to_string(),
                });
            }
            // The **inputs**: one of them is enough, and both make a table
            // of two variables.
            let row_input = match &self.row_input {
                Some(text) => Some(crate::whatifs::cell_of(&text.0, &id, "row input")?),
                None => None,
            };
            let column_input = match &self.column_input {
                Some(text) => Some(crate::whatifs::cell_of(&text.0, &id, "column input")?),
                None => None,
            };
            if row_input.is_none() && column_input.is_none() {
                return Err(EngineError::Failed {
                    id,
                    message: "give the row input, the column input, or both".to_string(),
                });
            }
            // An **input cell** may not be inside the table: the tree's own
            // rule.
            for input in [row_input, column_input].into_iter().flatten() {
                if area.covers(input) {
                    return Err(EngineError::Failed {
                        id,
                        message: format!("the input cell {} is inside the table", input.to_text()),
                    });
                }
            }
            // The **body** of the table: the range without its first row
            // and its first column.
            let body = Area::new(CellRef { row: area.start.row + 1, col: area.start.col + 1 }, area.end);
            if body.count() > MAX_CELLS as u64 {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the table body is too large ({MAX_CELLS} cells max)"),
                });
            }
            // The **cells** of the sheet: the formulas and the inputs are
            // read from them.
            let cells = crate::cells_of(node);
            let number_at = |cell: CellRef| -> Result<f64, EngineError> {
                let value = cells.get(cell).and_then(|entry| entry.value.clone());
                match value {
                    Some(Value::Number(number)) => Ok(number),
                    _ => Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the value in {} is not a number", cell.to_text()),
                    }),
                }
            };
            // The **results**: the sheet is asked, in a copy of the
            // workbook, and the answers are written into the body.
            let mut book = crate::book_of(session.document());
            let mut results: Vec<(CellRef, Value)> = Vec::new();
            let mode = match (row_input, column_input) {
                // The **column** of inputs: the values down the first
                // column, and the formulas in the first row.
                (None, Some(column)) => {
                    for row in area.start.row + 1..=area.end.row {
                        let value = number_at(CellRef { row, col: area.start.col })?;
                        for col in area.start.col + 1..=area.end.col {
                            let read = CellRef { row: area.start.row, col };
                            let answer = crate::whatifs::trial(&mut book, &name, column, read, value, &id)?;
                            results.push((CellRef { row, col }, answer));
                        }
                    }
                    "column"
                }
                // The **row** of inputs: the values across the first row,
                // and the formulas down the first column.
                (Some(row_cell), None) => {
                    for col in area.start.col + 1..=area.end.col {
                        let value = number_at(CellRef { row: area.start.row, col })?;
                        for row in area.start.row + 1..=area.end.row {
                            let read = CellRef { row, col: area.start.col };
                            let answer = crate::whatifs::trial(&mut book, &name, row_cell, read, value, &id)?;
                            results.push((CellRef { row, col }, answer));
                        }
                    }
                    "row"
                }
                // The table of **two variables**: the formula is in the
                // corner, the row of inputs goes into the row input and
                // the column of inputs into the column input.
                (Some(row_cell), Some(column_cell)) => {
                    for row in area.start.row + 1..=area.end.row {
                        let down = number_at(CellRef { row, col: area.start.col })?;
                        for col in area.start.col + 1..=area.end.col {
                            let across = number_at(CellRef { row: area.start.row, col })?;
                            let read = CellRef { row: area.start.row, col: area.start.col };
                            let Some(cells) = book.sheet_mut(&name) else {
                                return Err(EngineError::Failed {
                                    id,
                                    message: format!("there is no sheet `{name}`"),
                                });
                            };
                            cells.set_value(row_cell, Value::Number(across));
                            cells.set_value(column_cell, Value::Number(down));
                            let answer = book.value_of(&name, read);
                            results.push((CellRef { row, col }, answer));
                        }
                    }
                    "two-variable"
                }
                (None, None) => {
                    return Err(EngineError::Failed {
                        id,
                        message: "give the row input, the column input, or both".to_string(),
                    });
                }
            };
            // The **results** are written into the body: they are values,
            // and not a live formula.
            let count = results.len();
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed { id, message: format!("there is no sheet `{sheet}`") });
            };
            let mut cells = crate::sheet_cells_mut(node);
            for (cell, value) in results {
                cells.set_value(cell, value);
            }
            node.data["cells"] = crate::cells_into(&cells);
            Ok(Outcome::changed(format!("the table of {} is written", body.to_text())).with_data(json!({
                "sheet": sheet,
                "range": body.to_text(),
                "cells": count,
                "mode": mode,
                "static": true,
            })))

        }
    }
}
