//! The **filling** of a range.
//!
//! A fill takes the value of one cell and puts it in a range: the tree
//! calls it the **AutoFill**, and it has two shapes — the plain drag,
//! which **copies**, and the series, which **counts on**.
//!
//! The tree's plain drag copies a lone number, so a drag of `1` gives
//! `1, 1, 1`; the tree's series mode counts it up, and its `Ctrl`+drag
//! does the same. Here the fill has a **mode**, and it says which:
//! `copy` and `series`.
//!
//! The patterns of the series are the tree's:
//!
//! - a **number** counts on by one;
//! - **text with a number in it** — `Item 1`, `Q003x` — counts the
//!   number on and keeps the width of the digits;
//! - anything else is **copied**.
//!
//! The tree's **lists** — the days, the months, the quarters — and the
//! tree's **dates** are not here: a text that is a day of the week is
//! copied here, and the tree would step through the week.
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
//! # What it does not do
//!
//! - **A formula is not filled.** The tree copies a formula and
//!   **shifts its relative references** for every cell it writes, and
//!   that is a thing of its own; here a source that holds a formula is
//!   refused.
//! - **The tree's dates and lists are not here**, and neither is the
//!   tree's **quarter** and its **ordinal** (`1st`, `2nd`).

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{self, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Value};
use crate::{cells_into, cells_of, sheet_cells_mut};

/// The most cells one fill may write.
///
/// The tree bounds its fill at five million cells; this one is bounded
/// far lower, because the cells of a sheet live in a **payload** and a
/// payload is not a place for a hundred thousand cells.
const MAX_FILL_CELLS: u64 = 10_000;

/// What a fill writes.
#[derive(Debug, Clone)]
enum Pattern {
    /// The value is copied.
    Copy(Value),
    /// A number that counts on.
    Linear {
        /// The number it starts from.
        base: f64,
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
        base: i64,
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
            Pattern::Copy(value) => Some(value.clone()),
            Pattern::Linear { base, step } => Some(Value::Number(round15(base + step * (k * sign) as f64))),
            Pattern::TextNum { prefix, suffix, base, step, width } => {
                let number = step.checked_mul(k)?.checked_mul(sign)?.checked_add(*base)?;
                let number = number.checked_abs()?;
                Some(Value::Text(format!("{prefix}{number:0width$}{suffix}", width = *width)))
            }
        }
    }

    /// The pattern of a **series** of one value.
    fn of(value: &Value) -> Pattern {
        match value {
            Value::Number(number) => Pattern::Linear { base: *number, step: 1.0 },
            Value::Text(text) => match split_num(text) {
                Some((prefix, number, suffix, width)) => {
                    Pattern::TextNum { prefix, suffix, base: number, step: 1, width }
                }
                None => Pattern::Copy(value.clone()),
            },
            _ => Pattern::Copy(value.clone()),
        }
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

/// **Fill** a range from a cell.
///
/// The target must **hold the source at a corner**, and the fill goes
/// down or up, or right or left, from there. The mode is `copy` — the
/// value is copied — or `series`, and the series counts on: a number
/// counts by one, and a text with a number in it counts its number on.
///
/// A source that holds a **formula** is refused: the tree copies the
/// formula and shifts its references, and that is a thing of its own.
#[derive(Command)]
#[command(id = "grid.range.fill", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct FillRange {
    /// The sheet it goes in.
    pub sheet: NodeRef,
    /// The cell the fill comes from.
    pub from: Text,
    /// The range the fill goes into, `A1:A10`.
    pub to: Text,
    /// `copy` or `series`. The tree's plain drag is `copy` and the
    /// tree's series is `series`; the tree's command defaults to the
    /// series, and so does this one.
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
        let source = CellRef::parse(self.from.0.trim()).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("`{}` is not a cell", self.from.0.trim()),
        })?;
        let target = area_at(&self.to.0, &id)?;
        // The tree's command has a mode as well, and it **defaults to
        // the series**: the tree's drag copies and the tree's command
        // counts on.
        let mode = match &self.mode {
            Some(mode) => mode.0.trim().to_lowercase(),
            None => "series".to_string(),
        };
        if mode != "copy" && mode != "series" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{mode}` is not a mode: a fill is `copy` or `series`"),
            });
        }
        // The target must **hold the source at a corner**.
        if !target.covers(source) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the target {} does not hold the source {}",
                    target.to_text(),
                    source.to_text()
                ),
            });
        }
        let at_corner = (source.row == target.start.row || source.row == target.end.row)
            && (source.col == target.start.col || source.col == target.end.col);
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
        // The **direction**: down or up when the target has rows, and
        // right or left when it has one row.
        let vertical = target.end.row > target.start.row;
        let forward = if vertical { target.end.row > source.row } else { target.end.col > source.col };
        let mut cells: Vec<CellRef> = Vec::new();
        if vertical {
            if forward {
                for row in source.row + 1..=target.end.row {
                    cells.push(CellRef { col: source.col, row });
                }
            } else {
                for row in (target.start.row..source.row).rev() {
                    cells.push(CellRef { col: source.col, row });
                }
            }
        } else if forward {
            for col in source.col + 1..=target.end.col {
                cells.push(CellRef { col, row: source.row });
            }
        } else {
            for col in (target.start.col..source.col).rev() {
                cells.push(CellRef { col, row: source.row });
            }
        }
        if cells.len() as u64 > MAX_FILL_CELLS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the fill is too large: it would write {} cells, and a fill writes at most {MAX_FILL_CELLS}",
                    cells.len()
                ),
            });
        }
        // The **source**: what it holds.
        let (source_value, source_format) = {
            let sheet_cells = cells_of(node);
            let format = sheet_cells.format(source).map(|code| code.to_string());
            let value = match sheet_cells.get(source) {
                Some(entry) if entry.formula.is_some() => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the source {} holds a formula, and a fill of a formula is not here: the tree shifts the references of the formula it copies",
                            source.to_text()
                        ),
                    });
                }
                Some(entry) => entry.value.clone(),
                None => None,
            };
            (value, format)
        };
        let Some(value) = source_value else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is nothing at the source {}", source.to_text()),
            });
        };
        let pattern = match mode.as_str() {
            "series" => Pattern::of(&value),
            _ => Pattern::Copy(value),
        };
        // The **fill**: every cell of the range past the source.
        let mut written: Vec<serde_json::Value> = Vec::with_capacity(cells.len());
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            let mut sheet_cells = sheet_cells_mut(node);
            for cell in cells {
                let Some(value) = pattern.value(written.len() as i64 + 1, forward) else {
                    // A number that would overflow is nothing: the cell
                    // keeps what it had.
                    continue;
                };
                sheet_cells.set_value(cell, value.clone());
                // The tree's fill copies the **style** of the source,
                // and the style holds the format.
                if let Some(format) = &source_format {
                    sheet_cells.set_format(cell, Some(format.clone()));
                }
                written.push(json!({ "cell": cell.to_text(), "value": value.to_json() }));
            }
            node.data["cells"] = cells_into(&sheet_cells);
        }
        let count = written.len();
        Ok(Outcome::changed(format!("filled {count} cell(s)"))
            .with_data(json!({ "sheet": sheet, "source": source.to_text(), "mode": mode, "cells": written })))
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
