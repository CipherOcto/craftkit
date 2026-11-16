//! The **sum** of a range of numbers, written in a cell.
//!
//! A column of numbers has a sum under it, and a row of numbers has a
//! sum beside it, and the tree's **sum** writes that sum down as a
//! formula: the formula is in the cell, and the value of it is
//! computed from the sheet as it is.
//!
//! The rules:
//!
//! - the **range** is the numbers that are summed, and it may be
//!   named; without it the range is the **run of numbers above** the
//!   cell, and when there is none the run of numbers **beside** it;
//! - a run of numbers ends where a cell that holds no number begins,
//!   and a cell that holds a **formula** counts when the formula gives
//!   a number: the value of a formula cell is computed, and it is
//!   computed here;
//! - a cell with **no numbers** above it and none beside it is
//!   refused: there is nothing to sum, and a command that quietly does
//!   nothing is a command that answers a question nobody asked;
//! - the **way** of the sum is `sum`, `average`, `count`, `max` or
//!   `min`, and it is `sum` without it; a way that is not one of the
//!   tree's is refused by name;
//! - the sum is written as a **formula**, and the rule of a formula is
//!   the rule of the tree's formula command: a formula that reads
//!   itself is refused.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{Area, Book, CellRef, Value};

/// The **ways** a sum may be summed in: the name that is given and the
/// name of the function.
const WAYS: [(&str, &str); 5] =
    [("sum", "SUM"), ("average", "AVERAGE"), ("count", "COUNT"), ("max", "MAX"), ("min", "MIN")];

/// Whether a cell holds a **number**: a number of its own, or a
/// formula that gives one.
fn is_number(book: &Book, sheet: &str, cell: CellRef) -> bool {
    matches!(book.value_of(sheet, cell), Value::Number(_))
}

/// The **run of numbers above** a cell: the numbers that reach it from
/// above, without a gap.
fn above(book: &Book, sheet: &str, cell: CellRef) -> Option<Area> {
    if cell.row == 0 {
        return None;
    }
    let last = cell.row - 1;
    if !is_number(book, sheet, CellRef { col: cell.col, row: last }) {
        return None;
    }
    let mut first = last;
    while first > 0 {
        let higher = CellRef { col: cell.col, row: first - 1 };
        if !is_number(book, sheet, higher) {
            break;
        }
        first -= 1;
    }
    Some(Area::new(CellRef { col: cell.col, row: first }, CellRef { col: cell.col, row: last }))
}

/// The **run of numbers beside** a cell: the numbers that reach it from
/// the left, without a gap.
fn beside(book: &Book, sheet: &str, cell: CellRef) -> Option<Area> {
    if cell.col == 0 {
        return None;
    }
    let last = cell.col - 1;
    if !is_number(book, sheet, CellRef { col: last, row: cell.row }) {
        return None;
    }
    let mut first = last;
    while first > 0 {
        let nearer = CellRef { col: first - 1, row: cell.row };
        if !is_number(book, sheet, nearer) {
            break;
        }
        first -= 1;
    }
    Some(Area::new(CellRef { col: first, row: cell.row }, CellRef { col: last, row: cell.row }))
}

commands! {
    /// **Sum** a range of numbers into a cell.
    ///
    /// A change: it writes a formula in the cell, and the value of the
    /// formula is computed from the sheet as it is.
    grid.formula.sum {
        area: "grid",
        layer: "grid",
        icon: "calculator",
        mutates,
        undoable,
        params: {
            /// The sheet it goes in.
            sheet: NodeRef,
            /// The cell's address.
            cell: Text,
            /// The **range** of numbers that are summed.
            ///
            /// Without it, the run of numbers above the cell is summed,
            /// and when there is none the run of numbers beside it.
            range: Option<Text>,
            /// The **way** the numbers are summed in: `sum`, `average`,
            /// `count`, `max` or `min`.
            ///
            /// It is `sum` without it.
            function: Option<Text>,
        },
        run: {
            let id = "grid.formula.sum".to_string();
            let sheet = self.sheet.0;
            let cell = crate::cell_at(&self.cell.0, &id)?;
            // The **way** of the sum.
            let given = self.function.as_ref().map(|way| way.0.trim().to_lowercase());
            let function = match &given {
                None => "SUM",
                Some(way) => match WAYS.iter().find(|(name, _)| name == way) {
                    Some((_, function)) => *function,
                    None => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "`{way}` is not a way of summing: it is one of {}",
                                WAYS.iter().map(|(name, _)| *name).collect::<Vec<_>>().join(", ")
                            ),
                        });
                    }
                },
            };
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
            let name = node.name.clone();
            // The **range** that is summed.
            let area = match &self.range {
                Some(text) => crate::fill::area_at(&text.0, &id)?,
                None => {
                    let book = crate::book_of(session.document());
                    match above(&book, &name, cell) {
                        Some(area) => area,
                        None => match beside(&book, &name, cell) {
                            Some(area) => area,
                            None => {
                                return Err(EngineError::Failed {
                                    id: id.clone(),
                                    message: format!(
                                        "there are no numbers above or beside the cell {}",
                                        cell.to_text()
                                    ),
                                });
                            }
                        },
                    }
                }
            };
            // The **formula** that is written.
            let text = format!("{function}({})", area.to_text());
            let outcome = crate::formula::set_formula(session, sheet, cell, &text, &id)?;
            let mut data = outcome.data.clone();
            if let Some(object) = data.as_object_mut() {
                object.insert("range".to_string(), json!(area.to_text()));
                object.insert("function".to_string(), json!(function));
            }
            Ok(Outcome { data, ..outcome })

        }
    }
}
