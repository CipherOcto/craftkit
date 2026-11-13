//! The **fields** of a pivot: the columns of the range it reads.
//!
//! The tree's own command: it asks what a pivot is built from — the
//! **fields** of its source, and the **areas** of the report each of
//! them sits in.
//!
//! # The rules
//!
//! - The **fields** are the columns of the range the pivot reads, and
//!   they are named by the **header** of them: the header cell of a
//!   field names the field, and a cell that holds nothing or a number
//!   does not name a field — so the fields are **refused** and the
//!   refusal says which cell.
//! - A field is **numeric** when every value of it that holds
//!   something is a number, and it holds something at all: a column
//!   of nothing is not a column of numbers.
//! - A field is a **date** when it is numeric and the cell of its
//!   first row carries a **date format**: a date here is a number
//!   with a date written on it, as it is in the tree.
//! - The **samples** of a field are up to five of the values of it,
//!   without repeats, in the tree's own order: the numbers first and
//!   in their own order, then the text without regard to its case,
//!   then the booleans, then the errors, and the empty cells last.
//! - The **areas** of a field are the places of the report it sits
//!   in: `rows`, `columns`, `values` and `filters` — a field may sit
//!   in more than one of them.
//!
//! # What it does not do
//!
//! - The tree's **calculated fields** are not here: this tree has no
//!   calculated fields, so a field of a pivot is always a column of
//!   its range.
//! - The samples are **labels**, not values: a number of a date field
//!   is written as the date it is.

use std::cmp::Ordering;

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{CellRef, Value};

/// The most **samples** a field shows: the tree's own five.
const MAX_SAMPLES: usize = 5;

/// The **class** of a value: the order the tree puts the values in.
///
/// The numbers come first, then the text, then the booleans, then the
/// errors, and the empty cells last.
fn class_of(value: &Value) -> u8 {
    match value {
        Value::Number(_) => 0,
        Value::Text(_) => 1,
        Value::Bool(_) => 2,
        Value::Error(_) => 3,
        Value::Empty => 4,
    }
}

/// The **key** a value is remembered by: the tree's own.
///
/// Two texts that differ only in their case are one item, and two
/// numbers that are the same number are one item.
fn key_of(value: &Value) -> String {
    match value {
        Value::Number(number) => format!("n{}", number.to_bits()),
        Value::Text(text) => format!("t{}", text.to_lowercase()),
        Value::Bool(flag) => format!("l{flag}"),
        Value::Error(message) => format!("e{}", message.as_str()),
        Value::Empty => "b".to_string(),
    }
}

/// The **order** of two values: the tree's own.
///
/// The numbers come before the text, and the text before the
/// booleans, and the empty cells last.
fn compare(left: &Value, right: &Value) -> Ordering {
    let class = class_of(left).cmp(&class_of(right));
    if class != Ordering::Equal {
        return class;
    }
    match (left, right) {
        (Value::Number(one), Value::Number(other)) => one.total_cmp(other),
        (Value::Text(one), Value::Text(other)) => one.to_lowercase().cmp(&other.to_lowercase()),
        (Value::Bool(one), Value::Bool(other)) => one.cmp(other),
        (Value::Error(one), Value::Error(other)) => one.as_str().cmp(other.as_str()),
        _ => Ordering::Equal,
    }
}

/// The **text** a value is written with, when it is not a number with
/// a format on it.
fn plain_text(value: &Value) -> String {
    match value {
        Value::Number(number) => {
            if number.fract() == 0.0 && number.abs() < 9.0e15 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => if *flag { "TRUE" } else { "FALSE" }.to_string(),
        Value::Error(message) => message.clone(),
        Value::Empty => "(blank)".to_string(),
    }
}

commands! {
    /// **Read** the **fields** of a pivot.
    ///
    /// The fields are **derived**: they are worked out from the cells of
    /// the sheet as they are, so they cannot be out of date.
    grid.pivot.fields {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The name of the pivot.
            name: Text,
        },
        run: {
            let id = "grid.pivot.fields".to_string();
            let sheet = self.sheet.0;
            let node = crate::pivots::sheet_of(session, &id, sheet)?;
            let name = self.name.0.trim().to_string();
            let pivots = crate::pivots::pivots_of(&node);
            let pivot = crate::pivots::pivot_of(&pivots, &name, &id)?;
            let cells = crate::cells_of(&node);
            let book = crate::book_of(session.document());
            let sheet_name = node.name.clone();
            let system = crate::system_of(session, sheet);

            // The **value** of a cell: what it holds, or what its formula
            // computes.
            let value_at = |cell: CellRef| -> Value {
                match cells.get(cell) {
                    Some(entry) => match (&entry.value, &entry.formula) {
                        (Some(value), _) => value.clone(),
                        (None, Some(_)) => book.value_of(&sheet_name, cell),
                        (None, None) => Value::Empty,
                    },
                    None => Value::Empty,
                }
            };

            let area = pivot.range;
            let mut fields: Vec<Json> = Vec::new();
            // The **records**: the rows of the range that hold something.
            let mut records: u64 = 0;
            for col in area.start.col..=area.end.col {
                let header = CellRef { col, row: area.start.row };
                // The **name** of the field is the text of its header, and
                // a header that holds nothing does not name a field.
                let name = crate::pivots::header_text(&cells, header)
                    .map_err(|message| EngineError::Failed { id: id.clone(), message })?;

                // The **values** of the column, and whether they are
                // numbers.
                let mut values: Vec<Value> = Vec::new();
                let mut numeric = true;
                let mut any = false;
                for row in area.start.row + 1..=area.end.row {
                    let value = value_at(CellRef { col, row });
                    if !matches!(value, Value::Empty) {
                        any = true;
                        if !matches!(value, Value::Number(_)) {
                            numeric = false;
                        }
                    }
                    values.push(value);
                }
                // The **format** of the field: the one on the cell of its
                // first row, as it is in the tree.
                let first = CellRef { col, row: area.start.row + 1 };
                let code = cells.get(first).and_then(|entry| entry.format.clone());
                let format = code.as_deref().and_then(crate::format::Format::parse);
                let is_date = numeric && any && matches!(format, Some(crate::format::Format::Date { .. }));

                // The **samples**: the values of the column, without
                // repeats, in the tree's own order.
                let mut items: Vec<Value> = Vec::new();
                let mut seen: Vec<String> = Vec::new();
                for value in values {
                    let key = key_of(&value);
                    if seen.contains(&key) {
                        continue;
                    }
                    seen.push(key);
                    items.push(value);
                }
                items.sort_by(compare);
                let samples: Vec<Json> = items
                    .iter()
                    .take(MAX_SAMPLES)
                    .map(|value| {
                        let label = match (value, &format) {
                            (Value::Number(_), Some(format)) => format.text(value, system),
                            _ => plain_text(value),
                        };
                        json!(label)
                    })
                    .collect();

                // The **areas** the field sits in.
                let mut areas: Vec<&str> = Vec::new();
                if col == pivot.row_field {
                    areas.push("rows");
                }
                if pivot.column_field == Some(col) {
                    areas.push("columns");
                }
                if pivot.value_field == Some(col) {
                    areas.push("values");
                }
                if pivot.filter.as_ref().is_some_and(|filter| filter.field == col) {
                    areas.push("filters");
                }

                fields.push(json!({
                    "name": name,
                    "column": CellRef::column_text(col),
                    "numeric": numeric && any,
                    "date": is_date,
                    "samples": samples,
                    "areas": areas,
                }));
            }

            // The **records**: the rows of the range that hold something at
            // all.
            for row in area.start.row + 1..=area.end.row {
                let holds = (area.start.col..=area.end.col)
                    .any(|col| !matches!(value_at(CellRef { col, row }), Value::Empty));
                if holds {
                    records += 1;
                }
            }

            let mut out = crate::pivots::pivot_json(&pivot);
            if let Some(map) = out.as_object_mut() {
                map.insert("sheet".to_string(), json!(sheet));
                map.insert("fields".to_string(), Json::Array(fields));
                map.insert("records".to_string(), json!(records));
            }
            Ok(Outcome::new(format!(
                "the pivot `{name}` reads {} fields",
                out["fields"].as_array().map(|f| f.len()).unwrap_or(0)
            ))
            .with_data(out))

        }
    }
}
