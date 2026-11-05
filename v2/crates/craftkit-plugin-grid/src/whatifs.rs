//! The **what-if** of a sheet.
//!
//! A what-if asks a question of a sheet by **trying** values: the
//! input cell takes this number, and the result cell says what the
//! sheet says then.
//!
//! The sheet is **not changed**: the trial is made in a copy of the
//! workbook, and the copy is thrown away when the table is done. A
//! what-if that left the sheet changed would be a second edit wearing
//! a question's coat.
//!
//! # What the port found
//!
//! - **The tree's what-if has two forms**: a **data table**, which
//!   tries a list of inputs and says what each of them gives, and a
//!   **goal seek**, which tries inputs until the result reaches a
//!   target. Here the **data table** is the command, and the goal
//!   seek is not written yet: a goal seek is a search, and a search
//!   needs to say how far to look and when to stop.
//! - **The tree's data table takes a whole range of inputs and lays
//!   the results out beside them.** Here the inputs are a **list** and
//!   the results are a **list** too: one input, one result, in the
//!   order they were given.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{Book, CellRef, Value};

/// How many values a what-if may try.
///
/// The work is one evaluation of the sheet for each value, and the
/// number of them is bounded so that a what-if cannot take the machine
/// for the whole afternoon.
const MAX_VALUES: usize = 256;

/// The **cell** a name says, if it says one.
pub(crate) fn cell_of(text: &str, id: &str, what: &str) -> Result<CellRef, EngineError> {
    crate::cell_at(text, id).map_err(|_| EngineError::Failed {
        id: id.to_string(),
        message: format!("`{text}` is not a cell: the {what} is a cell"),
    })
}

/// Ask a sheet what it would say for a list of values.
#[derive(Command)]
#[command(id = "grid.whatif", area = "grid", layer = "grid", icon = "list")]
pub struct WhatIf {
    /// The sheet.
    pub sheet: NodeRef,
    /// The cell whose value is **varied**.
    pub input: Text,
    /// The cell whose value is **read**.
    pub result: Text,
    /// The values to try: a list of numbers.
    pub values: Json,
}

impl WhatIf {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.whatif".to_string();
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
        let input = cell_of(&self.input.0, &id, "input")?;
        let result = cell_of(&self.result.0, &id, "result")?;
        if input == result {
            return Err(EngineError::Failed {
                id,
                message: "the input and the result are the same cell".to_string(),
            });
        }
        let Some(list) = self.values.as_array() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the values of a what-if are a list of numbers".to_string(),
            });
        };
        if list.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a what-if tries a value or more".to_string(),
            });
        }
        if list.len() > MAX_VALUES {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a what-if tries {} values or fewer", MAX_VALUES),
            });
        }
        let mut values: Vec<f64> = Vec::with_capacity(list.len());
        for value in list {
            let Some(number) = value.as_f64() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the values of a what-if are numbers".to_string(),
                });
            };
            if !number.is_finite() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the values of a what-if are numbers".to_string(),
                });
            }
            values.push(number);
        }
        // The **copy** of the workbook: the trials are made in it and
        // the sheet itself is not touched.
        let mut book = crate::book_of(session.document());
        let name = node.name.clone();
        let mut table: Vec<Json> = Vec::with_capacity(values.len());
        for value in values {
            let result = trial(&mut book, &name, input, result, value, &id)?;
            table.push(json!({ "value": value, "result": result.to_json() }));
        }
        Ok(Outcome::new(format!("the sheet `{sheet}` was asked {} question(s)", table.len())).with_data(
            json!({
                "sheet": sheet,
                "input": self.input.0,
                "result": self.result.0,
                "table": table,
            }),
        ))
    }
}

/// The **trial**: the input cell takes a value in a copy of the
/// workbook, and the result cell says what the sheet says then.
///
/// The workbook is a **copy** — the sheet itself is never touched — and
/// the trial is where the what-if and the goal seek meet.
pub(crate) fn trial(
    book: &mut Book,
    sheet: &str,
    input: CellRef,
    result: CellRef,
    value: f64,
    id: &str,
) -> Result<Value, EngineError> {
    let Some(cells) = book.sheet_mut(sheet) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    cells.set_value(input, Value::Number(value));
    Ok(book.value_of(sheet, result))
}
