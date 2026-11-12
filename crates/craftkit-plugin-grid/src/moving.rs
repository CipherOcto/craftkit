//! The **facts that move**.
//!
//! A sheet holds facts that name an **area** of cells: a conditional
//! format, a validation, a sparkline, a table, a pivot. When rows or
//! columns are inserted or deleted, those facts move with the area
//! they name — and the area they name may grow or shrink, because the
//! area is a pair of corners and the corners move.
//!
//! So there is **one** way of moving an area here, and the four
//! commands that move things — the row that is inserted, the row that
//! is deleted, the column that is inserted, the column that is
//! deleted — use it.
//!
//! The rules:
//!
//! - an area that is **wholly** in the band that is deleted goes away
//!   with it: a conditional format of rows that are gone is a
//!   conditional format of nothing;
//! - an area that **covers part** of the band refuses: the tree has
//!   no `#REF!` yet, and an area that names half of what is gone is
//!   not an area;
//! - an area that is **beside** the band moves with it, and an area
//!   the band falls **inside** grows or shrinks by the size of the
//!   band.

use craftkit_sdk::serde_json::{Map, Value as Json};

use crate::formula::{Anchor, CellRef};

/// The way a fact moves: with the **rows**, or with the **columns**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Way {
    /// With the rows.
    Rows,
    /// With the columns.
    Columns,
}

/// The **corners** of an area, from the text that names one.
///
/// A text that names one cell has one corner twice.
pub fn corners(text: &str) -> Option<(CellRef, CellRef)> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), last.trim()),
        None => (trimmed, trimmed),
    };
    let start = CellRef::parse(first)?;
    let end = CellRef::parse(last)?;
    Some((start, end))
}

/// The **number** of a corner in the way that is moving: the row of a
/// cell, or its column.
fn number(cell: CellRef, way: Way) -> u64 {
    match way {
        Way::Rows => cell.row as u64 + 1,
        Way::Columns => cell.col as u64 + 1,
    }
}

/// Whether an area is **wholly** in the band that is deleted.
pub fn wholly_inside(start: CellRef, end: CellRef, way: Way, at: u64, count: u64) -> bool {
    let first = number(start, way);
    let last = number(end, way);
    first >= at && last < at + count
}

/// Whether an area **covers part** of the band that is deleted.
pub fn covers_inside(start: CellRef, end: CellRef, way: Way, at: u64, count: u64) -> bool {
    let first = number(start, way);
    let last = number(end, way);
    first < at + count && last >= at
}

/// The **area** a text names, moved.
///
/// It is `None` when the area names a row or a column that is
/// deleted.
pub fn moved_area(text: &str, way: Way, at: u64, count: u64, insert: bool) -> Option<String> {
    let trimmed = text.trim();
    let (first, last) = match trimmed.split_once(':') {
        Some((first, last)) => (first.trim(), Some(last.trim())),
        None => (trimmed, None),
    };
    let anchor = Anchor::parse(first)?;
    let moved = match (way, insert) {
        (Way::Rows, true) => anchor.inserted(at, count),
        (Way::Rows, false) => anchor.deleted(at, count)?,
        (Way::Columns, true) => anchor.inserted_column(at, count),
        (Way::Columns, false) => anchor.deleted_column(at, count)?,
    };
    match last {
        Some(last) => {
            let anchor = Anchor::parse(last)?;
            let last_moved = match (way, insert) {
                (Way::Rows, true) => anchor.inserted(at, count),
                (Way::Rows, false) => anchor.deleted(at, count)?,
                (Way::Columns, true) => anchor.inserted_column(at, count),
                (Way::Columns, false) => anchor.deleted_column(at, count)?,
            };
            Some(format!("{}:{}", moved.to_text(), last_moved.to_text()))
        }
        None => Some(moved.to_text()),
    }
}

/// The **cell** an address names, moved.
///
/// It is `None` when the cell is in the band that is deleted.
pub fn moved_cell(text: &str, way: Way, at: u64, count: u64, insert: bool) -> Option<String> {
    let cell = CellRef::parse(text)?;
    let number = number(cell, way);
    let moved = match (way, insert) {
        (Way::Rows, true) => {
            if number < at {
                cell
            } else {
                CellRef { col: cell.col, row: cell.row + count as u32 }
            }
        }
        (Way::Rows, false) => {
            if number >= at && number < at + count {
                return None;
            }
            if number >= at + count { CellRef { col: cell.col, row: cell.row - count as u32 } } else { cell }
        }
        (Way::Columns, true) => {
            if number < at {
                cell
            } else {
                CellRef { col: cell.col + count as u32, row: cell.row }
            }
        }
        (Way::Columns, false) => {
            if number >= at && number < at + count {
                return None;
            }
            if number >= at + count { CellRef { col: cell.col - count as u32, row: cell.row } } else { cell }
        }
    };
    Some(moved.to_text())
}

/// The **ranges** of a list of facts: the ranges move and the list is
/// written again.
///
/// A fact whose area is wholly in the band that is deleted goes away
/// with it, and a fact whose area covers part of the band refuses,
/// because an area that names half of what is gone is not an area.
pub fn moved_ranges(value: &Json, way: Way, at: u64, count: u64, insert: bool) -> Result<Json, String> {
    let Some(list) = value.as_array() else { return Ok(value.clone()) };
    let mut moved = Vec::with_capacity(list.len());
    for fact in list {
        let Some(text) = fact.get("range").and_then(|range| range.as_str()) else {
            moved.push(fact.clone());
            continue;
        };
        let Some((start, end)) = corners(text) else {
            moved.push(fact.clone());
            continue;
        };
        if !insert {
            if wholly_inside(start, end, way, at, count) {
                continue;
            }
            if covers_inside(start, end, way, at, count) {
                return Err(format!("the range `{text}` names a {} that is deleted", word(way)));
            }
        }
        let area = moved_area(text, way, at, count, insert)
            .ok_or_else(|| format!("the range `{text}` names a {} that is deleted", word(way)))?;
        let mut moved_fact = fact.clone();
        moved_fact["range"] = Json::String(area);
        moved.push(moved_fact);
    }
    Ok(Json::Array(moved))
}

/// The **sparklines** of a sheet: the cells move and the ranges move.
///
/// A sparkline of a cell that is deleted goes away with the cell.
pub fn moved_sparklines(value: &Json, way: Way, at: u64, count: u64, insert: bool) -> Result<Json, String> {
    let Some(map) = value.as_object() else { return Ok(value.clone()) };
    let mut moved = Map::new();
    for (cell, sparkline) in map {
        let Some(address) = moved_cell(cell, way, at, count, insert) else { continue };
        let mut moved_sparkline = sparkline.clone();
        if let Some(text) = sparkline.get("range").and_then(|range| range.as_str())
            && let Some((start, end)) = corners(text)
        {
            if !insert {
                if wholly_inside(start, end, way, at, count) {
                    continue;
                }
                if covers_inside(start, end, way, at, count) {
                    return Err(format!("the range `{text}` names a {} that is deleted", word(way)));
                }
            }
            let area = moved_area(text, way, at, count, insert)
                .ok_or_else(|| format!("the range `{text}` names a {} that is deleted", word(way)))?;
            moved_sparkline["range"] = Json::String(area);
        }
        moved.insert(address, moved_sparkline);
    }
    Ok(Json::Object(moved))
}

/// The **column** a letter names, moved.
///
/// It is `None` when the column is deleted.
pub fn moved_column(text: &str, at: u64, count: u64, insert: bool) -> Option<String> {
    let index = CellRef::column_index(text.trim())?;
    let number = index as u64 + 1;
    let moved = if insert {
        if number < at { number } else { number + count }
    } else {
        if number >= at && number < at + count {
            return None;
        }
        if number >= at + count { number - count } else { number }
    };
    Some(CellRef::column_text(moved as u32 - 1))
}

/// The **pivots** of a sheet: the ranges move and the fields move with
/// them.
///
/// A pivot gathers by **columns**, and those columns are columns of
/// the range it reads, so they move with the range — and a pivot that
/// gathers by a column that is deleted refuses the delete, because a
/// pivot that gathers by nothing is not a pivot.
pub fn moved_pivots(value: &Json, way: Way, at: u64, count: u64, insert: bool) -> Result<Json, String> {
    let Some(list) = value.as_array() else { return Ok(value.clone()) };
    let mut moved = Vec::with_capacity(list.len());
    for fact in list {
        let Some(text) = fact.get("range").and_then(|range| range.as_str()) else {
            moved.push(fact.clone());
            continue;
        };
        let Some((start, end)) = corners(text) else {
            moved.push(fact.clone());
            continue;
        };
        if !insert && wholly_inside(start, end, way, at, count) {
            continue;
        }
        let mut moved_fact = fact.clone();
        if way == Way::Columns {
            for key in ["row", "column", "value"] {
                let Some(letter) = moved_fact.get(key).and_then(|letter| letter.as_str()) else {
                    continue;
                };
                let Some(moved_letter) = moved_column(letter, at, count, insert) else {
                    return Err(format!("the pivot gathers by the column {letter}, which is deleted"));
                };
                moved_fact[key] = Json::String(moved_letter);
            }
        }
        if !insert && covers_inside(start, end, way, at, count) {
            return Err(format!("the range `{text}` names a {} that is deleted", word(way)));
        }
        let area = moved_area(text, way, at, count, insert)
            .ok_or_else(|| format!("the range `{text}` names a {} that is deleted", word(way)))?;
        moved_fact["range"] = Json::String(area);
        moved.push(moved_fact);
    }
    Ok(Json::Array(moved))
}

/// A **corner** of an area, moved.
///
/// It is `None` when the corner is in the band that is deleted.
fn moved_corner(cell: CellRef, way: Way, at: u64, count: u64, insert: bool) -> Option<CellRef> {
    let number = match way {
        Way::Rows => cell.row as u64 + 1,
        Way::Columns => cell.col as u64 + 1,
    };
    let moved = if insert {
        if number < at {
            cell
        } else {
            match way {
                Way::Rows => CellRef { col: cell.col, row: cell.row + count as u32 },
                Way::Columns => CellRef { col: cell.col + count as u32, row: cell.row },
            }
        }
    } else {
        if number >= at && number < at + count {
            return None;
        }
        if number >= at + count {
            match way {
                Way::Rows => CellRef { col: cell.col, row: cell.row - count as u32 },
                Way::Columns => CellRef { col: cell.col - count as u32, row: cell.row },
            }
        } else {
            cell
        }
    };
    Some(moved)
}

/// The **area** an area is, moved.
///
/// It is `None` when a corner of it is in the band that is deleted.
pub fn moved_area_of(
    area: &crate::formula::Area,
    way: Way,
    at: u64,
    count: u64,
    insert: bool,
) -> Option<crate::formula::Area> {
    let start = moved_corner(area.start, way, at, count, insert)?;
    let end = moved_corner(area.end, way, at, count, insert)?;
    Some(crate::formula::Area::new(start, end))
}

/// The **charts** of a sheet: the ranges they read move.
///
/// It is the chart's **id** and the range it reads that is returned, so
/// that the caller may write them back where they belong. A chart that
/// reads a range that is wholly deleted has no range left to read, and
/// it is **taken away**: `None` says the chart goes.
pub fn moved_charts(
    sheet: &craftkit_sdk::Node,
    way: Way,
    at: u64,
    count: u64,
    insert: bool,
) -> Result<Vec<(u64, Option<String>)>, String> {
    let mut moved = Vec::new();
    for child in sheet.children.iter() {
        if child.kind != "grid.chart" {
            continue;
        }
        let Some(text) = child.data.get("range").and_then(|range| range.as_str()) else { continue };
        let Some((start, end)) = corners(text) else { continue };
        if !insert {
            if wholly_inside(start, end, way, at, count) {
                moved.push((child.id, None));
                continue;
            }
            if covers_inside(start, end, way, at, count) {
                return Err(format!("the chart reads `{text}`, which names a {} that is deleted", word(way)));
            }
        }
        let area = moved_area(text, way, at, count, insert).ok_or_else(|| {
            format!("the chart reads `{text}`, which names a {} that is deleted", word(way))
        })?;
        moved.push((child.id, Some(area)));
    }
    Ok(moved)
}

/// The **names** of the workbook that point at a sheet: their areas
/// move with the sheet.
///
/// It is the workbook's **id** and its names that are returned, and it
/// is `None` when no name of the workbook points at the sheet.
pub fn moved_names(
    session: &craftkit_sdk::Session,
    sheet: u64,
    name: &str,
    way: Way,
    at: u64,
    count: u64,
    insert: bool,
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
    let mut moved = Map::new();
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
        if !insert {
            if wholly_inside(area.start, area.end, way, at, count) {
                changed = true;
                continue;
            }
            if covers_inside(area.start, area.end, way, at, count) {
                return Err(format!("the name `{name_of_name}` points at a {} that is deleted", word(way)));
            }
        }
        let Some(moved_area) = moved_area_of(&area, way, at, count, insert) else {
            return Err(format!("the name `{name_of_name}` points at a {} that is deleted", word(way)));
        };
        changed = true;
        moved.insert(name_of_name.clone(), Json::String(format!("{named}!{}", moved_area.to_text())));
    }
    if !changed {
        return Ok(None);
    }
    Ok(Some((workbook.id, Json::Object(moved))))
}

/// The **word** a way is called, in a message.
fn word(way: Way) -> &'static str {
    match way {
        Way::Rows => "row",
        Way::Columns => "column",
    }
}
