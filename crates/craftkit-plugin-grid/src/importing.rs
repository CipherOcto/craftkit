//! The **import** of a text into a sheet.
//!
//! A text of values separated by a delimiter — a **CSV** — is a table
//! in writing, and it may be brought into a workbook as a sheet.
//!
//! The rules:
//!
//! - the **delimiter** is one character, and the tree's is a comma;
//! - the **rows** of the text are the same length, and a text whose
//!   rows are not is refused: a table with rows of different lengths
//!   is a table with holes in it, and this tree refuses those too
//!   (the module `tables`);
//! - a **quoted** field is a field with a comma in it, and a quote
//!   inside it is written twice; a quoted field that is never closed
//!   is refused;
//! - a field is a **value** and not a formula: a field that reads
//!   `=1+1` is the text `=1+1`, because a text is a text;
//! - the text is **bounded**: a text of ten thousand rows is refused,
//!   because a text is from outside.
//!
//! And a **list of objects** — the tree's JSON — is a table as well: the
//! **keys** of the objects are the columns of the sheet, in the order
//! they first appear, and the first row of the sheet holds them.
//!
//! - a thing that is **not an object** is refused: a list of objects is
//!   a table and a list of numbers is not;
//! - a value that is an **object or a list** is refused: a cell holds a
//!   value and not a structure;
//! - a key that an object does not have is **nothing** in that row,
//!   and a null is nothing as well.
//!
//! # What the port found
//!
//! - **The tree's import makes a table of the text and asks about it.**
//!   Its `data.fromTextCsv` imports a text as a table and its
//!   `data.getData` does the same with a JSON array of objects. Here
//!   the import makes a **sheet** — because a sheet is what a
//!   spreadsheet has — and the text of the file is not kept beside
//!   it: the cells are the file, and a second copy of the file is a
//!   copy that can be wrong.
//! - **The tree is lenient about the rows.** A short row is padded
//!   and a long one is cut. Here the rows are the same length or the
//!   text is refused, because a sheet with a hole in a row is a
//!   sheet whose cells say nothing about where the hole is.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{CellRef, Value};

/// The most **rows** a text may have.
pub const MAX_ROWS: usize = 10_000;

/// The most **columns** a text may have.
pub const MAX_COLUMNS: usize = 256;

/// The **delimiter** the tree uses.
pub const DELIMITER: char = ',';

/// The **fields** of a text: the rows, and the fields of each row.
fn fields_of(text: &str, delimiter: char) -> Result<Vec<Vec<String>>, String> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut characters = text.chars().peekable();
    let mut ended = false;
    while let Some(character) = characters.next() {
        if quoted {
            if character == '"' {
                if characters.peek() == Some(&'"') {
                    characters.next();
                    field.push('"');
                    continue;
                }
                quoted = false;
                continue;
            }
            field.push(character);
            continue;
        }
        if character == '"' && field.is_empty() {
            quoted = true;
            continue;
        }
        if character == delimiter {
            row.push(std::mem::take(&mut field));
            continue;
        }
        if character == '\n' || character == '\r' {
            if character == '\r' && characters.peek() == Some(&'\n') {
                characters.next();
            }
            row.push(std::mem::take(&mut field));
            rows.push(std::mem::take(&mut row));
            ended = true;
            continue;
        }
        ended = false;
        field.push(character);
    }
    if quoted {
        return Err("a quoted field is never closed".to_string());
    }
    if !ended && (!field.is_empty() || !row.is_empty()) {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

/// The **value** of a field: a number, or a text, or nothing.
fn value_of(field: &str) -> Option<Value> {
    let trimmed = field.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(number) = trimmed.parse::<f64>()
        && number.is_finite()
    {
        return Some(Value::Number(number));
    }
    Some(Value::Text(field.to_string()))
}

commands! {
    /// **Import** a text of values into a workbook, as a sheet.
    ///
    /// The text is a **CSV**: its fields are separated by a delimiter, and
    /// a field may be quoted.
    grid.import.csv {
        area: "grid",
        layer: "grid",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The workbook the sheet goes in.
            workbook: NodeRef,
            /// The text.
            text: Text,
            /// What to call the sheet. Without it, the sheet is called
            /// `imported`.
            name: Option<Text>,
            /// The **delimiter**: one character. The default is a comma.
            delimiter: Option<Text>,
        },
        run: {
            let id = "grid.import.csv".to_string();
            let workbook = self.workbook.0;
            let Some(node) = session.document().node(workbook) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no workbook `{workbook}`"),
                });
            };
            if node.kind != "grid.workbook" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
                });
            }
            // The **delimiter**: one character.
            let delimiter = match &self.delimiter {
                Some(delimiter) => {
                    let trimmed = delimiter.0.trim();
                    let mut characters = trimmed.chars();
                    let Some(character) = characters.next() else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "a delimiter is one character, and this is none".to_string(),
                        });
                    };
                    if characters.next().is_some() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("`{trimmed}` is not a delimiter: a delimiter is one character"),
                        });
                    }
                    character
                }
                None => DELIMITER,
            };
            // The **rows** of the text.
            let rows = fields_of(&self.text.0, delimiter)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            if rows.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the text holds nothing to import".to_string(),
                });
            }
            if rows.len() > MAX_ROWS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the text holds {} rows, and a text holds at most {MAX_ROWS}", rows.len()),
                });
            }
            let width = rows[0].len();
            if width > MAX_COLUMNS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the text holds {width} columns, and a text holds at most {MAX_COLUMNS}"),
                });
            }
            // The **rows** are the same length, or the text is refused.
            for (number, row) in rows.iter().enumerate() {
                if row.len() != width {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the row {} holds {} fields and the first row holds {width}: the rows of a text are the same length",
                            number + 1,
                            row.len()
                        ),
                    });
                }
            }
            // The **name** of the sheet: the tree's rules for a name.
            let name = match &self.name {
                Some(name) => name.0.trim().to_string(),
                None => "imported".to_string(),
            };
            let taken: Vec<String> = node
                .children
                .iter()
                .filter(|child| child.kind == "grid.sheet")
                .map(|child| child.name.clone())
                .collect();
            let taken = taken.iter().find(|other| other.eq_ignore_ascii_case(&name)).cloned();
            if let Some(reason) = crate::sheetnames::why_not(&name, taken.as_deref()) {
                return Err(EngineError::Failed { id: id.clone(), message: reason });
            }
            // The **cells**: the text, read.
            let mut cells = craftkit_sdk::serde_json::Map::new();
            let mut filled = 0u64;
            for (row, fields) in rows.iter().enumerate() {
                for (column, field) in fields.iter().enumerate() {
                    let Some(value) = value_of(field) else { continue };
                    let address = CellRef { col: column as u32, row: row as u32 }.to_text();
                    cells.insert(address, json!({ "value": value.to_json() }));
                    filled += 1;
                }
            }
            let data = json!({
                "cells": craftkit_sdk::serde_json::Value::Object(cells),
                "view": crate::view::SheetView::default().to_json(),
                "zoom": 100,
            });
            let sheet = session.document_mut()?.add_node_with_under(
                Some(workbook),
                name.clone(),
                "grid.sheet",
                None,
                data,
            );
            Ok(Outcome::changed(format!("imported {filled} cell(s) into the sheet `{name}`")).with_data(json!({
                "workbook": workbook,
                "id": sheet,
                "name": name,
                "rows": rows.len(),
                "columns": width,
                "cells": filled,
            })))

        }
    }

    /// **Import** a list of objects into a workbook, as a sheet.
    ///
    /// The **keys** of the objects are the columns of the sheet, in the
    /// order they first appear, and the first row of the sheet holds
    /// them.
    grid.import.json {
        area: "grid",
        layer: "grid",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The workbook the sheet goes in.
            workbook: NodeRef,
            /// The **list of objects**: `[{ "who": "ann", "amount": 3 }]`.
            data: Json,
            /// What to call the sheet. Without it, the sheet is called
            /// `imported`.
            name: Option<Text>,
        },
        run: {
            let id = "grid.import.json".to_string();
            let workbook = self.workbook.0;
            let Some(node) = session.document().node(workbook) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no workbook `{workbook}`"),
                });
            };
            if node.kind != "grid.workbook" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
                });
            }
            let Some(list) = self.data.as_array() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the data is a list of objects".to_string(),
                });
            };
            if list.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the data holds nothing to import".to_string(),
                });
            }
            if list.len() > MAX_ROWS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the data holds {} rows, and a text holds at most {MAX_ROWS}", list.len()),
                });
            }
            // The **keys**: the columns, in the order they first appear.
            let mut keys: Vec<String> = Vec::new();
            for entry in list {
                let Some(object) = entry.as_object() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the data is a list of objects, and a thing in it is not one".to_string(),
                    });
                };
                for key in object.keys() {
                    if !keys.contains(key) {
                        keys.push(key.clone());
                    }
                }
            }
            if keys.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the data holds nothing to import".to_string(),
                });
            }
            if keys.len() > MAX_COLUMNS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the data holds {} columns, and a text holds at most {MAX_COLUMNS}",
                        keys.len()
                    ),
                });
            }
            // The **cells**: the keys are the first row and the values are
            // the rest.
            let mut cells = craftkit_sdk::serde_json::Map::new();
            for (index, key) in keys.iter().enumerate() {
                let address = CellRef { col: index as u32, row: 0 }.to_text();
                cells.insert(address, json!({ "value": key }));
            }
            let mut filled = keys.len() as u64;
            for (row, entry) in list.iter().enumerate() {
                let Some(object) = entry.as_object() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the data is a list of objects, and a thing in it is not one".to_string(),
                    });
                };
                for (index, key) in keys.iter().enumerate() {
                    let Some(value) = object.get(key) else { continue };
                    let Some(value) =
                        json_value(value).map_err(|message| EngineError::Failed { id: id.clone(), message })?
                    else {
                        continue;
                    };
                    let address = CellRef { col: index as u32, row: row as u32 + 1 }.to_text();
                    cells.insert(address, json!({ "value": value.to_json() }));
                    filled += 1;
                }
            }
            // The **name** of the sheet: the tree's rules for a name.
            let name = match &self.name {
                Some(name) => name.0.trim().to_string(),
                None => "imported".to_string(),
            };
            let taken: Vec<String> = node
                .children
                .iter()
                .filter(|child| child.kind == "grid.sheet")
                .map(|child| child.name.clone())
                .collect();
            let taken = taken.iter().find(|other| other.eq_ignore_ascii_case(&name)).cloned();
            if let Some(reason) = crate::sheetnames::why_not(&name, taken.as_deref()) {
                return Err(EngineError::Failed { id: id.clone(), message: reason });
            }
            let data = json!({
                "cells": craftkit_sdk::serde_json::Value::Object(cells),
                "view": crate::view::SheetView::default().to_json(),
                "zoom": 100,
            });
            let sheet = session.document_mut()?.add_node_with_under(
                Some(workbook),
                name.clone(),
                "grid.sheet",
                None,
                data,
            );
            Ok(Outcome::changed(format!("imported {filled} cell(s) into the sheet `{name}`")).with_data(json!({
                "workbook": workbook,
                "id": sheet,
                "name": name,
                "rows": list.len() + 1,
                "columns": keys.len(),
                "cells": filled,
            })))

        }
    }
}

/// The **value** of a JSON value, if it is one a cell may hold.
fn json_value(value: &craftkit_sdk::serde_json::Value) -> Result<Option<Value>, String> {
    Ok(match value {
        craftkit_sdk::serde_json::Value::Null => None,
        craftkit_sdk::serde_json::Value::Bool(flag) => Some(Value::Bool(*flag)),
        craftkit_sdk::serde_json::Value::Number(number) => {
            let number = number.as_f64().unwrap_or(0.0);
            if number.is_finite() { Some(Value::Number(number)) } else { None }
        }
        craftkit_sdk::serde_json::Value::String(text) => Some(Value::Text(text.clone())),
        craftkit_sdk::serde_json::Value::Object(_) | craftkit_sdk::serde_json::Value::Array(_) => {
            return Err("a cell holds a value and not a structure".to_string());
        }
    })
}
