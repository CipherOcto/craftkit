//! The **replacing** of text in a sheet.
//!
//! A sheet may be **searched** and what was found may be **replaced**:
//! the cells whose text holds what was looked for hold something else
//! afterwards.
//!
//! The rules:
//!
//! - the replacement is the **text as it is**: a wildcard is not
//!   something that can be replaced, and a search that holds one is
//!   refused;
//! - the **case** does not matter unless the search says it does, and
//!   when it does not matter the text is found whatever its case and
//!   the replacement is written as it was given;
//! - a cell that holds a **number** is a cell whose text may be
//!   changed, and the new text is read again as a value: a text that
//!   is a number is a number, and a text that is not is a text;
//! - a cell that holds a **formula** is changed only when the search
//!   asks for the formulas, and a formula that does not parse after
//!   the replacing is **refused** — a formula that is not a formula
//!   is not written down;
//! - a cell that is **locked** by the sheet's protection or covered
//!   by a merge is not written.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Text};

use crate::formula::{CellRef, Cells, Value};

/// The **text** of a cell with the **what** in it replaced by the
/// **with**.
///
/// The case of the search decides whether the text is found whatever
/// its case, and the replacement is written as it was given.
fn replaced(text: &str, what: &str, with: &str, case: bool) -> Option<String> {
    if case {
        if !text.contains(what) {
            return None;
        }
        return Some(text.replace(what, with));
    }
    let lower_text = text.to_lowercase();
    let lower_what = what.to_lowercase();
    if !lower_text.contains(&lower_what) {
        return None;
    }
    // The text is cut at the places the text holds the what, and the
    // pieces are put together again with the with.
    let mut out = String::new();
    let mut rest = text;
    let mut lower_rest: String = lower_text.clone();
    while let Some(at) = lower_rest.find(&lower_what) {
        out.push_str(&rest[..at]);
        out.push_str(with);
        rest = &rest[at + what.len()..];
        lower_rest = lower_rest[at + lower_what.len()..].to_string();
    }
    out.push_str(rest);
    Some(out)
}

commands! {
    /// **Replace** text in the cells of a sheet.
    ///
    /// A change: it writes the text it replaces with.
    grid.replace {
        area: "grid",
        layer: "grid",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The sheet to look in. Without it, the whole workbook is looked
            /// in.
            sheet: Option<NodeRef>,
            /// What to look for.
            what: Text,
            /// What to write in its place.
            with: Text,
            /// Whether the **case** matters.
            match_case: Option<Bool>,
            /// Whether the **whole cell** must match.
            whole_cell: Option<Bool>,
            /// Whether to look in the **formulas** rather than in the values.
            formulas: Option<Bool>,
        },
        run: {
            let id = "grid.replace".to_string();
            let what = self.what.0.clone();
            if what.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "there is nothing to look for".to_string(),
                });
            }
            if what.contains(['*', '?']) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a wildcard is something to look for, and not something to replace".to_string(),
                });
            }
            let with = self.with.0.clone();
            let case = self.match_case.map(|flag| flag.0).unwrap_or(false);
            let whole = self.whole_cell.map(|flag| flag.0).unwrap_or(false);
            let formulas = self.formulas.map(|flag| flag.0).unwrap_or(false);
            // The **sheets** that are looked in: the one that was named, or
            // every sheet of the workbook.
            let sheets: Vec<(u64, String)> = match self.sheet {
                Some(sheet) => {
                    let Some(node) = session.document().node(sheet.0) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("there is no sheet `{}`", sheet.0),
                        });
                    };
                    if node.kind != "grid.sheet" {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the node `{}` is a `{}` and not a sheet", sheet.0, node.kind),
                        });
                    }
                    vec![(sheet.0, node.name.clone())]
                }
                None => session
                    .document()
                    .walk()
                    .filter(|node| node.kind == "grid.sheet")
                    .map(|node| (node.id, node.name.clone()))
                    .collect(),
            };
            // The **cells** that hold the what, and what they hold
            // afterwards.
            let mut changes: Vec<(u64, CellRef, Json)> = Vec::new();
            for (sheet, name) in sheets {
                let Some(node) = session.document().node(sheet) else { continue };
                let cells = crate::cells_of(node);
                for (cell, _) in cells.iter() {
                    let text = crate::finding::text_of(session, sheet, &name, &cells, cell, formulas);
                    if !crate::finding::holds(&text, &what, case, whole) {
                        continue;
                    }
                    let new = if whole {
                        with.clone()
                    } else {
                        match replaced(&text, &what, &with, case) {
                            Some(new) => new,
                            None => continue,
                        }
                    };
                    // A **formula** cell: the formula is changed, and a
                    // formula that does not parse is refused.
                    let entry = cells.get(cell);
                    if formulas && entry.and_then(|entry| entry.formula.clone()).is_some() {
                        if crate::formula::parse(&new).is_err() {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!(
                                    "the formula of the cell {} would not be a formula any more",
                                    cell.to_text()
                                ),
                            });
                        }
                        changes.push((sheet, cell, json!({ "formula": new })));
                        continue;
                    }
                    // A **value**: the new text is read again as a value.
                    let value = match new.parse::<f64>() {
                        Ok(number) if number.is_finite() => Value::Number(number),
                        _ => Value::Text(new),
                    };
                    changes.push((sheet, cell, json!({ "value": value.to_json() })));
                }
            }
            // The cells that are **locked** or **covered** are not written.
            for (sheet, cell, _) in &changes {
                let Some(node) = session.document().node(*sheet) else { continue };
                crate::protection::refuse_locked(node, &crate::style_locks(session, *sheet), &[*cell], &id)?;
                crate::merges::refuse_covered(node, &[*cell], &id)?;
            }
            // The **writing**: the cells are changed in the order they
            // were found, so the report says what was changed.
            let mut written = 0usize;
            for (sheet, cell, change) in &changes {
                let Some(node) = session.document_mut()?.node_mut(*sheet) else { continue };
                let mut cells: Cells = crate::sheet_cells_mut(node);
                if let Some(formula) = change.get("formula").and_then(|formula| formula.as_str()) {
                    cells.set_formula(*cell, formula.to_string());
                } else if let Some(value) = change.get("value").and_then(|value| Value::from_json(value).into()) {
                    cells.set_value(*cell, value);
                }
                node.data["cells"] = crate::cells_into(&cells);
                written += 1;
            }
            Ok(Outcome::changed(format!("replaced in {written} cell(s)")).with_data(json!({
                "what": what,
                "with": with,
                "count": written,
            })))

        }
    }
}
