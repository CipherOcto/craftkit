//! **Reading** a range of cells.
//!
//! A range of cells may be **read**: the answer is the **values** of
//! the cells, row by row, and a cell that holds a formula has the
//! value its formula computes — the value is **derived**, so it is
//! computed every time the range is read and it cannot be wrong.
//!
//! The tree's *read range* reads a range of a sheet and its default
//! is the **used range** — the stretch of the sheet that holds
//! something. So does this one.
//!
//! # What the port found
//!
//! - **The tree's bound is a million cells and this tree's bound is
//!   the sheet.** The tree refuses to read a range of more than a
//!   million cells and says so; here the bound of the reading is the
//!   **sheet** itself — a sheet is a million rows tall and sixteen
//!   thousand columns wide, and a range may not leave the sheet — and
//!   the range is refused when it is not a range of cells.
//! - **The tree's read may show the formulas and here it may too.** The
//!   tree's read takes a `formulas` flag and gives the formulas
//!   instead of the values, and it takes a `formatted` flag and gives
//!   the text the cells show. Here the formulas may be asked for, and
//!   the text is what `grid.cell.text` is for — one cell at a time,
//!   with the format and the conditional format and the date system of
//!   the workbook in it.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Cells, Value};

/// The **used range** of a sheet: the stretch of it that holds
/// something.
///
/// A sheet with nothing in it has no used range, and that is the
/// tree's own: there is nothing to read.
pub(crate) fn used_range(cells: &Cells) -> Option<Area> {
    let mut start: Option<CellRef> = None;
    let mut end: Option<CellRef> = None;
    for (cell, _) in cells.iter() {
        start = Some(match start {
            Some(first) => CellRef { col: first.col.min(cell.col), row: first.row.min(cell.row) },
            None => cell,
        });
        end = Some(match end {
            Some(last) => CellRef { col: last.col.max(cell.col), row: last.row.max(cell.row) },
            None => cell,
        });
    }
    match (start, end) {
        (Some(start), Some(end)) => Some(Area::new(start, end)),
        _ => None,
    }
}

/// **Read** a range of cells.
///
/// The answer is the values of the cells, row by row. Without a range
/// the **used range** is read.
#[derive(Command)]
#[command(id = "grid.range.read", area = "grid", layer = "grid", icon = "list")]
pub struct ReadRange {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The range: `A1:D10`. Without it, the used range is read.
    pub range: Option<Text>,
    /// Whether the **formulas** are given instead of the values.
    pub formulas: Option<Bool>,
}

impl ReadRange {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.range.read".to_string();
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
        let name = node.name.clone();
        let cells = crate::cells_of(node);
        let area = match &self.range {
            Some(range) => crate::fill::area_at(&range.0, &id)?,
            None => match used_range(&cells) {
                Some(area) => area,
                None => {
                    return Ok(Outcome::new(format!("{name} holds nothing"))
                        .with_data(json!({ "sheet": sheet, "range": Value::Empty.to_json(), "values": Vec::<Json>::new() })));
                }
            },
        };
        // The **values** are derived: the book computes them.
        let book = crate::book_of(session.document());
        let formulas = self.formulas.map(|formulas| formulas.0).unwrap_or(false);
        let mut rows: Vec<Vec<Json>> = Vec::new();
        for row in area.start.row..=area.end.row {
            let mut cells_in_row: Vec<Json> = Vec::new();
            for col in area.start.col..=area.end.col {
                let cell = CellRef { col, row };
                let value = match cells.get(cell) {
                    Some(entry) => match (&entry.value, &entry.formula) {
                        (Some(value), _) => value.clone(),
                        (None, Some(_)) => book.value_of(&name, cell),
                        (None, None) => Value::Empty,
                    },
                    None => Value::Empty,
                };
                if formulas {
                    let formula = cells.get(cell).and_then(|entry| entry.formula.clone());
                    cells_in_row.push(match formula {
                        Some(formula) => Json::String(formula),
                        None => value.to_json(),
                    });
                } else {
                    cells_in_row.push(value.to_json());
                }
            }
            rows.push(cells_in_row);
        }
        let count = area.count();
        Ok(Outcome::new(format!("{name}!{} holds {count} cell(s)", area.to_text()))
            .with_data(json!({ "sheet": sheet, "range": area.to_text(), "values": rows })))
    }
}
