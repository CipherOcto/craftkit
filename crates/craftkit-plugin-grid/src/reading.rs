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

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Text};

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

commands! {
    /// **Read** a range of cells.
    ///
    /// The answer is the values of the cells, row by row. Without a range
    /// the **used range** is read.
    grid.range.read {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range: `A1:D10`. Without it, the used range is read.
            range: Option<Text>,
            /// Whether the **formulas** are given instead of the values.
            formulas: Option<Bool>,
            /// Whether the cells are given as they are **shown**: the
            /// text the cell shows, written with the format of the cell.
            formatted: Option<Bool>,
        },
        run: {
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
            let formatted = self.formatted.map(|formatted| formatted.0).unwrap_or(false);
            if formulas && formatted {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a range is read for its formulas or for the text it shows, and not for both"
                        .to_string(),
                });
            }
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
                    } else if formatted {
                        // The **text** the cell shows: the value written
                        // with the format of the cell — the conditional
                        // format that matches it first, and the format
                        // of the style it names otherwise.
                        let looks = crate::conditional::looks_for(
                                &crate::book_of(session.document()),
                                node,
                                cell,
                                &value,
                                crate::system_of(session, sheet),
                            );
                        let format = match &looks.format {
                            Some(code) => crate::format::Format::parse(code).unwrap_or(crate::format::Format::General),
                            None => crate::format_of(session, sheet, &cells, cell),
                        };
                        let width = crate::sizes::column_width(node, cell.col);
                        let text = format.text_in(&value, crate::system_of(session, sheet), width);
                        cells_in_row.push(Json::String(text));
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

    /// The **statistics** of a range: how many cells hold something,
    /// how many of them hold numbers, and what the numbers add up to.
    ///
    /// The values are **derived**: the book computes them, so a cell
    /// that holds a formula is counted by what it computes.
    ///
    /// A **hidden row** is not counted: a row that is hidden is a row
    /// that is not there, as far as a summary of the sheet is concerned.
    grid.range.stats {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The range: `A1:D10`. Without it, the used range is read.
            range: Option<Text>,
        },
        run: {
            let id = "grid.range.stats".to_string();
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
                        return Ok(Outcome::new(format!("{name} holds nothing")).with_data(json!({
                            "sheet": sheet,
                            "range": Value::Empty.to_json(),
                            "count": 0,
                            "numeric": 0,
                            "sum": 0.0,
                            "average": Value::Empty.to_json(),
                            "min": Value::Empty.to_json(),
                            "max": Value::Empty.to_json(),
                        })));
                    }
                },
            };
            // The **hidden rows**: they are not counted. The rows are
            // counted from one, as the tree counts them.
            let hidden = crate::hiding::hidden_rows(node);
            // The **values**: the book computes them.
            let book = crate::book_of(session.document());
            let mut count = 0u64;
            let mut numeric = 0u64;
            let mut sum = 0.0;
            let mut smallest: Option<f64> = None;
            let mut largest: Option<f64> = None;
            for row in area.start.row..=area.end.row {
                if hidden.contains(&(u64::from(row) + 1)) {
                    continue;
                }
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
                    match value {
                        Value::Empty => continue,
                        Value::Number(number) => {
                            count += 1;
                            numeric += 1;
                            sum += number;
                            smallest = Some(smallest.map_or(number, |smallest| smallest.min(number)));
                            largest = Some(largest.map_or(number, |largest| largest.max(number)));
                        }
                        _ => count += 1,
                    }
                }
            }
            let average = if numeric > 0 { Some(sum / numeric as f64) } else { None };
            Ok(Outcome::new(format!("{count} cell(s) of {name} hold something")).with_data(json!({
                "sheet": sheet,
                "range": area.to_text(),
                "count": count,
                "numeric": numeric,
                "sum": sum,
                "average": match average {
                    Some(average) => json!(average),
                    None => Value::Empty.to_json(),
                },
                "min": match smallest {
                    Some(smallest) => json!(smallest),
                    None => Value::Empty.to_json(),
                },
                "max": match largest {
                    Some(largest) => json!(largest),
                    None => Value::Empty.to_json(),
                },
            })))

        }
    }
}
