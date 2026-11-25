//! The **text of a column**, cut at fixed widths.
//!
//! Some text is not separated by a delimiter — it is **laid out**:
//! a line of a fixed-width report, a record of a mainframe, a column
//! of a listing where every field is eight characters wide. The tree's
//! **fixed widths** cuts such a text at the widths that are given.
//!
//! The rules:
//!
//! - the **widths** are counts of characters, and they are a set: the
//!   order they are written in does not matter and the same width
//!   twice is one width;
//! - a width of **no characters** is refused, because a cut at nothing
//!   is not a cut;
//! - a width that is **past the end of a text** cuts nothing there:
//!   the text is shorter than the layout says, and what is left of it
//!   is the last part;
//! - every part is read as a **value**: a text that is a number is a
//!   number, and a text that is nothing is nothing — an empty value
//!   takes the cell away;
//! - a cell that holds no text is **left alone**: there is nothing to
//!   cut.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Text};

use crate::formula::{CellRef, MAX_COLS, MAX_FORMULA_ROWS, Value};

/// The **parts** a text holds when it is cut at the given widths.
///
/// The widths are counts of characters, and a width past the end of
/// the text cuts nothing: the text is shorter than the layout says.
pub fn cut(text: &str, widths: &[u64]) -> Vec<String> {
    let characters: Vec<char> = text.chars().collect();
    let length = characters.len() as u64;
    let mut cuts: Vec<u64> = widths.iter().copied().filter(|width| *width > 0 && *width < length).collect();
    cuts.sort_unstable();
    cuts.dedup();
    let mut parts: Vec<String> = Vec::new();
    let mut start = 0u64;
    for at in cuts {
        let part: String = characters[start as usize..at as usize].iter().collect();
        parts.push(part);
        start = at;
    }
    let last: String = characters[start as usize..].iter().collect();
    parts.push(last);
    parts
}

commands! {
    /// **Cut** the text of a column at fixed widths.
    ///
    /// A change: it writes the parts it cuts into the cells beside the
    /// column they came from.
    grid.text.widths {
        area: "grid",
        layer: "grid",
        icon: "columns",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.
            sheet: NodeRef,
            /// The column the text is in — the letters of it, such as `A`.
            column: Text,
            /// The widths the text is cut at, in characters.
            ///
            /// They are written as a list of numbers, such as `8,12,20`.
            widths: Text,
            /// The column the parts are written into.
            ///
            /// It is the column they came from unless the file says otherwise.
            destination: Option<Text>,
        },
        run: {
            let id = "grid.text.widths".to_string();
            let sheet = self.sheet.0;
            // The **locks** the styles of the workbook carry: they are
            // read before the document is borrowed for the change.
            let locks = crate::style_locks(session, sheet);
            let source = CellRef::column_index(self.column.0.trim()).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("`{}` is not a column", self.column.0.trim()),
            })?;
            // The **widths**: a list of counts of characters.
            let mut widths: Vec<u64> = Vec::new();
            for part in self.widths.0.split(',') {
                let part = part.trim();
                if part.is_empty() {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the widths hold an empty width".to_string(),
                    });
                }
                let Ok(width) = part.parse::<u64>() else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{part}` is not a width"),
                    });
                };
                if width == 0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "a width of no characters is not a cut".to_string(),
                    });
                }
                if width > MAX_FORMULA_ROWS {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("a width is at most {MAX_FORMULA_ROWS} characters"),
                    });
                }
                widths.push(width);
            }
            if widths.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "there are no widths to cut at".to_string(),
                });
            }
            widths.sort_unstable();
            widths.dedup();
            let destination = match &self.destination {
                None => source,
                Some(text) => CellRef::column_index(text.0.trim()).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a column", text.0.trim()),
                })?,
            };
            // The **parts** are a row of values, and a sheet has so many
            // columns.
            if destination as u64 + widths.len() as u64 > MAX_COLS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the parts of the text do not fit in the {MAX_COLS} columns of a sheet"),
                });
            }
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
            // The **rows** of the column: the ones that hold a text.
            let mut rows: Vec<u32> = cells
                .iter()
                .filter(|(cell, entry)| cell.col == source && matches!(entry.value, Some(Value::Text(_))))
                .map(|(cell, _)| cell.row)
                .collect();
            rows.sort_unstable();
            let mut cut_count = 0usize;
            let mut written = 0usize;
            // The **plan**: the cells the parts are written into and what
            // they hold.
            let mut plan: Vec<(CellRef, Value)> = Vec::new();
            for row in rows {
                let cell = CellRef { col: source, row };
                let Some(entry) = cells.get(cell) else { continue };
                let Some(Value::Text(text)) = entry.value.clone() else { continue };
                let parts = cut(&text, &widths);
                for (offset, part) in parts.iter().enumerate() {
                    let target = CellRef { col: destination + offset as u32, row };
                    plan.push((target, crate::textcolumns::value_of(part)));
                    written += 1;
                }
                cut_count += 1;
            }
            // The cells that are **covered** by a merge or **locked** by the
            // sheet's protection are not written.
            let targets: Vec<CellRef> = plan.iter().map(|(cell, _)| *cell).collect();
            crate::merges::refuse_covered(node, &targets, "grid.text.widths")?;
            crate::protection::refuse_locked(node, &locks, &targets, "grid.text.widths")?;
            let mut cells = crate::sheet_cells_mut(node);
            for (target, value) in plan {
                if value == Value::Empty {
                    cells.clear(target);
                } else {
                    cells.set_value(target, value);
                }
            }
            node.data["cells"] = crate::cells_into(&cells);
            Ok(Outcome::changed(format!("cut the text of {cut_count} cell(s) at fixed widths")).with_data(json!({
                "sheet": sheet,
                "column": CellRef::column_text(source),
                "destination": CellRef::column_text(destination),
                "widths": widths,
                "rows": cut_count,
                "written": written,
            })))

        }
    }
}
