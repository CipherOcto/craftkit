//! The **filter** of a sheet.
//!
//! A filter is a **column** and a **criterion**, and the criterion is
//! an operation and a value: `B` is `>` `10`, and the rows whose `B`
//! is not more than ten are the rows the filter **hides**.
//!
//! And the rows a filter hides are **derived**: they are worked out
//! from the cells each time they are asked for, and they are not
//! written down. A row hidden **by hand** (§345) is a fact of the
//! sheet and a row hidden by a **filter** is a consequence of one —
//! and the tree's `SUBTOTAL` tells them apart: the rows a filter
//! hides are left out whatever the function number, and the rows
//! hidden by hand are left out only in the hundred-and-more form.
//!
//! The tree's criterion is a pair of strings — an operation and a
//! value — and the value may hold the tree's `*` wildcard: `*abc*` is
//! "contains abc" and `abc*` is "begins with abc".
//!
//! # What the port found
//!
//! - **The tree's filter has many kinds of criterion and this one has
//!   one.** The tree's criterion may be a list of values, a pair of
//!   comparisons joined by and or or, a top-ten, an above-the-average
//!   or a colour. Here a criterion is **one** operation and **one**
//!   value — the pair of comparisons — and the other kinds are
//!   commands of their own when they come.
//! - **The tree's filter is over a range and this one's is over a
//!   column.** The tree's filter has a range and a criterion for each
//!   column of it. Here a sheet has **one** filter and it is over one
//!   column, and the rows it hides are the rows of the sheet.
//! - **The tree's comparison is Excel's and this one's is simpler.**
//!   The tree compares two numbers rounded to fifteen significant
//!   digits, treats an empty cell as zero, as `""` or as `FALSE`
//!   depending on the other side, and orders values of different kinds
//!   by rank. Here the comparison operations compare **numbers** — an
//!   empty cell is zero — and the equality operations compare the
//!   **text** of the cell, without case, with the tree's `*`.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

use crate::formula::{CellRef, FILTER_OPS, Filter};

/// **Filter** a sheet by a column.
///
/// The criterion is an operation and a value: `=` `!=` `<` `<=` `>` or
/// `>=`, and a value that may hold the tree's `*`.
#[derive(Command)]
#[command(id = "grid.filter.set", area = "grid", layer = "grid", icon = "filter", mutates, undoable)]
pub struct SetFilter {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// Which column it filters: `A`, `B`, `AA`.
    pub column: Text,
    /// The operation: `=`, `!=`, `<`, `<=`, `>` or `>=`.
    pub op: Text,
    /// The value it compares with. A comparison operation needs a
    /// **number**, and the value may hold the tree's `*`.
    pub value: Text,
}

impl SetFilter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.filter.set".to_string();
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
        let column = self.column.0.trim().to_uppercase();
        let Some(index) = CellRef::column_index(&column) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
            });
        };
        let op = self.op.0.trim().to_string();
        if !Filter::op_is_known(&op) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no operation `{op}`: the operations are {}",
                    FILTER_OPS.join(", ")
                ),
            });
        }
        let value = self.value.0.clone();
        if (op == "<" || op == "<=" || op == ">" || op == ">=") && value.trim().parse::<f64>().is_err() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the operation `{op}` compares numbers, and `{value}` is not one"),
            });
        }
        let filter = Filter { column: index, op, value: value.clone() };
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["filter"] = json!({
                "column": column,
                "op": filter.op,
                "value": value,
            });
        }
        Ok(Outcome::changed(format!("the sheet `{sheet}` is filtered by the column {column}"))
            .with_data(json!({ "sheet": sheet, "column": column, "op": filter.op, "value": value })))
    }
}

/// Take the **filter** away from a sheet.
#[derive(Command)]
#[command(id = "grid.filter.clear", area = "grid", layer = "grid", icon = "filter", mutates, undoable)]
pub struct ClearFilter {
    /// The sheet.
    pub sheet: NodeRef,
}

impl ClearFilter {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.filter.clear".to_string();
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
        if node.data.get("filter").is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the sheet `{sheet}` is not filtered"),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["filter"] = json!(null);
        }
        Ok(Outcome::changed(format!("the filter of the sheet `{sheet}` is taken away"))
            .with_data(json!({ "sheet": sheet })))
    }
}

/// The **rows** a sheet's filter hides.
///
/// The rows are **derived**: they are worked out from the cells each
/// time they are asked for.
#[derive(Command)]
#[command(id = "grid.filter.rows", area = "grid", layer = "grid", icon = "list")]
pub struct FilteredRows {
    /// The sheet.
    pub sheet: NodeRef,
}

impl FilteredRows {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.filter.rows".to_string();
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
        let book = crate::book_of(session.document());
        let name = node.name.clone();
        let rows = book.filtered_rows(&name);
        let count = rows.len();
        Ok(Outcome::new(format!("{count} row(s) are hidden by the filter")).with_data(json!({
            "sheet": sheet,
            "rows": rows,
        })))
    }
}
