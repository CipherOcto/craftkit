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
//!   of the formats do not carry them;
//! - the **links** of the copied cells are the links of the cells they
//!   land on: a link belongs to the cell it is on;
//! - the **validation** of the copied cells is the validation of the
//!   cells they land on: the rules move with the cells, and a rule
//!   that reaches out of the copied range does not travel — half a
//!   rule is not a rule. A paste of the values and a paste of the
//!   formats do not carry it, and a transposed paste does not either.
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
//!
//! And a paste may be a paste of **links**: the cells hold a formula
//! that reads the cells they came from, and the reference is anchored,
//! so it does not move.
//!
//! And a paste may be worked out in an **operation**: the number of the
//! clipboard is put together with the number that is there — added,
//! subtracted, multiplied or divided. A cell that holds a text or an
//! error keeps what it holds, for a number cannot be put together with
//! a text, and a division by zero is the tree's own error.

use std::collections::BTreeMap;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Text};

use crate::formula::{CellRef, Cells, MAX_FORMULA_COLUMNS, MAX_FORMULA_ROWS};

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

/// The **operations** a paste may be worked out in: the name that is
/// given and the mark it is written with.
const OPERATIONS: [(&str, &str); 4] = [("add", "+"), ("subtract", "-"), ("multiply", "*"), ("divide", "/")];

/// The **combination** of the cell that is there and the cell that is
/// pasted: what is written over it.
#[derive(Debug, Clone, PartialEq)]
enum Combination {
    /// The cell that is there **keeps** what it holds.
    Keep,
    /// The cell holds a **value**.
    Value(crate::formula::Value),
    /// The cell holds a **formula**.
    Formula(String),
}

/// The **combination** of a cell and a number, worked out in the
/// operation that is named.
///
/// The rules:
///
/// - the cell that is there and the number must both be **numbers**:
///   a text is not a number, and a number cannot be put together with a
///   text;
/// - a cell that holds **nothing** is given the number: there is
///   nothing to put the number together with, and nothing is not a
///   zero;
/// - a cell that holds a **formula** takes the operation into the
///   formula: the formula and the number, with the operation between
///   them;
/// - a division by **zero** is an error, and the error is the tree's
///   own: `#DIV/0!`.
fn combine(operation: &str, current: &crate::formula::Cell, source: &crate::formula::Value) -> Combination {
    let crate::formula::Value::Number(number) = source else {
        return Combination::Keep;
    };
    let number = *number;
    let text = crate::formula::display_text(&crate::formula::Value::Number(number));
    if let Some(formula) = &current.formula {
        let Some((_, symbol)) = OPERATIONS.iter().find(|(name, _)| *name == operation) else {
            return Combination::Keep;
        };
        let combined = format!("({formula}){symbol}{text}");
        // A formula the tree cannot read is not written: the cell keeps
        // what it holds.
        if crate::formula::parse(&combined).is_err() {
            return Combination::Keep;
        }
        return Combination::Formula(combined);
    }
    match &current.value {
        Some(crate::formula::Value::Number(old)) => {
            let old = *old;
            let value = match operation {
                "add" => crate::formula::Value::Number(old + number),
                "subtract" => crate::formula::Value::Number(old - number),
                "multiply" => crate::formula::Value::Number(old * number),
                "divide" => {
                    if number == 0.0 {
                        crate::formula::Value::Error("#DIV/0!".to_string())
                    } else {
                        crate::formula::Value::Number(old / number)
                    }
                }
                _ => return Combination::Keep,
            };
            Combination::Value(value)
        }
        // The cell holds **nothing**: there is nothing to put the number
        // together with.
        None => Combination::Value(crate::formula::Value::Number(number)),
        // A **text** or an **error** stays as it is.
        _ => Combination::Keep,
    }
}

/// The **sheet** the cells of the clipboard were copied from.
fn clipboard_sheet(node: &Node) -> Option<u64> {
    node.data.get("clipboard")?.get("sheet")?.as_u64()
}

/// The **facts of the cells** that travel with them: the borders, the
/// comments, the notes and the validation.
///
/// They are facts of the sheet and not of the cell — a border is none
/// of the four things a cell is — so the clipboard keeps them as
/// themselves, and the paste writes them.
struct Carried {
    /// The borders, by the address of the cell they are on.
    borders: BTreeMap<String, BTreeMap<String, crate::borders::Border>>,
    /// The comments, by the address of the cell they are on.
    comments: BTreeMap<String, crate::comments::Comment>,
    /// The notes, by the address of the cell they are on.
    notes: BTreeMap<String, crate::notes::Note>,
    /// The links, by the address of the cell they are on.
    links: BTreeMap<String, crate::links::Link>,
    /// The **validation** of the cells: the rules that lie inside the
    /// range.
    validations: Vec<craftkit_sdk::serde_json::Value>,
}

impl Carried {
    /// The facts of the cells of a range of a sheet.
    fn of(node: &Node, area: &crate::formula::Area) -> Carried {
        let borders = crate::borders::borders_of(node);
        let comments = crate::comments::comments_of(node);
        let notes = crate::notes::notes_of(node);
        let links = crate::links::links_of(node);
        // The **validation**: the rules that lie inside the range. A rule
        // that reaches out of the range does not travel — half a rule is
        // not a rule.
        let mut validations: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
        for rule in crate::validation::rules_of(node) {
            let Some(rule_area) = crate::validation::area_of(&rule) else { continue };
            if rule_area.start.row >= area.start.row
                && rule_area.end.row <= area.end.row
                && rule_area.start.col >= area.start.col
                && rule_area.end.col <= area.end.col
            {
                validations.push(rule.to_json());
            }
        }
        Carried { borders, comments, notes, links, validations }
    }
}

/// Put the **cells** into the clipboard of a workbook.
///
/// The cells are stored with the addresses they have, and the place
/// they were copied from — so that a paste knows how far they have
/// travelled.
fn copy_into(node: &mut Node, sheet: u64, cells: &Cells, first: CellRef, carried: &Carried) {
    let mut copied = craftkit_sdk::serde_json::Map::new();
    for (cell, entry) in cells.iter() {
        copied.insert(cell.to_text(), crate::cell_into(entry));
    }
    // The **borders** of the copied cells: a border is part of the look
    // of a cell, and the look is pasted with it.
    let mut held = BTreeMap::new();
    for (address, edges) in &carried.borders {
        if copied.contains_key(address) {
            held.insert(address.clone(), edges.clone());
        }
    }
    // The **comments** and the **notes** of the copied cells: a comment
    // travels with the cell it is on.
    let mut held_comments = BTreeMap::new();
    for (address, comment) in &carried.comments {
        if copied.contains_key(address) {
            held_comments.insert(address.clone(), comment.clone());
        }
    }
    let mut held_notes = BTreeMap::new();
    for (address, note) in &carried.notes {
        if copied.contains_key(address) {
            held_notes.insert(address.clone(), note.clone());
        }
    }
    // The **validation** of the copied cells: the rules that lie inside
    // the range that was copied.
    let mut held_links = BTreeMap::new();
    for (address, link) in &carried.links {
        if copied.contains_key(address) {
            held_links.insert(address.clone(), link.clone());
        }
    }
    let validations: Vec<craftkit_sdk::serde_json::Value> = carried.validations.to_vec();
    node.data["clipboard"] = json!({
        "sheet": sheet,
        "at": first.to_text(),
        "cells": Json::Object(copied),
        "borders": crate::borders::borders_into(&held),
        "comments": crate::comments::comments_into(&held_comments),
        "notes": crate::notes::notes_into(&held_notes),
        "links": crate::links::links_into(&held_links),
        "validations": Json::Array(validations),
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
            let carried = Carried::of(node, &area);
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                copy_into(node, sheet, &copied, area.start, &carried);
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
            let carried = Carried::of(node, &area);
            if let Some(node) = session.document_mut()?.node_mut(workbook) {
                copy_into(node, sheet, &copied, area.start, &carried);
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
            /// The **operation** the paste is worked out in: `add`,
            /// `subtract`, `multiply` or `divide`.
            ///
            /// The number of the clipboard is put together with the number
            /// that is there.
            operation: Option<Text>,
            /// Whether the cells hold a **formula that reads** the cells they
            /// came from.
            ///
            /// The reference is anchored, so it does not move.
            link: Option<Bool>,
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
            // The **operation** the paste is worked out in.
            let operation = match &self.operation {
                None => None,
                Some(text) => {
                    let given = text.0.trim().to_lowercase();
                    match OPERATIONS.iter().find(|(name, _)| *name == given) {
                        Some((name, _)) => Some(*name),
                        None => {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!(
                                    "`{given}` is not an operation: an operation is one of {}",
                                    OPERATIONS.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ")
                                ),
                            });
                        }
                    }
                }
            };
            // A paste of **values** and a paste of **formats** are two
            // different pastes, and a paste is one of them.
            if formats && values {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "paste the values or the formats, and not both".to_string(),
                });
            }
            // A **link** is a formula, and a formula is not a value and not
            // a look.
            let link = matches!(self.link, Some(Bool(true)));
            if link && (values || formats || operation.is_some()) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a link is a formula, and it may not be pasted with the values, the formats \
                              or an operation"
                        .to_string(),
                });
            }
            let source = if values || operation.is_some() || link {
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
            let book_of_values =
                if values || operation.is_some() { Some(crate::book_of(session.document())) } else { None };
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
                if row < 0 || col < 0 || row as u64 >= MAX_FORMULA_ROWS || col as u64 >= MAX_FORMULA_COLUMNS {
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
                if link {
                    // The **formula** that reads the cell it was copied
                    // from: the reference is anchored, so it does not move.
                    let anchor = crate::formula::Anchor { cell, row_abs: true, col_abs: true };
                    let mut entry = entry.clone();
                    entry.formula = Some(format!("{source}!{}", anchor.to_text()));
                    entry.value = None;
                    pasted.push((cell, (row as u32, col as u32), entry));
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
            for (cell, (row, col), entry) in &pasted {
                let landing = CellRef { col: *col, row: *row };
                if let Some(operation) = operation {
                    // The **number** of the cell that is pasted: the value
                    // of the cell it was copied from, computed in the sheet
                    // it was copied from.
                    let source_value = book_of_values
                        .as_ref()
                        .map(|book| book.value_of(&source, *cell))
                        .unwrap_or(crate::formula::Value::Empty);
                    let current = written.get(landing).cloned().unwrap_or_default();
                    let mut entry = entry.clone();
                    match combine(operation, &current, &source_value) {
                        Combination::Keep => continue,
                        Combination::Value(value) => {
                            entry.value = Some(value);
                            entry.formula = None;
                        }
                        Combination::Formula(formula) => {
                            entry.formula = Some(formula);
                            entry.value = None;
                        }
                    }
                    written.set_entry(landing, entry);
                    continue;
                }
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
                // The **links** of the copied cells are the links of the
                // cells they land on: a link belongs to the cell it is
                // on.
                let links = book
                    .data
                    .get("clipboard")
                    .and_then(|clipboard| clipboard.get("links"))
                    .and_then(|links| links.as_object())
                    .map(|links| {
                        links
                            .iter()
                            .filter_map(|(address, link)| {
                                crate::links::link_from(link).map(|link| (address.to_string(), link))
                            })
                            .collect::<BTreeMap<String, crate::links::Link>>()
                    })
                    .unwrap_or_default();
                if let Some(node) = session.document_mut()?.node_mut(sheet) {
                    let mut sheet_links = crate::links::links_of(node);
                    for (cell, (row, col), _) in &pasted {
                        let address = CellRef { col: *col, row: *row }.to_text();
                        let source = cell.to_text();
                        match links.get(&source) {
                            Some(link) => {
                                sheet_links.insert(address, link.clone());
                            }
                            None => {
                                sheet_links.remove(&address);
                            }
                        }
                    }
                    node.data["links"] = crate::links::links_into(&sheet_links);
                }
            }
            // The **validation** that travelled and the rules that were
            // left behind.
            let mut moved_count = 0usize;
            let mut left_behind_count = 0usize;
            // The **validation** of the copied cells is the validation of
            // the cells they land on: the rules move with the cells, and
            // the rules that were there are taken away. A rule that
            // would land off the sheet is not written, and it is
            // counted.
            if !values && !formats && !transpose {
                let stored = book
                    .data
                    .get("clipboard")
                    .and_then(|clipboard| clipboard.get("validations"))
                    .and_then(|validations| validations.as_array())
                    .cloned()
                    .unwrap_or_default();
                let block_row = at.row as i64 - from.row as i64;
                let block_col = at.col as i64 - from.col as i64;
                let mut moved: Vec<crate::validation::Rule> = Vec::new();
                let mut left_behind = 0usize;
                for value in &stored {
                    let Some(mut rule) = crate::validation::Rule::from_json(value) else { continue };
                    let Some(rule_area) = crate::validation::area_of(&rule) else { continue };
                    let start = CellRef {
                        col: (rule_area.start.col as i64 + block_col) as u32,
                        row: (rule_area.start.row as i64 + block_row) as u32,
                    };
                    let end = CellRef {
                        col: (rule_area.end.col as i64 + block_col) as u32,
                        row: (rule_area.end.row as i64 + block_row) as u32,
                    };
                    if start.row as u64 >= MAX_FORMULA_ROWS
                        || end.row as u64 >= MAX_FORMULA_ROWS
                        || start.col as u64 >= MAX_FORMULA_COLUMNS
                        || end.col as u64 >= MAX_FORMULA_COLUMNS
                    {
                        left_behind += 1;
                        continue;
                    }
                    rule.range = crate::formula::Area::new(start, end).to_text();
                    moved.push(rule);
                }
                moved_count = moved.len();
                left_behind_count = left_behind;
                // The **landing**: the block the cells come to.
                let landing = pasted.iter().fold(
                    None::<crate::formula::Area>,
                    |held, (_, (row, col), _)| {
                        let cell = CellRef { col: *col, row: *row };
                        Some(match held {
                            None => crate::formula::Area::new(cell, cell),
                            Some(area) => crate::formula::Area::new(
                                crate::formula::CellRef {
                                    col: area.start.col.min(cell.col),
                                    row: area.start.row.min(cell.row),
                                },
                                crate::formula::CellRef {
                                    col: area.end.col.max(cell.col),
                                    row: area.end.row.max(cell.row),
                                },
                            ),
                        })
                    },
                );
                if let Some(node) = session.document_mut()?.node_mut(sheet) {
                    let mut rules = crate::validation::rules_of(node);
                    if let Some(landing) = landing {
                        // The rules that were there are taken away: the
                        // validation of the pasted cells is the validation
                        // of the copied cells.
                        rules.retain(|rule| {
                            let Some(rule_area) = crate::validation::area_of(rule) else {
                                return true;
                            };
                            let over = rule_area.start.row <= landing.end.row
                                && rule_area.end.row >= landing.start.row
                                && rule_area.start.col <= landing.end.col
                                && rule_area.end.col >= landing.start.col;
                            !over
                        });
                    }
                    for rule in moved {
                        rules.push(rule);
                    }
                    node.data["validations"] =
                        craftkit_sdk::serde_json::Value::Array(rules.iter().map(|rule| rule.to_json()).collect());
                }
            }
            // A **change of a header** follows the formulas that name it:
            // a paste may write over the header of a table, and the
            // field it names is a field with a new name.
            let followed = crate::tables::follow_headers(session, sheet, &cells, &written);
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
                    "followed": followed,
                    "operation": match operation {
                        Some(operation) => json!(operation),
                        None => Json::Null,
                    },
                    "validations": moved_count,
                    "validationsLeftBehind": left_behind_count,
                    "link": link,
                })))

        }
    }
}
