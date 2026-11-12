//! **Clearing** a cell.
//!
//! There are two things in a cell and they are cleared apart: what the
//! cell **holds** — a value or a formula — and how it is **shown** —
//! its number format and its style.
//!
//! The tree's *clear contents* empties the cell and keeps its style,
//! and the tree's *clear formats* takes the style away and keeps what
//! the cell holds. So here they are two commands, and the tree's
//! *clear all* is the clearing of the whole cell that is already here.
//!
//! # What the port found
//!
//! - **The tree's format is a style and this tree's format is the
//!   cell's own.** The tree's style is shared between the cells that
//!   name it, so clearing the format of a cell in the tree takes the
//!   style away from every cell that shares it — and the tree's
//!   *clear formats* clears the style of the cell and not the styles
//!   of the others. Here the format is the cell's own, so clearing it
//!   is the cell's business alone; a **style** is still shared, and a
//!   cell that names one keeps the name and loses the format it had.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// **Clear the contents** of a cell: its value and its formula.
///
/// The cell keeps its format and its style.
#[derive(Command)]
#[command(id = "grid.cell.clear.contents", area = "grid", layer = "grid", icon = "eraser", mutates, undoable)]
pub struct ClearContents {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
}

impl ClearContents {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.clear.contents".to_string();
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
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        crate::merges::refuse_covered(node, &[cell], &id)?;
        crate::protection::refuse_locked(node, &locks, &[cell], &id)?;
        let mut cells = crate::cells_of(node);
        // The **checkbox** of the cell: the tree's own rule is that
        // delete on a checkbox works in two passes, as it does in
        // Excel. A ticked box is unchecked first, and only when every
        // box is unchecked is the box removed.
        let ticked = cells.get(cell).is_some_and(|entry| {
            crate::checkboxes::is_checkbox(entry.format.as_deref())
                && entry.value == Some(crate::formula::Value::Bool(true))
        });
        let checkbox =
            cells.get(cell).is_some_and(|entry| crate::checkboxes::is_checkbox(entry.format.as_deref()));
        let had = if ticked {
            // The box is **unchecked** and it stays.
            cells.set_value(cell, crate::formula::Value::Bool(false));
            true
        } else {
            let had = cells.clear_contents(cell);
            if checkbox {
                // The box is **removed** with the contents.
                cells.set_format(cell, None);
            }
            had
        };
        node.data["cells"] = crate::cells_into(&cells);
        Ok(Outcome::changed(format!("emptied the cell {}", cell.to_text()))
            .with_data(json!({ "sheet": sheet, "cell": cell.to_text(), "had": had })))
    }
}

/// **Clear the format** of a cell: its number format and its style.
///
/// The cell keeps what it holds.
#[derive(Command)]
#[command(id = "grid.cell.clear.format", area = "grid", layer = "grid", icon = "eraser", mutates, undoable)]
pub struct ClearFormat {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
}

impl ClearFormat {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.clear.format".to_string();
        let sheet = self.sheet.0;
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
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        crate::merges::refuse_covered(node, &[cell], &id)?;
        crate::protection::refuse_format(node, &id)?;
        let mut cells = crate::cells_of(node);
        let had = cells.clear_format(cell);
        node.data["cells"] = crate::cells_into(&cells);
        // The **border** is part of the formatting: the tree's *clear
        // formats* clears the style of the cell, and the border is in
        // the style.
        let had_border = crate::borders::take_borders(node, &cell.to_text());
        Ok(Outcome::changed(format!("the format of the cell {} is gone", cell.to_text())).with_data(
            json!({ "sheet": sheet, "cell": cell.to_text(), "had": had, "had_border": had_border }),
        ))
    }
}
