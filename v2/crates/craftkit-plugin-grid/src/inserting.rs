//! The **row that is inserted**.
//!
//! A row may be **inserted** into a sheet: the rows at and below it
//! move down, and everything that named a row is moved with them.
//!
//! So a row is inserted by **moving** the facts that name rows:
//!
//! - the **cells** of the sheet move down, and the **formulas** in
//!   them are rewritten: a reference to a cell that moved now names
//!   the cell it moved to, and a reference that names **another
//!   sheet** is left alone, because the rows of that sheet are not the
//!   rows that were inserted;
//! - the **borders**, the **notes**, the **links** and the
//!   **pictures** of a cell move with the cell;
//! - the **merges**, the **print area** and the **title rows** move
//!   with the rows they cover;
//! - the **hidden rows**, the **row heights**, the **outline levels**
//!   and the **row breaks** move with the rows they name.
//!
//! And what this tree does not know how to move is **refused**: a
//! sheet that holds a conditional format, a validation, a sparkline, a
//! table, a pivot or a chart refuses the insert and says what it
//! holds, because a fact left behind is a fact that is wrong.
//!
//! The rules:
//!
//! - a row is **counted from one**, and the number of rows inserted is
//!   a whole number above zero;
//! - the rows may not run **past the last row of the sheet**;
//! - a sheet that is **protected** may not have a row inserted into
//!   it: its cells are locked.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

use crate::formula::{Anchor, CellRef, MAX_ROWS};

/// The keys of a sheet that this tree **moves** when rows are
/// inserted.
pub(crate) const MOVED: [&str; 17] = [
    "cells",
    "borders",
    "notes",
    "links",
    "pictures",
    "merges",
    "hidden_rows",
    "row_outline",
    "rows",
    "print_area",
    "title_rows",
    "row_breaks",
    "conditionals",
    "validations",
    "sparklines",
    "tables",
    "pivots",
];

/// The keys of a sheet that name **no row**, and so are left as they
/// are.
pub(crate) const UNMOVED: [&str; 16] = [
    "print",
    "print_gridlines",
    "print_headings",
    "columns",
    "column_outline",
    "column_breaks",
    "filter",
    "hidden_columns",
    "protection",
    "zoom",
    "view",
    "tab_color",
    "summary_below",
    "summary_right",
    "hidden",
    "title_columns",
];

/// The **address** of a cell, moved down by the rows inserted.
///
/// It is `None` when the address is not one.
fn moved_address(address: &str, at: u64, count: u64) -> Option<String> {
    let cell = CellRef::parse(address)?;
    let row = cell.row as u64 + 1;
    if row < at {
        return Some(address.to_string());
    }
    Some(CellRef { col: cell.col, row: cell.row + count as u32 }.to_text())
}

/// The **area** a text names, moved down by the rows inserted.
///
/// The text is written as it was written: a range of two corners stays
/// a range of two corners.
fn moved_area(text: &str, at: u64, count: u64) -> Option<String> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), Some(last.trim())),
        None => (trimmed, None),
    };
    let anchor = Anchor::parse(first)?;
    let moved = anchor.inserted(at, count).to_text();
    match last {
        Some(last) => {
            let anchor = Anchor::parse(last)?;
            Some(format!("{moved}:{}", anchor.inserted(at, count).to_text()))
        }
        None => Some(moved),
    }
}

/// A **map of cells**: the cells move and the map is written again.
fn moved_cells_map(value: &Json, at: u64, count: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (address, entry) in map {
        let address = moved_address(address, at, count)
            .ok_or_else(|| format!("`{address}` is not a cell, and this tree does not know what it is"))?;
        moved.insert(address, entry.clone());
    }
    Ok(Json::Object(moved))
}

/// A **list of rows**: the rows move and the list is written again.
fn moved_rows(value: &Json, at: u64, count: u64) -> Json {
    let Some(list) = value.as_array() else { return value.clone() };
    let moved: Vec<Json> = list
        .iter()
        .map(|row| {
            let row = row.as_u64().unwrap_or(0);
            if row >= at { json!(row + count) } else { json!(row) }
        })
        .collect();
    Json::Array(moved)
}

/// A **map of rows**: the rows move and the map is written again.
fn moved_rows_map(value: &Json, at: u64, count: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (row, entry) in map {
        let row_number: u64 = row.parse().map_err(|_| format!("`{row}` is not a row"))?;
        let row = if row_number >= at { row_number + count } else { row_number };
        moved.insert(row.to_string(), entry.clone());
    }
    Ok(Json::Object(moved))
}

/// The **breaks** of a print: the rows move and the list is written
/// again. The breaks are counted from zero, as the tree counts them.
fn moved_breaks(value: &Json, at: u64, count: u64) -> Json {
    let Some(list) = value.as_array() else { return value.clone() };
    let moved: Vec<Json> = list
        .iter()
        .map(|line| {
            let line = line.as_u64().unwrap_or(0);
            if line + 1 >= at { json!(line + count) } else { json!(line) }
        })
        .collect();
    Json::Array(moved)
}

/// The **rows of a sheet** that are moved, and how many cells moved.
pub fn insert_rows(sheet: &mut Node, name: &str, at: u64, count: u64) -> Result<usize, String> {
    // What this tree does not know how to move is **refused**, and the
    // refusal says what it holds.
    if let Some(map) = sheet.data.as_object() {
        for key in map.keys() {
            if !MOVED.contains(&key.as_str()) && !UNMOVED.contains(&key.as_str()) {
                return Err(format!("the sheet holds `{key}`, and this tree does not move it yet"));
            }
        }
    }
    // The **cells**: they move, and the formulas in them are written
    // again with the references that moved.
    let cells = crate::cells_of(sheet);
    let mut moved = crate::formula::Cells::new();
    let mut moved_cells = 0usize;
    for (cell, entry) in cells.iter() {
        let row = cell.row as u64 + 1;
        let new_cell = if row >= at {
            moved_cells += 1;
            CellRef { col: cell.col, row: cell.row + count as u32 }
        } else {
            cell
        };
        let mut entry = entry.clone();
        if let Some(formula) = &entry.formula
            && let Some(rewritten) = crate::formula::insert_in_formula(formula, name, at, count)
        {
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
    // The **merges**: the areas they cover move.
    if let Some(list) = sheet.data.get("merges").and_then(|merges| merges.as_array()).cloned() {
        let mut moved = Vec::with_capacity(list.len());
        for merge in &list {
            let Some(text) = merge.as_str() else {
                return Err("a merge is not a range, and this tree does not know what it is".to_string());
            };
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
    // The **titles** of the print, and the **area** it prints.
    if let Some(rows) = sheet.data.get("title_rows").cloned() {
        sheet.data["title_rows"] = moved_rows(&rows, at, count);
    }
    if let Some(area) = sheet.data.get("print_area").cloned()
        && let Some(text) = area.as_str()
    {
        let moved = moved_area(text, at, count).ok_or_else(|| format!("`{text}` is not a range of cells"))?;
        sheet.data["print_area"] = Json::String(moved);
    }
    // The **breaks** of the print.
    if let Some(breaks) = sheet.data.get("row_breaks").cloned() {
        sheet.data["row_breaks"] = moved_breaks(&breaks, at, count);
    }
    // The **facts that name an area**: the conditional formats, the
    // validations and the sparklines.
    for key in ["conditionals", "validations"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        let moved = crate::moving::moved_ranges(&value, crate::moving::Way::Rows, at, count, true)?;
        sheet.data[key] = moved;
    }
    if let Some(value) = sheet.data.get("sparklines").cloned() {
        let moved = crate::moving::moved_sparklines(&value, crate::moving::Way::Rows, at, count, true)?;
        sheet.data["sparklines"] = moved;
    }
    if let Some(value) = sheet.data.get("tables").cloned() {
        let moved = crate::moving::moved_ranges(&value, crate::moving::Way::Rows, at, count, true)?;
        sheet.data["tables"] = moved;
    }
    if let Some(value) = sheet.data.get("pivots").cloned() {
        let moved = crate::moving::moved_pivots(&value, crate::moving::Way::Rows, at, count, true)?;
        sheet.data["pivots"] = moved;
    }
    Ok(moved_cells)
}

/// The **names** of the workbook a sheet is in that point at it.
pub(crate) fn names_of_the_workbook(session: &Session, sheet: u64) -> Vec<String> {
    let Some(workbook) = session
        .document()
        .walk()
        .find(|node| node.kind == "grid.workbook" && node.children.iter().any(|child| child.id == sheet))
    else {
        return Vec::new();
    };
    let Some(names) = workbook.data.get("names").and_then(|names| names.as_object()) else {
        return Vec::new();
    };
    let mut pointing = Vec::new();
    for refers_to in names.values() {
        let Some(text) = refers_to.as_str() else { continue };
        let Ok((name, _area)) = crate::names::parse_refers_to(text) else { continue };
        pointing.push(name);
    }
    pointing
}

/// The **last row** of a sheet: the row of the last cell in it.
pub fn last_row(sheet: &Node) -> u64 {
    crate::cells_of(sheet).iter().map(|(cell, _)| cell.row as u64 + 1).max().unwrap_or(0)
}

/// **Insert** a row into a sheet.
///
/// The rows at and below it move down, and everything that named a row
/// is moved with them.
#[derive(Command)]
#[command(id = "grid.row.insert", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct InsertRow {
    /// The sheet it goes in.
    pub sheet: NodeRef,
    /// The row it is inserted at, counted from one.
    pub row: Number,
    /// How many rows are inserted. Without it, one row.
    pub count: Option<Number>,
}

impl InsertRow {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.row.insert".to_string();
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
        // The rows may not run **past the last row of the sheet**.
        let last = last_row(&node);
        if last + count > MAX_ROWS {
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
        for name in names_of_the_workbook(session, sheet) {
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
        let moved = insert_rows(&mut sheet_node, &name, at, count)
            .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data = sheet_node.data;
        }
        Ok(Outcome::changed(format!("{count} row(s) were inserted at the row {at} of `{sheet}`"))
            .with_data(json!({ "sheet": sheet, "row": at, "count": count, "moved": moved })))
    }
}
