//! The **import** of a workbook file.
//!
//! A workbook file — an **XLSX** file — is a package of XML parts, and
//! the reader of §904 reads it as a workbook: its sheets, their names
//! and their cells. This module puts that workbook **in the document**:
//! a workbook node, a sheet node for each sheet, and the cells of each
//! sheet in the sheet.
//!
//! # The rules
//!
//! - The file is **read within a limit**: the bytes are bounded before
//!   the file is read, so a file of ten gigabytes is refused without
//!   being read.
//! - A file that is **not a workbook** is refused, and the reason is
//!   the reader's own.
//! - The **date system** of the file is the date system of the
//!   workbook: a number counted in the 1904 system is not a number
//!   counted in the 1900 one, and the workbook says which.
//! - A **sheet's name** is the tree's: a name that the tree refuses is
//!   refused here too, and the import says which name it was.
//! - A **formula** the tree cannot read is not imported as a formula:
//!   the cell keeps the value the file cached for it, and the import
//!   says how many formulas were not read. A formula that is quietly
//!   wrong is worse than a formula that is missing.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Outcome, Text};

use crate::formula::{CellRef, Value};

/// The most **bytes** a workbook file may hold.
const MAX_BYTES: u64 = 512 * 1024 * 1024;

/// The most **cells** the links of one sheet may cover.
///
/// The tree's own bound: a file may put one link on a whole column,
/// and a sheet of ten thousand cells of links is enough.
const MAX_LINK_CELLS: u64 = 10_000;

/// The most **bytes** the targets of the links of one sheet may hold.
///
/// The tree's own bound: the same link is copied to every cell of its
/// range, so the length of a target times the cells it covers must
/// stay bounded.
const MAX_LINK_BYTES: u64 = 8 * 1024 * 1024;

/// The **colour** a border has when the file names none.
///
/// The tree's own colour: a line with no colour of its own is black.
const DEFAULT_COLOUR: &str = "#000000";

/// The **text of a file's formula**, as the tree reads it.
///
/// A file spells some functions with a prefix — `_xlfn.` — and some
/// parameters with another — `_xlpm.` — and the tree knows neither.
/// The prefixes are dropped, and what is left is the tree's own text.
fn tree_formula(text: &str) -> String {
    text.replace("_xlfn.", "").replace("_xlpm.", "")
}

commands! {
    /// **Import** a workbook file into the document.
    ///
    /// The file is an **XLSX** file: a package of XML parts. It becomes
    /// a workbook with a sheet for each sheet the file holds, and the
    /// cells of each sheet go into that sheet.
    ///
    /// The workbook is **not** put under another workbook: a file holds
    /// a whole workbook, with its own sheets and its own date system,
    /// and it becomes one.
    grid.import.xlsx {
        area: "grid",
        layer: "grid",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The file to read.
            path: Text,
            /// What to call the workbook. Without it, the name of the
            /// file.
            name: Option<Text>,
        },
        run: {
            let id = "grid.import.xlsx".to_string();
            let path = self.path.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed { id: id.clone(), message: "a file has a path".to_string() });
            }
            // The **bytes** are bounded before the file is read.
            let bytes = craftkit_sdk::files::read_bounded(std::path::Path::new(&path), MAX_BYTES)
                .map_err(|message| {
                    EngineError::Failed { id: id.clone(), message: format!("the file cannot be read: {message}") }
                })?;
            let workbook = craftkit_xlsx::read(&bytes)
                .map_err(|error| {
                    EngineError::Failed { id: id.clone(), message: error.to_string() }
                })?;
            if workbook.sheets.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the file holds no sheet".to_string(),
                });
            }
            // The **name** of the workbook: the name of the file, unless
            // the caller names it.
            let name = match &self.name {
                Some(name) if !name.0.trim().is_empty() => name.0.trim().to_string(),
                _ => std::path::Path::new(&path)
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_else(|| "the workbook".to_string()),
            };
            if let Some(reason) = crate::sheetnames::why_not(&name, None) {
                return Err(EngineError::Failed { id: id.clone(), message: reason });
            }
            // The **system** the file's numbers are counted in.
            let system = match workbook.system {
                craftkit_xlsx::DateSystem::Y1900 => crate::dates::System::Y1900,
                craftkit_xlsx::DateSystem::Y1904 => crate::dates::System::Y1904,
            };
            // The **workbook** itself.
            let workbook_id = session.document_mut()?.add_node_with_under(
                None,
                name.clone(),
                "grid.workbook",
                None,
                json!({
                    "date_system": system.name(),
                    "custom_lists": Vec::<String>::new(),
                    "styles": Vec::<String>::new(),
                }),
            );
            // The **sheets**: one for each sheet of the file, in the
            // order the file holds them.
            let mut filled = 0u64;
            let mut formulas = 0u64;
            let mut unread = 0u64;
            let mut formats = 0u64;
            let mut unread_formats = 0u64;
            let mut fonts = 0u64;
            let mut fills = 0u64;
            let mut aligns = 0u64;
            let mut unread_aligns = 0u64;
            let mut imported_borders = 0u64;
            let mut names = 0u64;
            let mut unread_names = 0u64;
            let mut taken: Vec<String> = Vec::new();
            for sheet in &workbook.sheets {
                // The **name** of the sheet: the tree's rules, and the
                // tree's uniqueness.
                let mut sheet_name = sheet.name.clone();
                if let Some(reason) = crate::sheetnames::why_not(&sheet_name, None) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the sheet `{}` cannot be named here: {reason}", sheet.name),
                    });
                }
                if taken.iter().any(|other| other.eq_ignore_ascii_case(&sheet_name)) {
                    // Two sheets of one name: the second is given a
                    // name of its own, as the tree's reader does.
                    let mut number = 2;
                    loop {
                        let candidate = format!("{} ({number})", sheet_name);
                        if !taken.iter().any(|other| other.eq_ignore_ascii_case(&candidate)) {
                            sheet_name = candidate;
                            break;
                        }
                        number += 1;
                    }
                }
                taken.push(sheet_name.clone());
                // The **cells**: the file's cells, read.
                let mut cells = craftkit_sdk::serde_json::Map::new();
                // The **borders** of the sheet: the lines on the edges
                // of its cells, by the address of the cell they are on.
                let mut borders_of_sheet: std::collections::BTreeMap<
                    String,
                    std::collections::BTreeMap<String, Json>,
                > = std::collections::BTreeMap::new();
                for cell in &sheet.cells {
                    let address = CellRef { col: cell.column, row: cell.row }.to_text();
                    let mut entry = craftkit_sdk::serde_json::Map::new();
                    if let Some(formula) = &cell.formula {
                        let text = tree_formula(formula);
                        // A formula the tree cannot read is not a
                        // formula: the cell keeps the value the file
                        // cached for it, and the import says so.
                        if crate::formula::parse(&text).is_ok() {
                            entry.insert("formula".to_string(), json!(text));
                            formulas += 1;
                        } else {
                            unread += 1;
                        }
                    }
                    let value = value_of(cell.value.clone());
                    if let Some(value) = value {
                        entry.insert("value".to_string(), value.to_json());
                        filled += 1;
                    }
                    // The **format** of the cell: the file's own code,
                    // and only when this tree's format language can
                    // read it. A code that is not read is not carried:
                    // a format that is quietly wrong is worse than a
                    // format that is missing.
                    if let Some(code) = &cell.format
                        && crate::format::Format::parse(code).is_some()
                    {
                        entry.insert("format".to_string(), json!(code));
                        formats += 1;
                    } else if cell.format.is_some() {
                        unread_formats += 1;
                    }
                    // The **font** of the cell: the face of its text
                    // and the marks on it.
                    if let Some(font) = &cell.font {
                        entry.insert("font".to_string(), font_of(font));
                        fonts += 1;
                    }
                    // The **fill** of the cell: the colour behind its
                    // text. A fill that is not a colour — a pattern —
                    // is not carried.
                    if let Some(fill) = &cell.fill
                        && let Some(colour) = &fill.colour
                    {
                        entry.insert("fill".to_string(), json!(colour));
                        fills += 1;
                    }
                    // The **alignment** of the cell: how its text lies.
                    // An alignment this tree does not have is not
                    // carried: a way of lying that is quietly wrong is
                    // worse than one that is missing.
                    if let Some(align) = &cell.align {
                        match align_of(align) {
                            Some(align) => {
                                entry.insert("align".to_string(), align);
                                aligns += 1;
                            }
                            None => unread_aligns += 1,
                        }
                    }
                    // The **borders** of the cell: the lines on its
                    // edges. They are the sheet's, and not the cell's,
                    // as they are here.
                    if let Some(borders) = &cell.borders {
                        for border in &borders.edges {
                            let edge = match border.edge.as_str() {
                                "top" | "right" | "bottom" | "left" => border.edge.as_str(),
                                _ => continue,
                            };
                            let colour = border.colour.clone().unwrap_or_else(|| DEFAULT_COLOUR.to_string());
                            let line = json!({ "style": border.style.clone(), "colour": colour });
                            let edges = borders_of_sheet
                                .entry(address.clone())
                                .or_default();
                            edges.insert(edge.to_string(), line);
                            imported_borders += 1;
                        }
                    }
                    if entry.is_empty() {
                        continue;
                    }
                    cells.insert(address, craftkit_sdk::serde_json::Value::Object(entry));
                }
                let mut data = json!({
                    "cells": craftkit_sdk::serde_json::Value::Object(cells),
                    "view": crate::view::SheetView::default().to_json(),
                    "zoom": 100,
                });
                if !borders_of_sheet.is_empty() {
                    let mut borders: craftkit_sdk::serde_json::Map<String, Json> =
                        craftkit_sdk::serde_json::Map::new();
                    for (address, edges) in borders_of_sheet {
                        let mut one: craftkit_sdk::serde_json::Map<String, Json> =
                            craftkit_sdk::serde_json::Map::new();
                        for (edge, line) in edges {
                            one.insert(edge, line);
                        }
                        borders.insert(address, Json::Object(one));
                    }
                    data["borders"] = Json::Object(borders);
                }
                session.document_mut()?.add_node_with_under(
                    Some(workbook_id),
                    sheet_name,
                    "grid.sheet",
                    None,
                    data,
                );
            }
            // The **names** of the workbook: the words its formulas use.
            //
            // A name here stands for a **range**, and so a name whose
            // text is not a range — a number, a function — is not a
            // name this tree has, and it is not imported.
            let mut imported_names = craftkit_sdk::serde_json::Map::new();
            let sheets: Vec<String> =
                workbook.sheets.iter().map(|sheet| sheet.name.clone()).collect();
            for defined in &workbook.names {
                let text = defined.refers_to.trim().trim_start_matches('=').trim();
                // A name of **one sheet** may say the sheet by its
                // number instead of its name, and the scope says which
                // sheet that is.
                let text = match text.rsplit_once('!') {
                    Some(_) => text.to_string(),
                    None => match &defined.scope {
                        Some(sheet) => format!("{sheet}!{text}"),
                        None => text.to_string(),
                    },
                };
                match crate::names::parse_refers_to(&text) {
                    Ok((sheet, area)) => {
                        // The **sheet** the name stands in must be one
                        // of the sheets that came in.
                        let Some(sheet) = sheets.iter().find(|name| name.eq_ignore_ascii_case(&sheet)) else {
                            unread_names += 1;
                            continue;
                        };
                        if let Some(reason) = crate::names::why_not(&defined.name) {
                            unread_names += 1;
                            let _ = reason;
                            continue;
                        }
                        let refers_to = format!("{sheet}!{}", area_text(&area));
                        imported_names.insert(defined.name.clone(), json!(refers_to));
                        names += 1;
                    }
                    Err(_) => unread_names += 1,
                }
            }
            if !imported_names.is_empty() {
                let names = craftkit_sdk::serde_json::Value::Object(imported_names);
                if let Some(node) = session.document_mut()?.node_mut(workbook_id) {
                    node.data["names"] = names;
                }
            }
            // The **filters** of the sheets: the rows each sheet hides.
            //
            // A file's filter is over a **range** and its criteria are
            // the **offsets** of their columns in that range; a filter
            // here is over **one column** and the column is the column
            // itself. So the offset is counted from the first column
            // of the range, and a criterion this tree's filter cannot
            // hold is not imported.
            let mut imported_filters = 0u64;
            let mut unread_filters = 0u64;
            for (sheet_name, filter) in &workbook.filters {
                let Some(filter) = filter_of(filter) else {
                    unread_filters += 1;
                    continue;
                };
                let Some(sheet_id) = session
                    .document()
                    .walk()
                    .find(|node| node.kind == "grid.sheet" && node.name == *sheet_name)
                    .map(|node| node.id)
                else {
                    continue;
                };
                if let Some(node) = session.document_mut()?.node_mut(sheet_id) {
                    node.data["filter"] = filter;
                }
                imported_filters += 1;
            }
            // The **links** of the sheets: the places their cells go
            // to.
            //
            // A file's link is over a **range** of cells and is copied
            // to each of them, and a link here is on **one cell** — so
            // the range is walked, within the tree's own bounds.
            let mut imported_links = 0u64;
            let mut dropped_links = 0u64;
            for (sheet_name, links) in &workbook.links {
                let Some(sheet_id) = session
                    .document()
                    .walk()
                    .find(|node| node.kind == "grid.sheet" && node.name == *sheet_name)
                    .map(|node| node.id)
                else {
                    continue;
                };
                let mut written: Vec<(String, Json)> = Vec::new();
                let mut cells = 0u64;
                let mut bytes = 0u64;
                for link in links {
                    let link_bytes = (link.target.len() + link.tooltip.as_ref().map_or(0, |tooltip| tooltip.len())) as u64;
                    for row in link.row..=link.last_row {
                        for column in link.column..=link.last_column {
                            if cells >= MAX_LINK_CELLS {
                                dropped_links += 1;
                                break;
                            }
                            if bytes.saturating_add(link_bytes) > MAX_LINK_BYTES {
                                dropped_links += 1;
                                break;
                            }
                            cells += 1;
                            bytes = bytes.saturating_add(link_bytes);
                            let address =
                                crate::formula::CellRef { col: column, row: row as u32 }.to_text();
                            let mut one = craftkit_sdk::serde_json::Map::new();
                            one.insert("target".to_string(), json!(link.target.clone()));
                            if let Some(tooltip) = &link.tooltip {
                                one.insert("tooltip".to_string(), json!(tooltip.clone()));
                            }
                            written.push((address, craftkit_sdk::serde_json::Value::Object(one)));
                            imported_links += 1;
                        }
                    }
                }
                if written.is_empty() {
                    continue;
                }
                if let Some(node) = session.document_mut()?.node_mut(sheet_id) {
                    let mut map = node
                        .data
                        .get("links")
                        .and_then(|links| links.as_object())
                        .cloned()
                        .unwrap_or_default();
                    for (address, link) in written {
                        map.insert(address, link);
                    }
                    node.data["links"] = craftkit_sdk::serde_json::Value::Object(map);
                }
            }
            // The **comments** of the sheets: the notes their cells
            // carry.
            let mut imported_comments = 0u64;
            for (sheet_name, comments) in &workbook.comments {
                let Some(sheet_id) = session
                    .document()
                    .walk()
                    .find(|node| node.kind == "grid.sheet" && node.name == *sheet_name)
                    .map(|node| node.id)
                else {
                    continue;
                };
                if comments.is_empty() {
                    continue;
                }
                let mut map = craftkit_sdk::serde_json::Map::new();
                for comment in comments {
                    let address =
                        crate::formula::CellRef { col: comment.column, row: comment.row }.to_text();
                    let mut one = craftkit_sdk::serde_json::Map::new();
                    one.insert("text".to_string(), json!(comment.text.clone()));
                    if !comment.author.is_empty() {
                        one.insert("author".to_string(), json!(comment.author.clone()));
                    }
                    one.insert("resolved".to_string(), json!(false));
                    one.insert("replies".to_string(), json!(Vec::<String>::new()));
                    map.insert(address, craftkit_sdk::serde_json::Value::Object(one));
                    imported_comments += 1;
                }
                if let Some(node) = session.document_mut()?.node_mut(sheet_id) {
                    node.data["comments"] = craftkit_sdk::serde_json::Value::Object(map);
                }
            }
            // The **tables** of the sheets: the ranges with a name and
            // a way of being totalled.
            let mut imported_tables = 0u64;
            for (sheet_name, tables) in &workbook.tables {
                let Some(sheet_id) = session
                    .document()
                    .walk()
                    .find(|node| node.kind == "grid.sheet" && node.name == *sheet_name)
                    .map(|node| node.id)
                else {
                    continue;
                };
                if tables.is_empty() {
                    continue;
                }
                let mut list: Vec<Json> = Vec::new();
                for table in tables {
                    // The **range** of the table: the file writes it
                    // with its dollars, and a range here is written
                    // without them.
                    let range = table.range.replace('$', "");
                    let Some((_, area)) = crate::merges::parse_area(&range) else { continue };
                    let ways: Vec<Json> = table
                        .ways
                        .iter()
                        .map(|(column, way)| json!({ "column": column, "way": way }))
                        .collect();
                    list.push(json!({
                        "name": table.name,
                        "range": area.to_text(),
                        "totals": table.totals,
                        "way": table.way,
                        "label": table.label,
                        "ways": ways,
                    }));
                    imported_tables += 1;
                }
                if list.is_empty() {
                    continue;
                }
                if let Some(node) = session.document_mut()?.node_mut(sheet_id) {
                    node.data["tables"] = Json::Array(list);
                }
            }
            let mut summary = format!("imported {filled} cell(s) into the workbook `{name}`");
            if unread > 0 {
                summary.push_str(&format!("; {unread} formula(s) could not be read and were not imported"));
            }
            if unread_formats > 0 {
                summary.push_str(&format!(
                    "; {unread_formats} format(s) could not be read and were not imported"
                ));
            }
            if unread_names > 0 {
                summary.push_str(&format!(
                    "; {unread_names} name(s) could not be read and were not imported"
                ));
            }
            if unread_filters > 0 {
                summary.push_str(&format!(
                    "; {unread_filters} filter(s) could not be read and were not imported"
                ));
            }
            if unread_aligns > 0 {
                summary.push_str(&format!(
                    "; {unread_aligns} alignment(s) could not be read and were not imported"
                ));
            }
            Ok(Outcome::changed(summary).with_data(json!({
                "id": workbook_id,
                "name": name,
                "sheets": workbook.sheets.len(),
                "cells": filled,
                "formulas": formulas,
                "unread_formulas": unread,
                "formats": formats,
                "unread_formats": unread_formats,
                "fonts": fonts,
                "fills": fills,
                "aligns": aligns,
                "unread_aligns": unread_aligns,
                "borders": imported_borders,
                "names": names,
                "unread_names": unread_names,
                "filters": imported_filters,
                "unread_filters": unread_filters,
                "links": imported_links,
                "dropped_links": dropped_links,
                "comments": imported_comments,
                "tables": imported_tables,
                "system": system.name(),
                "warnings": workbook.warnings,
            })))

        }
    }
}

/// The **font** of a cell, as this tree's font.
///
/// A file's font names the marks of the text in the file's own words:
/// `singleAccounting` and `doubleAccounting`. This tree's words are
/// `single-accounting` and `double-accounting`, and the names are
/// changed here.
fn font_of(font: &craftkit_xlsx::Font) -> Json {
    let underline = match font.underline {
        craftkit_xlsx::Underline::None => "none",
        craftkit_xlsx::Underline::Single => "single",
        craftkit_xlsx::Underline::Double => "double",
        craftkit_xlsx::Underline::SingleAccounting => "single-accounting",
        craftkit_xlsx::Underline::DoubleAccounting => "double-accounting",
    };
    let vert = match font.vert {
        craftkit_xlsx::Vert::Baseline => "baseline",
        craftkit_xlsx::Vert::Superscript => "superscript",
        craftkit_xlsx::Vert::Subscript => "subscript",
    };
    json!({
        "name": match &font.name {
            Some(name) => json!(name),
            None => Json::Null,
        },
        "size": match font.size {
            Some(size) => json!(size),
            None => Json::Null,
        },
        "bold": font.bold,
        "italic": font.italic,
        "strike": font.strike,
        "underline": underline,
        "vert": vert,
        "colour": match &font.colour {
            Some(colour) => json!(colour),
            None => Json::Null,
        },
    })
}

/// The **alignment** of a cell, as this tree's alignment.
///
/// The names the reader gives are this tree's own, and an alignment
/// this tree does not have is **nothing**: the cell keeps the
/// alignment it had, which is none.
fn align_of(align: &craftkit_xlsx::Alignment) -> Option<Json> {
    let horizontal = align.horizontal.as_deref()?;
    let vertical = align.vertical.as_deref()?;
    if crate::formula::AlignH::parse(horizontal).is_none()
        || crate::formula::AlignV::parse(vertical).is_none()
    {
        return None;
    }
    Some(json!({
        "horizontal": horizontal,
        "vertical": vertical,
        "indent": align.indent,
        "wrap": align.wrap,
    }))
}

/// The **filter** of a sheet, as this tree's filter.
///
/// A file's filter is over a **range**, and the criteria it holds are
/// the **offsets** of their columns in that range: the first column
/// of the range is offset zero. A filter here is over **one column**,
/// and the column is the column itself — so the offset is counted from
/// the first column of the range.
///
/// A file's filter may hold criteria for several columns, and this
/// tree's filter holds one. So a filter of several columns becomes a
/// **group** of criteria, which is what this tree's filter calls the
/// same thing: the rows that match the group are kept.
fn filter_of(filter: &craftkit_xlsx::AutoFilter) -> Option<Json> {
    let first = first_column(filter.range.as_deref())?;
    let mut group: Vec<Json> = Vec::new();
    for (offset, criterion) in &filter.criteria {
        let column = first + offset;
        match criterion {
            craftkit_xlsx::FilterCriterion::Custom { first: (op, value), second, and } => {
                let op = op_of(op);
                if !crate::formula::Criterion::op_is_known(op) {
                    return None;
                }
                group.push(json!({ "column": crate::formula::CellRef::column_text(column), "op": op, "value": value }));
                if let Some((op, value)) = second {
                    let op = op_of(op);
                    if !crate::formula::Criterion::op_is_known(op) {
                        return None;
                    }
                    group.push(json!({ "column": crate::formula::CellRef::column_text(column), "op": op, "value": value }));
                }
                if !and {
                    // Two comparisons joined by **or**: this tree's
                    // group joins by **and**, so two groups of one
                    // criterion each are what an **or** is.
                    return None;
                }
            }
            _ => return None,
        }
    }
    if group.is_empty() { None } else { Some(json!({ "groups": [group] })) }
}

/// The **column** a range begins at, counted from zero.
fn first_column(range: Option<&str>) -> Option<u32> {
    let range = range?;
    let first = range.split(':').next()?.trim();
    let letters: String = first.chars().take_while(|character| character.is_ascii_alphabetic()).collect();
    crate::formula::CellRef::column_index(&letters)
}

/// The **operation** a file's filter writes, as this tree writes it.
fn op_of(op: &str) -> &str {
    match op {
        "<>" => "!=",
        other => other,
    }
}

/// The **area** of a range, written back the way it is written.
fn area_text(area: &crate::formula::Area) -> String {
    if area.start == area.end {
        area.start.to_text()
    } else {
        format!("{}:{}", area.start.to_text(), area.end.to_text())
    }
}

/// The **value** of a cell, as the tree's own value.
fn value_of(value: craftkit_xlsx::Value) -> Option<Value> {
    match value {
        craftkit_xlsx::Value::Empty => None,
        craftkit_xlsx::Value::Number(number) => Some(Value::Number(number)),
        craftkit_xlsx::Value::Text(text) => Some(Value::Text(text)),
        craftkit_xlsx::Value::Bool(flag) => Some(Value::Bool(flag)),
        craftkit_xlsx::Value::Error(message) => Some(Value::Error(message)),
    }
}
