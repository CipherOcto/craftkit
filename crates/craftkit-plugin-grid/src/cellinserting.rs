//! The **cells that are inserted** and the **cells that are deleted**.
//!
//! A range of cells may be **inserted**: the cells beside it move, and
//! the range holds nothing afterwards. A range of cells may be
//! **deleted**: the cells beside it move back, and the cells that were
//! in the range are gone.
//!
//! The difference from the inserting of a **row** or of a **column** is
//! the **band**: the cells move only **within** the rows of the range
//! when they move sideways, and only within its columns when they move
//! down. A cell above the band is not touched by a sideways move, and
//! a cell beside the band is not touched by a downward one.
//!
//! So:
//!
//! - the **cells** move, and their **formulas** are written again;
//! - the **borders**, the **notes**, the **comments**, the **links**
//!   and the **pictures** of a cell move with the cell;
//! - the **merges**, the **area** the sheet prints, the
//!   **validations**, the **conditional formats**, the **sparklines**,
//!   the **tables**, the **pivots** and the **charts** move with the
//!   columns (or the rows) they cover;
//! - the **names** of the workbook that point at the sheet move with it.
//!
//! And what this tree does not know how to move is **refused**: a
//! sheet that holds a fact this tree does not know refuses the move and
//! says what it holds.
//!
//! # The rule of the references
//!
//! A reference is written again when it lies **within the band** — the
//! rows of the band when the cells move sideways, and its columns when
//! they move down — and it is left as it was when it reaches outside
//! it: the cells it names are not the cells that moved.
//!
//! A reference that lies within the band is written as if the columns
//! (or the rows) were inserted (or deleted) at the edge of the band,
//! which is the rule of the inserting and the deleting of a column.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Text};

use crate::formula::{Area, CellRef, Way};
use crate::moving::{moved_address_in_band, moved_area_in_band};

/// The **range** a text names, and the way it moves.
///
/// The way is **right** or **down** for an inserting, and **left** or
/// **up** for a deleting.
fn band_of(text: &str, shift: &str) -> Result<(Area, Way, bool), String> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), last.trim()),
        None => (trimmed, trimmed),
    };
    let Some(first) = CellRef::parse(first) else {
        return Err(format!("`{text}` is not a range of cells, and this tree does not know what it is"));
    };
    let Some(last) = CellRef::parse(last) else {
        return Err(format!("`{text}` is not a range of cells, and this tree does not know what it is"));
    };
    let band = Area::new(first, last);
    match shift {
        "right" => Ok((band, Way::Columns, true)),
        "down" => Ok((band, Way::Rows, true)),
        "left" => Ok((band, Way::Columns, false)),
        "up" => Ok((band, Way::Rows, false)),
        _ => Err(format!(
            "`{shift}` is not a way for the cells to move: it is `right`, `down`, `left` or `up`"
        )),
    }
}

/// The **cells** of a sheet, moved by the cells that are inserted or
/// deleted, and how many moved.
///
/// The cells that move are the ones **within the band** — the rows of
/// the band when the cells move sideways, and its columns when they
/// move down — and the ones **beside** the band.
fn moved_cells(
    cells: &crate::formula::Cells,
    band: &Area,
    way: Way,
    insert: bool,
) -> Result<Vec<(CellRef, crate::formula::Cell)>, String> {
    let at = match way {
        Way::Columns => band.start.col as u64 + 1,
        Way::Rows => band.start.row as u64 + 1,
    };
    let count = match way {
        Way::Columns => (band.end.col - band.start.col + 1) as u64,
        Way::Rows => (band.end.row - band.start.row + 1) as u64,
    };
    let mut moved = Vec::new();
    for (cell, entry) in cells.iter() {
        // The cell is within the band when it is in the **rows** of the
        // band and the cells move sideways, and in its **columns** when
        // they move down.
        //
        // The rows and the columns are counted from **one**, as the
        // tree counts them.
        let row = cell.row as u64 + 1;
        let column = cell.col as u64 + 1;
        let in_band = match way {
            Way::Columns => row > band.start.row as u64 && row <= band.end.row as u64 + 1,
            Way::Rows => column > band.start.col as u64 && column <= band.end.col as u64 + 1,
        };
        if !in_band {
            // The cell is **beside** the band: it does not move.
            moved.push((cell, entry.clone()));
            continue;
        }
        let cell = match way {
            Way::Columns => {
                if insert {
                    if column < at {
                        // The cell is **before** the band: it stays.
                        cell
                    } else if column + count > crate::formula::MAX_FORMULA_COLUMNS {
                        // The cell would be pushed **off** the sheet,
                        // and the tree refuses rather than leave a cell
                        // behind (§962).
                        return Err(format!(
                            "the cell {} would be pushed off the sheet, and a cell may not be lost",
                            CellRef { col: (column - 1) as u32, row: cell.row }.to_text()
                        ));
                    } else {
                        CellRef { col: (column + count - 1) as u32, row: cell.row }
                    }
                } else if column >= at && column < at + count {
                    // The cell is in the band and the cells are being
                    // deleted: it is gone.
                    continue;
                } else if column >= at + count {
                    CellRef { col: (column - count - 1) as u32, row: cell.row }
                } else {
                    cell
                }
            }
            Way::Rows => {
                if insert {
                    if row < at {
                        // The cell is **before** the band: it stays.
                        cell
                    } else if row + count > crate::formula::MAX_FORMULA_ROWS {
                        return Err(format!(
                            "the cell {} would be pushed off the sheet, and a cell may not be lost",
                            CellRef { col: cell.col, row: (row - 1) as u32 }.to_text()
                        ));
                    } else {
                        CellRef { col: cell.col, row: (row + count - 1) as u32 }
                    }
                } else if row >= at && row < at + count {
                    continue;
                } else if row >= at + count {
                    CellRef { col: cell.col, row: (row - count - 1) as u32 }
                } else {
                    cell
                }
            }
        };
        moved.push((cell, entry.clone()));
    }
    Ok(moved)
}

/// The **facts of a sheet** that name cells, moved with the cells.
///
/// The **areas** move with the columns (or the rows) they cover, and a
/// range that reaches outside the band does not move at all. The
/// **maps of cells** move with their cell, and a cell that is deleted
/// takes its note, its border, its comment, its link and its picture
/// with it.
fn moved_facts(sheet: &mut Node, band: &Area, way: Way, insert: bool) {
    // The maps of cells: the borders, the notes, the comments, the
    // links and the pictures.
    for key in ["borders", "notes", "comments", "links", "pictures"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        let Some(map) = value.as_object() else { continue };
        let mut moved = craftkit_sdk::serde_json::Map::new();
        for (address, entry) in map {
            let Some(address) = moved_address_in_band(address, band, way, insert) else { continue };
            moved.insert(address, entry.clone());
        }
        sheet.data[key] = Json::Object(moved);
    }
    // The **merges**: the areas they cover move, and a merge of one
    // cell is not a merge.
    if let Some(list) = sheet.data.get("merges").and_then(|merges| merges.as_array()).cloned() {
        let mut moved = Vec::with_capacity(list.len());
        for merge in &list {
            let Some(text) = merge.as_str() else { continue };
            let Some(area) = moved_area_in_band(text, band, way, insert) else { continue };
            let (first, last) = match crate::moving::corners(&area) {
                Some(corners) => corners,
                None => continue,
            };
            if first == last {
                continue;
            }
            moved.push(Json::String(area));
        }
        sheet.data["merges"] = Json::Array(moved);
    }
    // The **area** the sheet prints.
    if let Some(area) = sheet.data.get("print_area").cloned()
        && let Some(text) = area.as_str()
        && let Some(moved) = moved_area_in_band(text, band, way, insert)
    {
        sheet.data["print_area"] = Json::String(moved);
    }
    // The **facts that name an area**: the conditional formats, the
    // validations, the sparklines, the tables and the pivots.
    for key in ["conditionals", "validations", "tables"] {
        let Some(value) = sheet.data.get(key).cloned() else { continue };
        let Some(list) = value.as_array() else { continue };
        let mut moved = Vec::with_capacity(list.len());
        for fact in list {
            let mut moved_fact = fact.clone();
            if let Some(text) = fact.get("range").and_then(|range| range.as_str())
                && let Some(area) = moved_area_in_band(text, band, way, insert)
            {
                moved_fact["range"] = Json::String(area);
            }
            moved.push(moved_fact);
        }
        sheet.data[key] = Json::Array(moved);
    }
    if let Some(value) = sheet.data.get("sparklines").cloned()
        && let Some(map) = value.as_object()
    {
        let mut moved = craftkit_sdk::serde_json::Map::new();
        for (cell, sparkline) in map {
            let Some(address) = moved_address_in_band(cell, band, way, insert) else { continue };
            let mut moved_sparkline = sparkline.clone();
            if let Some(text) = sparkline.get("range").and_then(|range| range.as_str())
                && let Some(area) = moved_area_in_band(text, band, way, insert)
            {
                moved_sparkline["range"] = Json::String(area);
            }
            moved.insert(address, moved_sparkline);
        }
        sheet.data["sparklines"] = Json::Object(moved);
    }
    if let Some(value) = sheet.data.get("pivots").cloned()
        && let Some(list) = value.as_array()
    {
        let mut moved = Vec::with_capacity(list.len());
        for fact in list {
            let mut moved_fact = fact.clone();
            if let Some(text) = fact.get("range").and_then(|range| range.as_str())
                && let Some(area) = moved_area_in_band(text, band, way, insert)
            {
                moved_fact["range"] = Json::String(area);
            }
            moved.push(moved_fact);
        }
        sheet.data["pivots"] = Json::Array(moved);
    }
}

/// The **charts** of a sheet: the ranges they read move.
///
/// It is the chart's **id** and the range it reads that is returned.
fn moved_charts(sheet: &Node, band: &Area, way: Way, insert: bool) -> Vec<(u64, String)> {
    let mut moved = Vec::new();
    for child in sheet.children.iter() {
        if child.kind != "grid.chart" {
            continue;
        }
        let Some(text) = child.data.get("range").and_then(|range| range.as_str()) else { continue };
        let Some(range) = moved_area_in_band(text, band, way, insert) else { continue };
        moved.push((child.id, range));
    }
    moved
}

/// The **names** of the workbook that point at a sheet: their areas move
/// with it.
///
/// It is the workbook's **id** and its names that are returned.
fn moved_names(
    session: &craftkit_sdk::Session,
    sheet: u64,
    name: &str,
    band: &Area,
    way: Way,
    insert: bool,
) -> Option<(u64, Json)> {
    let workbook = session
        .document()
        .walk()
        .find(|node| node.kind == "grid.workbook" && node.children.iter().any(|child| child.id == sheet))?;
    let names = workbook.data.get("names").and_then(|names| names.as_object()).cloned()?;
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
        let Some(moved_area) = moved_area_in_band(&area.to_text(), band, way, insert) else {
            moved.insert(name_of_name.clone(), refers_to.clone());
            continue;
        };
        changed = true;
        moved.insert(name_of_name.clone(), Json::String(format!("{named}!{moved_area}")));
    }
    if !changed {
        return None;
    }
    Some((workbook.id, Json::Object(moved)))
}

/// Move the **cells** of a sheet: the cells that are inserted, or the
/// cells that are deleted.
///
/// The answer is how many cells moved.
fn move_cells(
    session: &mut craftkit_sdk::Session,
    id: &str,
    sheet: u64,
    range: &str,
    shift: &str,
) -> Result<usize, String> {
    let Some(node) = session.document().node(sheet).cloned() else {
        return Err(format!("there is no sheet `{sheet}`"));
    };
    if node.kind != "grid.sheet" {
        return Err(format!("the node `{sheet}` is a `{}` and not a sheet", node.kind));
    }
    let (band, way, insert) = band_of(range, shift)?;
    // What this tree does not know how to move is **refused**, and the
    // refusal says what it holds.
    if let Some(map) = node.data.as_object() {
        for key in map.keys() {
            if !crate::columninserting::MOVED.contains(&key.as_str())
                && !crate::columninserting::UNMOVED.contains(&key.as_str())
            {
                return Err(format!("the sheet holds `{key}`, and this tree does not move it yet"));
            }
        }
    }
    // A **protected** sheet has locked cells, and a locked cell may not
    // be changed.
    if crate::protection::Protection::of(&node).is_some_and(|protection| protection.locked) {
        return Err(format!(
            "the sheet `{sheet}` is protected, and the cells of a protected sheet may not be changed"
        ));
    }
    let name = node.name.clone();
    let mut sheet_node = node;
    // The **cells**: they move, and the formulas in them are written
    // again — once, over every sheet of the workbook, because a
    // formula may name this sheet from another one.
    let cells = crate::cells_of(&sheet_node);
    let moved = moved_cells(&cells, &band, way, insert)?;
    let mut new_cells = crate::formula::Cells::new();
    for (cell, entry) in moved {
        new_cells.set_entry(cell, entry);
    }
    sheet_node.data["cells"] = crate::cells_into(&new_cells);
    // The **facts** of the sheet.
    moved_facts(&mut sheet_node, &band, way, insert);
    let charts = moved_charts(&sheet_node, &band, way, insert);
    let names = moved_names(session, sheet, &name, &band, way, insert);
    if let Some(node) = session
        .document_mut()
        .map_err(|why| format!("the document could not be changed: {why}"))?
        .node_mut(sheet)
    {
        node.data = sheet_node.data;
    }
    for (chart, range) in charts {
        if let Some(node) = session
            .document_mut()
            .map_err(|why| format!("the document could not be changed: {why}"))?
            .node_mut(chart)
        {
            node.data["range"] = Json::String(range);
        }
    }
    if let Some((workbook, names)) = names
        && let Some(node) = session
            .document_mut()
            .map_err(|why| format!("the document could not be changed: {why}"))?
            .node_mut(workbook)
    {
        node.data["names"] = names;
    }
    // The **formulas of every sheet**: a formula may name this sheet
    // from another one, and a reference follows the cells it names.
    let sheets: Vec<u64> =
        session.document().walk().filter(|node| node.kind == "grid.sheet").map(|node| node.id).collect();
    for other in sheets {
        let Some(node) = session
            .document_mut()
            .map_err(|why| format!("the document could not be changed: {why}"))?
            .node_mut(other)
        else {
            continue;
        };
        let mut cells = crate::cells_of(node);
        cells.shifted_cells(&name, &band, way, insert);
        node.data["cells"] = crate::cells_into(&cells);
    }
    let _ = id;
    Ok(new_cells.len())
}

commands! {
    /// **Insert** cells into a sheet: the cells beside them move.
    ///
    /// The cells move **within the band**: the rows of the range when
    /// they move **right**, and its columns when they move **down**.
    grid.cell.insert {
        area: "grid",
        layer: "grid",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The sheet it goes in.
            sheet: NodeRef,
            /// The **range** of cells that are inserted: `A1`, `B2:D4`.
            range: Text,
            /// Which way the cells beside them move: `right` or `down`.
            shift: Text,
        },
        run: {
            let id = "grid.cell.insert".to_string();
            let moved = move_cells(session, &id, self.sheet.0, &self.range.0, &self.shift.0)
                .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
            Ok(Outcome::changed(format!(
                "the cells `{}` were inserted, and {} cell(s) moved",
                self.range.0, moved
            ))
            .with_data(json!({
                "sheet": self.sheet.0,
                "range": self.range.0,
                "shift": self.shift.0,
                "moved": moved,
            })))

        }
    }

    /// **Delete** cells from a sheet: the cells beside them move back.
    ///
    /// The cells move **within the band**: the rows of the range when
    /// they move **left**, and its columns when they move **up**.
    grid.cell.delete {
        area: "grid",
        layer: "grid",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The sheet it goes in.
            sheet: NodeRef,
            /// The **range** of cells that are deleted: `A1`, `B2:D4`.
            range: Text,
            /// Which way the cells beside them move: `left` or `up`.
            shift: Text,
        },
        run: {
            let id = "grid.cell.delete".to_string();
            let moved = move_cells(session, &id, self.sheet.0, &self.range.0, &self.shift.0)
                .map_err(|why| EngineError::Failed { id: id.clone(), message: why })?;
            Ok(Outcome::changed(format!(
                "the cells `{}` were deleted, and {} cell(s) moved",
                self.range.0, moved
            ))
            .with_data(json!({
                "sheet": self.sheet.0,
                "range": self.range.0,
                "shift": self.shift.0,
                "moved": moved,
            })))

        }
    }
}
