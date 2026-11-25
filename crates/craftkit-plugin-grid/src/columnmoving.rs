//! The **column that moves**.
//!
//! A column may be **moved** to another place in a sheet: it is taken
//! out of where it is and put **before** another column, and everything
//! that named a column moves with it.
//!
//! So a column is moved by **moving** the facts that name columns:
//!
//! - the **cells** of the sheet move, and the **formulas** in them are
//!   written again — a reference follows the **cells** it names;
//! - the **borders**, the **notes**, the **comments**, the **links**
//!   and the **pictures** of a cell move with the cell;
//! - the **merges**, the **area** the sheet prints, the
//!   **validations**, the **conditionals**, the **sparklines**, the
//!   **tables**, the **pivots** and the **charts** move with the
//!   columns they cover;
//! - the **widths** of the columns, the **hidden columns**, the
//!   **outline levels**, the **title columns** and the **breaks** move
//!   with the columns they name;
//! - the **filter** moves with the column it filters;
//! - the **names** of the workbook that point at the sheet move with it.
//!
//! And what this tree does not know how to move is **refused**: a
//! sheet that holds a fact this tree does not know refuses the move and
//! says what it holds.
//!
//! # The rule of the references
//!
//! **A reference follows the cells it names.**
//!
//! - a reference to a **cell** of the moved block names the cell it
//!   moved to;
//! - a reference to a cell of the **band** between the column and its
//!   place names the cell it shifted to;
//! - a reference to an **area** that lies **wholly** in the moved
//!   block follows it;
//! - a reference to an area with an **edge on** the moved block names
//!   a column that is not there any more: in a **formula** it is the
//!   error `#REF!` — which is what the tree writes where a reference
//!   names a column that is gone — and a **fact** that cannot be
//!   written **refuses** the move, because a fact may not be an error.
//!
//! The `$` does not stop a move: it says a reference does not move when
//! a column is **inserted**, and a move is not an insertion — the cell
//! moves, and a reference names a cell.
//!
//! # The rules
//!
//! - a column is named by its **letters** — `A`, `B`, `AA`;
//! - the column goes **before** the column that is named, and the
//!   columns between them shift by as many columns as move;
//! - the column may not go before **itself**: a move that moves nothing
//!   is refused;
//! - a **protected** sheet may not have a column moved in it: its
//!   cells are locked.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Text};

use crate::formula::{Anchor, CellRef, MAX_FORMULA_COLUMNS};

/// The **column** a text names, and the number it is.
///
/// The columns are counted from **one**, as the tree counts them.
fn column_of(text: &str) -> Option<u64> {
    let trimmed = text.trim().to_ascii_uppercase();
    let index = CellRef::column_index(&trimmed)?;
    Some(index as u64 + 1)
}

/// The **column** a number is, when the block that moves goes from
/// `from` to `to` — `count` columns of it.
///
/// The columns between them shift by the size of the block: the ones
/// **between** move **left** when the block moves to the right, and
/// **right** when it moves to the left.
fn moved_number(number: u64, from: u64, count: u64, to: u64) -> u64 {
    crate::formula::moved_column_number(number, from, count, to)
}

/// The **column** a letter names, moved.
fn moved_column(text: &str, from: u64, count: u64, to: u64) -> Result<String, String> {
    let number =
        column_of(text).ok_or_else(|| format!("`{text}` is not a column: a column is `A`, `B` or `AA`"))?;
    Ok(CellRef::column_text(moved_number(number, from, count, to) as u32 - 1))
}

/// The **address** of a cell, moved.
///
/// The row does not move: only the column does.
fn moved_address(address: &str, from: u64, count: u64, to: u64) -> Result<String, String> {
    let cell = CellRef::parse(address)
        .ok_or_else(|| format!("`{address}` is not a cell, and this tree does not know what it is"))?;
    let column = moved_number(cell.col as u64 + 1, from, count, to);
    Ok(CellRef { col: column as u32 - 1, row: cell.row }.to_text())
}

/// The **area** a text names, moved.
///
/// It is an **error** when the area has an **edge on** the column that
/// moves: the area names a column that is not there any more, and a
/// fact that names one cannot be written.
fn moved_area(text: &str, from: u64, count: u64, to: u64) -> Result<String, String> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), Some(last.trim())),
        None => (trimmed, None),
    };
    let start = Anchor::parse(first)
        .ok_or_else(|| format!("`{text}` is not a range of cells, and this tree does not know what it is"))?;
    let end = match last {
        Some(last) => Anchor::parse(last).ok_or_else(|| {
            format!("`{text}` is not a range of cells, and this tree does not know what it is")
        })?,
        None => start,
    };
    let first_column = start.cell.col as u64 + 1;
    let last_column = end.cell.col as u64 + 1;
    let in_the_block = |column: u64| column >= from && column < from + count;
    // The area lies **wholly** in the block that moves: it follows it.
    if in_the_block(first_column) && in_the_block(last_column) {
        let moved = start.moved_column(from, count, to);
        return Ok(moved.to_text());
    }
    // An **edge** on the block that moves: the area names a column
    // that is not there any more.
    if in_the_block(first_column) || in_the_block(last_column) {
        return Err(format!("the range `{text}` names the column that moves, and it cannot be written"));
    }
    let moved_start = start.moved_column(from, count, to);
    match last {
        Some(_) => {
            let moved_end = end.moved_column(from, count, to);
            Ok(format!("{}:{}", moved_start.to_text(), moved_end.to_text()))
        }
        None => Ok(moved_start.to_text()),
    }
}

/// The **ranges** of a list of facts: the ranges move and the list is
/// written again.
///
/// A fact whose range names the column that moves **refuses** the
/// move: the fact cannot be written, and a fact left behind is a fact
/// that is wrong.
fn moved_ranges(value: &Json, from: u64, count: u64, to: u64) -> Result<Json, String> {
    let Some(list) = value.as_array() else { return Ok(value.clone()) };
    let mut moved = Vec::with_capacity(list.len());
    for fact in list {
        let Some(text) = fact.get("range").and_then(|range| range.as_str()) else {
            moved.push(fact.clone());
            continue;
        };
        let area = moved_area(text, from, count, to)?;
        let mut moved_fact = fact.clone();
        moved_fact["range"] = Json::String(area);
        moved.push(moved_fact);
    }
    Ok(Json::Array(moved))
}

/// The **sparklines** of a sheet: the cells move and the ranges move.
fn moved_sparklines(value: &Json, from: u64, count: u64, to: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (cell, sparkline) in map {
        let address = moved_address(cell, from, count, to)?;
        let mut moved_sparkline = sparkline.clone();
        if let Some(text) = sparkline.get("range").and_then(|range| range.as_str()) {
            moved_sparkline["range"] = Json::String(moved_area(text, from, count, to)?);
        }
        moved.insert(address, moved_sparkline);
    }
    Ok(Json::Object(moved))
}

/// The **pivots** of a sheet: the ranges move and the fields move with
/// them.
///
/// A pivot gathers by a **column**, and that column is a column of the
/// range it reads, so it moves with the range.
fn moved_pivots(value: &Json, from: u64, count: u64, to: u64) -> Result<Json, String> {
    let Some(list) = value.as_array() else { return Ok(value.clone()) };
    let mut moved = Vec::with_capacity(list.len());
    for fact in list {
        let mut moved_fact = fact.clone();
        if let Some(text) = fact.get("range").and_then(|range| range.as_str()) {
            moved_fact["range"] = Json::String(moved_area(text, from, count, to)?);
        }
        for key in ["row", "column", "value"] {
            let Some(letter) = moved_fact.get(key).and_then(|letter| letter.as_str()) else {
                continue;
            };
            moved_fact[key] = Json::String(moved_column(letter, from, count, to)?);
        }
        moved.push(moved_fact);
    }
    Ok(Json::Array(moved))
}

/// A **map of cells**: the cells move and the map is written again.
fn moved_cells_map(value: &Json, from: u64, count: u64, to: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (address, entry) in map {
        moved.insert(moved_address(address, from, count, to)?, entry.clone());
    }
    Ok(Json::Object(moved))
}

/// A **map of columns**: the columns move and the map is written again.
///
/// The map is keyed by the **letters** of the column.
fn moved_columns_map(value: &Json, from: u64, count: u64, to: u64) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (column, entry) in map {
        moved.insert(moved_column(column, from, count, to)?, entry.clone());
    }
    Ok(Json::Object(moved))
}

/// A **list of columns**: the columns move and the list is written
/// again.
///
/// The list is written as **letters**.
fn moved_columns_list(value: &Json, from: u64, count: u64, to: u64) -> Result<Json, String> {
    let Some(list) = value.as_array() else { return Ok(value.clone()) };
    let mut moved = Vec::with_capacity(list.len());
    for column in list {
        let Some(text) = column.as_str() else {
            return Err("a column is written as its letters, and this one is not".to_string());
        };
        moved.push(Json::String(moved_column(text, from, count, to)?));
    }
    Ok(Json::Array(moved))
}

/// A **list of column numbers**: the columns move and the list is
/// written again.
///
/// The numbers are counted from **zero**, as the tree counts the
/// columns of a print.
fn moved_indices(value: &Json, from: u64, count: u64, to: u64) -> Json {
    let Some(list) = value.as_array() else { return value.clone() };
    let moved: Vec<Json> = list
        .iter()
        .map(|column| {
            let column = column.as_u64().unwrap_or(0);
            json!(moved_number(column + 1, from, count, to) - 1)
        })
        .collect();
    Json::Array(moved)
}

/// The **filter** of a sheet: the column it filters moves with the
/// columns.
fn moved_filter(value: &Json, from: u64, count: u64, to: u64) -> Result<Json, String> {
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
                    let mut moved_criterion = criterion.clone();
                    moved_criterion["column"] = Json::String(moved_column(column, from, count, to)?);
                    moved_criteria.push(moved_criterion);
                }
                moved_groups.push(Json::Array(moved_criteria));
            }
            moved.insert(key.clone(), Json::Array(moved_groups));
            continue;
        }
        if key == "column" {
            let Some(text) = entry.as_str() else { return Ok(value.clone()) };
            moved.insert(key.clone(), Json::String(moved_column(text, from, count, to)?));
            continue;
        }
        moved.insert(key.clone(), entry.clone());
    }
    Ok(Json::Object(moved))
}

/// The **columns of a sheet** that move, and how many cells moved.
///
/// The **formulas** are not written here: they are written once, over
/// every sheet of the workbook, because a formula may name this sheet
/// from another one.
pub fn move_columns(sheet: &mut Node, from: u64, count: u64, to: u64) -> Result<usize, String> {
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
    // The **cells**: they move, and what they hold moves with them.
    //
    // The **formulas** in them are written again by the caller — once,
    // over every sheet of the workbook — because a formula may name
    // this sheet from another one.
    let cells = crate::cells_of(sheet);
    let mut moved = crate::formula::Cells::new();
    let mut moved_cells = 0usize;
    for (cell, entry) in cells.iter() {
        let column = moved_number(cell.col as u64 + 1, from, count, to);
        let new_cell = CellRef { col: column as u32 - 1, row: cell.row };
        moved.set_entry(new_cell, entry.clone());
        moved_cells += 1;
    }
    sheet.data["cells"] = crate::cells_into(&moved);
    // The maps of cells: the borders, the notes, the comments, the
    // links and the pictures.
    for key in ["borders", "notes", "comments", "links", "pictures"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        let moved = moved_cells_map(&value, from, count, to)?;
        sheet.data[key] = moved;
    }
    // The **merges**: the areas they cover move.
    if let Some(list) = sheet.data.get("merges").and_then(|merges| merges.as_array()).cloned() {
        let mut moved_merges = Vec::with_capacity(list.len());
        for merge in &list {
            let Some(text) = merge.as_str() else {
                return Err("a merge is not a range, and this tree does not know what it is".to_string());
            };
            moved_merges.push(Json::String(moved_area(text, from, count, to)?));
        }
        sheet.data["merges"] = Json::Array(moved_merges);
    }
    // The **widths** of the columns, and the **levels** of the columns
    // that are grouped.
    for key in ["columns", "column_outline"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        sheet.data[key] = moved_columns_map(&value, from, count, to)?;
    }
    // The **columns that are hidden**.
    if let Some(columns) = sheet.data.get("hidden_columns").cloned() {
        sheet.data["hidden_columns"] = moved_columns_list(&columns, from, count, to)?;
    }
    // The **titles** of the print: the columns that are printed on
    // every page.
    if let Some(columns) = sheet.data.get("title_columns").cloned() {
        sheet.data["title_columns"] = moved_indices(&columns, from, count, to);
    }
    // The **area** the sheet prints.
    if let Some(area) = sheet.data.get("print_area").cloned()
        && let Some(text) = area.as_str()
    {
        sheet.data["print_area"] = Json::String(moved_area(text, from, count, to)?);
    }
    // The **breaks** of the print.
    if let Some(breaks) = sheet.data.get("column_breaks").cloned() {
        sheet.data["column_breaks"] = moved_indices(&breaks, from, count, to);
    }
    // The **filter** of the sheet.
    if let Some(filter) = sheet.data.get("filter").cloned() {
        sheet.data["filter"] = moved_filter(&filter, from, count, to)?;
    }
    // The **facts that name an area**: the conditional formats, the
    // validations, the sparklines, the tables and the pivots.
    for key in ["conditionals", "validations", "tables"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        sheet.data[key] = moved_ranges(&value, from, count, to)?;
    }
    if let Some(value) = sheet.data.get("sparklines").cloned() {
        sheet.data["sparklines"] = moved_sparklines(&value, from, count, to)?;
    }
    if let Some(value) = sheet.data.get("pivots").cloned() {
        sheet.data["pivots"] = moved_pivots(&value, from, count, to)?;
    }
    Ok(moved_cells)
}

/// The **charts** of a sheet: the ranges they read move.
///
/// It is the chart's **id** and the range it reads that is returned, so
/// that the caller may write them back where they belong.
fn moved_charts(sheet: &Node, from: u64, count: u64, to: u64) -> Result<Vec<(u64, String)>, String> {
    let mut moved = Vec::new();
    for child in sheet.children.iter() {
        if child.kind != "grid.chart" {
            continue;
        }
        let Some(text) = child.data.get("range").and_then(|range| range.as_str()) else { continue };
        moved.push((child.id, moved_area(text, from, count, to)?));
    }
    Ok(moved)
}

/// The **names** of the workbook that point at a sheet: their areas move
/// with it.
///
/// It is the workbook's **id** and its names that are returned.
fn moved_names(
    session: &craftkit_sdk::Session,
    sheet: u64,
    name: &str,
    from: u64,
    count: u64,
    to: u64,
) -> Result<Option<(u64, Json)>, String> {
    let Some(workbook) = session
        .document()
        .walk()
        .find(|node| node.kind == "grid.workbook" && node.children.iter().any(|child| child.id == sheet))
    else {
        return Ok(None);
    };
    let Some(names) = workbook.data.get("names").and_then(|names| names.as_object()).cloned() else {
        return Ok(None);
    };
    let mut changed = false;
    let mut moved = craftkit_sdk::serde_json::Map::new();
    for (name_of_name, refers_to) in &names {
        let Some(text) = refers_to.as_str() else {
            moved.insert(name_of_name.clone(), refers_to.clone());
            continue;
        };
        let Ok((named, area)) = crate::names::parse_refers_to(text) else {
            moved.insert(name_of_name.clone(), refers_to.clone());
            continue;
        };
        if !named.eq_ignore_ascii_case(name) {
            moved.insert(name_of_name.clone(), refers_to.clone());
            continue;
        }
        let area = moved_area(&area.to_text(), from, count, to).map_err(|why| {
            format!("the name `{name_of_name}` points at a range that cannot be written: {why}")
        })?;
        changed = true;
        moved.insert(name_of_name.clone(), Json::String(format!("{named}!{area}")));
    }
    if !changed {
        return Ok(None);
    }
    Ok(Some((workbook.id, Json::Object(moved))))
}

commands! {
    /// **Move** a column to another place in a sheet.
    ///
    /// The column is taken out of where it is and put **before** the
    /// column that is named, and everything that named a column moves
    /// with it. More than one column may move at a time: the **count**
    /// says how many.
    ///
    /// The **formulas** are written again: a reference follows the
    /// **cells** it names.
    grid.column.move {
        area: "grid",
        layer: "grid",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The sheet it moves in.
            sheet: NodeRef,
            /// The column that moves: `A`, `B`, `AA`.
            column: Text,
            /// How many columns move. Without it, one column.
            count: Option<Number>,
            /// The column it goes **before**: `A`, `B`, `AA`.
            before: Text,
        },
        run: {
            let id = "grid.column.move".to_string();
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
            let text = self.column.0.trim().to_ascii_uppercase();
            let Some(from) = column_of(&text) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a column: a column is `A`, `B` or `AA`", self.column.0),
                });
            };
            let before_text = self.before.0.trim().to_ascii_uppercase();
            let Some(before) = column_of(&before_text) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a column: a column is `A`, `B` or `AA`", self.before.0),
                });
            };
            let count = self.count.map(|count| count.0).unwrap_or(1.0);
            if !count.is_finite() || count < 1.0 || count.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{count} is not a number of columns: the columns are counted from one"),
                });
            }
            let count = count as u64;
            // The block may not go before **itself**: a move that moves
            // nothing is not a move.
            if before >= from && before < from + count {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the column `{text}` is already before `{before_text}`"),
                });
            }
            // The place the block goes: it takes the place of the
            // column it is put before.
            let to = if before > from { before - count } else { before };
            if to + count > MAX_FORMULA_COLUMNS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a sheet has at most {MAX_FORMULA_COLUMNS} columns"),
                });
            }
            // A **protected** sheet has locked cells, and a locked cell
            // may not be changed.
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
            let moved = move_columns(&mut sheet_node, from, count, to)
                .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
            // The **charts** of the sheet: the ranges they read move with
            // the cells.
            let charts = moved_charts(&sheet_node, from, count, to)
                .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
            // The **names** of the workbook that point at the sheet: their
            // areas move with it.
            let names = moved_names(session, sheet, &name, from, count, to)
                .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data = sheet_node.data;
            }
            for (chart, range) in charts {
                if let Some(node) = session.document_mut()?.node_mut(chart) {
                    node.data["range"] = Json::String(range);
                }
            }
            if let Some((workbook, names)) = names
                && let Some(node) = session.document_mut()?.node_mut(workbook)
            {
                node.data["names"] = names;
            }
            // The **formulas of every sheet**: a formula may name this
            // sheet from another one, and a reference follows the cells
            // it names.
            let sheets: Vec<u64> = session
                .document()
                .walk()
                .filter(|node| node.kind == "grid.sheet")
                .map(|node| node.id)
                .collect();
            for other in sheets {
                let Some(node) = session.document_mut()?.node_mut(other) else { continue };
                let mut cells = crate::cells_of(node);
                cells.moved_column(&name, from, count, to);
                node.data["cells"] = crate::cells_into(&cells);
            }
            Ok(Outcome::changed(format!("the column {text} of `{sheet}` was moved before {before_text}"))
                .with_data(json!({
                    "sheet": sheet,
                    "column": text,
                    "before": before_text,
                    "from": from,
                    "count": count,
                    "to": to,
                    "moved": moved,
                })))

        }
    }
}
