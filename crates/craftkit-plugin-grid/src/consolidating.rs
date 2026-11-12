//! The **consolidation** of several ranges of cells into one.
//!
//! The tree's own command: the ranges are added up into one table, and
//! the rows and the columns of it are the rows and the columns of the
//! sources.
//!
//! # The rules
//!
//! - The **sources** are ranges of cells, and they may be on different
//!   sheets. The tree's own rule: a source is a range with the name of
//!   its sheet, and here a source is a range and the sheet it is in.
//! - The **way** the values are added up is one of the tree's own: `sum`
//!   — the default — `count`, `countNums`, `average`, `max`, `min` or
//!   `product`. A way that is not one is refused by name.
//! - The **rows** of the result are the rows of the sources, and the
//!   **columns** of it are the columns of them: a source without a
//!   header row has a row a position, and a source with one has a row
//!   a **label** — the text of the header, matched without regard to
//!   its case.
//! - The **labels** of the rows and of the columns go in the first
//!   column and in the first row of the result, and the **values** go
//!   in the body of it.
//! - The result goes **below and to the right** of the cell it is
//!   given, and a result that does not fit is refused.
//!
//! # What it does not do
//!
//! - **The styles of the cells of the result are not the styles of the
//!   source cells**: the tree keeps the style of the cell that is
//!   written over, and the styles of a cell are another thing here.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Text};

use crate::formula::{CellRef, Cells, Value};

/// The most **cells** the sources may hold.
const MAX_CELLS: u64 = 5_000_000;

/// The most **sources** there may be.
const MAX_SOURCES: usize = 255;

/// The **ways** the values may be added up in: the tree's own list.
const WAYS: [&str; 7] = ["sum", "count", "countnums", "average", "max", "min", "product"];

/// A **key** of a row or of a column: a position, or a label matched
/// without regard to its case.
#[derive(Clone, PartialEq, Eq)]
enum Key {
    /// A position in the source.
    Pos(u32),
    /// A label of the source.
    Label(String),
}

/// The **values** of one cell of the result, and the way they are added
/// up in.
#[derive(Clone, Copy, Default)]
struct Agg {
    /// How many cells held something.
    count: u64,
    /// How many of them held a number.
    numbers: u64,
    /// The sum of the numbers.
    sum: f64,
    /// The product of the numbers.
    product: f64,
    /// The least of the numbers.
    min: f64,
    /// The greatest of the numbers.
    max: f64,
}

impl Agg {
    /// Add a **value** to the aggregation.
    fn add(&mut self, value: &Value) {
        if matches!(value, Value::Empty) {
            return;
        }
        self.count += 1;
        if let Value::Number(number) = value {
            if self.numbers == 0 {
                self.min = *number;
                self.max = *number;
                self.product = 1.0;
            }
            self.numbers += 1;
            self.sum += number;
            self.product *= number;
            self.min = self.min.min(*number);
            self.max = self.max.max(*number);
        }
    }

    /// The **value** of the aggregation, in the way that was asked for.
    fn result(&self, way: &str) -> Value {
        match way {
            "count" => Value::Number(self.count as f64),
            "countnums" => Value::Number(self.numbers as f64),
            "average" => {
                if self.numbers == 0 {
                    Value::Error("#DIV/0!".to_string())
                } else {
                    Value::Number(self.sum / self.numbers as f64)
                }
            }
            "max" => Value::Number(if self.numbers == 0 { 0.0 } else { self.max }),
            "min" => Value::Number(if self.numbers == 0 { 0.0 } else { self.min }),
            "product" => Value::Number(if self.numbers == 0 { 0.0 } else { self.product }),
            _ => Value::Number(self.sum),
        }
    }
}

/// The **range** of cells a name says.
fn area_of(range: &str) -> Option<crate::formula::Area> {
    let trimmed = range.trim();
    let area = match trimmed.split_once(':') {
        Some((first, second)) => {
            let first = crate::formula::Anchor::parse(first)?;
            let second = crate::formula::Anchor::parse(second)?;
            crate::formula::Area::new(first.cell, second.cell)
        }
        None => {
            let only = crate::formula::Anchor::parse(trimmed)?;
            crate::formula::Area::new(only.cell, only.cell)
        }
    };
    Some(area)
}

/// The **text** of a value, as a cell shows it.
fn display_of(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(number) => {
            if number.fract() == 0.0 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => {
            if *flag {
                "TRUE".to_string()
            } else {
                "FALSE".to_string()
            }
        }
        Value::Error(message) => message.clone(),
    }
}

/// The **cells** of a sheet.
fn cells_of(node: &Node) -> Cells {
    crate::cells_of(node)
}

/// The **index** of a key, and the key itself when it is new.
fn index_of(keys: &mut Vec<Key>, key: Key) -> usize {
    match keys.iter().position(|known| *known == key) {
        Some(index) => index,
        None => {
            keys.push(key);
            keys.len() - 1
        }
    }
}

commands! {
    /// **Consolidate** the ranges of cells into one table.
    ///
    /// The tree's own command.
    grid.data.consolidate {
        area: "grid",
        layer: "grid",
        icon: "list",
        mutates,
        undoable,
        params: {
            /// The **sources**: a list of `{"sheet": id, "range": "A1:C3"}`.

            sources: Json,
            /// The **sheet** the result goes in.

            sheet: NodeRef,
            /// The **cell** the result goes at: its top left corner.

            destination: Text,
            /// The way the values are **added up**.
            ///
            /// `sum` — the default — `count`, `countNums`, `average`, `max`,
            /// `min` or `product`.

            function: Option<Text>,
            /// Whether the first row of each source is a **header**.
            ///
            /// It is not, without it being said.

            top_row: Option<Bool>,
            /// Whether the first column of each source holds the **labels** of
            /// the rows.
            ///
            /// It does not, without it being said.

            left_column: Option<Bool>,
        },
        run: {

            let id = "grid.data.consolidate".to_string();
            let way = self
                .function
                .as_ref()
                .map(|function| function.0.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "sum".to_string());
            if !WAYS.contains(&way.as_str()) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the way `{way}` is not one of this tree's: it is one of sum, count, countNums, average, max, min, product"
                    ),
                });
            }
            let top = self.top_row.as_ref().map(|flag| flag.0).unwrap_or(false);
            let left = self.left_column.as_ref().map(|flag| flag.0).unwrap_or(false);
            // The **sources**: a list of a sheet and a range.
            let Some(list) = self.sources.as_array() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the sources are a list of a sheet and a range".to_string(),
                });
            };
            if list.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "give at least one source range in `sources`".to_string(),
                });
            }
            if list.len() > MAX_SOURCES {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "there are {} sources, and the most there may be is {MAX_SOURCES}",
                        list.len()
                    ),
                });
            }
            // The **sheet** the result goes in.
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet).cloned() else {
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
            if crate::protection::Protection::of(&node).is_some_and(|protection| protection.locked) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the sheet `{sheet}` is protected, and the cells of a protected sheet may not be changed"
                    ),
                });
            }
            let Some(destination) = crate::formula::Anchor::parse(self.destination.0.trim()) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a cell", self.destination.0.trim()),
                });
            };
            // The **rows** and the **columns** of the result, and the values
            // of each of their cells.
            let mut row_keys: Vec<Key> = Vec::new();
            let mut column_keys: Vec<Key> = Vec::new();
            let mut row_labels: Vec<String> = Vec::new();
            let mut column_labels: Vec<String> = Vec::new();
            let mut values: Vec<(usize, usize, Agg)> = Vec::new();
            let mut total: u64 = 0;
            for source in list {
                let Some(source) = source.as_object() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a source is a sheet and a range".to_string(),
                    });
                };
                let Some(sheet_id) = source.get("sheet").and_then(|value| value.as_u64()) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a source names the sheet it is in".to_string(),
                    });
                };
                let Some(range) = source.get("range").and_then(|value| value.as_str()) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a source names the range of it".to_string(),
                    });
                };
                let Some(area) = area_of(range) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{range}` is not a range of cells"),
                    });
                };
                total = total.saturating_add(area.count());
                if total > MAX_CELLS {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the sources are too large".to_string(),
                    });
                }
                let Some(source_node) = session.document().node(sheet_id).cloned() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no sheet `{sheet_id}`"),
                    });
                };
                if source_node.kind != "grid.sheet" {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the node `{sheet_id}` is a `{}` and not a sheet", source_node.kind),
                    });
                }
                let cells = cells_of(&source_node);
                let first_row = area.start.row + u32::from(top);
                let first_col = area.start.col + u32::from(left);
                if first_row > area.end.row || first_col > area.end.col {
                    continue;
                }
                // The **columns** of this source: a label when there is a
                // header row, and a position when there is not.
                let mut columns: Vec<(u32, usize)> = Vec::new();
                for col in first_col..=area.end.col {
                    let key = if top {
                        let label = label_of(&cells, CellRef { row: area.start.row, col });
                        if label.is_empty() {
                            continue;
                        }
                        let key = Key::Label(label.to_lowercase());
                        let before = column_keys.len();
                        let index = index_of(&mut column_keys, key);
                        if column_keys.len() > before {
                            column_labels.push(label);
                        }
                        index
                    } else {
                        index_of(&mut column_keys, Key::Pos(col - first_col))
                    };
                    columns.push((col, key));
                }
                for row in first_row..=area.end.row {
                    let key = if left {
                        let label = label_of(&cells, CellRef { row, col: area.start.col });
                        if label.is_empty() {
                            continue;
                        }
                        let key = Key::Label(label.to_lowercase());
                        let before = row_keys.len();
                        let index = index_of(&mut row_keys, key);
                        if row_keys.len() > before {
                            row_labels.push(label);
                        }
                        index
                    } else {
                        index_of(&mut row_keys, Key::Pos(row - first_row))
                    };
                    for (col, column) in &columns {
                        let value = cells.get(CellRef { row, col: *col }).and_then(|entry| entry.value.clone());
                        match values
                            .iter_mut()
                            .find(|(known_row, known_column, _)| *known_row == key && *known_column == *column)
                        {
                            Some((_, _, agg)) => {
                                if let Some(value) = value {
                                    agg.add(&value);
                                }
                            }
                            None => {
                                let mut agg = Agg::default();
                                if let Some(value) = value {
                                    agg.add(&value);
                                }
                                values.push((key, *column, agg));
                            }
                        }
                    }
                }
            }
            // The result goes **below and to the right** of the cell it is
            // given.
            let out_row = destination.cell.row + u32::from(top);
            let out_col = destination.cell.col + u32::from(left);
            if u64::from(out_row) + row_keys.len() as u64 > crate::formula::MAX_ROWS
                || u64::from(out_col) + column_keys.len() as u64 > crate::formula::MAX_COLS
            {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the consolidated data does not fit below and right of the destination".to_string(),
                });
            }
            // The **cells** of the result.
            let mut written: Vec<(CellRef, Value)> = Vec::new();
            if top {
                for (column, label) in column_labels.iter().enumerate() {
                    written.push((
                        CellRef { row: destination.cell.row, col: out_col + column as u32 },
                        Value::Text(label.clone()),
                    ));
                }
            }
            if left {
                for (row, label) in row_labels.iter().enumerate() {
                    written.push((
                        CellRef { row: out_row + row as u32, col: destination.cell.col },
                        Value::Text(label.clone()),
                    ));
                }
            }
            for row in 0..row_keys.len() {
                for column in 0..column_keys.len() {
                    let value = values
                        .iter()
                        .find(|(known_row, known_column, _)| *known_row == row && *known_column == column)
                        .map(|(_, _, agg)| if agg.count == 0 { Value::Empty } else { agg.result(&way) })
                        .unwrap_or(Value::Empty);
                    written.push((CellRef { row: out_row + row as u32, col: out_col + column as u32 }, value));
                }
            }
            // The **last** cell of the result.
            let last = CellRef {
                row: (out_row + row_keys.len() as u32).saturating_sub(1).max(destination.cell.row),
                col: (out_col + column_keys.len() as u32).saturating_sub(1).max(destination.cell.col),
            };
            let mut node = node;
            let mut cells = crate::sheet_cells_mut(&mut node);
            for (cell, value) in &written {
                if matches!(value, Value::Empty) && cells.style(*cell).is_none() {
                    cells.clear(*cell);
                } else {
                    cells.set_value(*cell, value.clone());
                }
            }
            node.data["cells"] = crate::cells_into(&cells);
            if let Some(sheet) = session.document_mut()?.node_mut(sheet) {
                sheet.data = node.data.clone();
            }
            Ok(Outcome::changed(format!(
                "{} row(s) and {} column(s) consolidated",
                row_keys.len(),
                column_keys.len()
            ))
            .with_data(json!({
                "sheet": sheet,
                "range": crate::formula::Area::new(destination.cell, last).to_text(),
                "rows": row_keys.len(),
                "columns": column_keys.len(),
                "function": way,
            })))

        }
    }
}

/// The **label** of a cell: the text of it as it is shown.
fn label_of(cells: &Cells, cell: CellRef) -> String {
    match cells.get(cell).and_then(|entry| entry.value.as_ref()) {
        Some(value) => display_of(value).trim().to_string(),
        None => String::new(),
    }
}
