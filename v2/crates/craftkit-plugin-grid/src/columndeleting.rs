//! The **column that is deleted**.
//!
//! A column may be **deleted** from a sheet: the columns to the right
//! of it move left, and the cells of the deleted columns are taken
//! away with them.
//!
//! So a column is deleted by **moving** the facts that name columns,
//! exactly as a column is inserted — the other way:
//!
//! - the **cells** to the right move left, and the **formulas** are
//!   rewritten: a reference to a cell that moved now names the cell
//!   it moved to;
//! - the **borders**, the **notes**, the **links** and the **pictures**
//!   move with their cell, and the ones of the deleted cells go with
//!   the cells;
//! - the **merges** and the **area** the sheet prints move left, and
//!   the ones that name a deleted column are refused;
//! - the **widths** of the columns, the **hidden columns**, the
//!   **outline levels**, the **title columns** and the **breaks** of
//!   the print move left, and the ones of the deleted columns go with
//!   the columns;
//! - the **filter** moves left, and a filter of a deleted column
//!   refuses the delete, because a filter that filters nothing is not
//!   a filter.
//!
//! And what this tree does not know how to move is **refused**, as it
//! is when a column is inserted (§498).
//!
//! The rules:
//!
//! - a column is named by its **letters** — `A`, `B`, `AA` — and the
//!   number of columns deleted is a whole number above zero;
//! - a **formula that names a cell in the deleted columns** refuses
//!   the delete: this tree has no `#REF!` yet, and a formula that
//!   names nothing is refused rather than written;
//! - a **merge** that covers one of the deleted columns refuses it,
//!   and so does a **print area** that names one;
//! - a sheet that is **protected** refuses: its cells are locked.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::formula::{Anchor, CellRef, MAX_COLS};

/// The **address** of a cell, moved left by the columns deleted.
///
/// It is `None` when the address names a cell that is deleted.
fn moved_address(address: &str, at: u64, count: u64) -> Option<String> {
    let cell = CellRef::parse(address)?;
    let col = cell.col as u64 + 1;
    if col >= at && col < at + count {
        return None;
    }
    if col >= at + count {
        return Some(CellRef { col: cell.col - count as u32, row: cell.row }.to_text());
    }
    Some(address.to_string())
}

/// The **area** a text names, moved left by the columns deleted.
///
/// It is `None` when the area names a column that is deleted.
fn moved_area(text: &str, at: u64, count: u64) -> Option<String> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), Some(last.trim())),
        None => (trimmed, None),
    };
    let anchor = Anchor::parse(first)?;
    let moved = anchor.deleted_column(at, count)?.to_text();
    match last {
        Some(last) => {
            let anchor = Anchor::parse(last)?;
            Some(format!("{moved}:{}", anchor.deleted_column(at, count)?.to_text()))
        }
        None => Some(moved),
    }
}

/// A **map of cells**: the cells move left and the ones that are
/// deleted go away.
fn moved_cells_map(value: &Json, at: u64, count: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (address, entry) in map {
        let Some(address) = moved_address(address, at, count) else { continue };
        moved.insert(address, entry.clone());
    }
    Ok(Json::Object(moved))
}

/// A **map of columns**: the columns move left and the ones that are
/// deleted go away.
///
/// The map is keyed by the **letters** of the column.
fn moved_columns_map(value: &Json, at: u64, count: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (column, entry) in map {
        let Some(index) = CellRef::column_index(column.trim()) else {
            return Err(format!("`{column}` is not a column: a column is `A`, `B` or `AA`"));
        };
        let number = index as u64 + 1;
        if number >= at && number < at + count {
            continue;
        }
        let moved_number = if number >= at + count { number - count } else { number };
        moved.insert(CellRef::column_text(moved_number as u32 - 1), entry.clone());
    }
    Ok(Json::Object(moved))
}

/// A **list of columns**: the columns move left and the ones that are
/// deleted go away.
///
/// The list is written as **letters**.
fn moved_columns_list(value: &Json, at: u64, count: u64) -> Result<Json, String> {
    let Some(list) = value.as_array() else { return Ok(value.clone()) };
    let mut moved = Vec::with_capacity(list.len());
    for column in list {
        let Some(text) = column.as_str() else {
            return Err("a column is written as its letters, and this one is not".to_string());
        };
        let Some(index) = CellRef::column_index(text.trim()) else {
            return Err(format!("`{text}` is not a column: a column is `A`, `B` or `AA`"));
        };
        let number = index as u64 + 1;
        if number >= at && number < at + count {
            continue;
        }
        let moved_number = if number >= at + count { number - count } else { number };
        moved.push(Json::String(CellRef::column_text(moved_number as u32 - 1)));
    }
    Ok(Json::Array(moved))
}

/// A **list of column numbers**: the columns move left and the ones
/// that are deleted go away.
///
/// The numbers are counted from zero, as the tree counts the columns
/// of a print.
fn moved_indices(value: &Json, at: u64, count: u64) -> Json {
    let Some(list) = value.as_array() else { return value.clone() };
    let moved: Vec<Json> = list
        .iter()
        .filter_map(|column| {
            let column = column.as_u64()?;
            let number = column + 1;
            if number >= at && number < at + count {
                return None;
            }
            Some(json!(if number >= at + count { column - count } else { column }))
        })
        .collect();
    Json::Array(moved)
}

/// The **filter** of a sheet: the column it filters moves left with
/// the columns, and a filter of a deleted column refuses the delete.
fn moved_filter(value: &Json, at: u64, count: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (key, entry) in map {
        if key == "groups" {
            let Some(groups) = entry.as_array() else { return Ok(value.clone()) };
            let mut moved_groups = Vec::with_capacity(groups.len());
            for group in groups {
                let Some(criteria) = group.as_array() else { return Ok(value.clone()) };
                let mut moved_criteria = Vec::with_capacity(criteria.len());
                for criterion in criteria {
                    let Some(column) = criterion.get("column").and_then(|column| column.as_str()) else {
                        moved_criteria.push(criterion.clone());
                        continue;
                    };
                    let Some(index) = CellRef::column_index(column.trim()) else {
                        return Err(format!("`{column}` is not a column: a column is `A`, `B` or `AA`"));
                    };
                    let number = index as u64 + 1;
                    if number >= at && number < at + count {
                        return Err(format!("the filter filters the column {column}, which is deleted"));
                    }
                    let moved_number = if number >= at + count { number - count } else { number };
                    let mut moved_criterion = criterion.clone();
                    moved_criterion["column"] = Json::String(CellRef::column_text(moved_number as u32 - 1));
                    moved_criteria.push(moved_criterion);
                }
                moved_groups.push(Json::Array(moved_criteria));
            }
            moved.insert(key.clone(), Json::Array(moved_groups));
            continue;
        }
        if key == "column" {
            let Some(text) = entry.as_str() else { return Ok(value.clone()) };
            let Some(index) = CellRef::column_index(text.trim()) else {
                return Err(format!("`{text}` is not a column: a column is `A`, `B` or `AA`"));
            };
            let number = index as u64 + 1;
            if number >= at && number < at + count {
                return Err(format!("the filter filters the column {text}, which is deleted"));
            }
            let moved_number = if number >= at + count { number - count } else { number };
            moved.insert(key.clone(), Json::String(CellRef::column_text(moved_number as u32 - 1)));
            continue;
        }
        moved.insert(key.clone(), entry.clone());
    }
    Ok(Json::Object(moved))
}

/// The **columns of a sheet** that are deleted, and how many cells
/// went.
pub fn delete_columns(sheet: &mut Node, name: &str, at: u64, count: u64) -> Result<usize, String> {
    // What this tree does not know how to move is **refused**, and the
    // refusal says what it holds.
    if let Some(map) = sheet.data.as_object() {
        for key in map.keys() {
            if !crate::columninserting::MOVED.contains(&key.as_str())
                && !crate::columninserting::UNMOVED.contains(&key.as_str())
            {
                return Err(format!("the sheet holds `{key}`, and this tree does not move it yet"));
            }
        }
    }
    // The **cells**: the ones to the right move left, the ones of the
    // deleted columns go away, and the formulas are written again.
    let cells = crate::cells_of(sheet);
    let mut moved = crate::formula::Cells::new();
    let mut deleted = 0usize;
    for (cell, entry) in cells.iter() {
        let col = cell.col as u64 + 1;
        if col >= at && col < at + count {
            deleted += 1;
            continue;
        }
        let new_cell =
            if col >= at + count { CellRef { col: cell.col - count as u32, row: cell.row } } else { cell };
        let mut entry = entry.clone();
        if let Some(formula) = &entry.formula {
            let rewritten = crate::formula::delete_column_in_formula(formula, name, at, count)
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
    // The **merges**: a merge that covers a deleted column refuses the
    // delete, and a merge that is wholly in the deleted columns goes
    // with them.
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
            let Some(start) = Anchor::parse(first) else {
                return Err(format!("`{text}` is not a range of cells"));
            };
            let Some(end) = Anchor::parse(last) else {
                return Err(format!("`{text}` is not a range of cells"));
            };
            let first_col = start.cell.col as u64 + 1;
            let last_col = end.cell.col as u64 + 1;
            if first_col < at + count && last_col >= at {
                return Err(format!("the merge `{text}` covers a column that is deleted"));
            }
            if first_col >= at && last_col < at + count {
                continue;
            }
            let moved_text =
                moved_area(text, at, count).ok_or_else(|| format!("`{text}` is not a range of cells"))?;
            moved.push(Json::String(moved_text));
        }
        sheet.data["merges"] = Json::Array(moved);
    }
    // The **widths** of the columns, and the **levels** of the columns
    // that are grouped.
    if let Some(columns) = sheet.data.get("columns").cloned() {
        sheet.data["columns"] = moved_columns_map(&columns, at, count)?;
    }
    if let Some(levels) = sheet.data.get("column_outline").cloned() {
        sheet.data["column_outline"] = moved_columns_map(&levels, at, count)?;
    }
    // The **columns that are hidden**.
    if let Some(columns) = sheet.data.get("hidden_columns").cloned() {
        sheet.data["hidden_columns"] = moved_columns_list(&columns, at, count)?;
    }
    // The **titles** of the print: the columns that are printed on
    // every page.
    if let Some(columns) = sheet.data.get("title_columns").cloned() {
        sheet.data["title_columns"] = moved_indices(&columns, at, count);
    }
    // The **area** the sheet prints: an area that names a deleted
    // column refuses the delete.
    if let Some(area) = sheet.data.get("print_area").cloned()
        && let Some(text) = area.as_str()
    {
        let moved = moved_area(text, at, count)
            .ok_or_else(|| format!("the print area `{text}` names a column that is deleted"))?;
        sheet.data["print_area"] = Json::String(moved);
    }
    // The **breaks** of the print.
    if let Some(breaks) = sheet.data.get("column_breaks").cloned() {
        sheet.data["column_breaks"] = moved_indices(&breaks, at, count);
    }
    // The **filter** of the sheet.
    if let Some(filter) = sheet.data.get("filter").cloned() {
        sheet.data["filter"] = moved_filter(&filter, at, count)?;
    }
    // The **facts that name an area**: the conditional formats, the
    // validations and the sparklines. A fact whose area is wholly in
    // the columns that are deleted goes away with them, and a fact
    // whose area covers part of them refuses.
    for key in ["conditionals", "validations"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        let moved = crate::moving::moved_ranges(&value, crate::moving::Way::Columns, at, count, false)?;
        sheet.data[key] = moved;
    }
    if let Some(value) = sheet.data.get("sparklines").cloned() {
        let moved = crate::moving::moved_sparklines(&value, crate::moving::Way::Columns, at, count, false)?;
        sheet.data["sparklines"] = moved;
    }
    if let Some(value) = sheet.data.get("tables").cloned() {
        let moved = crate::moving::moved_ranges(&value, crate::moving::Way::Columns, at, count, false)?;
        sheet.data["tables"] = moved;
    }
    if let Some(value) = sheet.data.get("pivots").cloned() {
        let moved = crate::moving::moved_pivots(&value, crate::moving::Way::Columns, at, count, false)?;
        sheet.data["pivots"] = moved;
    }
    Ok(deleted)
}

/// **Delete** a column from a sheet.
///
/// The columns to the right of it move left, and the cells of the
/// deleted columns are taken away with them.
#[derive(Command)]
#[command(id = "grid.column.delete", area = "grid", layer = "grid", icon = "minus", mutates, undoable)]
pub struct DeleteColumn {
    /// The sheet it goes from.
    pub sheet: NodeRef,
    /// The column that is deleted: `A`, `B`, `AA`.
    pub column: Text,
    /// How many columns are deleted. Without it, one column.
    pub count: Option<Number>,
}

impl DeleteColumn {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.column.delete".to_string();
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
        let text = self.column.0.trim().to_ascii_uppercase();
        let Some(index) = CellRef::column_index(&text) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{}` is not a column: a column is `A`, `B` or `AA`", self.column.0),
            });
        };
        let at = index as u64 + 1;
        let count = self.count.map(|count| count.0).unwrap_or(1.0);
        if !count.is_finite() || count < 1.0 || count.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("{count} is not a number of columns: the columns are counted from one"),
            });
        }
        let count = count as u64;
        if at + count > MAX_COLS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a sheet has at most {MAX_COLS} columns"),
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
        let name = node.name.clone();
        let mut sheet_node = node;
        let deleted = delete_columns(&mut sheet_node, &name, at, count)
            .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
        // The **charts** of the sheet: the ranges they read move with
        // the cells.
        let charts = crate::moving::moved_charts(&sheet_node, crate::moving::Way::Columns, at, count, false)
            .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
        // The **names** of the workbook that point at the sheet: their
        // areas move with it.
        let names =
            crate::moving::moved_names(session, sheet, &name, crate::moving::Way::Columns, at, count, false)
                .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data = sheet_node.data;
        }
        for (chart, range) in charts {
            match range {
                Some(range) => {
                    if let Some(node) = session.document_mut()?.node_mut(chart) {
                        node.data["range"] = Json::String(range);
                    }
                }
                None => {
                    session.document_mut()?.remove_node(chart);
                }
            }
        }
        if let Some((workbook, names)) = names
            && let Some(node) = session.document_mut()?.node_mut(workbook)
        {
            node.data["names"] = names;
        }
        Ok(Outcome::changed(format!("{count} column(s) were deleted at the column {text} of `{sheet}`"))
            .with_data(json!({ "sheet": sheet, "column": text, "count": count, "deleted": deleted })))
    }
}
