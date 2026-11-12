//! The **checkbox** of a cell.
//!
//! A checkbox is a **boolean** that is written as a **box**: a ticked
//! one and an empty one. The tree's format for it is the number format
//! `checkbox`, and a cell that carries it is a cell control.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A cell that is made a checkbox **keeps what it holds**: a value
//!   that is not a boolean shows as it is. An **empty** cell starts
//!   **unchecked**, that is, it holds `FALSE`.
//! - A checkbox is **toggled**: `TRUE` becomes `FALSE`, `FALSE`
//!   becomes `TRUE`, and an empty checkbox cell becomes `TRUE` — an
//!   empty checkbox cell counts as `FALSE`. A cell that **holds a
//!   formula** is not toggled, and a cell that is **not a checkbox**
//!   is left as it is.
//! - **Delete** on a checkbox works in **two passes**, as it does in
//!   Excel: a ticked box is **unchecked first**, and only when every
//!   box is unchecked is the box **removed**. A cell that is not a
//!   checkbox is emptied at once.
//! - **Clear Formats** removes the box and **keeps the value**.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::Value;

/// The **code** of the format of a checkbox: the tree's own.
pub const CHECKBOX: &str = "checkbox";

/// Whether a cell **is** a checkbox: its format says so.
///
/// The tree's own rule: the format of the cell is the format of the
/// tree's checkbox.
pub fn is_checkbox(format: Option<&str>) -> bool {
    format.is_some_and(|format| format.trim().eq_ignore_ascii_case(CHECKBOX))
}

commands! {
    /// **Make** a cell a checkbox.
    ///
    /// The cell keeps what it holds, and an empty cell starts unchecked.
    grid.cell.checkbox.set {
        area: "grid",
        layer: "grid",
        icon: "check",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The cell's address.

            cell: Text,
        },
        run: {

            let id = "grid.cell.checkbox.set".to_string();
            let sheet = self.sheet.0;
            // The **locks** the styles of the workbook carry: they are
            // read before the document is borrowed for the change.
            let locks = crate::style_locks(session, sheet);
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
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
            crate::merges::refuse_covered(node, &[cell], &id)?;
            crate::protection::refuse_locked(node, &locks, &[cell], &id)?;
            let mut cells = crate::cells_of(node);
            // The cell keeps what it holds, and an **empty** cell starts
            // **unchecked**.
            let was_empty = cells
                .get(cell)
                .and_then(|entry| entry.value.clone())
                .is_none_or(|value| matches!(value, Value::Empty))
                && cells.get(cell).and_then(|entry| entry.formula.clone()).is_none();
            cells.set_format(cell, Some(CHECKBOX.to_string()));
            if was_empty {
                cells.set_value(cell, Value::Bool(false));
            }
            let value = cells.get(cell).and_then(|entry| entry.value.clone()).map(|value| value.to_json());
            node.data["cells"] = crate::cells_into(&cells);
            Ok(Outcome::changed(format!("the cell {} is a checkbox", cell.to_text()))
                .with_data(json!({ "sheet": sheet, "cell": cell.to_text(), "value": value })))

        }
    }
    /// **Toggle** the checkbox of a cell.
    ///
    /// A cell that holds a formula is not toggled, and a cell that is not
    /// a checkbox is left as it is.
    grid.cell.checkbox.toggle {
        area: "grid",
        layer: "grid",
        icon: "check",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The cell's address.

            cell: Text,
        },
        run: {

            let id = "grid.cell.checkbox.toggle".to_string();
            let sheet = self.sheet.0;
            // The **locks** the styles of the workbook carry: they are
            // read before the document is borrowed for the change.
            let locks = crate::style_locks(session, sheet);
            let cell = crate::cell_at(&self.cell.0, &id)?;
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
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
            crate::merges::refuse_covered(node, &[cell], &id)?;
            crate::protection::refuse_locked(node, &locks, &[cell], &id)?;
            let mut cells = crate::cells_of(node);
            let Some(entry) = cells.get(cell).cloned() else {
                return Ok(Outcome::new(format!("the cell {} holds nothing", cell.to_text())).with_data(json!({
                    "sheet": sheet,
                    "cell": cell.to_text(),
                    "toggled": false,
                })));
            };
            // A cell that holds a **formula** is not toggled.
            if entry.formula.is_some() {
                return Ok(Outcome::new(format!("the cell {} holds a formula", cell.to_text())).with_data(
                    json!({
                        "sheet": sheet,
                        "cell": cell.to_text(),
                        "toggled": false,
                    }),
                ));
            }
            // A cell that is **not a checkbox** is left as it is.
            if !is_checkbox(entry.format.as_deref()) {
                return Ok(Outcome::new(format!("the cell {} is not a checkbox", cell.to_text())).with_data(
                    json!({
                        "sheet": sheet,
                        "cell": cell.to_text(),
                        "toggled": false,
                    }),
                ));
            }
            // The **box** is toggled: an empty checkbox cell counts as
            // `FALSE`, and a cell that holds **anything else** — a text, a
            // number — is left as it is, as the tree leaves it.
            let next = match entry.value {
                Some(Value::Bool(value)) => Value::Bool(!value),
                None | Some(Value::Empty) => Value::Bool(true),
                Some(other) => other,
            };
            cells.set_value(cell, next.clone());
            node.data["cells"] = crate::cells_into(&cells);
            Ok(Outcome::changed(format!("the checkbox of the cell {} is ticked", cell.to_text())).with_data(
                json!({ "sheet": sheet, "cell": cell.to_text(), "toggled": true, "value": next.to_json() }),
            ))

        }
    }
}
