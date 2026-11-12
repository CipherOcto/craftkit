//! The formulas that become their **values**.
//!
//! The tree's Convert to Number turns the formulas of a range into
//! the numbers they give: a formula that gives a word, or a date's
//! text, or a text result, is left as it is.
//!
//! The rules are the tree's own:
//!
//! - the cells of a **range** are the ones that are converted;
//! - a formula that gives a **number** becomes that number, and the
//!   **format** of the cell is kept;
//! - a cell whose formula gives anything else is left as it is.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::fill::area_at;
use crate::formula::{CellRef, Value};
use crate::{book_of, cells_into, cells_of};

commands! {
    /// Turn the formulas of a range into the **values** they give.
    grid.formula.values {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The range of cells, `A1:A5`.

            range: Text,
        },
        run: {

            let id = "grid.formula.values".to_string();
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
            let area = area_at(&self.range.0, &id)?;
            let name = node.name.clone();
            let mut cells = cells_of(node);
            // The **book** the values are computed in: the sheet's own
            // cells are the ones it reads.
            let mut book = book_of(session.document());
            if let Some(sheet_cells) = book.sheet_mut(&name) {
                *sheet_cells = cells.clone();
            }
            // The cells whose formula gives a **number**.
            let mut converted: Vec<CellRef> = Vec::new();
            for row in area.start.row..=area.end.row {
                for column in area.start.col..=area.end.col {
                    let cell = CellRef { col: column, row };
                    let Some(entry) = cells.get(cell) else {
                        continue;
                    };
                    if entry.formula.is_none() {
                        continue;
                    }
                    if let Value::Number(number) = book.value_of(&name, cell) {
                        cells.set_value(cell, Value::Number(number));
                        converted.push(cell);
                    }
                }
            }
            if converted.is_empty() {
                return Ok(Outcome::new("no formula of the range gave a number")
                    .with_data(json!({ "sheet": sheet, "converted": 0 })));
            }
            crate::merges::refuse_covered(node, &converted, &id)?;
            crate::protection::refuse_locked(node, &crate::style_locks(session, sheet), &converted, &id)?;
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed { id, message: format!("there is no sheet `{sheet}`") });
            };
            node.data["cells"] = cells_into(&cells);
            Ok(Outcome::changed(format!("{} formula(s) became their values", converted.len()))
                .with_data(json!({ "sheet": sheet, "converted": converted.len() })))

        }
    }
}
