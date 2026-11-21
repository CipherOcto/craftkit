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

/// The most **cells** one merge may cover.
///
/// A file may name a whole column as merged, and the walk of the cells
/// of that range would be a million cells for one merge. A merge that
/// covers more cells than this is not imported, and the import says so.
const MAX_MERGE_CELLS: u64 = 10_000;

/// The **text of a file's formula**, as the tree reads it.
///
/// The text is the **file's own**: a formula that a file spells with a
/// prefix — `_xlfn.XLOOKUP` — is kept as it was written, and the
/// parser reads the name after the prefix. The prefix belongs to a
/// **function's name** and to nothing else, so a text that holds one
/// is a text that holds it.
fn tree_formula(text: &str) -> String {
    text.to_string()
}

/// The **properties** of a workbook, as the answer holds them.
///
/// A property the file does not say is nothing, and the answer says
/// so.
fn properties_to_json(properties: &craftkit_xlsx::Properties) -> craftkit_sdk::serde_json::Value {
    let text = |value: &Option<String>| match value {
        Some(value) => json!(value),
        None => craftkit_sdk::serde_json::Value::Null,
    };
    json!({
        "title": text(&properties.title),
        "subject": text(&properties.subject),
        "author": text(&properties.creator),
        "keywords": text(&properties.keywords),
        "description": text(&properties.description),
        "last_modified_by": text(&properties.last_modified_by),
        "company": text(&properties.company),
    })
}

/// The **calculation settings** of a workbook, as the answer holds them.
fn calculation_to_json(calculation: &craftkit_xlsx::Calculation) -> craftkit_sdk::serde_json::Value {
    json!({
        "mode": calculation.mode.name(),
        "iterative": calculation.iterative,
        "max_iterations": calculation.max_iterations,
        "max_change": calculation.max_change,
        "precision_as_displayed": calculation.precision_as_displayed,
    })
}

/// The **theme** of a workbook, as the answer holds it.
///
/// A colour the theme does not have is nothing, and the answer says so.
fn theme_to_json(theme: &craftkit_xlsx::Theme) -> craftkit_sdk::serde_json::Value {
    let mut colours = craftkit_sdk::serde_json::Map::new();
    for (index, name) in craftkit_xlsx::SLOTS.iter().enumerate() {
        let colour = match theme.colour(index as u64) {
            Some(colour) => json!(colour),
            None => craftkit_sdk::serde_json::Value::Null,
        };
        colours.insert(name.to_string(), colour);
    }
    json!({
        "name": match &theme.name {
            Some(name) => json!(name),
            None => craftkit_sdk::serde_json::Value::Null,
        },
        "colours": craftkit_sdk::serde_json::Value::Object(colours),
        "major_font": match &theme.major_font {
            Some(face) => json!(face),
            None => craftkit_sdk::serde_json::Value::Null,
        },
        "minor_font": match &theme.minor_font {
            Some(face) => json!(face),
            None => craftkit_sdk::serde_json::Value::Null,
        },
    })
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
            /// The **password** of the file, when the file is
            /// encrypted.
            password: Option<Text>,
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
            // A file that is **encrypted** is read with the password
            // that opens it, and a file that is not is read without.
            let password = self.password.as_ref().map(|password| password.0.trim().to_string());
            let workbook = match &password {
                Some(password) if !password.is_empty() => {
                    craftkit_xlsx::read_with_password(&bytes, password)
                }
                _ => craftkit_xlsx::read(&bytes),
            }
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
            // The **protection** of the workbook: what the file locks.
            // The tree's protection has no password, so a password the
            // file names is not carried, and it is counted.
            let mut protection = json!({ "structure": false });
            let mut unread_passwords = 0u64;
            if let Some(locked) = &workbook.protection {
                if locked.structure {
                    protection = json!({ "structure": true });
                }
                if locked.password {
                    unread_passwords += 1;
                }
            }
            // The **named styles** of the workbook: the styles the file
            // names, each with the look it carries.
            let mut styles: Vec<Json> = Vec::new();
            let mut unread_style_aligns = 0u64;
            for style in &workbook.named_styles {
                let mut entry = craftkit_sdk::serde_json::Map::new();
                entry.insert("name".to_string(), json!(style.name));
                entry.insert(
                    "format".to_string(),
                    json!(style.format.clone().unwrap_or_else(|| "general".to_string())),
                );
                entry.insert("locked".to_string(), json!(style.locked));
                match style.align.as_ref().map(align_of) {
                    Some(Some(align)) => {
                        entry.insert("align".to_string(), align);
                    }
                    Some(None) => unread_style_aligns += 1,
                    None => {}
                }
                if let Some(font) = style.font.as_ref() {
                    entry.insert("font".to_string(), font_of(font));
                }
                styles.push(Json::Object(entry));
            }
            // The **workbook** itself.
            let workbook_id = session.document_mut()?.add_node_with_under(
                None,
                name.clone(),
                "grid.workbook",
                None,
                json!({
                    "date_system": system.name(),
                    "custom_lists": Vec::<String>::new(),
                    "styles": styles,
                    "properties": properties_to_json(&workbook.properties),
                    "theme": theme_to_json(&workbook.theme),
                    "calculation": calculation_to_json(&workbook.calculation),
                    "objects": workbook.objects.name(),
                    "protection": protection,
                }),
            );
            // The **sheets**: one for each sheet of the file, in the
            // order the file holds them.
            let mut filled = 0u64;
            let mut formulas = 0u64;
            let mut unread = 0u64;
            let mut formats = 0u64;
            let mut unread_formats = 0u64;
            let mut checkboxes = 0u64;
            let mut readings = 0u64;
            let mut fonts = 0u64;
            let mut fills = 0u64;
            let mut aligns = 0u64;
            let mut unread_aligns = 0u64;
            let mut imported_borders = 0u64;
            let mut locks = 0u64;
            let mut protections = 0u64;
            let mut imported_merges = 0u64;
            let mut unread_merges = 0u64;
            let mut views = 0u64;
            let mut imported_conditionals = 0u64;
            let mut unread_conditionals = 0u64;
            let mut imported_validations = 0u64;
            let mut unread_validations = 0u64;
            let mut printed = 0u64;
            let mut unread_prints = 0u64;
            let mut unread_headers = 0u64;
            let mut centred = 0u64;
            let mut marks = 0u64;
            let mut unread_edit_ranges = 0u64;
            let mut unread_range_names = 0u64;
            let mut outlines = 0u64;
            let mut setups = 0u64;
            let mut breaks = 0u64;
            let mut imported_sparklines = 0u64;
            let mut unread_sparklines = 0u64;
            let mut arrays = 0u64;
            let mut data_tables = 0u64;
            let mut unread_setups = 0u64;
            let mut names = 0u64;
            let mut unread_names = 0u64;
            let mut imported_pivots = 0u64;
            let mut unread_pivots = 0u64;
            let mut hidden_sheets = 0u64;
            let mut kept_sheets = 0u64;
            let mut tab_colours = 0u64;
            // The **facts of a row and a column** that the tree's model
            // has no place for: the format of the line, the group of
            // the line that is folded away, and the size the user set.
            // They are read, and they are not kept, and the import says
            // how many of them it left out.
            let mut unread_line_formats = 0u64;
            let mut unread_collapsed_lines = 0u64;
            let mut unread_custom_sizes = 0u64;
            let mut taken: Vec<String> = Vec::new();
            // The **hidden sheets**: a file may hide a sheet, and the
            // tree hides it too. A workbook must keep **one visible
            // sheet** — the tree's own rule — so a file that hides
            // them all keeps its first sheet in sight.
            let all_hidden = !workbook.sheets.is_empty()
                && workbook.sheets.iter().all(|sheet| {
                    matches!(sheet.state.as_deref(), Some("hidden") | Some("veryHidden"))
                });
            for (index, sheet) in workbook.sheets.iter().enumerate() {
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
                    match cell.formula_kind {
                        // A formula this tree does not have — an
                        // **array**, which fills a range of cells, or a
                        // what-if **data table** — is not a formula of a
                        // cell here: the cell keeps the value the file
                        // cached for it, and the import says so.
                        Some(craftkit_xlsx::FormulaKind::Array) => arrays += 1,
                        Some(craftkit_xlsx::FormulaKind::DataTable) => data_tables += 1,
                        None => {
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
                            } else if let Some(shared) = &cell.shared {
                                // The cell **shares** a formula with the cells
                                // of its group: the master's text, moved by the
                                // distance from the master. A text that cannot
                                // be moved is not a formula, and the cell keeps
                                // the value the file cached for it.
                                let text = tree_formula(&shared.text);
                                match crate::formula::shift_in_formula(&text, shared.row_delta, shared.column_delta) {
                                    Some(shifted) if crate::formula::parse(&shifted).is_ok() => {
                                        entry.insert("formula".to_string(), json!(shifted));
                                        formulas += 1;
                                    }
                                    _ => unread += 1,
                                }
                            }
                        }
                    }
                    let value = value_of(cell.value.clone());
                    if let Some(value) = value {
                        entry.insert("value".to_string(), value.to_json());
                        filled += 1;
                    }
                    // The **reading** of the text: the phonetic guide
                    // the file holds beside it.
                    if let Some(phonetic) = &cell.phonetic {
                        entry.insert(
                            "phonetic".to_string(),
                            crate::phonetic::Phonetic {
                                runs: phonetic
                                    .runs
                                    .iter()
                                    .map(|run| crate::phonetic::PhoneticRun {
                                        start: run.start,
                                        end: run.end,
                                        text: run.text.clone(),
                                    })
                                    .collect(),
                                props: phonetic.props.as_ref().map(|props| crate::phonetic::PhoneticProps {
                                    font_id: props.font_id,
                                    kind: props.kind.clone(),
                                    alignment: props.alignment.clone(),
                                }),
                            }
                            .to_json(),
                        );
                        readings += 1;
                    }
                    // The **checkbox** of the cell: a cell control the
                    // file holds beside its cells. The tree's own format
                    // for it is the format of the tree's checkbox, and a
                    // cell that is a checkbox is that and not a number
                    // format: a box is not a way of writing a number.
                    if cell.checkbox {
                        entry.insert("format".to_string(), json!(crate::checkboxes::CHECKBOX));
                        checkboxes += 1;
                    } else if let Some(code) = &cell.format
                        // The **format** of the cell: the file's own code,
                        // and only when this tree's format language can
                        // read it. A code that is not read is not carried:
                        // a format that is quietly wrong is worse than a
                        // format that is missing.
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
                    // The **lock** of the cell: a cell that says of
                    // itself whether it is locked says more than the
                    // sheet, and so its own word comes first.
                    if let Some(protection) = &cell.protection {
                        entry.insert("locked".to_string(), json!(protection.locked));
                        locks += 1;
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
                // The **rows** of the sheet: the height of each row
                // that has one, and the rows that are hidden.
                let mut row_heights: craftkit_sdk::serde_json::Map<String, Json> =
                    craftkit_sdk::serde_json::Map::new();
                let mut hidden_rows: Vec<Json> = Vec::new();
                let mut row_outline: craftkit_sdk::serde_json::Map<String, Json> =
                    craftkit_sdk::serde_json::Map::new();
                for row in &sheet.rows {
                    if let Some(height) = row.height {
                        row_heights.insert(row.row.to_string(), json!(height));
                    }
                    if row.hidden {
                        hidden_rows.push(json!(row.row));
                    }
                    // The **outline level** of the row: how deep in a
                    // group it is. A row in no group is not in the map.
                    if row.outline > 0 {
                        row_outline.insert(row.row.to_string(), json!(row.outline));
                    }
                    // The **format** of the row: a fact of the row, and
                    // the tree's model has no place for it.
                    if row.format.is_some() {
                        unread_line_formats += 1;
                    }
                    // The **group** of the row, folded away: the tree
                    // works its own collapsed mark out from the rows.
                    if row.collapsed {
                        unread_collapsed_lines += 1;
                    }
                    // The **height** the user set: the tree's model has
                    // the height, and not the hand that set it.
                    if row.custom {
                        unread_custom_sizes += 1;
                    }
                }
                // The **columns** of the sheet: the width of each
                // column that has one, and the columns that are hidden.
                let mut column_widths: craftkit_sdk::serde_json::Map<String, Json> =
                    craftkit_sdk::serde_json::Map::new();
                let mut hidden_columns: Vec<Json> = Vec::new();
                let mut column_outline: craftkit_sdk::serde_json::Map<String, Json> =
                    craftkit_sdk::serde_json::Map::new();
                for column in &sheet.columns {
                    let letter = crate::formula::CellRef::column_text(column.column);
                    if let Some(width) = column.width {
                        column_widths.insert(letter.clone(), json!(width));
                    }
                    if column.hidden {
                        hidden_columns.push(json!(letter));
                    }
                    if column.outline > 0 {
                        column_outline.insert(letter.clone(), json!(column.outline));
                    }
                    // The **format** of the column: a fact of the
                    // column, and the tree's model has no place for it.
                    if column.format.is_some() {
                        unread_line_formats += 1;
                    }
                    // The **group** of the column, folded away: the tree
                    // works its own collapsed mark out from the rows.
                    if column.collapsed {
                        unread_collapsed_lines += 1;
                    }
                    // The **width** the user set: the tree's model has
                    // the width, and not the hand that set it.
                    if column.custom {
                        unread_custom_sizes += 1;
                    }
                }
                // The **merges** of the sheet: the ranges that are one
                // cell.
                //
                // A merge is refused when a cell of the range but the
                // top left one holds a value — the value of a merged
                // region is the value of its top left cell, and the
                // values of the others would be lost. The file is
                // right and the tree is careful, so a merge with a
                // value under it is **not** imported, and the import
                // says how many.
                let mut merges: Vec<String> = Vec::new();
                let mut merged_areas: Vec<crate::formula::Area> = Vec::new();
                for range in &sheet.merges {
                    let Some((_, area)) = crate::merges::parse_area(range) else {
                        unread_merges += 1;
                        continue;
                    };
                    if area.start == area.end {
                        unread_merges += 1;
                        continue;
                    }
                    let covered = area.count().saturating_sub(1);
                    if covered > MAX_MERGE_CELLS {
                        // A range of more cells than this is not
                        // walked: the sheet may name a whole column,
                        // and the walk would be a million cells for
                        // one merge.
                        unread_merges += 1;
                        continue;
                    }
                    let mut holds = false;
                    for row in area.start.row..=area.end.row {
                        for col in area.start.col..=area.end.col {
                            let cell = crate::formula::CellRef { col, row };
                            if cell == area.start {
                                continue;
                            }
                            if cells.get(&cell.to_text()).is_some_and(|entry| {
                                entry.get("value").is_some_and(|value| !value.is_null())
                                    || entry.get("formula").is_some()
                            }) {
                                holds = true;
                                break;
                            }
                        }
                        if holds {
                            break;
                        }
                    }
                    if holds {
                        unread_merges += 1;
                        continue;
                    }
                    // A merge that **touches** another one is not
                    // imported either: a cell that is in two regions is
                    // in none of them.
                    if merged_areas.iter().any(|other| crate::merges::areas_touch(other, &area)) {
                        unread_merges += 1;
                        continue;
                    }
                    merged_areas.push(area);
                    merges.push(format!("{}:{}", area.start.to_text(), area.end.to_text()));
                    imported_merges += 1;
                }
                // The **pivot tables** of the sheet: they become the
                // tree's own pivots, when a pivot of this tree can be
                // the same pivot.
                let mut pivots: Vec<crate::pivots::Pivot> = Vec::new();
                for pivot in &sheet.pivots {
                    let Some(pivot) = pivot_of(pivot, &sheet.name, &cells) else {
                        unread_pivots += 1;
                        continue;
                    };
                    if pivots.iter().any(|other| other.name == pivot.name) {
                        // Two pivots of one name are one pivot: the
                        // tree's own rule.
                        unread_pivots += 1;
                        continue;
                    }
                    pivots.push(pivot);
                    imported_pivots += 1;
                }
                let mut data = json!({
                    "cells": craftkit_sdk::serde_json::Value::Object(cells),
                    "view": crate::view::SheetView::default().to_json(),
                    "zoom": 100,
                });
                if !pivots.is_empty() {
                    data["pivots"] = Json::Array(pivots.iter().map(crate::pivots::pivot_json).collect());
                }
                // The **view** of the sheet: what is drawn of it, and
                // what is not. A sheet with no view of its own has the
                // default one, and the default one is what is written.
                if let Some(view) = &sheet.view {
                    data["view"] = json!({
                        "gridlines": view.gridlines,
                        "headings": view.headings,
                        "zeros": view.zeros,
                        "right_to_left": view.right_to_left,
                        "outline_symbols": view.outline_symbols,
                        "show_formulas": view.show_formulas,
                        // The **way** the sheet is looked at.
                        "mode": view.mode.clone(),
                        "freeze": match view.freeze {
                            Some((rows, cols)) => json!([rows, cols]),
                            None => Json::Null,
                        },
                    });
                    views += 1;
                }
                if let Some(zoom) = sheet.view.as_ref().and_then(|view| view.zoom) {
                    data["zoom"] = json!(zoom);
                }
                if !row_heights.is_empty() {
                    data["rows"] = Json::Object(row_heights);
                }
                if !column_widths.is_empty() {
                    data["columns"] = Json::Object(column_widths);
                }
                // The **default size** of the sheet: the width of a
                // column and the height of a row that has no size of
                // its own. A sheet that says nothing has none.
                if let Some(width) = sheet.default_width {
                    data["default_width"] = json!(width);
                }
                if let Some(height) = sheet.default_height {
                    data["default_height"] = json!(height);
                }
                if !hidden_rows.is_empty() {
                    data["hidden_rows"] = Json::Array(hidden_rows);
                }
                if !hidden_columns.is_empty() {
                    data["hidden_columns"] = Json::Array(hidden_columns);
                }
                // The **outline** of the sheet: the rows and the columns
                // that are in a group, and how deep in it they are.
                if !row_outline.is_empty() {
                    data["row_outline"] = Json::Object(row_outline);
                    outlines += 1;
                }
                if !column_outline.is_empty() {
                    data["column_outline"] = Json::Object(column_outline);
                    outlines += 1;
                }
                // Where the **summary** of a group sits: below the
                // group, or to the right of it.
                if let Some(below) = sheet.summary_below {
                    data["summary_below"] = json!(below);
                }
                if let Some(right) = sheet.summary_right {
                    data["summary_right"] = json!(right);
                }
                // The **validations** of the sheet: the rules on what a
                // cell may hold.
                let mut validations: Vec<Json> = Vec::new();
                for rule in &sheet.validations {
                    let Some(kind) = validation_kind(&rule.kind) else {
                        unread_validations += 1;
                        continue;
                    };
                    let operator = match &rule.operator {
                        Some(operator) => match crate::validation::Operator::parse(operator) {
                            Some(operator) => operator.name(),
                            None => {
                                unread_validations += 1;
                                continue;
                            }
                        },
                        None => crate::validation::Operator::Between.name(),
                    };
                    let mut entry = craftkit_sdk::serde_json::Map::new();
                    entry.insert("range".to_string(), json!(rule.ranges[0]));
                    entry.insert("kind".to_string(), json!(kind));
                    entry.insert("operator".to_string(), json!(operator));
                    entry.insert(
                        "formula1".to_string(),
                        json!(match &rule.formula1 {
                            Some(text) => validation_formula(text, kind),
                            None => String::new(),
                        }),
                    );
                    entry.insert(
                        "formula2".to_string(),
                        match &rule.formula2 {
                            Some(formula) => json!(validation_formula(formula, kind)),
                            None => Json::Null,
                        },
                    );
                    entry.insert("allow_blank".to_string(), json!(rule.allow_blank));
                    entry.insert("dropdown".to_string(), json!(rule.in_cell_dropdown));
                    entry.insert("input_title".to_string(), json!(rule.input_title));
                    entry.insert("input_message".to_string(), json!(rule.input_message));
                    entry.insert("show_input".to_string(), json!(rule.show_input));
                    entry.insert("error_title".to_string(), json!(rule.error_title));
                    entry.insert("error_message".to_string(), json!(rule.error_message));
                    entry.insert("error_style".to_string(), json!(rule.error_style));
                    entry.insert("show_error".to_string(), json!(rule.show_error));
                    validations.push(Json::Object(entry));
                    imported_validations += 1;
                }
                if !validations.is_empty() {
                    data["validations"] = Json::Array(validations);
                }
                // The **printing** of the sheet: the area it prints, and
                // the titles it repeats on every page.
                if let Some(area) = &sheet.print_area {
                    match crate::fill::area_at(area, &id) {
                        Ok(area) if area.start != area.end => {
                            data["print_area"] = json!(area.to_text());
                            printed += 1;
                        }
                        // The tree's own rule: an area of one cell is a
                        // cell, and a cell is not a printed area.
                        _ => unread_prints += 1,
                    }
                }
                if let Some(text) = &sheet.print_titles {
                    let (rows, columns) = print_titles_of(text);
                    if rows.is_empty() && columns.is_empty() {
                        unread_prints += 1;
                    } else {
                        if !rows.is_empty() {
                            data["title_rows"] = json!(rows);
                        }
                        if !columns.is_empty() {
                            data["title_columns"] = json!(columns);
                        }
                        printed += 1;
                    }
                }
                // The **page setup** of the sheet: how it prints.
                if let Some(print) = &sheet.print {
                    data["print"] = print_setup(print, &mut unread_setups).to_json();
                    setups += 1;
                }
                // The **marks** of the print: the gridlines and the
                // headings the file prints.
                if let Some(options) = &sheet.print_options {
                    if options.gridlines {
                        data["print_gridlines"] = json!(true);
                        marks += 1;
                    }
                    if options.headings {
                        data["print_headings"] = json!(true);
                        marks += 1;
                    }
                }
                // The **centre** of the sheet on the page: where the
                // file says the sheet sits.
                if let Some(options) = &sheet.print_options
                    && (options.centred_horizontally || options.centred_vertically)
                {
                    data["print_centred"] = json!({
                        "horizontal": options.centred_horizontally,
                        "vertical": options.centred_vertically,
                    });
                    centred += 1;
                }
                // The **header** and the **footer** of the pages: the
                // texts of the odd, the even and the first pages. A
                // text the tree does not print — one that names the
                // machine, or a font — is not carried, and it is
                // counted.
                if let Some(header_footer) = &sheet.header_footer {
                    let mut header = craftkit_sdk::serde_json::Map::new();
                    let mut footer = craftkit_sdk::serde_json::Map::new();
                    for (kind, text) in [
                        ("odd", &header_footer.odd_header),
                        ("even", &header_footer.even_header),
                        ("first", &header_footer.first_header),
                    ] {
                        let Some(text) = text else { continue };
                        if crate::headers::check(text).is_ok() {
                            header.insert(kind.to_string(), json!(text));
                        } else {
                            unread_headers += 1;
                        }
                    }
                    for (kind, text) in [
                        ("odd", &header_footer.odd_footer),
                        ("even", &header_footer.even_footer),
                        ("first", &header_footer.first_footer),
                    ] {
                        let Some(text) = text else { continue };
                        if crate::headers::check(text).is_ok() {
                            footer.insert(kind.to_string(), json!(text));
                        } else {
                            unread_headers += 1;
                        }
                    }
                    if !header.is_empty() {
                        data["print_header"] = Json::Object(header);
                    }
                    if !footer.is_empty() {
                        data["print_footer"] = Json::Object(footer);
                    }
                    // The **kinds of pages** the file prints: whether
                    // the first page and the even pages have a header
                    // and a footer of their own. A file that says
                    // nothing leaves the tree's own way.
                    let mut options = craftkit_sdk::serde_json::Map::new();
                    if let Some(flag) = header_footer.different_first {
                        options.insert("differentFirst".to_string(), json!(flag));
                    }
                    if let Some(flag) = header_footer.different_odd_even {
                        options.insert("differentOddEven".to_string(), json!(flag));
                    }
                    if !options.is_empty() {
                        data["print_header_options"] = Json::Object(options);
                    }
                }
                // The **breaks** of the printing: the lines a page
                // begins at.
                if !sheet.breaks.rows.is_empty() {
                    data["row_breaks"] =
                        Json::Array(sheet.breaks.rows.iter().map(|line| json!(line)).collect());
                    breaks += 1;
                }
                if !sheet.breaks.columns.is_empty() {
                    data["column_breaks"] =
                        Json::Array(sheet.breaks.columns.iter().map(|line| json!(line)).collect());
                    breaks += 1;
                }
                // The **sparklines** of the sheet: the little drawings
                // in its cells.
                if !sheet.sparklines.is_empty() {
                    let mut sparklines = craftkit_sdk::serde_json::Map::new();
                    for sparkline in &sheet.sparklines {
                        let Some(cell) = crate::formula::CellRef::parse(sparkline.cell.trim()) else {
                            unread_sparklines += 1;
                            continue;
                        };
                        // The **range** it reads: the file's own words,
                        // without the sheet it names and without the `$`
                        // of an absolute address.
                        let source = sparkline.source.trim();
                        let own = source.rsplit_once('!').map(|(_, own)| own).unwrap_or(source);
                        let own = own.replace('$', "");
                        let Ok(area) = crate::fill::area_at(&own, &id) else {
                            unread_sparklines += 1;
                            continue;
                        };
                        if area.start == area.end {
                            // The tree's own rule: a sparkline reads a
                            // range, and a range of one cell is a cell.
                            unread_sparklines += 1;
                            continue;
                        }
                        // The **kind**: the file says `stacked` and the
                        // tree says `win_loss`.
                        let kind = match sparkline.kind.as_str() {
                            "column" => "column",
                            "stacked" => "win_loss",
                            _ => "line",
                        };
                        let colour = match &sparkline.colour {
                            Some(colour) => match craftkit_sdk::Colour::from_hex(colour) {
                                Ok(colour) => json!([colour.0[0], colour.0[1], colour.0[2], colour.0[3]]),
                                Err(_) => Json::Null,
                            },
                            None => Json::Null,
                        };
                        let mut one = craftkit_sdk::serde_json::Map::new();
                        one.insert("range".to_string(), json!(area.to_text()));
                        one.insert("kind".to_string(), json!(kind));
                        one.insert("colour".to_string(), colour);
                        one.insert("markers".to_string(), json!(sparkline.markers));
                        sparklines.insert(cell.to_text(), Json::Object(one));
                        imported_sparklines += 1;
                    }
                    if !sparklines.is_empty() {
                        data["sparklines"] = Json::Object(sparklines);
                    }
                }
                if !merges.is_empty() {
                    data["merges"] = Json::Array(merges.into_iter().map(Json::String).collect());
                }
                // The **conditional formats** of the sheet: the rules
                // that are about a range of cells.
                let mut conditionals: Vec<Json> = Vec::new();
                for rule in &sheet.conditionals {
                    // A rule is a **criterion**, a **formula** or a
                    // kind of rule that is about the range as a whole:
                    // the cells whose formula is true are the cells
                    // that match, and the cells of a scale or of a bar
                    // are filled or drawn by it.
                    let about_the_range = rule.colour_scale.is_some()
                        || rule.top.is_some()
                        || rule.average.is_some()
                        || rule.duplicate.is_some()
                        || rule.data_bar.is_some()
                        || rule.icon_set.is_some();
                    let criterion = match (&rule.formula, about_the_range) {
                        (Some(formula), _) => {
                            // A formula that does not parse is not a
                            // rule this tree can judge.
                            if crate::formula::parse(formula).is_err() {
                                unread_conditionals += 1;
                                continue;
                            }
                            None
                        }
                        (None, true) => None,
                        (None, false) => {
                            if !crate::formula::Criterion::op_is_known(&rule.op) {
                                unread_conditionals += 1;
                                continue;
                            }
                            Some((rule.op.clone(), rule.value.clone()))
                        }
                    };
                    // The **format** of the rule: only when this tree's
                    // format language can read it.
                    let format = match &rule.format {
                        Some(code) if crate::format::Format::parse(code).is_some() => Some(code.clone()),
                        Some(_) => {
                            unread_conditionals += 1;
                            continue;
                        }
                        None => None,
                    };
                    let mut entry = craftkit_sdk::serde_json::Map::new();
                    entry.insert("range".to_string(), json!(rule.range));
                    match (&criterion, &rule.colour_scale) {
                        (Some((op, value)), _) => {
                            entry.insert("op".to_string(), json!(op));
                            entry.insert("value".to_string(), json!(value));
                        }
                        (None, Some(scale)) => {
                            entry.insert(
                                "colour_scale".to_string(),
                                json!({ "colours": scale.colours.clone() }),
                            );
                        }
                        (None, None) => {
                            entry.insert("formula".to_string(), json!(rule.formula));
                        }
                    }
                    // The **kinds of rule** that are about the range as
                    // a whole.
                    if let Some(top) = &rule.top {
                        entry.insert(
                            "top".to_string(),
                            json!({ "count": top.count, "bottom": top.bottom, "percent": top.percent }),
                        );
                    }
                    if let Some(average) = &rule.average {
                        entry.insert("average".to_string(), json!({ "above": average.above }));
                    }
                    if let Some(duplicate) = &rule.duplicate {
                        entry.insert("duplicate".to_string(), json!({ "unique": duplicate.unique }));
                    }
                    if let Some(bar) = &rule.data_bar {
                        entry.insert("data_bar".to_string(), json!({ "colour": bar.colour.clone() }));
                    }
                    if let Some(icons) = &rule.icon_set {
                        entry.insert(
                            "icon_set".to_string(),
                            json!({ "high": icons.high, "low": icons.low }),
                        );
                    }
                    if let Some(format) = format {
                        entry.insert("format".to_string(), json!(format));
                    }
                    if let Some(fill) = &rule.fill {
                        entry.insert("fill".to_string(), json!(fill));
                    }
                    if let Some(colour) = &rule.colour {
                        entry.insert("colour".to_string(), json!(colour));
                    }
                    // The **borders** the rule draws: the lines on the
                    // edges of the cells.
                    if !rule.borders.edges.is_empty() {
                        let mut borders = craftkit_sdk::serde_json::Map::new();
                        for border in &rule.borders.edges {
                            let mut line = craftkit_sdk::serde_json::Map::new();
                            line.insert("style".to_string(), json!(border.style));
                            line.insert(
                                "colour".to_string(),
                                json!(border.colour.clone().unwrap_or_else(|| "#000000".to_string())),
                            );
                            borders.insert(border.edge.clone(), Json::Object(line));
                        }
                        entry.insert("border".to_string(), Json::Object(borders));
                    }
                    if let Some(stop) = rule.stop {
                        entry.insert("stop".to_string(), json!(stop));
                    }
                    if let Some(priority) = rule.priority {
                        entry.insert("priority".to_string(), json!(priority));
                    }
                    conditionals.push(Json::Object(entry));
                    imported_conditionals += 1;
                }
                if !conditionals.is_empty() {
                    data["conditionals"] = Json::Array(conditionals);
                }
                // The **protection** of the sheet: what may still be
                // done to it. A file that protects a sheet says so,
                // and the flags say what is still allowed.
                if let Some(protection) = &sheet.protection {
                    data["protection"] = json!({
                        "sort": protection.sort,
                        "filter": protection.filter,
                        "format": protection.format,
                        "locked": true,
                    });
                    protections += 1;
                }
                // The **edit ranges** of the sheet: the ranges that
                // stay editable while the sheet is protected. The name
                // of a range and the password that opens it are the
                // file's own, and the tree's edit ranges are ranges
                // and nothing else: what is not kept is counted.
                let mut edit_ranges: Vec<String> = Vec::new();
                for range in &sheet.edit_ranges {
                    for text in &range.ranges {
                        if crate::merges::parse_area(text).is_some() {
                            edit_ranges.push(text.clone());
                        } else {
                            unread_edit_ranges += 1;
                        }
                    }
                    if range.name.is_some() {
                        unread_range_names += 1;
                    }
                    if range.password {
                        unread_passwords += 1;
                    }
                }
                if !edit_ranges.is_empty() {
                    data["edit_ranges"] = json!(edit_ranges);
                }
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
                // The **state** of the sheet: hidden or not.
                if matches!(sheet.state.as_deref(), Some("hidden") | Some("veryHidden")) {
                    if all_hidden && index == 0 {
                        // The tree's own rule: a workbook keeps one
                        // sheet in sight.
                        kept_sheets += 1;
                    } else {
                        data["hidden"] = json!(true);
                        hidden_sheets += 1;
                    }
                }
                // The **colour** of the sheet's tab.
                if let Some(colour) = &sheet.tab_colour {
                    data["tab_color"] = json!(colour);
                    tab_colours += 1;
                }
                session.document_mut()?.add_node_with_under(
                    Some(workbook_id),
                    sheet_name,
                    "grid.sheet",
                    None,
                    data,
                );
            }
            // The **charts** of the sheets: the drawings of their
            // cells.
            //
            // A chart of a file names the cells it reads by ranges of
            // their own, and the tree's chart reads one range: the
            // chart is imported when those ranges are a block, and it
            // is left out when they are not.
            let mut imported_charts = 0u64;
            let mut unread_charts = 0u64;
            for (sheet_name, charts) in &workbook.charts {
                let Some(sheet_id) = session
                    .document()
                    .walk()
                    .find(|node| node.kind == "grid.sheet" && node.name == *sheet_name)
                    .map(|node| node.id)
                else {
                    continue;
                };
                for chart in charts {
                    let Some(kind) = chart_kind(&chart.kind) else {
                        unread_charts += 1;
                        continue;
                    };
                    let Some(area) = chart_range(chart, sheet_name) else {
                        unread_charts += 1;
                        continue;
                    };
                    // The **smooth line**: the tree's chart has one
                    // line for all of its series, and a chart whose
                    // series do not all say the same is a chart the
                    // tree cannot draw as it is.
                    let Some(smooth) = chart.smooth else {
                        unread_charts += 1;
                        continue;
                    };
                    // The tree's own rule: a chart needs a name and
                    // something to show.
                    if area.start.row == area.end.row || area.start.col == area.end.col {
                        unread_charts += 1;
                        continue;
                    }
                    let legend = chart.legend.clone().unwrap_or_else(|| {
                        if kind == "pie" {
                            "right".to_string()
                        } else {
                            "bottom".to_string()
                        }
                    });
                    let name = chart.title.clone().unwrap_or_else(|| "the chart".to_string());
                    let node = json!({
                        "range": area.to_text(),
                        "kind": kind,
                        "stacking": match &chart.stacking {
                            Some(stacking) => json!(stacking),
                            None => Json::Null,
                        },
                        "title": chart.title,
                        "legend": legend,
                        "dataLabels": chart.data_labels,
                        "gridlines": chart.gridlines,
                        "xTitle": chart.x_title,
                        "yTitle": chart.y_title,
                        "smooth": smooth,
                        "markers": chart.markers,
                        "colours": chart.colours.clone(),
                        // The **cell** the chart is drawn in.
                        "cell": chart.cell.clone(),
                        // The **trendlines**: the fits of the lines of
                        // the series.
                        "trendlines": chart
                            .trendlines
                            .iter()
                            .map(|trendline| {
                                json!({
                                    "series": trendline.series,
                                    "kind": trendline.kind,
                                    "order": trendline.order,
                                    "period": trendline.period,
                                    "name": trendline.name,
                                    "forward": trendline.forward,
                                    "backward": trendline.backward,
                                    "intercept": trendline.intercept,
                                    "equation": trendline.equation,
                                    "r_squared": trendline.r_squared,
                                })
                            })
                            .collect::<Vec<Json>>(),
                    });
                    session.document_mut()?.add_node_with_under(Some(sheet_id), name, "grid.chart", None, node);
                    imported_charts += 1;
                }
            }
            // The **pictures** of the sheets: the images they show.
            //
            // The bytes of a picture that came from a workbook are **in
            // the document**: the workbook is a file and the document
            // is not beside it, and a picture the document cannot show
            // is a picture that is lost.
            //
            // The tree draws a **PNG** and a **JPEG**, and a cell holds
            // **one** picture: another format, and a second picture in
            // a cell that has one, are counted and not imported.
            let mut imported_pictures = 0u64;
            let mut unread_pictures = 0u64;
            for (sheet_name, pictures) in &workbook.pictures {
                let Some(sheet_id) = session
                    .document()
                    .walk()
                    .find(|node| node.kind == "grid.sheet" && node.name == *sheet_name)
                    .map(|node| node.id)
                else {
                    continue;
                };
                for picture in pictures {
                    if !craftkit_image::is_png(&picture.bytes) && !craftkit_image::is_jpeg(&picture.bytes) {
                        unread_pictures += 1;
                        continue;
                    }
                    let Ok(size) = craftkit_image::size(&picture.bytes) else {
                        unread_pictures += 1;
                        continue;
                    };
                    let address = picture.cell.clone();
                    let entry = json!({
                        "bytes": craftkit_sdk::bytes::encode(&picture.bytes),
                        "length": picture.bytes.len(),
                        "width": size.width,
                        "height": size.height,
                        "format": picture.format,
                        "alt": picture.alt,
                    });
                    let Some(node) = session.document_mut()?.node_mut(sheet_id) else { continue };
                    if !node.data["pictures"].is_object() {
                        node.data["pictures"] = json!({});
                    }
                    let Some(map) = node.data["pictures"].as_object_mut() else { continue };
                    if map.contains_key(&address) {
                        // A cell holds one picture in this tree.
                        unread_pictures += 1;
                        continue;
                    }
                    map.insert(address, entry);
                    imported_pictures += 1;
                }
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
                    one.insert("resolved".to_string(), json!(comment.resolved));
                    if let Some(date) = &comment.date {
                        one.insert("date".to_string(), json!(date.clone()));
                    }
                    // Whether the note is **shown** on its cell.
                    one.insert("visible".to_string(), json!(comment.visible));
                    // The **box** the note is shown in.
                    one.insert(
                        "note_box".to_string(),
                        match &comment.note_box {
                            Some(box_of) => json!({
                                "from": {
                                    "column": box_of.from.column,
                                    "column_px": box_of.from.column_px,
                                    "row": box_of.from.row,
                                    "row_px": box_of.from.row_px,
                                },
                                "to": {
                                    "column": box_of.to.column,
                                    "column_px": box_of.to.column_px,
                                    "row": box_of.to.row,
                                    "row_px": box_of.to.row_px,
                                },
                                "size_pt": match box_of.size_pt {
                                    Some((width, height)) => json!({ "width": width, "height": height }),
                                    None => Json::Null,
                                },
                            }),
                            None => Json::Null,
                        },
                    );
                    // The **replies** under the note: a thread of a
                    // newer file.
                    let replies: Vec<Json> = comment
                        .replies
                        .iter()
                        .map(|reply| {
                            let mut one = craftkit_sdk::serde_json::Map::new();
                            one.insert("text".to_string(), json!(reply.text.clone()));
                            if !reply.author.is_empty() {
                                one.insert("author".to_string(), json!(reply.author.clone()));
                            }
                            if let Some(date) = &reply.date {
                                one.insert("date".to_string(), json!(date.clone()));
                            }
                            craftkit_sdk::serde_json::Value::Object(one)
                        })
                        .collect();
                    one.insert("replies".to_string(), Json::Array(replies));
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
            if checkboxes > 0 {
                summary.push_str(&format!("; {checkboxes} checkbox(es) were imported"));
            }
            if readings > 0 {
                summary.push_str(&format!("; {readings} reading(s) were imported"));
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
            if unread_style_aligns > 0 {
                summary.push_str(&format!(
                    "; {unread_style_aligns} style alignment(s) could not be read and were not imported"
                ));
            }
            if unread_aligns > 0 {
                summary.push_str(&format!(
                    "; {unread_aligns} alignment(s) could not be read and were not imported"
                ));
            }
            if arrays > 0 {
                summary.push_str(&format!(
                    "; {arrays} array formula(s) are not formulas of a cell here and were not imported"
                ));
            }
            if data_tables > 0 {
                summary.push_str(&format!(
                    "; {data_tables} data table(s) were not imported and their values were kept"
                ));
            }
            if unread_prints > 0 {
                summary.push_str(&format!(
                    "; {unread_prints} printing(s) could not be read and were not imported"
                ));
            }
            if unread_edit_ranges > 0 {
                summary.push_str(&format!(
                    "; {unread_edit_ranges} edit range(s) could not be read and were not imported"
                ));
            }
            if unread_range_names > 0 {
                summary.push_str(&format!(
                    "; {unread_range_names} range name(s) could not be kept: the tree's edit ranges have none"
                ));
            }
            if unread_pictures > 0 {
                summary.push_str(&format!(
                    "; {unread_pictures} picture(s) could not be drawn by the tree and were not imported"
                ));
            }
            if unread_passwords > 0 {
                summary.push_str(&format!(
                    "; {unread_passwords} password(s) could not be kept: the tree's protection has none"
                ));
            }
            if unread_headers > 0 {
                summary.push_str(&format!(
                    "; {unread_headers} header(s) or footer(s) could not be printed and were not imported"
                ));
            }
            if unread_line_formats > 0 {
                summary.push_str(&format!(
                    "; {unread_line_formats} format(s) of a row or a column could not be kept: \
                     the tree's model has no place for them"
                ));
            }
            if unread_collapsed_lines > 0 {
                summary.push_str(&format!(
                    "; {unread_collapsed_lines} collapsed group(s) of rows or columns could not be \
                     kept: the tree works its own collapsed mark out from the rows"
                ));
            }
            if unread_custom_sizes > 0 {
                summary.push_str(&format!(
                    "; {unread_custom_sizes} size(s) set by the user could not be kept: \
                     the tree's model has the size and not the hand that set it"
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
                "checkboxes": checkboxes,
                "fonts": fonts,
                "fills": fills,
                "aligns": aligns,
                "unread_aligns": unread_aligns,
                "borders": imported_borders,
                "locks": locks,
                "protections": protections,
                "merges": imported_merges,
                "unread_merges": unread_merges,
                "views": views,
                "conditionals": imported_conditionals,
                "unread_conditionals": unread_conditionals,
                "validations": imported_validations,
                "unread_validations": unread_validations,
                "prints": printed,
                "unread_prints": unread_prints,
                "unread_headers": unread_headers,
                "unread_passwords": unread_passwords,
                "unread_line_formats": unread_line_formats,
                "unread_collapsed_lines": unread_collapsed_lines,
                "unread_custom_sizes": unread_custom_sizes,
                "outlines": outlines,
                "setups": setups,
                "centred": centred,
                "marks": marks,
                "unread_edit_ranges": unread_edit_ranges,
                "unread_range_names": unread_range_names,
                "unread_style_aligns": unread_style_aligns,
                "breaks": breaks,
                "sparklines": imported_sparklines,
                "unread_sparklines": unread_sparklines,
                "arrays": arrays,
                "data_tables": data_tables,
                "unread_setups": unread_setups,
                "names": names,
                "hidden_sheets": hidden_sheets,
                "kept_sheets": kept_sheets,
                "tab_colours": tab_colours,
                "unread_names": unread_names,
                "filters": imported_filters,
                "unread_filters": unread_filters,
                "links": imported_links,
                "dropped_links": dropped_links,
                "comments": imported_comments,
                "tables": imported_tables,
                "pivots": imported_pivots,
                "unread_pivots": unread_pivots,
                "charts": imported_charts,
                "unread_charts": unread_charts,
                "pictures": imported_pictures,
                "unread_pictures": unread_pictures,
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

/// The **formula** of a validation rule, as this tree's formula.
///
/// A file writes the list of a list rule in one of two ways: as the
/// words of the list, in quotes, or as a reference to a range of
/// cells. The tree's own list rule reads both, and reads the words
/// with their quotes — so the words are kept as they are, and a
/// reference keeps its sheet and its cells.
fn validation_formula(text: &str, kind: &str) -> String {
    let text = tree_formula(text);
    let text = text.trim();
    if kind == "list" && !text.starts_with('"') {
        // A reference: the tree reads it without the `=` a file may
        // write before it.
        return text.trim_start_matches('=').trim().to_string();
    }
    text.to_string()
}

/// The **page setup** of a sheet, as this tree's setup.
///
/// The file's numbers are the file's own: the margins are in **inches**
/// and the tree's are in **points**, and the paper is the file's own
/// name — a paper this tree does not have is not a paper.
fn print_setup(print: &craftkit_xlsx::Print, unread: &mut u64) -> crate::printing::Setup {
    let mut setup = crate::printing::Setup::default();
    if let Some(paper) = &print.paper {
        match crate::printing::paper_of(paper) {
            Some((name, width, height)) => {
                setup.paper = name.to_string();
                setup.width = width;
                setup.height = height;
            }
            None => *unread += 1,
        }
    }
    if let Some(orientation) = &print.orientation {
        if orientation == "portrait" || orientation == "landscape" {
            setup.orientation = orientation.clone();
        } else {
            *unread += 1;
        }
    }
    if let Some(margins) = print.margins {
        // The tree's margins are in points.
        setup.margins = [margins[0] * 72.0, margins[1] * 72.0, margins[2] * 72.0, margins[3] * 72.0];
    }
    if let Some(scale) = print.scale {
        let scale = f64::from(scale);
        if (crate::scaling::MIN_SCALE..=crate::scaling::MAX_SCALE).contains(&scale) {
            setup.scale = scale;
        } else {
            // The tree's own rule: a scale outside the bounds is
            // refused, and not clamped.
            *unread += 1;
        }
    }
    if print.fit_to_page
        && let Some((wide, tall)) = print.fit
    {
        if (wide == 0 && tall == 0)
            || wide > crate::scaling::MAX_FIT_PAGES
            || tall > crate::scaling::MAX_FIT_PAGES
        {
            // A fit of no pages at all is not a fit, and a fit of more
            // pages than the tree allows is not a fit either.
            *unread += 1;
        } else {
            setup.fit = Some((wide as u16, tall as u16));
        }
    }
    setup
}

/// The **titles** of a sheet: the rows and the columns that are
/// printed on every page.
///
/// A file writes the titles as one text — the sheet's own words — and a
/// part of it is a range of rows or a range of columns. The rows of
/// the file are counted from one and the tree's are counted from zero.
fn print_titles_of(text: &str) -> (Vec<u32>, Vec<u32>) {
    let mut rows: Vec<u32> = Vec::new();
    let mut columns: Vec<u32> = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        let own = part.rsplit_once('!').map(|(_, own)| own).unwrap_or(part);
        let own = own.replace('$', "");
        let Some((first, second)) = own.split_once(':') else { continue };
        let (first, second) = (first.trim(), second.trim());
        if let (Ok(first), Ok(second)) = (first.parse::<u32>(), second.parse::<u32>()) {
            // A range of **rows**: the rows of the file count from one,
            // and a sheet is not taller than the tree's own bound.
            if first >= 1 && second >= first && second as u64 <= crate::formula::MAX_ROWS {
                for row in (first - 1)..=(second - 1) {
                    if !rows.contains(&row) {
                        rows.push(row);
                    }
                }
            }
            continue;
        }
        // A range of **columns**.
        let (Some(first), Some(second)) =
            (crate::formula::CellRef::column_index(first), crate::formula::CellRef::column_index(second))
        else {
            continue;
        };
        if second < first || second as u64 >= crate::formula::MAX_COLS {
            continue;
        }
        for column in first..=second {
            if !columns.contains(&column) {
                columns.push(column);
            }
        }
    }
    rows.sort_unstable();
    columns.sort_unstable();
    (rows, columns)
}

/// The **header** of a column of a range: the text of the cell at the
/// top of it.
///
/// A field of a pivot is named by its header, and this is the tree's
/// own rule.
fn header_of(
    cells: &craftkit_sdk::serde_json::Map<String, Json>,
    area: &crate::formula::Area,
    column: u32,
) -> Option<String> {
    let address = crate::formula::CellRef { col: column, row: area.start.row }.to_text();
    let text = cells.get(&address)?.get("value")?.as_str()?;
    let text = text.trim();
    if text.is_empty() { None } else { Some(text.to_string()) }
}

/// The **column** a field names, when a column of the range is named
/// by it.
fn column_of(
    cells: &craftkit_sdk::serde_json::Map<String, Json>,
    area: &crate::formula::Area,
    name: &str,
) -> Option<u32> {
    let name = name.trim();
    (area.start.col..=area.end.col).find(|column| header_of(cells, area, *column).as_deref() == Some(name))
}

/// The **pivot** of a file, as this tree's pivot.
///
/// The tree's pivot gathers the rows by **one** field, spreads the
/// groups over one and puts one together — so a pivot of a file that
/// has more of any of them is not a pivot of this tree.
fn pivot_of(
    pivot: &craftkit_xlsx::PivotTable,
    sheet: &str,
    cells: &craftkit_sdk::serde_json::Map<String, Json>,
) -> Option<crate::pivots::Pivot> {
    // The **source**: the range the pivot reads, in this sheet.
    let source = pivot.source.trim();
    let own = match source.rsplit_once('!') {
        Some((name, own)) => {
            let name = name.trim().trim_matches('\'');
            if !name.eq_ignore_ascii_case(sheet) {
                return None;
            }
            own
        }
        None => source,
    };
    let own = own.replace('$', "");
    let (_, area) = crate::merges::parse_area(&own)?;
    if area.start.row == area.end.row || area.start.col == area.end.col {
        return None;
    }
    // The **row field**: the one the rows are gathered by.
    let [row_field] = pivot.row_fields.as_slice() else { return None };
    let row_field = column_of(cells, &area, row_field)?;
    // The **column field**: the one the groups are spread over.
    let column_field = match pivot.column_fields.as_slice() {
        [] => None,
        [name] => Some(column_of(cells, &area, name)?),
        _ => return None,
    };
    // The **value**: the field that is put together, and the way of
    // putting it together.
    let (value_field, aggregate) = match &pivot.value {
        None => (None, crate::pivots::Aggregate::Sum),
        Some((name, way)) => {
            let column = column_of(cells, &area, name)?;
            let aggregate = crate::pivots::Aggregate::parse(way)?;
            (Some(column), aggregate)
        }
    };
    Some(crate::pivots::Pivot {
        name: pivot.name.clone().unwrap_or_else(|| "the pivot".to_string()),
        range: area,
        row_field,
        column_field,
        value_field,
        aggregate,
        // The **grand totals**: the file says whether the pivot has
        // the column at the right that totals the rows, and the line
        // at the bottom that totals the columns. A file that says
        // nothing has both.
        grand_totals_rows: pivot.grand_totals_rows.unwrap_or(true),
        grand_totals_cols: pivot.grand_totals_cols.unwrap_or(true),

        // The **filter** of the pivot: the file says of the field it
        // filters by, and of the items it keeps.
        filter: match &pivot.filter {
            None => None,
            Some((name, items)) => {
                let field = column_of(cells, &area, name)?;
                Some(crate::pivots::Filter { field, selected: items.clone() })
            }
        },
        // The **order** of the items: the file says of the field they
        // are ordered by, and of the order. A file that orders them
        // by hand says nothing.
        sort: match &pivot.sort {
            None => None,
            Some((name, order)) => {
                let column = column_of(cells, &area, name)?;
                let order = crate::pivots::Sort::parse(order)?;
                Some((column, order))
            }
        },
        // The **grouping** of the dates: the file says of the field
        // and of the way its dates are gathered.
        group: match &pivot.group {
            None => None,
            Some((name, way)) => {
                let column = column_of(cells, &area, name)?;
                let way = crate::pivots::DateGroup::parse(way)?;
                Some((column, way))
            }
        },
        // The way the pivot **shows its value**: the file says, and a
        // file that says nothing shows the value as it is.
        show_as: pivot
            .show_as
            .as_deref()
            .and_then(crate::pivots::ShowAs::parse)
            .unwrap_or(crate::pivots::ShowAs::Normal),
        // The **subtotals** of the groups: the file says where they
        // are, or that they are not there. A file that says nothing
        // keeps the tree's own way.
        subtotals: pivot
            .subtotals
            .as_deref()
            .and_then(crate::pivots::Subtotals::parse)
            .unwrap_or(crate::pivots::Subtotals::None),
    })
}

/// The **kind** of a chart, as this tree's kind.
///
/// The tree has four kinds of chart, and a file has more: a chart of
/// a kind this tree does not have is not a chart here.
fn chart_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "column" => Some("column"),
        "bar" => Some("bar"),
        "line" => Some("line"),
        "pie" => Some("pie"),
        _ => None,
    }
}

/// The **range** a chart reads, as this tree's range.
///
/// The tree's chart reads **one** range: the categories are its first
/// column and the series are the columns after it. A file's chart
/// names the categories and each of its series by a range of their
/// own, and those ranges must be a **block** — the categories in the
/// first column, the series in the columns beside it — for the
/// tree's chart to be the same drawing.
fn chart_range(chart: &craftkit_xlsx::Chart, sheet: &str) -> Option<crate::formula::Area> {
    let categories = chart_area(chart.categories.as_ref()?, sheet)?;
    let mut column = categories.start.col;
    let mut series = 0u32;
    for values in &chart.series {
        let area = chart_area(values, sheet)?;
        // A **column** of the same rows, and the column beside the
        // one before it.
        if area.start.col != area.end.col {
            return None;
        }
        if area.start.row != categories.start.row || area.end.row != categories.end.row {
            return None;
        }
        if area.start.col != column + 1 {
            return None;
        }
        column = area.end.col;
        series += 1;
    }
    if series == 0 {
        return None;
    }
    Some(crate::formula::Area::new(
        categories.start,
        crate::formula::CellRef { col: column, row: categories.end.row },
    ))
}

/// The **area** a reference of a file names, when it names one of the
/// cells of the sheet.
///
/// A file writes the cells a chart reads as a reference: the sheet
/// they are in and the range of them. A reference to another sheet is
/// not a range of this sheet.
fn chart_area(text: &str, sheet: &str) -> Option<crate::formula::Area> {
    let text = text.trim();
    let own = match text.rsplit_once('!') {
        Some((name, own)) => {
            let name = name.trim().trim_matches('\'');
            if !name.eq_ignore_ascii_case(sheet) {
                return None;
            }
            own
        }
        None => text,
    };
    let own = own.replace('$', "");
    let (_, area) = crate::merges::parse_area(&own)?;
    Some(area)
}

/// The **kind** of a validation, as this tree's kind.
///
/// The file names the kind in its own words, and the tree's own kind
/// for "anything at all" is `any` and the file's is `none`. A kind
/// this tree does not have — a **custom** rule, which is a formula of
/// its own — is nothing here.
fn validation_kind(kind: &str) -> Option<&'static str> {
    match kind {
        "none" => Some("any"),
        "whole" => Some("whole"),
        "decimal" => Some("decimal"),
        "list" => Some("list"),
        "date" => Some("date"),
        "time" => Some("time"),
        "textLength" => Some("textLength"),
        _ => None,
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
