//! The **numbers stored as text** of a sheet.
//!
//! A cell that holds the text `42` is not a number: it is a text that
//! *reads* as one. A spreadsheet application marks such a cell, and
//! offers to turn it into the number it stands for.
//!
//! The list is **derived**: every cell is looked at, every time, and
//! the ones that hold a number as text are listed. No flag is stored,
//! so no flag can be stale.
//!
//! The rules — the tree's own:
//!
//! - a cell with a **formula** is not one: the tree flags a
//!   **constant** that would be a number if it were typed;
//! - a **text** is a number when it is a plain number — with a sign,
//!   a currency sign, a percent sign and thousands separators as the
//!   tree reads them — and when it is a number in **accounting
//!   parentheses**: `(5)` is `-5`, and `$(5)` is `(-5)`;
//! - a **date** is not a number here, and a **fraction** is not
//!   either: the tree flags what would be a number if typed, and a
//!   date is a date;
//! - a sheet with more than **ten thousand** such cells is
//!   **refused**: the answer would be a wall, and the tree stops at
//!   ten thousand and says nothing.
//!
//! Turning them into numbers is a **change**: the cell keeps its
//! format, and a cell that is locked by the sheet's protection, or
//! covered by a merge, is not written.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

use crate::formula::{CellRef, Cells, Value};

/// The most **numbers stored as text** a sheet may have.
const MAX_STORED: usize = 10_000;

/// The **number** a text stands for, if it is one.
///
/// The tree's own rule: a plain number, and a number in accounting
/// parentheses — `(5)` is `-5` and `$(5)` is `(-5)`. A text with a
/// sign inside the parentheses, or with parentheses inside them, is
/// not a number: the tree reads no sign and no nested parentheses.
fn number_of(text: &str) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Some(inner) = parenthesized(text) {
        let inner = inner.trim();
        if inner.starts_with(['-', '+', '(']) {
            return None;
        }
        return plain(inner).map(|number| -number);
    }
    plain(text)
}

/// The text **inside** a pair of parentheses, when the text is one
/// pair of them — with the currency sign that may stand before them
/// and the percent sign that may stand after.
fn parenthesized(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('$').map_or(text, str::trim_start);
    let rest = rest.strip_prefix('(')?;
    let inner = rest.strip_suffix(')')?;
    if inner.contains(['(', ')']) {
        return None;
    }
    Some(inner)
}

/// The **number** a plain text holds, if it is one.
///
/// The tree's own rule: a sign, a currency sign, a percent sign and
/// thousands separators may stand around the digits. A currency sign
/// and a percent sign together are not a number — the tree reads
/// `$5` and `5%` but not `$5%` — and the thousands separators must
/// be in groups of three.
fn plain(text: &str) -> Option<f64> {
    let mut rest = text.trim();
    let mut negative = false;
    if let Some(after) = rest.strip_prefix('-') {
        negative = true;
        rest = after.trim_start();
    } else if let Some(after) = rest.strip_prefix('+') {
        rest = after.trim_start();
    }
    let mut currency = false;
    if let Some(after) = rest.strip_prefix('$') {
        currency = true;
        rest = after.trim_start();
        if !negative && let Some(after) = rest.strip_prefix('-') {
            negative = true;
            rest = after;
        }
    }
    let mut percent = false;
    if let Some(after) = rest.strip_suffix('%') {
        percent = true;
        rest = after.trim_end();
    }
    if rest.is_empty() || currency && percent {
        return None;
    }
    // The **thousands separators**: groups of three, before the point.
    let digits = if rest.contains(',') {
        let (whole, fraction) =
            rest.split_once('.').map_or((rest, None), |(whole, rest)| (whole, Some(rest)));
        let groups: Vec<&str> = whole.split(',').collect();
        let first = groups.first()?;
        if first.is_empty() || first.len() > 3 || groups.iter().skip(1).any(|group| group.len() != 3) {
            return None;
        }
        let mut digits = groups.concat();
        if let Some(fraction) = fraction {
            digits.push('.');
            digits.push_str(fraction);
        }
        digits
    } else {
        rest.to_string()
    };
    let bytes = digits.as_bytes();
    if !bytes.iter().all(|byte| byte.is_ascii_digit() || matches!(byte, b'.' | b'e' | b'E' | b'+' | b'-')) {
        return None;
    }
    if !bytes.first().is_some_and(|byte| byte.is_ascii_digit() || *byte == b'.') {
        return None;
    }
    let mut number: f64 = digits.parse().ok()?;
    if !number.is_finite() {
        return None;
    }
    if percent {
        number /= 100.0;
    }
    if negative {
        number = -number;
    }
    Some(number)
}

/// The **cells** of a sheet that hold a number as text, with the
/// number each of them stands for.
fn stored_in(cells: &Cells) -> Vec<(CellRef, f64)> {
    let mut found = Vec::new();
    for (cell, entry) in cells.iter() {
        if entry.formula.is_some() {
            continue;
        }
        let Some(Value::Text(text)) = &entry.value else { continue };
        let Some(number) = number_of(text) else { continue };
        found.push((cell, number));
    }
    found
}

/// The **sheet** a command works on, and its name.
fn sheet_of(session: &Session, sheet: u64, id: &str) -> Result<(u64, String), EngineError> {
    let Some(node) = session.document().node(sheet) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
        });
    }
    Ok((sheet, node.name.clone()))
}

/// **Read** the cells of a sheet that hold a number as text.
///
/// The answer is every cell that holds a number as text, with the
/// **number** it stands for.
#[derive(Command)]
#[command(id = "grid.numbers.stored.as.text", area = "grid", layer = "grid", icon = "list")]
pub struct NumbersStoredAsText {
    /// The sheet.
    pub sheet: NodeRef,
}

impl NumbersStoredAsText {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.numbers.stored.as.text".to_string();
        let (sheet, _) = sheet_of(session, self.sheet.0, &id)?;
        let Some(node) = session.document().node(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let cells = crate::cells_of(node);
        let found = stored_in(&cells);
        if found.len() > MAX_STORED {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the sheet has more than {MAX_STORED} numbers stored as text, and the list is refused"
                ),
            });
        }
        let numbers: Vec<_> =
            found.iter().map(|(cell, number)| json!({ "cell": cell.to_text(), "number": number })).collect();
        let count = numbers.len();
        Ok(Outcome::new(format!("the sheet `{sheet}` has {count} number(s) stored as text"))
            .with_data(json!({ "sheet": sheet, "numbers": numbers })))
    }
}

/// **Turn** the numbers stored as text of a sheet into numbers.
///
/// A change: the cell holds the number it stood for, and it keeps its
/// format.
#[derive(Command)]
#[command(id = "grid.numbers.convert", area = "grid", layer = "grid", icon = "pencil", mutates, undoable)]
pub struct NumbersConvert {
    /// The sheet.
    pub sheet: NodeRef,
}

impl NumbersConvert {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.numbers.convert".to_string();
        let (sheet, _) = sheet_of(session, self.sheet.0, &id)?;
        let Some(node) = session.document().node(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let cells = crate::cells_of(node);
        let found = stored_in(&cells);
        if found.len() > MAX_STORED {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the sheet has more than {MAX_STORED} numbers stored as text, and they are not converted"
                ),
            });
        }
        // The cells that are **locked** or **covered** are not written.
        let addresses: Vec<CellRef> = found.iter().map(|(cell, _)| *cell).collect();
        crate::protection::refuse_locked(node, &crate::style_locks(session, sheet), &addresses, &id)?;
        crate::merges::refuse_covered(node, &addresses, &id)?;
        // The **writing**: every cell holds the number it stood for.
        let mut written = 0usize;
        for (cell, number) in &found {
            let Some(node) = session.document_mut()?.node_mut(sheet) else { continue };
            let mut cells: Cells = crate::sheet_cells_mut(node);
            cells.set_value(*cell, Value::Number(*number));
            node.data["cells"] = crate::cells_into(&cells);
            written += 1;
        }
        let converted: Vec<String> = found.iter().map(|(cell, _)| cell.to_text()).collect();
        Ok(Outcome::changed(format!("converted {written} number(s) stored as text"))
            .with_data(json!({ "sheet": sheet, "converted": converted, "count": written })))
    }
}
