//! The **look** of a cell, copied to other cells.
//!
//! A look is how a cell is **shown**: its number format, its font, its
//! alignment, its fill and its borders. A cell that is shown like
//! another is often a cell that is *written* like it, and the tree's
//! **copy of a look** takes the look of one cell and writes it onto
//! the cells of a range.
//!
//! The rules:
//!
//! - the **look** is the number format, the font, the alignment, the
//!   fill and the borders — and nothing else;
//! - the **value** and the **formula** of a cell are not part of its
//!   look, and they stay as they are;
//! - the **style** a cell names is not part of its look either: a
//!   style is the workbook's, and it carries a lock;
//! - a cell that is **covered by a merge** or **locked** by the sheet's
//!   protection is **refused**: the look would be drawn under another
//!   cell, or written into a cell that is not to be written into;
//! - the **formatting** of a protected sheet that does not allow the
//!   formatting is refused, for the look is the formatting.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{Align, Cell, CellRef, Font};

/// The **look** of a cell: how it is shown.
#[derive(Clone, Debug, Default)]
struct Look {
    /// The number format.
    format: Option<String>,
    /// The font.
    font: Option<Font>,
    /// The alignment.
    align: Option<Align>,
    /// The fill.
    fill: Option<String>,
}

impl Look {
    /// The look of a cell, as it is now.
    fn of(cell: &Cell) -> Look {
        Look {
            format: cell.format.clone(),
            font: cell.font.clone(),
            align: cell.align,
            fill: cell.fill.clone(),
        }
    }
}

commands! {
    /// **Copy** the look of a cell onto a range of cells.
    ///
    /// A change: it writes the look onto the cells of the range, and
    /// what the cells hold is not changed.
    grid.cell.look.copy {
        area: "grid",
        layer: "grid",
        icon: "paint",
        mutates,
        undoable,
        params: {
            /// The sheet the cells are in.
            sheet: NodeRef,
            /// The cell the look is taken from.
            from: Text,
            /// The **range** the look is written onto: `A1`, `B2:D4`.
            to: Text,
        },
        run: {
            let id = "grid.cell.look.copy".to_string();
            let sheet = self.sheet.0;
            let from = crate::cell_at(&self.from.0, &id)?;
            let area = crate::fill::area_at(&self.to.0, &id)?;
            // The **locks** the styles of the workbook carry: they are
            // read before the document is borrowed for the change.
            let locks = crate::style_locks(session, sheet);
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
            let cells = crate::cells_of(node);
            let Some(source) = cells.get(from) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no cell `{}` to take a look from", from.to_text()),
                });
            };
            let look = Look::of(source);
            // The **borders** of the cell the look is taken from.
            let borders = crate::borders::borders_of_cell(node, &from.to_text());
            // The **cells** the look is written onto.
            let mut targets: Vec<CellRef> = Vec::new();
            for row in area.start.row..=area.end.row {
                for col in area.start.col..=area.end.col {
                    targets.push(CellRef { col, row });
                }
            }
            crate::merges::refuse_covered(node, &targets, &id)?;
            crate::protection::refuse_locked(node, &locks, &targets, &id)?;
            crate::protection::refuse_format(node, &id)?;
            let mut cells = crate::sheet_cells_mut(node);
            let mut with_look = 0usize;
            for target in &targets {
                let mut entry = cells.take(*target).unwrap_or(Cell {
                    value: None,
                    formula: None,
                    format: None,
                    style: None,
                    align: None,
                    font: None,
                    locked: None,
                    fill: None,
                    phonetic: None,
                });
                entry.format = look.format.clone();
                entry.font = look.font.clone();
                entry.align = look.align;
                entry.fill = look.fill.clone();
                let empty = entry.value.is_none()
                    && entry.formula.is_none()
                    && entry.style.is_none()
                    && entry.format.is_none()
                    && entry.align.is_none()
                    && entry.font.is_none()
                    && entry.fill.is_none();
                if empty {
                    // A cell with nothing in it and nothing to show is
                    // not a cell: it is taken away rather than left
                    // behind empty.
                    continue;
                }
                cells.set_entry(*target, entry);
                with_look += 1;
            }
            node.data["cells"] = crate::cells_into(&cells);
            // The **borders** of the cells the look is written onto are
            // the borders of the cell the look was taken from.
            let mut sheet_borders = crate::borders::borders_of(node);
            for target in &targets {
                let address = target.to_text();
                if borders.is_empty() {
                    sheet_borders.remove(&address);
                } else {
                    sheet_borders.insert(address, borders.clone());
                }
            }
            node.data["borders"] = crate::borders::borders_into(&sheet_borders);
            Ok(Outcome::changed(format!(
                "the look of the cell {} is copied onto {} cell(s)",
                from.to_text(),
                targets.len()
            ))
            .with_data(json!({
                "sheet": sheet,
                "from": from.to_text(),
                "to": area.to_text(),
                "cells": targets.len(),
                "withLook": with_look,
            })))

        }
    }
}
