//! The **trace** of a cell: the cells it reads and the cells that read
//! it.
//!
//! The tree's own rule: the trace at the **first** level is the direct
//! one — the areas a formula reads and the cells that read it — and
//! the trace at **every** level walks the chain. The tree's
//! `Ctrl+Shift+[` and `Ctrl+Shift+]` are its own flags for it, and the
//! flag is named `all`.
//!
//! Two rules of the tree are here:
//!
//! - **A reference that names no sheet is a reference to the sheet
//!   the formula is in.** A formula in `Sheet2` that says `=A1` reads
//!   `Sheet2!A1` and not `Sheet1!A1`.
//! - **At every level, only the cells that are there are walked.** The
//!   tree walks the **stored** cells inside the area a formula reads —
//!   a blank is never one of them — and only a cell that holds a
//!   formula carries the walk on.

use crate::formula::{self, Book, CellRef, Cells};

/// The **cells** that read a cell: the **dependents** of a cell.
///
/// A formula reads a cell when one of the areas it reads covers it,
/// and a reference that names no sheet is a reference to the sheet the
/// formula is in.
pub fn dependents_of(book: &Book, sheet: &str, cell: CellRef) -> Vec<(String, CellRef)> {
    let mut found: Vec<(String, CellRef)> = Vec::new();
    for (name, cells) in book.sheets() {
        for (other, entry) in cells.iter() {
            let Some(formula) = &entry.formula else { continue };
            let Ok(expression) = formula::parse(formula) else { continue };
            for reference in formula::precedents(&expression, book.names()) {
                let read = reference.sheet.as_deref().unwrap_or(name.as_str());
                if read != sheet {
                    continue;
                }
                if reference.area.covers(cell) {
                    found.push((name.clone(), other));
                    break;
                }
            }
        }
    }
    found
}

/// The **cells** a cell reads, at **every** level.
///
/// The walk: the areas the formula reads, the **stored** cells inside
/// them, and a cell that holds a formula carries the walk on. The cell
/// the walk starts at is not one of its own precedents.
pub fn precedent_cells(book: &Book, sheet: &str, start: CellRef) -> Vec<(String, CellRef)> {
    let mut found: Vec<(String, CellRef)> = Vec::new();
    let mut seen: Vec<(String, CellRef)> = vec![(sheet.to_string(), start)];
    let mut frontier: Vec<(String, CellRef)> = vec![(sheet.to_string(), start)];
    while let Some((name, cell)) = frontier.pop() {
        let Some(cells) = book.sheet(&name) else { continue };
        let Some(entry) = cells.get(cell) else { continue };
        let Some(formula) = &entry.formula else { continue };
        let Ok(expression) = formula::parse(formula) else { continue };
        for reference in formula::precedents(&expression, book.names()) {
            let read = reference.sheet.as_deref().unwrap_or(name.as_str());
            let Some(area) = book.sheet(read) else { continue };
            for (at, entry) in stored_cells(area, &reference.area) {
                if seen.contains(&(read.to_string(), at)) {
                    continue;
                }
                seen.push((read.to_string(), at));
                found.push((read.to_string(), at));
                if entry.formula.is_some() {
                    frontier.push((read.to_string(), at));
                }
            }
        }
    }
    // The cell the walk starts at is not one of its own precedents.
    found.retain(|(name, cell)| !(name == sheet && *cell == start));
    found
}

/// The **stored** cells of a sheet that an area covers.
///
/// A cell that is not there is not read: the tree walks the cells that
/// are in the sheet, and a blank is not one of them.
fn stored_cells<'a>(
    cells: &'a Cells,
    area: &crate::formula::Area,
) -> Vec<(CellRef, &'a crate::formula::Cell)> {
    cells.iter().filter(|(at, entry)| area.covers(*at) && !blank(entry)).collect()
}

/// Whether a cell is **blank**: it holds no value and no formula.
fn blank(entry: &crate::formula::Cell) -> bool {
    entry.formula.is_none() && matches!(entry.value, None | Some(crate::formula::Value::Empty))
}

/// The **dependents** of a cell, at **every** level.
///
/// The walk: the cells that read the cell, and every cell that reads
/// one of those. The cell the walk starts at is not one of its own
/// dependents.
pub fn dependent_cells(book: &Book, sheet: &str, start: CellRef) -> Vec<(String, CellRef)> {
    let mut found: Vec<(String, CellRef)> = Vec::new();
    let mut seen: Vec<(String, CellRef)> = vec![(sheet.to_string(), start)];
    let mut frontier: Vec<(String, CellRef)> = vec![(sheet.to_string(), start)];
    while let Some((name, cell)) = frontier.pop() {
        for (other_name, other) in dependents_of(book, &name, cell) {
            if seen.contains(&(other_name.clone(), other)) {
                continue;
            }
            seen.push((other_name.clone(), other));
            found.push((other_name.clone(), other));
            frontier.push((other_name, other));
        }
    }
    found
}
