//! The **text of a column**, cut into columns.
//!
//! A column may hold text that is really **several values**: a list of
//! names separated by commas, a row of a file that was pasted in, a
//! line of a log. The tree's **text to columns** cuts such a text into
//! the columns beside it.
//!
//! The rules:
//!
//! - the **delimiter** is the text the parts are separated by, and it
//!   is a tab unless the file says otherwise; a delimiter of more than
//!   one character is **refused**, because a delimiter that is a word
//!   is a guess;
//! - the **qualifier** marks a part that holds the delimiter itself,
//!   and the qualifier is removed from the part — a double quote
//!   unless the file says otherwise;
//! - a **run** of delimiters is one delimiter when the file says so,
//!   and each one is a part when it does not;
//! - every part is read as a **value**: a text that is a number is a
//!   number, and a text that is nothing is nothing — an empty value
//!   takes the cell away;
//! - a cell that holds no text is **left alone**: there is nothing to
//!   cut.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Text};

use crate::formula::{CellRef, MAX_COLS, Value};

/// The **parts** a text holds, cut at the delimiter.
///
/// A part that is wrapped in the qualifier is one part, whatever it
/// holds, and the qualifier is taken away.
pub fn split(text: &str, delimiter: char, qualifier: char, treat_consecutive: bool) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut characters = text.chars().peekable();
    while let Some(character) = characters.next() {
        if character == qualifier {
            quoted = !quoted;
            continue;
        }
        if character == delimiter && !quoted {
            // A run of delimiters is one delimiter when the file says
            // so.
            if treat_consecutive {
                while characters.peek() == Some(&delimiter) {
                    characters.next();
                }
            }
            parts.push(current.clone());
            current.clear();
            continue;
        }
        current.push(character);
    }
    parts.push(current);
    parts
}

/// The **value** a part holds: a number if the text is one, and a text
/// if it is not.
fn value_of(part: &str) -> Value {
    if part.is_empty() {
        return Value::Empty;
    }
    match part.parse::<f64>() {
        Ok(number) if number.is_finite() => Value::Number(number),
        _ => Value::Text(part.to_string()),
    }
}

commands! {
    /// **Cut** the text of a column into columns.
    ///
    /// A change: it writes the parts it cuts into the cells beside the
    /// column they came from.
    grid.text.columns {
        area: "grid",
        layer: "grid",
        icon: "scissors",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The column the text is in — the letters of it, such as `A`.

            column: Text,
            /// The text the parts are separated by.
            ///
            /// It is a tab unless the file says otherwise.

            delimiter: Option<Text>,
            /// The character that marks a part that holds the delimiter.
            ///
            /// It is a double quote unless the file says otherwise.

            qualifier: Option<Text>,
            /// Whether a run of delimiters is one delimiter.

            treat_consecutive: Option<Bool>,
            /// The column the parts are written into.
            ///
            /// It is the column they came from unless the file says otherwise.

            destination: Option<Text>,
        },
        run: {

            let id = "grid.text.columns".to_string();
            let sheet = self.sheet.0;
            // The **locks** the styles of the workbook carry: they are
            // read before the document is borrowed for the change.
            let locks = crate::style_locks(session, sheet);
            let source = CellRef::column_index(self.column.0.trim()).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("`{}` is not a column", self.column.0.trim()),
            })?;
            let delimiter = match &self.delimiter {
                None => '\t',
                Some(text) => {
                    let mut characters = text.0.chars();
                    let Some(first) = characters.next() else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the delimiter is empty".to_string(),
                        });
                    };
                    if characters.next().is_some() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the delimiter `{}` is more than one character", text.0),
                        });
                    }
                    first
                }
            };
            let qualifier = match &self.qualifier {
                None => '"',
                Some(text) => {
                    let mut characters = text.0.chars();
                    let Some(first) = characters.next() else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the qualifier is empty".to_string(),
                        });
                    };
                    if characters.next().is_some() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the qualifier `{}` is more than one character", text.0),
                        });
                    }
                    first
                }
            };
            let treat_consecutive = self.treat_consecutive.map(|value| value.0).unwrap_or(false);
            let destination = match &self.destination {
                None => source,
                Some(text) => CellRef::column_index(text.0.trim()).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a column", text.0.trim()),
                })?,
            };
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
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
            // The **rows** of the column: the ones that hold something.
            let mut rows: Vec<u32> = cells
                .iter()
                .filter(|(cell, entry)| cell.col == source && entry.value.is_some())
                .map(|(cell, _)| cell.row)
                .collect();
            rows.sort_unstable();
            let mut cut = 0usize;
            let mut written = 0usize;
            // The **plan**: the cells the parts are written into and what
            // they hold.
            let mut plan: Vec<(CellRef, Value)> = Vec::new();
            for row in rows {
                let cell = CellRef { col: source, row };
                let Some(entry) = cells.get(cell) else { continue };
                let Some(Value::Text(text)) = entry.value.clone() else { continue };
                let parts = split(&text, delimiter, qualifier, treat_consecutive);
                // The parts are written into the columns from the
                // destination on, and a sheet has so many columns.
                if destination as u64 + parts.len() as u64 > MAX_COLS {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the parts of the text do not fit in the {MAX_COLS} columns of a sheet"),
                    });
                }
                for (offset, part) in parts.iter().enumerate() {
                    let target = CellRef { col: destination + offset as u32, row };
                    plan.push((target, value_of(part)));
                    written += 1;
                }
                cut += 1;
            }
            // The cells that are **covered** by a merge or **locked** by the
            // sheet's protection are not written.
            let targets: Vec<CellRef> = plan.iter().map(|(cell, _)| *cell).collect();
            crate::merges::refuse_covered(node, &targets, "grid.text.columns")?;
            crate::protection::refuse_locked(node, &locks, &targets, "grid.text.columns")?;
            let mut cells = crate::sheet_cells_mut(node);
            for (target, value) in plan {
                if value == Value::Empty {
                    cells.clear(target);
                } else {
                    cells.set_value(target, value);
                }
            }
            node.data["cells"] = crate::cells_into(&cells);
            Ok(Outcome::changed(format!("cut the text of {cut} cell(s) into columns")).with_data(json!({
                "sheet": sheet,
                "column": CellRef::column_text(source),
                "destination": CellRef::column_text(destination),
                "rows": cut,
                "written": written,
            })))

        }
    }
}
