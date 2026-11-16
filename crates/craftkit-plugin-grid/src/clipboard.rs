//! The **clipboard** of a workbook: the cells that were copied, waiting
//! to be pasted somewhere else.
//!
//! The clipboard is the **workbook's**, and not the sheet's: cells may
//! be copied from one sheet and pasted into another, and that is what a
//! spreadsheet does.
//!
//! So the tree has three commands:
//!
//! - **`grid.range.copy`** — the cells of a range are copied into the
//!   clipboard of the workbook;
//! - **`grid.range.cut`** — the cells are copied and **taken away**;
//! - **`grid.range.paste`** — the cells of the clipboard are put into
//!   another place.
//!
//! # The rule of the formulas
//!
//! A pasted formula keeps its **shape**: a **relative** reference moves
//! with the cell it names, and an **absolute** one stays where it is.
//! So a formula that says `=A1` in the cell `B2` says `=C4` in the
//! cell `D5` — and a formula that says `=$A$1` still says `=$A$1`.
//!
//! # The rules
//!
//! - the clipboard of a workbook with nothing copied is **empty**, and
//!   a paste of nothing is **refused**;
//! - a paste over cells that hold something is **refused**, unless the
//!   paste is told to **overwrite**;
//! - the cells that are pasted keep their **format**, their **font**
//!   and their **alignment**: a cell is more than its value.
//!
//! And a paste may be a paste of **values**: the formulas are dropped
//! and what they give is pasted instead. The value is computed in the
//! sheet the cells were copied from, because the value of a formula is
//! derived, and it is derived where the formula is.
//!
//! And a paste may be **transposed**: the rows of the clipboard become
//! the columns of the block, and the columns become the rows. A cell
//! may not land **off the sheet** — a sheet has so many rows and so
//! many columns.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Text};

use crate::formula::{CellRef, Cells, MAX_COLS, MAX_ROWS};

/// The **workbook** a sheet is in, and its **name**.
fn workbook_of(session: &craftkit_sdk::Session, sheet: u64) -> Option<(u64, String)> {
    let node = session
        .document()
        .walk()
        .find(|node| node.kind == "grid.workbook" && node.children.iter().any(|child| child.id == sheet))?;
    Some((node.id, node.name.clone()))
}

/// The **range** a text names, as the corners of it.
fn corners_of(text: &str) -> Option<(CellRef, CellRef)> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), last.trim()),
        None => (trimmed, trimmed),
    };
    let first = CellRef::parse(first)?;
    let last = CellRef::parse(last)?;
    Some((first, last))
}

/// The **clipboard** of a workbook: the cells that were copied.
///
/// It is `None` when nothing was copied.
fn clipboard_of(node: &Node) -> Option<(Cells, CellRef)> {
    let clipboard = node.data.get("clipboard")?;
    let first = CellRef::parse(clipboard.get("at")?.as_str()?)?;
    let copied = clipboard.get("cells")?.as_object()?;
    let mut held = Cells::new();
    for (address, entry) in copied {
        let Some(cell) = CellRef::parse(address) else { continue };
        held.set_entry(cell, crate::cell_from(entry));
    }
    Some((held, first))
}

/// The **sheet** the cells of the clipboard were copied from.
fn clipboard_sheet(node: &Node) -> Option<u64> {
    node.data.get("clipboard")?.get("sheet")?.as_u64()
}

/// Put the **cells** into the clipboard of a workbook.
///
/// The cells are stored with the addresses they have, and the place
/// they were copied from — so that a paste knows how far they have
/// travelled.
fn copy_into(node: &mut Node, sheet: u64, cells: &Cells, first: CellRef) {
    let mut copied = craftkit_sdk::serde_json::Map::new();
    for (cell, entry) in cells.iter() {
        copied.insert(cell.to_text(), crate::cell_into(entry));
    }
    node.data["clipboard"] = json!({
        "sheet": sheet,
        "at": first.to_text(),
        "cells": Json::Object(copied),
    });
}

commands! {
    /// **Copy** a range of cells into the clipboard of the workbook.
    ///
    /// The cells wait there until they are pasted somewhere else.
    grid.range.copy {
        area: "grid",
        layer: "grid",
        icon: "copy",
        mutates,
        undoable,
        params: {
            /// The sheet the cells are in.
            sheet: NodeRef,
            /// The **range** of cells that are copied: `A1`, `B2:D4`.
            range: Text,
        },
        run: {
            let id = "grid.range.copy".to_string();
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
            let Some((first, last)) = corners_of(&self.range.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not a range of cells, and this tree does not know what it is",
                        self.range.0
                    ),
                });
            };
            let area = crate::formula::Area::new(first, last);
            let cells = crate::cells_of(node);
            let mut copied = Cells::new();
            for (cell, entry) in cells.iter() {
                if area.covers(cell) {
                    copied.set_entry(cell, entry.clone());
                }
            }
            let count = copied.len();
            let Some((workbook, _)) = workbook_of(session, sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` is in no workbook"),
                });
            };
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                copy_into(node, sheet, &copied, area.start);
            }
            Ok(Outcome::changed(format!("{count} cell(s) of `{}` were copied", self.range.0))
                .with_data(json!({
                    "sheet": sheet,
                    "range": self.range.0,
                    "workbook": workbook,
                    "copied": count,
                })))

        }
    }

    /// **Cut** a range of cells into the clipboard of the workbook.
    ///
    /// The cells wait there until they are pasted somewhere else, and
    /// they are **taken away** from where they were.
    grid.range.cut {
        area: "grid",
        layer: "grid",
        icon: "scissors",
        mutates,
        undoable,
        params: {
            /// The sheet the cells are in.
            sheet: NodeRef,
            /// The **range** of cells that are cut: `A1`, `B2:D4`.
            range: Text,
        },
        run: {
            let id = "grid.range.cut".to_string();
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
            let Some((first, last)) = corners_of(&self.range.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not a range of cells, and this tree does not know what it is",
                        self.range.0
                    ),
                });
            };
            let area = crate::formula::Area::new(first, last);
            let cells = crate::cells_of(node);
            let mut copied = Cells::new();
            let mut taken = Cells::new();
            for (cell, entry) in cells.iter() {
                if area.covers(cell) {
                    copied.set_entry(cell, entry.clone());
                } else {
                    taken.set_entry(cell, entry.clone());
                }
            }
            let count = copied.len();
            let Some((workbook, _)) = workbook_of(session, sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` is in no workbook"),
                });
            };
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                copy_into(node, sheet, &copied, area.start);
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["cells"] = crate::cells_into(&taken);
            }
            Ok(Outcome::changed(format!("{count} cell(s) of `{}` were cut", self.range.0))
                .with_data(json!({
                    "sheet": sheet,
                    "range": self.range.0,
                    "workbook": workbook,
                    "cut": count,
                })))

        }
    }

    /// **Paste** the cells of the clipboard into a sheet.
    ///
    /// The cells land at the place that is named, and the formulas are
    /// **shifted**: a relative reference moves with the cell it names.
    grid.range.paste {
        area: "grid",
        layer: "grid",
        icon: "paste",
        mutates,
        undoable,
        params: {
            /// The sheet the cells are pasted into.
            sheet: NodeRef,
            /// The cell the **first** cell of the clipboard lands on.
            at: Text,
            /// Whether the cells that are there may be **overwritten**.
            overwrite: Option<Bool>,
            /// Whether the **formulas** are dropped and the **values** they
            /// give are pasted.
            values: Option<Bool>,
            /// Whether the cells land **transposed**: the rows of the
            /// clipboard become the columns of the block, and the columns
            /// become the rows.
            transpose: Option<Bool>,
        },
        run: {
            let id = "grid.range.paste".to_string();
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
            let Some(at) = CellRef::parse(self.at.0.trim()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a cell, and this tree does not know what it is", self.at.0),
                });
            };
            let Some((workbook, _)) = workbook_of(session, sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` is in no workbook"),
                });
            };
            let Some(book) = session.document().node(workbook).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no workbook `{workbook}`"),
                });
            };
            let Some((held, from)) = clipboard_of(&book) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the clipboard is empty: there is nothing to paste".to_string(),
                });
            };
            let transpose = matches!(self.transpose, Some(Bool(true)));
            // The **values**: a paste of values drops the formulas and
            // pastes what they give, and the value is computed in the
            // sheet the cells were copied from — the value of a formula
            // is derived, and it is derived where the formula is.
            let values = matches!(self.values, Some(Bool(true)));
            let source = if values {
                let Some(source) = clipboard_sheet(&book) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the clipboard does not say which sheet its cells came from".to_string(),
                    });
                };
                let Some(node) = session.document().node(source) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the sheet `{source}` the cells were copied from is not there"),
                    });
                };
                node.name.clone()
            } else {
                String::new()
            };
            let book_of_values = if values { Some(crate::book_of(session.document())) } else { None };
            // The cells that are pasted: they land at the place that
            // is named, and their formulas are shifted.
            let mut pasted = Vec::new();
            for (cell, entry) in held.iter() {
                // The **landing** of the cell: the place it comes to.
                // A transposed paste swaps the rows and the columns.
                let (row, col) = if transpose {
                    (
                        at.row as i64 + (cell.col as i64 - from.col as i64),
                        at.col as i64 + (cell.row as i64 - from.row as i64),
                    )
                } else {
                    (
                        at.row as i64 + (cell.row as i64 - from.row as i64),
                        at.col as i64 + (cell.col as i64 - from.col as i64),
                    )
                };
                if row < 0 || col < 0 || row as u64 >= MAX_ROWS || col as u64 >= MAX_COLS {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the cell {} would land at {}, and that is not in the sheet",
                            cell.to_text(),
                            CellRef { col: col as u32, row: row as u32 }.to_text()
                        ),
                    });
                }
                let row_delta = row - cell.row as i64;
                let col_delta = col - cell.col as i64;
                if values {
                    if entry.formula.is_none() {
                        pasted.push((cell, (row as u32, col as u32), entry.clone()));
                        continue;
                    }
                    // The **value** the formula gives, and no formula.
                    let mut entry = entry.clone();
                    entry.formula = None;
                    entry.value = Some(
                        book_of_values
                            .as_ref()
                            .map(|book| book.value_of(&source, cell))
                            .unwrap_or(crate::formula::Value::Empty),
                    );
                    pasted.push((cell, (row as u32, col as u32), entry));
                    continue;
                }
                let Some(formula) = &entry.formula else {
                    pasted.push((cell, (row as u32, col as u32), entry.clone()));
                    continue;
                };
                let mut entry = entry.clone();
                if let Some(shifted) = crate::formula::shift_in_formula(formula, row_delta, col_delta) {
                    entry.formula = Some(shifted);
                }
                pasted.push((cell, (row as u32, col as u32), entry));
            }
            // A paste over cells that hold something is **refused**,
            // unless the paste is told to overwrite.
            let overwrite = matches!(self.overwrite, Some(Bool(true)));
            if !overwrite {
                let cells = crate::cells_of(node);
                for (_, (row, col), _) in &pasted {
                    let landing = CellRef { col: *col, row: *row };
                    if let Some(entry) = cells.get(landing)
                        && entry.value.is_some()
                    {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the cell {} holds something, and a paste over it is refused",
                                landing.to_text()
                            ),
                        });
                    }
                }
            }
            let count = pasted.len();
            let cells = crate::cells_of(node);
            let mut written = Cells::new();
            for (cell, entry) in cells.iter() {
                written.set_entry(cell, entry.clone());
            }
            for (_, (row, col), entry) in pasted {
                written.set_entry(CellRef { col, row }, entry);
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["cells"] = crate::cells_into(&written);
            }
            Ok(Outcome::changed(format!("{count} cell(s) were pasted at {}", at.to_text()))
                .with_data(json!({
                    "sheet": sheet,
                    "at": at.to_text(),
                    "workbook": workbook,
                    "pasted": count,
                    "values": values,
                    "transpose": transpose,
                })))

        }
    }
}
