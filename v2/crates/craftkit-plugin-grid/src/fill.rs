//! The **filling** of a range.
//!
//! A fill takes the value of one cell — or of a **range** of cells —
//! and puts it in a range. The tree has **four** ways of doing it, and
//! they differ in what they do with a **lone number**:
//!
//! | the source | `copy` | `series` | `extend` | `alternate` |
//! |---|---|---|---|---|
//! | one number | copied | **copied** | counts up | counts up |
//! | two or more numbers | copied | the **trend** | the trend | copied |
//! | text with a number | copied | counts on | counts on | copied |
//! | text | copied | copied | copied | copied |
//!
//! That is the tree's own table, and it is a surprise: the tree's
//! **plain drag** copies a lone number, and the tree's **series** —
//! which is the tree's *default* — copies it too; only the tree's
//! right-drag **Fill Series** (`extend`) and the tree's `Ctrl`+drag
//! (`alternate`) count a lone number up.
//!
//! The tree's **trend** is the tree's: the **least squares** line
//! through the numbers of the source, so `1, 3` goes on as `5, 7, 9`.
//!
//! The tree's source may be a **range**, and then it fills **lane by
//! lane**: each column of the source continues down its own lane, and
//! each row of the source continues along its own lane.
//!
//! Two of the tree's rules are here because they are the shape of the
//! operation and not a detail of it:
//!
//! - the **target holds the source at a corner**: a fill from `A1` into
//!   `A1:A10` goes **down**, and a fill from `A10` into `A1:A10` goes
//!   **up**, and a target that does not hold the source at a corner is
//!   refused;
//! - the fill is **bounded**: the tree bounds it at five million cells,
//!   and this one is bounded at ten thousand, because a sheet's cells
//!   live in a payload and a payload is not a place for a hundred
//!   thousand cells.
//!
//! # The fill of a formula
//!
//! A source that holds a **formula** is filled too: the tree copies
//! the formula and **shifts its relative references** for every cell it
//! writes, so `=B1` in `A1` becomes `=B2` in `A2` and `=B3` in `A3`.
//!
//! The shifting is the tree's `shift_relative`, and it is where the
//! **`$` earns its keep**: an **anchored** reference — `$A$1`, `A$1`,
//! `$A1` — does not move, and a reference that would leave the sheet
//! is `#REF!` in the tree and a **refusal** here.
//!
//! A fill that would make a cell **read itself** is refused as well:
//! the cycle is found here, and not later in the middle of a
//! recalculation.
//!
//! # What it does not do
//!
//! - **The tree's lists are not here** — the days, the months, the
//!   quarters — and neither are the tree's **dates**: a text that is a
//!   day of the week is copied here, and the tree would step through
//!   the week.
//! - **The tree's `formats` and `values` modes are not here**: the
//!   first copies the style without the value, the second copies the
//!   value without the style.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{self, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{self, Area, CellRef, Value};
use crate::{cells_into, cells_of, sheet_cells_mut};

/// The most cells one fill may write.
///
/// The tree bounds its fill at five million cells; this one is bounded
/// far lower, because the cells of a sheet live in a **payload** and a
/// payload is not a place for a hundred thousand cells.
const MAX_FILL_CELLS: u64 = 10_000;

/// The ways a fill may write.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// The value is **copied**.
    Copy,
    /// The tree's **series**: the tree's default, and the tree's series
    /// copies a lone number.
    Series,
    /// The tree's right-drag **Fill Series**: a lone number counts up.
    Extend,
    /// The tree's `Ctrl`+drag: a lone number counts up and everything
    /// else is copied.
    Alternate,
}

impl Mode {
    /// The mode a name says, if it says one this tree knows.
    fn parse(name: &str) -> Option<Mode> {
        match name.trim().to_lowercase().as_str() {
            "copy" => Some(Mode::Copy),
            "series" => Some(Mode::Series),
            "extend" => Some(Mode::Extend),
            "alternate" => Some(Mode::Alternate),
            _ => None,
        }
    }

    /// Whether the mode **continues a series** rather than copying it.
    fn extends(self) -> bool {
        matches!(self, Mode::Series | Mode::Extend)
    }
}

/// What a fill writes.
#[derive(Debug, Clone)]
enum Pattern {
    /// The value is copied.
    Copy,
    /// A series of numbers.
    Linear {
        /// The number the series starts from.
        first: f64,
        /// The number it ends on, at the source.
        last: f64,
        /// How much it counts on by.
        step: f64,
    },
    /// Text with a number in it: `Item 1`.
    TextNum {
        /// The text before the number.
        prefix: String,
        /// The text after the number.
        suffix: String,
        /// The number it starts from.
        first: i64,
        /// The number it ends on, at the source.
        last: i64,
        /// How much the number counts on by.
        step: i64,
        /// How wide the digits are: `Q003x` keeps three.
        width: usize,
    },
}

impl Pattern {
    /// The value **k** steps beyond the source.
    ///
    /// A number that would overflow is **nothing**: the cell keeps
    /// what it had, as the tree does, rather than wrapping.
    fn value(&self, k: i64, forward: bool) -> Option<Value> {
        let sign = if forward { 1 } else { -1 };
        match self {
            Pattern::Copy => None,
            Pattern::Linear { first, last, step } => {
                let base = if forward { *last } else { *first };
                Some(Value::Number(round15(base + step * (k * sign) as f64)))
            }
            Pattern::TextNum { prefix, suffix, first, last, step, width } => {
                let base = if forward { *last } else { *first };
                let number = step.checked_mul(k)?.checked_mul(sign)?.checked_add(base)?;
                let number = number.checked_abs()?;
                Some(Value::Text(format!("{prefix}{number:0width$}{suffix}", width = *width)))
            }
        }
    }

    /// The pattern of a **lane**: the values of the source cells.
    ///
    /// A lane of **numbers** is a series — the least squares line
    /// through them — unless there is only one, which the tree copies.
    /// A lane of **text with a number** in it counts its number on. A
    /// lane of anything else is copied.
    fn of(values: &[Value]) -> Pattern {
        if values.is_empty() {
            return Pattern::Copy;
        }
        if values.iter().all(|value| matches!(value, Value::Number(_))) {
            if values.len() == 1 {
                return Pattern::Copy;
            }
            let numbers: Vec<f64> = values
                .iter()
                .filter_map(|value| match value {
                    Value::Number(number) => Some(*number),
                    _ => None,
                })
                .collect();
            // The **least squares** line through the numbers, as the
            // tree's trend is.
            let count = numbers.len() as f64;
            let mean_x = (count - 1.0) / 2.0;
            let mean_y = numbers.iter().sum::<f64>() / count;
            let mut numerator = 0.0;
            let mut denominator = 0.0;
            for (index, number) in numbers.iter().enumerate() {
                numerator += (index as f64 - mean_x) * (number - mean_y);
                denominator += (index as f64 - mean_x).powi(2);
            }
            let step = if denominator == 0.0 { 0.0 } else { numerator / denominator };
            let last = mean_y + step * (count - 1.0 - mean_x);
            let first = mean_y - step * mean_x;
            return Pattern::Linear { first, last, step };
        }
        if values.iter().all(|value| matches!(value, Value::Text(_))) {
            let texts: Vec<String> = values
                .iter()
                .filter_map(|value| match value {
                    Value::Text(text) => Some(text.clone()),
                    _ => None,
                })
                .collect();
            let parts: Vec<Option<(String, i64, String, usize)>> =
                texts.iter().map(|text| split_num(text)).collect();
            if parts.iter().all(|part| part.is_some()) {
                let parts: Vec<(String, i64, String, usize)> = parts.into_iter().flatten().collect();
                if parts.iter().all(|part| part.0 == parts[0].0 && part.2 == parts[0].2) {
                    let first = parts.first().map(|part| part.1).unwrap_or(0);
                    let last = parts.last().map(|part| part.1).unwrap_or(0);
                    let step = if parts.len() >= 2 { parts[1].1 - parts[0].1 } else { 1 };
                    return Pattern::TextNum {
                        prefix: parts[0].0.clone(),
                        suffix: parts[0].2.clone(),
                        first,
                        last,
                        step,
                        width: parts[0].3,
                    };
                }
            }
        }
        Pattern::Copy
    }

    /// Whether the pattern is a **copy** — and the tree's modes read
    /// that: the tree's `alternate` copies everything but a lone
    /// number, and the tree's `extend` continues a lone number that
    /// would otherwise be copied.
    fn is_copy(&self) -> bool {
        matches!(self, Pattern::Copy)
    }
}

/// A text with a number in it: the text before it, the number, the
/// text after it, and how wide the digits are.
///
/// `Item 7` is `Item `, `7` and nothing; `Q003x` is `Q`, `3`, `x` and
/// three digits wide.
fn split_num(text: &str) -> Option<(String, i64, String, usize)> {
    let characters: Vec<char> = text.chars().collect();
    let end = characters.iter().rposition(char::is_ascii_digit)?;
    let mut start = end;
    while start > 0 && characters.get(start - 1).is_some_and(char::is_ascii_digit) {
        start -= 1;
    }
    let digits: String = characters.get(start..=end)?.iter().collect();
    let number: i64 = digits.parse().ok()?;
    let prefix: String = characters.get(..start)?.iter().collect();
    let suffix: String = characters.get(end + 1..)?.iter().collect();
    let width = if digits.starts_with('0') && digits.len() > 1 { digits.len() } else { 1 };
    Some((prefix, number, suffix, width))
}

/// A number to **fifteen** digits.
///
/// The tree's rule: a series of numbers is not allowed to grow a tail
/// of floating point noise.
fn round15(number: f64) -> f64 {
    if number == 0.0 || !number.is_finite() {
        return number;
    }
    let magnitude = number.abs().log10().floor() as i32;
    let scale = 10f64.powi(14 - magnitude);
    if !scale.is_finite() || scale == 0.0 { number } else { (number * scale).round() / scale }
}

/// **Fill** a range from a cell or a range of cells.
///
/// The target must **hold the source at a corner**, and the fill goes
/// down or up, or right or left, from there. The mode is the tree's:
/// `copy`, `series` (the tree's default), `extend` or `alternate`.
///
/// A source that holds a **formula** is filled with the formula, and
/// the references of it are **shifted**: the tree's `shift_relative`,
/// and the tree's `$` — an anchored reference does not move.
#[derive(Command)]
#[command(id = "grid.range.fill", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct FillRange {
    /// The sheet it goes in.
    pub sheet: NodeRef,
    /// The cell — or the range of cells — the fill comes from.
    pub from: Text,
    /// The range the fill goes into, `A1:A10`.
    pub to: Text,
    /// `copy`, `series`, `extend` or `alternate`. The tree's default
    /// is the series, and so is this one.
    pub mode: Option<Text>,
}

impl FillRange {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.range.fill".to_string();
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
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        let source = area_at(&self.from.0, &id)?;
        let target = area_at(&self.to.0, &id)?;
        let mode = match &self.mode {
            Some(mode) => Mode::parse(&mode.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{}` is not a mode: a fill is `copy`, `series`, `extend` or `alternate`",
                    mode.0.trim()
                ),
            })?,
            None => Mode::Series,
        };
        // The target must **hold the source at a corner**.
        if !target.covers(source.start) || !target.covers(source.end) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the target {} does not hold the source {}",
                    target.to_text(),
                    source.to_text()
                ),
            });
        }
        let at_corner = (source.start.row == target.start.row || source.end.row == target.end.row)
            && (source.start.col == target.start.col || source.end.col == target.end.col);
        if !at_corner {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the source {} is not at a corner of the target {}",
                    source.to_text(),
                    target.to_text()
                ),
            });
        }
        // The **direction**: down or up when the target has rows the
        // source does not, and right or left when it has columns.
        let source_rows = source.end.row - source.start.row + 1;
        let source_columns = source.end.col - source.start.col + 1;
        let target_rows = target.end.row - target.start.row + 1;
        let target_columns = target.end.col - target.start.col + 1;
        let vertical = target_rows != source_rows || target_columns == source_columns;
        let forward =
            if vertical { target.end.row > source.end.row } else { target.end.col > source.end.col };
        // The **lanes**: the columns of the source when the fill goes
        // down, and its rows when it goes along.
        let mut lanes: Vec<Lane> = Vec::new();
        if vertical {
            for column in source.start.col..=source.end.col {
                let mut cells = Vec::new();
                for row in source.start.row..=source.end.row {
                    cells.push(CellRef { col: column, row });
                }
                let mut targets = Vec::new();
                if forward {
                    for row in source.end.row + 1..=target.end.row {
                        targets.push(CellRef { col: column, row });
                    }
                } else {
                    for row in (target.start.row..source.start.row).rev() {
                        targets.push(CellRef { col: column, row });
                    }
                }
                lanes.push(Lane { cells, targets });
            }
        } else {
            for row in source.start.row..=source.end.row {
                let mut cells = Vec::new();
                for column in source.start.col..=source.end.col {
                    cells.push(CellRef { col: column, row });
                }
                let mut targets = Vec::new();
                if forward {
                    for column in source.end.col + 1..=target.end.col {
                        targets.push(CellRef { col: column, row });
                    }
                } else {
                    for column in (target.start.col..source.start.col).rev() {
                        targets.push(CellRef { col: column, row });
                    }
                }
                lanes.push(Lane { cells, targets });
            }
        }
        let count: u64 = lanes.iter().map(|lane| lane.targets.len() as u64).sum();
        if count > MAX_FILL_CELLS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the fill is too large: it would write {count} cells, and a fill writes at most {MAX_FILL_CELLS}"
                ),
            });
        }
        // The **source**: what it holds.
        let sheet_cells = cells_of(node);
        let sheet_name = node.name.clone();
        let mut sources: Vec<Source> = Vec::new();
        for lane in &lanes {
            for cell in &lane.cells {
                let from = match sheet_cells.get(*cell) {
                    Some(entry) => Source {
                        cell: *cell,
                        formula: entry.formula.clone(),
                        value: entry.value.clone().unwrap_or(Value::Empty),
                    },
                    None => Source { cell: *cell, formula: None, value: Value::Empty },
                };
                sources.push(from);
            }
        }
        if sources.iter().all(|from| from.formula.is_none() && matches!(from.value, Value::Empty)) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is nothing at the source {}", source.to_text()),
            });
        }
        // What every cell of the fill will hold.
        let mut writes: Vec<Write> = Vec::new();
        let mut at = 0;
        for lane in &lanes {
            let lane_sources = &sources[at..at + lane.cells.len()];
            at += lane.cells.len();
            // The **pattern** of the lane: the tree's pattern is made
            // from the cells' values, and a cell that holds a formula
            // has no value here — its value is derived — so the pattern
            // is made from the cells that hold values.
            let lane_values: Vec<Value> = lane_sources
                .iter()
                .filter(|from| from.formula.is_none())
                .map(|from| from.value.clone())
                .collect();
            let pattern = the_pattern(&lane_values, mode);
            let per_cell: Vec<Pattern> = lane_sources
                .iter()
                .map(|from| {
                    if from.formula.is_some() {
                        Pattern::Copy
                    } else {
                        Pattern::of(std::slice::from_ref(&from.value))
                    }
                })
                .collect();
            for (index, cell) in lane.targets.iter().enumerate() {
                let step = index as i64 + 1;
                let source_index = if forward {
                    index % lane.cells.len()
                } else {
                    lane.cells.len() - 1 - index % lane.cells.len()
                };
                let from = &lane_sources[source_index];
                let format = sheet_cells.format(from.cell).map(|code| code.to_string());
                // The tree copies a **formula** and **shifts** the
                // references of it, so that the copy reads the cells
                // beside it: `=B1` in `A1` becomes `=B2` in `A2`.
                if let Some(text) = &from.formula {
                    let expression = formula::parse(text).map_err(|error| EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the formula in {} does not parse: {}",
                            from.cell.to_text(),
                            error.message
                        ),
                    })?;
                    let rows = cell.row as i64 - from.cell.row as i64;
                    let columns = cell.col as i64 - from.cell.col as i64;
                    let moved =
                        formula::shift(&expression, rows, columns).map_err(|error| EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the formula in {} cannot be copied: {}",
                                from.cell.to_text(),
                                error.message
                            ),
                        })?;
                    let prefix = if text.trim_start().starts_with('=') { "=" } else { "" };
                    let copied = format!("{prefix}{}", moved.to_text());
                    writes.push(Write { cell: *cell, formula: Some(copied), value: None, format });
                    continue;
                }
                let value = match &pattern {
                    Pattern::Copy => {
                        // The tree's copy: the cell keeps the value
                        // of the source cell — and when the mode
                        // continues a series and the source has
                        // more than one cell, every cell continues
                        // **its own** series.
                        if mode.extends() && lane.cells.len() > 1 {
                            per_cell[source_index].value(step / lane.cells.len() as i64 + 1, forward)
                        } else {
                            Some(from.value.clone())
                        }
                    }
                    pattern => pattern.value(step, forward),
                };
                let Some(value) = value else {
                    // A number that would overflow is nothing: the
                    // cell keeps what it had.
                    continue;
                };
                writes.push(Write { cell: *cell, formula: None, value: Some(value), format });
            }
        }
        // The **cycle**: a copied formula may read itself, and a fill
        // that makes a cell read itself is refused — the whole fill,
        // and not a part of it.
        {
            let mut book = crate::book_of(session.document());
            if let Some(sheet_cells) = book.sheet_mut(&sheet_name) {
                for write in &writes {
                    match (&write.formula, &write.value) {
                        (Some(text), _) => sheet_cells.set_formula(write.cell, text.clone()),
                        (None, Some(value)) => sheet_cells.set_value(write.cell, value.clone()),
                        (None, None) => {}
                    }
                }
            }
            for write in &writes {
                if write.formula.is_some() && book.reads_itself(&sheet_name, write.cell) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the fill would make {sheet_name}!{} read itself",
                            write.cell.to_text()
                        ),
                    });
                }
            }
        }
        // The **writes**.
        let mut written: Vec<serde_json::Value> = Vec::new();
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            let mut sheet_cells = sheet_cells_mut(node);
            for write in &writes {
                match (&write.formula, &write.value) {
                    (Some(text), _) => sheet_cells.set_formula(write.cell, text.clone()),
                    (None, Some(value)) => sheet_cells.set_value(write.cell, value.clone()),
                    (None, None) => {}
                }
                // The tree's fill copies the **style** of the source,
                // and the style holds the format.
                if let Some(format) = &write.format {
                    sheet_cells.set_format(write.cell, Some(format.clone()));
                }
                match (&write.formula, &write.value) {
                    (Some(text), _) => written.push(json!({ "cell": write.cell.to_text(), "formula": text })),
                    (None, Some(value)) => {
                        written.push(json!({ "cell": write.cell.to_text(), "value": value.to_json() }))
                    }
                    (None, None) => {}
                }
            }
            node.data["cells"] = cells_into(&sheet_cells);
        }
        let count = written.len();
        Ok(Outcome::changed(format!("filled {count} cell(s)")).with_data(
            json!({ "sheet": sheet, "source": source.to_text(), "mode": mode_name(mode), "cells": written }),
        ))
    }
}

/// A cell of the **source**: what it holds.
struct Source {
    /// The cell.
    cell: CellRef,
    /// The formula's text, when the cell holds one.
    formula: Option<String>,
    /// The value, when the cell holds one.
    value: Value,
}

/// What one cell of a fill will hold.
struct Write {
    /// The cell that is written.
    cell: CellRef,
    /// The formula's text, when the cell is given one.
    formula: Option<String>,
    /// The value, when the cell is given one.
    value: Option<Value>,
    /// The format, when the source had one.
    format: Option<String>,
}

/// A **lane** of a fill: the cells of the source it comes from, and
/// the cells it writes.
struct Lane {
    /// The cells of the source.
    cells: Vec<CellRef>,
    /// The cells the lane writes.
    targets: Vec<CellRef>,
}

/// The pattern a lane is written with, for a mode.
///
/// The tree's table: the `copy` mode copies, the `series` mode — the
/// tree's default — continues what it finds, the `extend` mode counts
/// a lone number up, and the `alternate` mode counts a lone number up
/// and copies everything else.
fn the_pattern(values: &[Value], mode: Mode) -> Pattern {
    let pattern = Pattern::of(values);
    match mode {
        Mode::Copy => Pattern::Copy,
        Mode::Series => pattern,
        Mode::Extend => {
            if pattern.is_copy()
                && values.len() == 1
                && let Some(Value::Number(number)) = values.first()
            {
                let number = *number;
                return Pattern::Linear { first: number, last: number, step: 1.0 };
            }
            pattern
        }
        Mode::Alternate => {
            if values.len() == 1
                && let Some(Value::Number(number)) = values.first()
            {
                let number = *number;
                return Pattern::Linear { first: number, last: number, step: 1.0 };
            }
            Pattern::Copy
        }
    }
}

/// The name of a mode.
fn mode_name(mode: Mode) -> &'static str {
    match mode {
        Mode::Copy => "copy",
        Mode::Series => "series",
        Mode::Extend => "extend",
        Mode::Alternate => "alternate",
    }
}

/// The area an address names, `A1:B4` or `A1`.
fn area_at(text: &str, id: &str) -> Result<Area, EngineError> {
    let text = text.trim();
    let mut parts = text.split(':');
    let first = parts.next().unwrap_or_default();
    let second = parts.next();
    if parts.next().is_some() {
        return Err(EngineError::Failed { id: id.to_string(), message: format!("`{text}` is not a range") });
    }
    let first = CellRef::parse(first).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("`{first}` is not a cell"),
    })?;
    let Some(second) = second else {
        return Ok(Area::new(first, first));
    };
    let second = CellRef::parse(second).ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("`{second}` is not a cell"),
    })?;
    Ok(Area::new(first, second))
}
