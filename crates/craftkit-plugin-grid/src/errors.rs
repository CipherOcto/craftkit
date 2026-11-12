//! The **errors** of a sheet.
//!
//! A formula that cannot be worked out — a division by zero, a text
//! where a number was wanted, a reference to a sheet that is not
//! there — is an **error**, and the cell that holds it says so.
//!
//! The errors are **derived**: every formula is computed, every time,
//! and the ones that come out as errors are listed. No value is
//! stored, so no error can be stale.
//!
//! The rules:
//!
//! - a cell with **no formula** is not an error: it holds what it
//!   holds;
//! - a sheet with more than **ten thousand** errors is **refused**:
//!   the answer would be a wall, and the tree stops at ten thousand
//!   and says nothing.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

/// The most **errors** a sheet may have.
pub const MAX_ERRORS: usize = 10_000;

commands! {
    /// **Read** the **errors** of a sheet.
    ///
    /// The answer is every cell whose formula comes out as an error, with
    /// the **error** it says and the **formula** that gave it.
    grid.errors {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet.

            sheet: NodeRef,
        },
        run: {

            let id = "grid.errors".to_string();
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
            let name = node.name.clone();
            let cells = crate::cells_of(node);
            let book = crate::book_of(session.document());
            let mut errors = Vec::new();
            for (cell, entry) in cells.iter() {
                let Some(formula) = &entry.formula else { continue };
                let crate::formula::Value::Error(message) = book.value_of(&name, cell) else { continue };
                errors.push(json!({
                    "cell": cell.to_text(),
                    "error": message,
                    "formula": formula,
                }));
                if errors.len() > MAX_ERRORS {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the sheet has more than {MAX_ERRORS} errors, and the list is refused"),
                    });
                }
            }
            let count = errors.len();
            Ok(Outcome::new(format!("the sheet `{sheet}` has {count} error(s)"))
                .with_data(json!({ "sheet": sheet, "errors": errors })))

        }
    }
}
