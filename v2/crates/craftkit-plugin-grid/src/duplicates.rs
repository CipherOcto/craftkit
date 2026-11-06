//! The **duplicates** of a range.
//!
//! A range may hold the same row twice: a list that was pasted in
//! twice, a table that was joined with itself. The tree's **remove
//! duplicates** keeps one row of each kind and takes the others away.
//!
//! The rules:
//!
//! - the **key** of a row is what the **chosen columns** hold, as
//!   they are shown — the display of a cell is what a person reads,
//!   and two cells that show the same thing are the same thing;
//! - the **first** row of a key is the one that is kept: the tree
//!   keeps the first occurrence and the rest go;
//! - the rows that go are **emptied**, not shifted: a row is a place
//!   in the sheet, and the place of a row that is gone is empty;
//! - a row whose key is **empty** is left alone: there is nothing to
//!   compare;
//! - the **header** is said and not guessed: a header is a fact, and a
//!   fact that is not said is not there;
//! - a range that holds a **merge** is refused, because a merged
//!   region is one cell and a row of a range is not.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Value};

/// The **columns** a range of letters names: `A` is one column and
/// `A:C` are three.
fn columns_of(text: &str, id: &str) -> Result<Vec<u32>, EngineError> {
    let mut columns: Vec<u32> = Vec::new();
    for part in text.split(':') {
        let Some(column) = CellRef::column_index(part.trim()) else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: format!("`{}` is not a column", part.trim()),
            });
        };
        if !columns.contains(&column) {
            columns.push(column);
        }
    }
    columns.sort_unstable();
    Ok(columns)
}

/// **Remove** the **duplicates** of a range.
///
/// A change: it empties the rows that hold what an earlier row holds
/// already.
#[derive(Command)]
#[command(id = "grid.duplicates.remove", area = "grid", layer = "grid", icon = "minus", mutates, undoable)]
pub struct RemoveDuplicates {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The range to look at. Without it, the used range is used.
    pub range: Option<Text>,
    /// The columns the **key** is made of, such as `A:C`.
    ///
    /// Without it, every column of the range is part of the key.
    pub columns: Option<Text>,
    /// Whether the first row of the range is a **header** and stays
    /// where it is.
    pub header: Option<Bool>,
}

impl RemoveDuplicates {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.duplicates.remove".to_string();
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
        crate::protection::refuse_sort(node, &id)?;
        let name = node.name.clone();
        let cells = crate::cells_of(node);
        // The **range**: the one that was named, or the used range.
        let area = match &self.range {
            Some(range) => crate::fill::area_at(&range.0, &id)?,
            None => match crate::reading::used_range(&cells) {
                Some(area) => area,
                None => return Ok(Outcome::new(format!("{name} holds nothing to look at"))),
            },
        };
        // The **key columns**: the ones that were named, or every
        // column of the range.
        let key_columns: Vec<u32> = match &self.columns {
            Some(text) => columns_of(&text.0, &id)?,
            None => (area.start.col..=area.end.col).collect(),
        };
        for column in &key_columns {
            if *column < area.start.col || *column > area.end.col {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the column {} is not in the range {}",
                        CellRef::column_text(*column),
                        area.to_text()
                    ),
                });
            }
        }
        // A **merge** in the range: the tree refuses, because a merged
        // region is one cell and a row of a range is not.
        for merge in crate::merges::merges_of(node) {
            if merge.start == merge.end {
                continue;
            }
            let touches = merge.start.row <= area.end.row
                && merge.end.row >= area.start.row
                && merge.start.col <= area.end.col
                && merge.end.col >= area.start.col;
            if touches {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the merge {} is in the range, and a merged region is one cell",
                        merge.to_text()
                    ),
                });
            }
        }
        let header = self.header.map(|flag| flag.0).unwrap_or(false);
        let body = if header && area.start.row < area.end.row {
            Area::new(CellRef { col: area.start.col, row: area.start.row + 1 }, area.end)
        } else {
            area
        };
        let book = crate::book_of(session.document());
        let system = crate::system_of(session, sheet);
        // The **keys** that have been seen: the first row of each is
        // the one that is kept.
        let mut seen: Vec<String> = Vec::new();
        let mut duplicates: Vec<u32> = Vec::new();
        let mut examined = 0usize;
        for row in body.start.row..=body.end.row {
            let mut parts: Vec<String> = Vec::new();
            let mut empty = true;
            for column in &key_columns {
                let cell = CellRef { col: *column, row };
                let value = match cells.get(cell) {
                    Some(entry) => match (&entry.value, &entry.formula) {
                        (Some(value), _) => value.clone(),
                        (None, Some(_)) => book.value_of(&name, cell),
                        (None, None) => Value::Empty,
                    },
                    None => Value::Empty,
                };
                if value != Value::Empty {
                    empty = false;
                }
                let format = crate::format_of(session, sheet, &cells, cell);
                parts.push(format.text(&value, system));
            }
            if empty {
                continue;
            }
            examined += 1;
            let key = parts.join("\u{1}");
            if seen.contains(&key) {
                duplicates.push(row);
            } else {
                seen.push(key);
            }
        }
        // The rows that are **gone**: their cells are emptied, and the
        // cells keep their format — the format of a cell is a fact of
        // its own.
        let mut targets: Vec<CellRef> = Vec::new();
        for row in &duplicates {
            for column in area.start.col..=area.end.col {
                targets.push(CellRef { col: column, row: *row });
            }
        }
        crate::protection::refuse_locked(node, &targets, &id)?;
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let mut cells = crate::sheet_cells_mut(node);
        for cell in &targets {
            cells.clear_contents(*cell);
        }
        node.data["cells"] = crate::cells_into(&cells);
        let removed = duplicates.len();
        Ok(Outcome::changed(format!("removed {removed} duplicate row(s)")).with_data(json!({
            "sheet": sheet,
            "range": area.to_text(),
            "rows": examined,
            "duplicates": removed,
            "keys": seen.len(),
        })))
    }
}
