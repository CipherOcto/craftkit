//! The **region** of a cell.
//!
//! A table of data is a **region**: a block of cells that hold
//! something, and the empty cells around it are not part of it. The
//! tree's **region** finds the block a cell is in, so that a program
//! can ask "what is around this cell" and get an answer that is a
//! range and not a guess.
//!
//! The rules:
//!
//! - a cell is **filled** when it holds a value or a formula, and an
//!   empty cell is not part of any region;
//! - the region **grows** while the line next to it holds something:
//!   the row above and the row below, and then the column to the left
//!   and the column to the right, and so on until nothing more grows;
//! - the line next to the region is read **with its diagonals**: a row
//!   that holds something one column further out still grows the
//!   region, and that is what makes a region a block and not a cross;
//! - a cell that holds nothing is a region of **one cell**: itself;
//! - the region never leaves the **sheet**: a sheet has so many rows
//!   and so many columns, and an address that is not in the sheet is
//!   not an address.

use std::collections::BTreeMap;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{Area, CellRef, MAX_COLS, MAX_FORMULA_ROWS};

/// The **filled cells** of a sheet: by row and by column, so that the
/// lines around a region are read without walking the sheet.
struct Filled {
    /// The columns of each row that holds something, in order.
    rows: BTreeMap<u32, Vec<u32>>,
    /// The rows of each column that holds something, in order.
    columns: BTreeMap<u32, Vec<u32>>,
}

impl Filled {
    /// The filled cells of a sheet.
    fn of(cells: &crate::formula::Cells) -> Filled {
        let mut rows: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        let mut columns: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
        for (cell, entry) in cells.iter() {
            if entry.value.is_none() && entry.formula.is_none() {
                continue;
            }
            rows.entry(cell.row).or_default().push(cell.col);
            columns.entry(cell.col).or_default().push(cell.row);
        }
        for line in rows.values_mut() {
            line.sort_unstable();
        }
        for line in columns.values_mut() {
            line.sort_unstable();
        }
        Filled { rows, columns }
    }

    /// Whether the **line** holds something between two places.
    ///
    /// The line is a sorted list of the places that hold something, and
    /// the answer is whether one of them is between the two ends.
    fn holds_between(line: Option<&Vec<u32>>, from: u64, to: u64) -> bool {
        let Some(line) = line else { return false };
        let first = line.partition_point(|place| (*place as u64) < from);
        match line.get(first) {
            Some(place) => (*place as u64) <= to,
            None => false,
        }
    }

    /// Whether the **row** holds something between two columns.
    fn row_holds(&self, row: u32, from: u64, to: u64) -> bool {
        Filled::holds_between(self.rows.get(&row), from, to)
    }

    /// Whether the **column** holds something between two rows.
    fn column_holds(&self, column: u32, from: u64, to: u64) -> bool {
        Filled::holds_between(self.columns.get(&column), from, to)
    }
}

/// The **region** a cell is in: the block of filled cells around it.
fn region(filled: &Filled, cell: CellRef) -> Area {
    let (mut top, mut bottom, mut left, mut right) = (cell.row, cell.row, cell.col, cell.col);
    loop {
        let before = (top, bottom, left, right);
        // The **rows** above and below, read with the diagonals.
        let (from_column, to_column) = (left.saturating_sub(1) as u64, (right as u64 + 1).min(MAX_COLS - 1));
        while top > 0 && filled.row_holds(top - 1, from_column, to_column) {
            top -= 1;
        }
        while (bottom as u64 + 1) < MAX_FORMULA_ROWS && filled.row_holds(bottom + 1, from_column, to_column) {
            bottom += 1;
        }
        // The **columns** to the left and to the right, read with the
        // diagonals.
        let (from_row, to_row) =
            (top.saturating_sub(1) as u64, (bottom as u64 + 1).min(MAX_FORMULA_ROWS - 1));
        while left > 0 && filled.column_holds(left - 1, from_row, to_row) {
            left -= 1;
        }
        while (right as u64 + 1) < MAX_COLS && filled.column_holds(right + 1, from_row, to_row) {
            right += 1;
        }
        if before == (top, bottom, left, right) {
            break;
        }
    }
    Area::new(CellRef { col: left, row: top }, CellRef { col: right, row: bottom })
}

commands! {
    /// The **region** of a cell: the block of filled cells it is in.
    ///
    /// It reads and does not write: the region is **derived** from the
    /// cells that hold something, and it is not stored.
    grid.cell.region {
        area: "grid",
        layer: "grid",
        icon: "grid",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
        },
        run: {
            let id = "grid.cell.region".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
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
            let cells = crate::cells_of(node);
            let filled = Filled::of(&cells);
            let area = region(&filled, cell);
            let count = area.count();
            Ok(Outcome::new(format!("the region of the cell {} is {}", cell.to_text(), area.to_text()))
                .with_data(json!({
                    "sheet": sheet,
                    "cell": cell.to_text(),
                    "region": area.to_text(),
                    "start": area.start.to_text(),
                    "end": area.end.to_text(),
                    "cells": count,
                })))

        }
    }
}
