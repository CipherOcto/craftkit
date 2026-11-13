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
                    if entry.is_empty() {
                        continue;
                    }
                    cells.insert(address, craftkit_sdk::serde_json::Value::Object(entry));
                }
                let data = json!({
                    "cells": craftkit_sdk::serde_json::Value::Object(cells),
                    "view": crate::view::SheetView::default().to_json(),
                    "zoom": 100,
                });
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
            Ok(Outcome::changed(summary).with_data(json!({
                "id": workbook_id,
                "name": name,
                "sheets": workbook.sheets.len(),
                "cells": filled,
                "formulas": formulas,
                "unread_formulas": unread,
                "formats": formats,
                "unread_formats": unread_formats,
                "names": names,
                "unread_names": unread_names,
                "filters": imported_filters,
                "unread_filters": unread_filters,
                "system": system.name(),
                "warnings": workbook.warnings,
            })))

        }
    }
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
