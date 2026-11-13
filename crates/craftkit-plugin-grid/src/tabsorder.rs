//! The **order** of the sheets.
//!
//! The sheets of a workbook are in an **order** — the tabs of the
//! workbook, left to right — and a sheet may be **moved** in it.
//!
//! The order is the order of the sheets under the workbook in the
//! **tree**: the tree of the document says which sheet is first, and
//! the tree of the document is the truth about the order, so moving a
//! sheet moves it in the tree and the order follows.
//!
//! # What the port found
//!
//! - **The tree's sheets are a list and this tree's sheets are
//!   children.** The tree holds its sheets in a `Vec` and moves one by
//!   taking it out and putting it back at another index; the index of
//!   a sheet is then its position. Here a sheet is a **node** under the
//!   workbook, and the order of the children is the order of the tabs,
//!   so moving a sheet moves the child and the order follows.
//! - **The tree clamps the place and this tree refuses it.** The tree's
//!   `sheet.move` puts the sheet at `to.min(n - 1)` — a place past the
//!   end is the end. Here a place that is not there is **refused** and
//!   the refusal says what the places are, which is the rule this tree
//!   follows everywhere: refuse, and do not clamp.
//! - **The tree's copy of a sheet is not here.** The tree's `sheet.move`
//!   copies a sheet when the caller says `copy`, and the copy gets a
//!   name of its own (`Ledger (2)`) and its own copies of the names
//!   that referred to it. Copying a sheet is copying its **cells** and
//!   its formulas, and that is a command of its own.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome};

commands! {
    /// **Move** a sheet in the order of the tabs.
    ///
    /// The place is counted from **one**, and a place that is not there is
    /// refused.
    grid.sheet.move {
        area: "grid",
        layer: "grid",
        icon: "arrow",
        mutates,
        undoable,
        params: {
            /// The sheet to move.
            sheet: NodeRef,
            /// The place it takes, counted from one.
            to: craftkit_sdk::Number,
        },
        run: {
            let id = "grid.sheet.move".to_string();
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
            let Some(workbook) = crate::tabs::workbook_of(session, sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` is in no workbook"),
                });
            };
            let to = self.to.0;
            if !to.is_finite() || to.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a place is a whole number".to_string(),
                });
            }
            // The sheets of the workbook, in the order they are in.
            let sheets: Vec<u64> = session
                .document()
                .node(workbook)
                .map(|node| {
                    node.children
                        .iter()
                        .filter(|child| child.kind == "grid.sheet")
                        .map(|child| child.id)
                        .collect()
                })
                .unwrap_or_default();
            let count = sheets.len();
            if to < 1.0 || to > count as f64 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no place {to}: the workbook has {count} sheet(s)"),
                });
            }
            let to = to as usize;
            let from = sheets.iter().position(|other| *other == sheet).unwrap_or(0);
            // The sheet is taken out of the order and put back at the
            // place: the children of the workbook are the tabs, and the
            // order of the children is the order of the tabs.
            let Some(node) = session.document_mut()?.node_mut(workbook) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no workbook `{workbook}`"),
                });
            };
            let Some(moved) = node.take_child(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            let children = &mut node.children;
            // The place is where the sheet **ends up**, counted among the
            // sheets of the workbook once the sheet is there again: the
            // sheet that was in that place moves along.
            let mut at = children.len();
            let mut seen = 0usize;
            for (index, child) in children.iter().enumerate() {
                if child.kind == "grid.sheet" {
                    seen += 1;
                    if seen == to {
                        at = index;
                        break;
                    }
                }
            }
            children.insert(at, std::sync::Arc::new(moved));
            Ok(Outcome::changed(format!("the sheet `{sheet}` is now the {to}th tab")).with_data(json!({
                "sheet": sheet,
                "workbook": workbook,
                "from": from + 1,
                "to": to,
            })))

        }
    }
}
