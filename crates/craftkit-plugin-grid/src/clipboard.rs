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
//!   and their **alignment**: a cell is more than its value;
//! - the **borders** of the copied cells are the borders of the cells
//!   they land on: a border is part of the look of a cell, and the
//!   look is pasted with it;
//! - the **comments** and the **notes** of the copied cells are the
//!   comments and the notes of the cells they land on: a comment
//!   travels with the cell it is on. A paste of the values and a paste
//!   of the formats do not carry them.
//!
//! And a paste may be a paste of the **formats**: the look of the
//! copied cells is written onto the cells they land on and what those
//! cells hold is not touched. A paste of the formats and a paste of
//! the values are two different pastes, and a paste is one of them.
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
//!
//! And a paste may **skip the blanks**: a cell of the clipboard that
//! holds nothing writes nothing over the cell it lands on, for a blank
//! is nothing and nothing writes nothing.

use std::collections::BTreeMap;

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
fn copy_into(
    node: &mut Node,
    sheet: u64,
    cells: &Cells,
    first: CellRef,
    borders: &BTreeMap<String, BTreeMap<String, crate::borders::Border>>,
    comments: &BTreeMap<String, crate::comments::Comment>,
    notes: &BTreeMap<String, crate::notes::Note>,
) {
    let mut copied = craftkit_sdk::serde_json::Map::new();
    for (cell, entry) in cells.iter() {
        copied.insert(cell.to_text(), crate::cell_into(entry));
    }
    // The **borders** of the copied cells: a border is part of the look
    // of a cell, and the look is pasted with it.
    let mut held = BTreeMap::new();
    for (address, edges) in borders {
        if copied.contains_key(address) {
            held.insert(address.clone(), edges.clone());
        }
    }
    // The **comments** and the **notes** of the copied cells: a comment
    // travels with the cell it is on.
    let mut held_comments = BTreeMap::new();
    for (address, comment) in comments {
        if copied.contains_key(address) {
            held_comments.insert(address.clone(), comment.clone());
        }
    }
    let mut held_notes = BTreeMap::new();
    for (address, note) in notes {
        if copied.contains_key(address) {
            held_notes.insert(address.clone(), note.clone());
        }
    }
    node.data["clipboard"] = json!({
        "sheet": sheet,
        "at": first.to_text(),
        "cells": Json::Object(copied),
        "borders": crate::borders::borders_into(&held),
        "comments": crate::comments::comments_into(&held_comments),
        "notes": crate::notes::notes_into(&held_notes),
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
            let borders = crate::borders::borders_of(node);
            let comments = crate::comments::comments_of(node);
            let notes = crate::notes::notes_of(node);
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                copy_into(node, sheet, &copied, area.start, &borders, &comments, &notes);
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
            let borders = crate::borders::borders_of(node);
            let comments = crate::comments::comments_of(node);
            let notes = crate::notes::notes_of(node);
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                copy_into(node, sheet, &copied, area.start, &borders, &comments, &notes);
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
            /// Whether only the **look** of the cells is pasted: the
            /// format, the font, the alignment, the fill and the borders.
            ///
            /// What the cells hold is not touched, and it may not be given
            /// with the values.
            formats: Option<Bool>,
            /// Whether the cells of the clipboard that hold **nothing** are
            /// skipped: a blank writes nothing over the cell it lands on.
            skip_blanks: Option<Bool>,
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
            let skip_blanks = matches!(self.skip_blanks, Some(Bool(true)));
            // A paste of **values** and a paste of **formats** are two
            // different pastes, and a paste is one of them.
            let formats = matches!(self.formats, Some(Bool(true)));
            // The **values**: a paste of values drops the formulas and
            // pastes what they give, and the value is computed in the
            // sheet the cells were copied from — the value of a formula
            // is derived, and it is derived where the formula is.
            let values = matches!(self.values, Some(Bool(true)));
            // A paste of **values** and a paste of **formats** are two
            // different pastes, and a paste is one of them.
            if formats && values {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "paste the values or the formats, and not both".to_string(),
                });
            }
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
                // A cell that holds **nothing** writes nothing over the
                // cell it lands on, when the file says so: a blank is
                // nothing, and nothing writes nothing.
                if skip_blanks && entry.value.is_none() && entry.formula.is_none() {
                    continue;
                }
                if formats {
                    // The **look** of the cell: its format, its font, its
                    // alignment and its fill. The value and the formula
                    // are not part of it, and the style and the lock are
                    // not either.
                    let mut look = entry.clone();
                    look.value = None;
                    look.formula = None;
                    look.style = None;
                    look.locked = None;
                    pasted.push((cell, (row as u32, col as u32), look));
                    continue;
                }
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
            // A paste of **formats** does not touch what the cells hold,
            // and so it is not refused over a cell that holds something.
            if !overwrite && !formats {
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
            // The **borders** of the copied cells are the borders of the
            // cells they land on: a border is part of the look of a cell,
            // and the look is pasted with it.
            let borders = book
                .data
                .get("clipboard")
                .and_then(|clipboard| clipboard.get("borders"))
                .map(crate::borders::borders_from)
                .unwrap_or_default();
            for (_, (row, col), entry) in &pasted {
                let landing = CellRef { col: *col, row: *row };
                if formats {
                    // What the cell holds **stays**: only the look of the
                    // cell that was copied is written onto it.
                    let mut merged = written.get(landing).cloned().unwrap_or_default();
                    merged.format = entry.format.clone();
                    merged.align = entry.align;
                    merged.font = entry.font.clone();
                    merged.fill = entry.fill.clone();
                    written.set_entry(landing, merged);
                    continue;
                }
                written.set_entry(landing, entry.clone());
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["cells"] = crate::cells_into(&written);
                let mut sheet_borders = crate::borders::borders_of(node);
                for (cell, (row, col), _) in &pasted {
                    let address = CellRef { col: *col, row: *row }.to_text();
                    let source = cell.to_text();
                    match borders.get(&source) {
                        Some(edges) => {
                            sheet_borders.insert(address, edges.clone());
                        }
                        None => {
                            sheet_borders.remove(&address);
                        }
                    }
                }
                node.data["borders"] = crate::borders::borders_into(&sheet_borders);
            }
            // The **comments** and the **notes** of the copied cells are
            // the comments and the notes of the cells they land on: a
            // comment travels with the cell it is on. A paste of the
            // values and a paste of the formats do not carry them.
            if !values && !formats {
                let comments = book
                    .data
                    .get("clipboard")
                    .and_then(|clipboard| clipboard.get("comments"))
                    .and_then(|comments| comments.as_object())
                    .map(|comments| {
                        comments
                            .iter()
                            .filter_map(|(address, comment)| {
                                crate::comments::comment_from(comment)
                                    .map(|comment| (address.to_string(), comment))
                            })
                            .collect::<BTreeMap<String, crate::comments::Comment>>()
                    })
                    .unwrap_or_default();
                let notes = book
                    .data
                    .get("clipboard")
                    .and_then(|clipboard| clipboard.get("notes"))
                    .and_then(|notes| notes.as_object())
                    .map(|notes| {
                        notes
                            .iter()
                            .filter_map(|(address, note)| {
                                crate::notes::note_from(note).map(|note| (address.to_string(), note))
                            })
                            .collect::<BTreeMap<String, crate::notes::Note>>()
                    })
                    .unwrap_or_default();
                if let Some(node) = session.document_mut()?.node_mut(sheet) {
                    let mut sheet_comments = crate::comments::comments_of(node);
                    let mut sheet_notes = crate::notes::notes_of(node);
                    for (cell, (row, col), _) in &pasted {
                        let address = CellRef { col: *col, row: *row }.to_text();
                        let source = cell.to_text();
                        match comments.get(&source) {
                            Some(comment) => {
                                sheet_comments.insert(address.clone(), comment.clone());
                                sheet_notes.remove(&address);
                            }
                            None => {
                                sheet_comments.remove(&address);
                                match notes.get(&source) {
                                    Some(note) => {
                                        sheet_notes.insert(address.clone(), note.clone());
                                    }
                                    None => {
                                        sheet_notes.remove(&address);
                                    }
                                }
                            }
                        }
                    }
                    node.data["comments"] = crate::comments::comments_into(&sheet_comments);
                    node.data["notes"] = crate::notes::notes_into(&sheet_notes);
                }
            }
            Ok(Outcome::changed(format!("{count} cell(s) were pasted at {}", at.to_text()))
                .with_data(json!({
                    "sheet": sheet,
                    "at": at.to_text(),
                    "workbook": workbook,
                    "pasted": count,
                    "values": values,
                    "transpose": transpose,
                    "formats": formats,
                    "skipBlanks": skip_blanks,
                })))

        }
    }
}
