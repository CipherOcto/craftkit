//! The **advanced filter** of a sheet: the tree's own **advanced
//! filter**.
//!
//! The tree's own: an **advanced filter** judges a **list** — a range
//! of rows under a row of **headings** — by a **criteria range**: a row
//! of headings and rows of **criteria** under it. The columns of a
//! criteria row are judged **together**, and the rows are judged **one
//! by one**: a row of the list is kept when it matches **any** of the
//! criteria rows.
//!
//! # The rules
//!
//! - The **headings** of the list and the headings of the criteria are
//!   the same words, and they are compared without case: the tree's own
//!   rule — and a criterion whose heading is not a heading of the list
//!   is **refused**: the tree's own rule.
//! - A criteria cell that is **empty** says nothing and is skipped: the
//!   tree's own rule.
//! - The **criterion** of a cell is the tree's own advanced-filter
//!   criterion: a **number** is the number, a text that begins with an
//!   operator is that comparison, and a text that begins with nothing
//!   **begins with** that text — the tree's own rule, and it is not the
//!   rule of a counting function's criterion.
//! - The **rows** that are kept are the rows of the list that match,
//!   and with the tree's own **unique** flag only the first row of a
//!   set of the same values is kept: the tree's own rule.
//! - The **copy**: the heading row of the list and the rows that are
//!   kept are written to the range the caller names, and that range
//!   may not **overlap** the list: the tree's own rule.
//!
//! # What it does not settle
//!
//! - **The tree's computed criteria are not here**: the tree may put a
//!   formula in a criteria cell and judge the row by what it
//!   computes, and here a criteria cell that holds a formula is
//!   **refused**.
//! - **The tree's own filtering in place is not here yet**: the tree
//!   hides the rows that do not match, and here the command copies the
//!   rows that do.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Text};

use crate::formula::{Anchor, Area, CellRef, Cells, Value};

/// The name of the command, in the messages.
const COMMAND: &str = "grid.filter.advanced";

/// The most columns a **criteria range** may have: the tree's own
/// bound.
const MAX_CRITERIA_COLUMNS: u64 = 1024;

/// The most rows a **criteria range** may have: the tree's own bound.
const MAX_CRITERIA_ROWS: u64 = 10_000;

/// The **area** a range names.
pub fn area_of(text: &str) -> Option<Area> {
    let trimmed = text.trim();
    let area = match trimmed.split_once(':') {
        Some((first, second)) => {
            let first = Anchor::parse(first)?;
            let second = Anchor::parse(second)?;
            Area::new(first.cell, second.cell)
        }
        None => {
            let only = Anchor::parse(trimmed)?;
            Area::new(only.cell, only.cell)
        }
    };
    Some(area)
}

/// The **text** a value shows: the tree's own text of a cell.
fn text_of(value: &Value) -> String {
    match value {
        Value::Empty => String::new(),
        Value::Number(number) => number.to_string(),
        Value::Text(text) => text.clone(),
        Value::Bool(flag) => if *flag { "TRUE" } else { "FALSE" }.to_string(),
        Value::Error(message) => message.clone(),
    }
}

/// The **operation** and the **operand** of a criterion, as the tree's
/// advanced filter reads them.
fn split_op(text: &str) -> (&str, &str) {
    for op in ["<>", ">=", "<=", "=", ">", "<"] {
        if let Some(rest) = text.strip_prefix(op) {
            return (op, rest);
        }
    }
    ("begins", text)
}

/// Whether a cell **matches** a criterion: the tree's own
/// `criterion_matches`.
fn criterion_matches(value: &Value, text: &str, criterion: &Value) -> bool {
    let criterion_text = match criterion {
        Value::Empty => return true,
        Value::Number(number) => {
            return value
                .as_number()
                .is_some_and(|other| (other - number).abs() <= 1e-12 * number.abs().max(1.0));
        }
        Value::Bool(flag) => return matches!(value, Value::Bool(other) if other == flag),
        Value::Error(message) => return matches!(value, Value::Error(other) if other == message),
        Value::Text(text) => text.clone(),
    };
    let (op, operand) = split_op(&criterion_text);
    if operand.is_empty() {
        return match op {
            "=" => value == &Value::Empty || text.is_empty(),
            "<>" => !(value == &Value::Empty || text.is_empty()),
            _ => true,
        };
    }
    let number = crate::criteria::number_text(operand);
    let lower_text = text.to_lowercase();
    let lower_operand = operand.to_lowercase();
    let comparison: Option<std::cmp::Ordering> = match (value, number) {
        (Value::Number(left), Some(right)) => left.partial_cmp(&right),
        (Value::Number(_), None) => None,
        (Value::Empty, _) => None,
        (_, Some(_)) if op != "begins" => None,
        _ => Some(crate::criteria::compare_text(text, operand)),
    };
    let wild = |pattern: &str| crate::criteria::wildcard_match(pattern, &lower_text);
    match op {
        "=" => {
            if number.is_none() && operand.contains(['*', '?']) {
                wild(&lower_operand)
            } else if number.is_none() {
                lower_text == lower_operand
            } else {
                comparison == Some(std::cmp::Ordering::Equal)
            }
        }
        "<>" => {
            if number.is_none() && operand.contains(['*', '?']) {
                !wild(&lower_operand)
            } else if number.is_none() {
                lower_text != lower_operand
            } else {
                comparison != Some(std::cmp::Ordering::Equal)
            }
        }
        ">" => comparison == Some(std::cmp::Ordering::Greater),
        "<" => comparison == Some(std::cmp::Ordering::Less),
        ">=" => matches!(comparison, Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)),
        "<=" => matches!(comparison, Some(std::cmp::Ordering::Less | std::cmp::Ordering::Equal)),
        _ => {
            // A plain text criterion **begins with** it: the tree's own
            // rule for an advanced filter.
            match (value, number) {
                (Value::Number(left), Some(right)) => (left - right).abs() <= 1e-12 * right.abs().max(1.0),
                _ => !matches!(value, Value::Empty) && wild(&format!("{lower_operand}*")),
            }
        }
    }
}

/// The **rows of the list** an advanced filter **keeps**.
///
/// The rows are **derived**: they are worked out from the cells each
/// time they are asked for, and they are not written down. The rules
/// are the tree's own: the columns of a criteria row are judged
/// together, the rows of the criteria are judged one by one, and with
/// the tree's own **unique** rule only the first row of a set of the
/// same values is kept.
pub fn kept_rows(cells: &Cells, list: &Area, criteria: &Area, unique: bool) -> Result<Vec<u32>, String> {
    // The **headings** of the list.
    let mut headings: Vec<String> = Vec::new();
    for column in list.start.col..=list.end.col {
        let cell = CellRef { col: column, row: list.start.row };
        let text = cells.get(cell).and_then(|entry| entry.value.clone()).map(|value| text_of(&value));
        headings.push(text.unwrap_or_default().trim().to_lowercase());
    }
    // The **criteria rows**: each a list of the columns it judges and
    // the criterion of each.
    let mut rows: Vec<Vec<(u32, Value)>> = Vec::new();
    for row in criteria.start.row + 1..=criteria.end.row {
        let mut conditions: Vec<(u32, Value)> = Vec::new();
        for column in criteria.start.col..=criteria.end.col {
            let cell = CellRef { col: column, row };
            let Some(entry) = cells.get(cell) else { continue };
            if entry.formula.is_some() {
                // The tree's own **computed** criteria are not here yet:
                // a criteria cell that holds a formula is refused.
                return Err("a criteria cell that holds a formula is not supported here".to_string());
            }
            let Some(value) = entry.value.clone() else { continue };
            if value == Value::Empty {
                continue;
            }
            let heading_cell = CellRef { col: column, row: criteria.start.row };
            let heading = cells
                .get(heading_cell)
                .and_then(|entry| entry.value.clone())
                .map(|value| text_of(&value))
                .unwrap_or_default()
                .trim()
                .to_lowercase();
            match headings.iter().position(|head| !head.is_empty() && *head == heading) {
                Some(offset) => conditions.push((offset as u32, value)),
                None => {
                    return Err(format!(
                        "the criteria label `{heading}` does not match a column of the list"
                    ));
                }
            }
        }
        rows.push(conditions);
    }
    let mut kept: Vec<u32> = Vec::new();
    let mut seen: Vec<Vec<String>> = Vec::new();
    for row in list.start.row + 1..=list.end.row {
        let mut matches = rows.is_empty();
        for conditions in &rows {
            let all = conditions.iter().all(|(offset, criterion)| {
                let cell = CellRef { col: list.start.col + offset, row };
                let value = cells.get(cell).and_then(|entry| entry.value.clone()).unwrap_or(Value::Empty);
                let text = text_of(&value);
                criterion_matches(&value, &text, criterion)
            });
            if all {
                matches = true;
                break;
            }
        }
        if matches && unique {
            let key: Vec<String> = (list.start.col..=list.end.col)
                .map(|column| {
                    let cell = CellRef { col: column, row };
                    let value = cells.get(cell).and_then(|entry| entry.value.clone()).unwrap_or(Value::Empty);
                    text_of(&value).to_lowercase()
                })
                .collect();
            if seen.contains(&key) {
                matches = false;
            } else {
                seen.push(key);
            }
        }
        if matches {
            kept.push(row);
        }
    }
    Ok(kept)
}

commands! {
    /// **Filter** a list of rows by a **criteria range** — the tree's own
    /// advanced filter.
    grid.filter.advanced {
        area: "grid",
        layer: "grid",
        icon: "filter",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The **list**: a range of rows under a row of headings.

            range: Text,
            /// The **criteria**: a range of headings and criteria rows.

            criteria: Text,
            /// The range the rows that are kept are **copied** to.
            ///
            /// Leave it out and the command only says which rows are kept.

            copy_to: Option<Text>,
            /// Whether only the **first** row of a set of the same values is
            /// kept.

            unique: Option<Bool>,
        },
        run: {

            let id = COMMAND.to_string();
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
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            // The **list**: a range with a row of headings at the top.
            let Some(list) = area_of(&self.range.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a range of cells", self.range.0.trim()),
                });
            };
            if list.start.row > list.end.row {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the list of the advanced filter is empty".to_string(),
                });
            }
            // The **criteria**: a range with a row of headings at the top.
            let Some(criteria) = area_of(&self.criteria.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a range of cells", self.criteria.0.trim()),
                });
            };
            if criteria.count() > MAX_CRITERIA_COLUMNS * MAX_CRITERIA_ROWS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the criteria range is too large".to_string(),
                });
            }
            let cells = crate::cells_of(node);
            // The **rows of the list** that match: the rows of the criteria
            // are judged one by one, and the columns of a row together.
            let unique = matches!(self.unique, Some(Bool(true)));
            let kept = kept_rows(&cells, &list, &criteria, unique)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            // The rows of the list are counted from **one**, as the rows of
            // a filter are: the tree's own counting.
            let kept: Vec<u64> = kept.iter().map(|row| *row as u64 + 1).collect();
            let matched = kept.len();
            // The **copy**: the heading row and the rows that are kept are
            // written to the range the caller names.
            let Some(copy_to) = &self.copy_to else {
                // The tree's own **filtering in place**: the filter is set
                // on the sheet, and the rows that do not match are hidden.
                let fact = json!({
                    "range": self.range.0.trim(),
                    "criteria": self.criteria.0.trim(),
                    "unique": unique,
                });
                if let Some(node) = session.document_mut()?.node_mut(sheet) {
                    node.data["advanced_filter"] = fact;
                }
                // The rows that do **not** match are hidden — and the rows a
                // filter hides are **derived**: they are worked out from the
                // cells each time they are asked for, and they are not
                // written down.
                let hidden: Vec<u64> = (list.start.row + 1..=list.end.row)
                    .map(|row| row as u64 + 1)
                    .filter(|row| !kept.contains(row))
                    .collect();
                return Ok(Outcome::changed(format!("{} rows of the list match", matched)).with_data(json!({
                    "sheet": sheet,
                    "matched": matched,
                    "rows": kept,
                    "hidden_rows": hidden,
                })));
            };
            let Some(destination) = area_of(&copy_to.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{}` is not a range of cells", copy_to.0.trim()),
                });
            };
            // The destination may not **overlap** the list: the tree's own
            // rule.
            if destination.start.row <= list.end.row
                && list.start.row <= destination.end.row
                && destination.start.col <= list.end.col
                && list.start.col <= destination.end.col
            {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the range the rows are copied to overlaps the list".to_string(),
                });
            }
            let width = list.end.col - list.start.col + 1;
            let rows_out: Vec<u32> =
                std::iter::once(list.start.row).chain(kept.iter().map(|row| *row as u32 - 1)).collect();
            let Some(node) = session.document_mut()?.node_mut(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let mut cells = crate::sheet_cells_mut(node);
            for (index, row) in rows_out.iter().enumerate() {
                for column in 0..width {
                    let from = CellRef { col: list.start.col + column, row: *row };
                    let to = CellRef {
                        col: destination.start.col + column,
                        row: destination.start.row + index as u32,
                    };
                    let entry = cells.get(from).cloned();
                    match entry {
                        Some(entry) => {
                            cells.set_value(to, entry.value.clone().unwrap_or(Value::Empty));
                            if let Some(format) = entry.format.clone() {
                                cells.set_format(to, Some(format));
                            }
                            if let Some(style) = entry.style.clone() {
                                cells.set_style(to, Some(style));
                            }
                        }
                        None => {
                            cells.clear_contents(to);
                        }
                    }
                }
            }
            node.data["cells"] = crate::cells_into(&cells);
            let end = CellRef {
                col: destination.start.col + width - 1,
                row: destination.start.row + rows_out.len() as u32 - 1,
            };
            Ok(Outcome::changed(format!("{} rows of the list are copied", matched)).with_data(json!({
                "sheet": sheet,
                "matched": matched,
                "rows": kept,
                "range": Area::new(destination.start, end).to_text(),
            })))

        }
    }
}
