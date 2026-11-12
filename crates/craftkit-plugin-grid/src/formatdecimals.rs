//! The **decimals** of a format: one more of them, or one less.
//!
//! The tree's rules:
//!
//! - a cell whose format is the **General** one has the decimals of
//!   the number it shows, and the format that is written is a plain
//!   `0` with that many decimals;
//! - a format with **sections** — `0.00;0.000` — has its sections
//!   moved one by one;
//! - the last **digit placeholder** of a section — a `0` or a `#` — is
//!   the one that counts, and the **point** is the one that separates
//!   the decimals from the whole part;
//! - a section with **no digit placeholder** — a date, say — is left
//!   as it is.
//!
//! # What the port found
//!
//! - **The tree's format is a style and this one's is the cell's own**
//!   (§ the format of a cell), and so the decimals that move are the
//!   decimals of the cell and not of a style that other cells share.
//! - **The tree takes the last decimal away and leaves an empty
//!   section** when the format has no decimals left: `0` becomes the
//!   empty text, which is not a format. Here a format that has no
//!   decimals to take away is **refused** and the cell is left as it
//!   was.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::formula::Value;

/// The most **decimals** a format may ask for: the tree's own bound on
/// a format.
const MAX_DECIMALS: usize = 15;

/// The most **characters** a format may hold: the tree's own bound.
const MAX_FORMAT_CHARS: usize = 255;

/// The **code** of a format with its decimals moved by `delta`, or
/// nothing when the format cannot hold them.
///
/// The tree's own rule: the last digit placeholder of a section is the
/// one that counts.
fn decimals_of(code: &str, delta: i32) -> Option<String> {
    let sections = crate::sections::split(code)?;
    let mut moved = Vec::with_capacity(sections.len());
    for section in &sections {
        moved.push(section_of(section, delta)?);
    }
    Some(moved.join(";"))
}

/// One **section** of a format, with its decimals moved by `delta`.
///
/// Nothing when the section has no decimals to take away.
fn section_of(section: &str, delta: i32) -> Option<String> {
    let characters: Vec<char> = section.chars().collect();
    let mut in_quote = false;
    let mut in_bracket = false;
    let mut last_digit: Option<usize> = None;
    let mut point: Option<usize> = None;
    for (at, character) in characters.iter().enumerate() {
        if *character == '"' {
            in_quote = !in_quote;
            continue;
        }
        if *character == '[' && !in_quote {
            in_bracket = true;
            continue;
        }
        if *character == ']' && !in_quote {
            in_bracket = false;
            continue;
        }
        if in_quote || in_bracket {
            continue;
        }
        if matches!(character, 'E' | 'e') {
            break;
        }
        if matches!(character, '0' | '#' | '?') {
            last_digit = Some(at);
        }
        if *character == '.' && point.is_none() {
            point = Some(at);
        }
    }
    let Some(last) = last_digit else {
        // A section with no digit placeholder has no decimals: the
        // tree's own rule, and it is left as it is.
        return Some(section.to_string());
    };
    let mut out = characters;
    if delta > 0 {
        out.insert(last + 1, '0');
        if point.is_none() {
            out.insert(last + 1, '.');
        }
        return Some(out.into_iter().collect());
    }
    // Taking a decimal away: the last digit placeholder has to be in
    // the decimals, and there has to be a point to take away with it.
    let point = point?;
    if last <= point {
        return None;
    }
    out.remove(last);
    if last == point + 1 {
        out.remove(point);
    }
    Some(out.into_iter().collect())
}

/// The **decimals** a number shows, as the tree counts them.
fn decimals_of_number(value: &Value) -> usize {
    let text = crate::formula::display_text(value);
    text.split_once('.').map(|(_, decimals)| decimals.chars().count()).unwrap_or(0)
}

/// **Move** the decimals of the format of a cell.
///
/// The tree's Increase Decimal and Decrease Decimal: the format of the
/// cell gains a decimal or loses one, and everything else of it stays
/// as it is.
#[derive(Command)]
#[command(
    id = "grid.cell.format.decimals",
    area = "grid",
    layer = "grid",
    icon = "pencil",
    mutates,
    undoable
)]
pub struct SetFormatDecimals {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// How many decimals to move: one more, or one less.
    pub delta: Number,
}

impl SetFormatDecimals {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.format.decimals".to_string();
        let sheet = self.sheet.0;
        let cell = crate::cell_at(&self.cell.0, &id)?;
        let delta = self.delta.0;
        if !delta.is_finite() || delta.fract() != 0.0 || delta == 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the decimals of a format move by a whole number, and {delta} is not one"),
            });
        }
        if delta.abs() > MAX_DECIMALS as f64 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the decimals of a format move by {MAX_DECIMALS} or fewer at a time"),
            });
        }
        let delta = delta as i32;
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
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
        crate::protection::refuse_format(node, &id)?;
        let cells = crate::cells_of(node);
        let entry = cells.get(cell);
        let code = entry.and_then(|entry| entry.format.clone());
        // The **General** format has the decimals of the number the
        // cell shows: the tree's own rule.
        let general = match &code {
            Some(code) => crate::format::Format::parse(code) == Some(crate::format::Format::General),
            None => true,
        };
        let moved = if general {
            let shown = entry.and_then(|entry| entry.value.as_ref()).map(decimals_of_number).unwrap_or(0);
            let count = (shown as i32 + delta).max(0) as usize;
            if count == 0 { "0".to_string() } else { format!("0.{}", "0".repeat(count)) }
        } else {
            let code = code.clone().unwrap_or_default();
            decimals_of(&code, delta).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the format `{code}` has no decimals to take away"),
            })?
        };
        if moved.chars().count() > MAX_FORMAT_CHARS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a format holds {MAX_FORMAT_CHARS} characters or fewer"),
            });
        }
        if crate::format::Format::parse(&moved).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the format `{moved}` is not one this tree writes"),
            });
        }
        let same = code.as_deref() == Some(moved.as_str());
        if !same {
            let mut cells = crate::sheet_cells_mut(node);
            cells.set_format(cell, Some(moved.clone()));
            node.data["cells"] = crate::cells_into(&cells);
        }
        let message = if same {
            format!("the format of the cell {} has no decimals to move", cell.to_text())
        } else {
            format!("the format of the cell {} is `{moved}`", cell.to_text())
        };
        let outcome = if same { Outcome::new(message) } else { Outcome::changed(message) };
        Ok(outcome.with_data(json!({
            "sheet": sheet,
            "cell": cell.to_text(),
            "format": moved,
            "delta": delta,
        })))
    }
}
