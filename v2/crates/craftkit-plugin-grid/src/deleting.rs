//! The **row that is deleted**.
//!
//! A row may be **deleted** from a sheet: the rows below it move up,
//! and the cells of the deleted rows are taken away with them.
//!
//! So a row is deleted by **moving** the facts that name rows, exactly
//! as a row is inserted — the other way:
//!
//! - the **cells** below the deleted rows move up, and the
//!   **formulas** are rewritten: a reference to a cell that moved now
//!   names the cell it moved to;
//! - the **borders**, the **notes**, the **links** and the
//!   **pictures** move with their cell, and the ones of the deleted
//!   cells go with the cells;
//! - the **merges**, the **print area** and the **title rows** move
//!   up, and the ones that name a deleted row are refused;
//! - the **hidden rows**, the **row heights**, the **outline levels**
//!   and the **row breaks** move up, and the ones of the deleted rows
//!   go with the rows.
//!
//! And what this tree does not know how to move is **refused**, as it
//! is when a row is inserted (§496).
//!
//! The rules:
//!
//! - a row is **counted from one**, and the number of rows deleted is
//!   a whole number above zero;
//! - a **formula that names a cell in the deleted rows** refuses the
//!   delete: this tree has no `#REF!` yet, and a formula that names
//!   nothing is refused rather than written;
//! - a **merge** that covers one of the deleted rows refuses it, and
//!   so does a **print area** that names one;
//! - a sheet that is **protected** refuses: its cells are locked.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

use crate::formula::{CellRef, MAX_ROWS};

/// The **address** of a cell, moved up by the rows deleted.
///
/// It is `None` when the address is not one.
fn moved_address(address: &str, at: u64, count: u64) -> Option<String> {
    let cell = CellRef::parse(address)?;
    let row = cell.row as u64 + 1;
    if row >= at && row < at + count {
        return None;
    }
    if row >= at + count {
        return Some(CellRef { col: cell.col, row: cell.row - count as u32 }.to_text());
    }
    Some(address.to_string())
}

/// The **area** a text names, moved up by the rows deleted.
///
/// It is `None` when the area names a row that is deleted.
fn moved_area(text: &str, at: u64, count: u64) -> Option<String> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), Some(last.trim())),
        None => (trimmed, None),
    };
    let anchor = crate::formula::Anchor::parse(first)?;
    let moved = anchor.deleted(at, count)?.to_text();
    match last {
        Some(last) => {
            let anchor = crate::formula::Anchor::parse(last)?;
            Some(format!("{moved}:{}", anchor.deleted(at, count)?.to_text()))
        }
        None => Some(moved),
    }
}

/// A **map of cells**: the cells move up and the ones that are deleted
/// go away.
fn moved_cells_map(value: &Json, at: u64, count: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (address, entry) in map {
        let Some(address) = moved_address(address, at, count) else { continue };
        moved.insert(address, entry.clone());
    }
    Ok(Json::Object(moved))
}

/// A **list of rows**: the rows move up and the ones that are deleted
/// go away.
fn moved_rows(value: &Json, at: u64, count: u64) -> Json {
    let Some(list) = value.as_array() else { return value.clone() };
    let moved: Vec<Json> = list
        .iter()
        .filter_map(|row| {
            let row = row.as_u64()?;
            if row >= at && row < at + count {
                return None;
            }
            Some(json!(if row >= at + count { row - count } else { row }))
        })
        .collect();
    Json::Array(moved)
}

/// A **map of rows**: the rows move up and the ones that are deleted go
/// away.
fn moved_rows_map(value: &Json, at: u64, count: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (row, entry) in map {
        let row_number: u64 = row.parse().map_err(|_| format!("`{row}` is not a row"))?;
        if row_number >= at && row_number < at + count {
            continue;
        }
        let row = if row_number >= at + count { row_number - count } else { row_number };
        moved.insert(row.to_string(), entry.clone());
    }
    Ok(Json::Object(moved))
}

/// The **breaks** of a print: the rows move up and the ones that are
/// deleted go away. The breaks are counted from zero, as the tree
/// counts them.
fn moved_breaks(value: &Json, at: u64, count: u64) -> Json {
    let Some(list) = value.as_array() else { return value.clone() };
    let moved: Vec<Json> = list
        .iter()
        .filter_map(|line| {
            let line = line.as_u64()?;
            if line + 1 >= at && line + 1 < at + count {
                return None;
            }
            Some(json!(if line + 1 >= at + count { line - count } else { line }))
        })
        .collect();
    Json::Array(moved)
}

/// The **rows of a sheet** that are deleted, and how many cells went.
pub fn delete_rows(sheet: &mut Node, name: &str, at: u64, count: u64) -> Result<usize, String> {
    // What this tree does not know how to move is **refused**, and the
    // refusal says what it holds.
    if let Some(map) = sheet.data.as_object() {
        for key in map.keys() {
            if !crate::inserting::MOVED.contains(&key.as_str())
                && !crate::inserting::UNMOVED.contains(&key.as_str())
            {
                return Err(format!("the sheet holds `{key}`, and this tree does not move it yet"));
            }
        }
    }
    // The **cells**: the ones below move up, the ones of the deleted
    // rows go away, and the formulas are written again.
    let cells = crate::cells_of(sheet);
    let mut moved = crate::formula::Cells::new();
    let mut deleted = 0usize;
    for (cell, entry) in cells.iter() {
        let row = cell.row as u64 + 1;
        if row >= at && row < at + count {
            deleted += 1;
            continue;
        }
        let new_cell =
            if row >= at + count { CellRef { col: cell.col, row: cell.row - count as u32 } } else { cell };
        let mut entry = entry.clone();
        if let Some(formula) = &entry.formula {
            let rewritten = crate::formula::delete_in_formula(formula, name, at, count)
                .map_err(|why| format!("the formula of the cell {}: {why}", cell.to_text()))?;
            entry.formula = Some(rewritten);
        }
        moved.set_entry(new_cell, entry);
    }
    sheet.data["cells"] = crate::cells_into(&moved);
    // The maps of cells: the borders, the notes, the links and the
    // pictures.
    for key in ["borders", "notes", "links", "pictures"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        let moved = moved_cells_map(&value, at, count)?;
        sheet.data[key] = moved;
    }
    // The **merges**: a merge that covers a deleted row refuses the
    // delete, and a merge that is wholly in the deleted rows goes with
    // them.
    if let Some(list) = sheet.data.get("merges").and_then(|merges| merges.as_array()).cloned() {
        let mut moved = Vec::with_capacity(list.len());
        for merge in &list {
            let Some(text) = merge.as_str() else {
                return Err("a merge is not a range, and this tree does not know what it is".to_string());
            };
            let (first, last) = match text.trim().split_once(':') {
                Some((first, last)) => (first.trim(), last.trim()),
                None => (text.trim(), text.trim()),
            };
            let Some(start) = crate::formula::Anchor::parse(first) else {
                return Err(format!("`{text}` is not a range of cells"));
            };
            let Some(end) = crate::formula::Anchor::parse(last) else {
                return Err(format!("`{text}` is not a range of cells"));
            };
            let first_row = start.cell.row as u64 + 1;
            let last_row = end.cell.row as u64 + 1;
            if first_row < at + count && last_row >= at {
                return Err(format!("the merge `{text}` covers a row that is deleted"));
            }
            if first_row >= at && last_row < at + count {
                continue;
            }
            let moved_text =
                moved_area(text, at, count).ok_or_else(|| format!("`{text}` is not a range of cells"))?;
            moved.push(Json::String(moved_text));
        }
        sheet.data["merges"] = Json::Array(moved);
    }
    // The **rows that are hidden**, and the **rows that are grouped**.
    if let Some(rows) = sheet.data.get("hidden_rows").cloned() {
        sheet.data["hidden_rows"] = moved_rows(&rows, at, count);
    }
    if let Some(levels) = sheet.data.get("row_outline").cloned() {
        sheet.data["row_outline"] = moved_rows_map(&levels, at, count)?;
    }
    // The **heights of the rows**.
    if let Some(rows) = sheet.data.get("rows").cloned() {
        sheet.data["rows"] = moved_rows_map(&rows, at, count)?;
    }
    // The **titles** of the print.
    if let Some(rows) = sheet.data.get("title_rows").cloned() {
        sheet.data["title_rows"] = moved_rows(&rows, at, count);
    }
    // The **area** the sheet prints: an area that names a deleted row
    // refuses the delete.
    if let Some(area) = sheet.data.get("print_area").cloned()
        && let Some(text) = area.as_str()
    {
        let moved = moved_area(text, at, count)
            .ok_or_else(|| format!("the print area `{text}` names a row that is deleted"))?;
        sheet.data["print_area"] = Json::String(moved);
    }
    // The **breaks** of the print.
    if let Some(breaks) = sheet.data.get("row_breaks").cloned() {
        sheet.data["row_breaks"] = moved_breaks(&breaks, at, count);
    }
    // The **facts that name an area**: the conditional formats, the
    // validations and the sparklines. A fact whose area is wholly in
    // the rows that are deleted goes away with them, and a fact whose
    // area covers part of them refuses.
    for key in ["conditionals", "validations"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        let moved = crate::moving::moved_ranges(&value, crate::moving::Way::Rows, at, count, false)?;
        sheet.data[key] = moved;
    }
    if let Some(value) = sheet.data.get("sparklines").cloned() {
        let moved = crate::moving::moved_sparklines(&value, crate::moving::Way::Rows, at, count, false)?;
        sheet.data["sparklines"] = moved;
    }
    Ok(deleted)
}

/// **Delete** a row from a sheet.
///
/// The rows below it move up, and the cells of the deleted rows are
/// taken away with them.
#[derive(Command)]
#[command(id = "grid.row.delete", area = "grid", layer = "grid", icon = "minus", mutates, undoable)]
pub struct DeleteRow {
    /// The sheet it goes from.
    pub sheet: NodeRef,
    /// The row that is deleted, counted from one.
    pub row: Number,
    /// How many rows are deleted. Without it, one row.
    pub count: Option<Number>,
}

impl DeleteRow {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.row.delete".to_string();
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
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        let at = self.row.0;
        if !at.is_finite() || at < 1.0 || at.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a row is counted from one: {at} is not one"),
            });
        }
        let at = at as u64;
        let count = self.count.map(|count| count.0).unwrap_or(1.0);
        if !count.is_finite() || count < 1.0 || count.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("{count} is not a number of rows: the rows are counted from one"),
            });
        }
        let count = count as u64;
        if at + count > MAX_ROWS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a sheet has at most {MAX_ROWS} rows"),
            });
        }
        // A **protected** sheet has locked cells, and a locked cell may
        // not be changed.
        if crate::protection::Protection::of(&node).is_some_and(|protection| protection.locked) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the sheet `{sheet}` is protected, and the cells of a protected sheet may not be changed"
                ),
            });
        }
        // A **chart** of the sheet reads a range, and this tree does not
        // move it yet.
        if node.children.iter().any(|child| child.kind == "grid.chart") {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the sheet `{sheet}` holds a chart, and this tree does not move it yet"),
            });
        }
        // A **name** of the workbook may point at this sheet, and this
        // tree does not move the name yet.
        for name in crate::inserting::names_of_the_workbook(session, sheet) {
            if name.eq_ignore_ascii_case(&node.name) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the workbook has a name that points at the sheet `{sheet}`, and this tree does not move it yet"
                    ),
                });
            }
        }
        let name = node.name.clone();
        let mut sheet_node = node;
        let deleted = delete_rows(&mut sheet_node, &name, at, count)
            .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data = sheet_node.data;
        }
        Ok(Outcome::changed(format!("{count} row(s) were deleted at the row {at} of `{sheet}`"))
            .with_data(json!({ "sheet": sheet, "row": at, "count": count, "deleted": deleted })))
    }
}
