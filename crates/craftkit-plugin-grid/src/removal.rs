//! The **taking away** of a sheet.
//!
//! A sheet may be taken away from its workbook, and the workbook must
//! keep **one visible sheet**: the tree's rule, and the rule the
//! hiding of a sheet obeys too — a workbook with no sheet to look at
//! is not a workbook.
//!
//! And the formulas that **named** the sheet are left naming it. The
//! tree rewrites them to `#REF!`, and here the formula is left as it
//! was written and the **evaluation** says what is wrong with it: the
//! value of the cell is the error "there is no sheet `Ledger`". The
//! error is **derived**, so it cannot be wrong, and a formula that
//! named the sheet is still the formula the caller wrote.
//!
//! # What the port found
//!
//! - **The tree writes `#REF!` and this tree says what is missing.**
//!   The tree's `adjust::delete_sheet` turns a reference to a sheet
//!   that is gone into `#REF!` — a mark, and not a word. Here a
//!   reference to a sheet that is gone is an **error value** with the
//!   name in it, and it is computed every time the cell is read, so a
//!   sheet that comes back makes the formula work again with nothing
//!   written down.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

commands! {
    /// **Take** a sheet away from its workbook.
    grid.sheet.remove {
        area: "grid",
        layer: "grid",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The sheet to take away.

            sheet: NodeRef,
        },
        run: {

            let id = "grid.sheet.remove".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let name = node.name.clone();
            let Some(workbook) = crate::tabs::workbook_of(session, sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` is in no workbook"),
                });
            };
            // The workbook must keep **one visible sheet**: the tree's rule,
            // and the rule the hiding of a sheet obeys. The tree counts the
            // sheets that are visible and refuses the taking away when
            // there is one of them, so a hidden sheet may not be taken away
            // either while it is the last one that can be looked at.
            let visible = crate::tabs::sheets_of(session, workbook).iter().filter(|(_, shown)| *shown).count();
            if visible <= 1 {
                return Err(EngineError::Failed {
                    id,
                    message: "a workbook must keep one visible sheet".to_string(),
                });
            }
            if !session.document_mut()?.remove_node(sheet) {
                return Err(EngineError::Failed { id, message: format!("there is no node `{sheet}`") });
            }
            Ok(Outcome::changed(format!("took the sheet `{name}` away"))
                .with_data(json!({ "sheet": sheet, "name": name })))

        }
    }
}
