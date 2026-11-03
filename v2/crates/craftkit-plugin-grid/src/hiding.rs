//! The **hiding** of a row and a column.
//!
//! A row may be **hidden** and a column may be hidden, and a hidden one
//! is still there: its cells are still there, they still hold what
//! they hold, and they are still worked out. The tree's hidden row is
//! not a row that is gone; it is a row that is not shown.
//!
//! And the tree's own finding is here: the tree's `SUBTOTAL` leaves
//! the hidden rows out of its sum when the function number says so,
//! and the tree's `SUM` does not. This tree has no `SUBTOTAL` — and so
//! nothing here changes what a formula reads: a hidden row is a row
//! that is not shown, and that is all it is.
//!
//! # What the port found
//!
//! - **The tree's hidden row is a flag on the row and this tree's is a
//!   list on the sheet.** The tree's sheet carries a line of
//!   information for every row, hidden or not, and the hidden flag is
//!   on that line. Here a sheet carries the **list** of the rows that
//!   are hidden and the list of the columns that are hidden, and a row
//!   that is not on the list is shown.
//! - **The tree's hidden row has its own size.** The tree's row of no
//!   height is a hidden row, and the tree's height command and its
//!   hide command are two ways of saying the same thing. Here they
//!   are not: a size of nothing is **refused** (the module `sizes`)
//!   and the hiding is a fact of its own.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The **rows** of a sheet that are hidden.
fn hidden_rows(node: &craftkit_sdk::Node) -> Vec<u64> {
    node.data
        .get("hidden_rows")
        .and_then(|rows| rows.as_array())
        .map(|rows| rows.iter().filter_map(|row| row.as_u64()).collect::<Vec<u64>>())
        .unwrap_or_default()
}

/// The **columns** of a sheet that are hidden.
fn hidden_columns(node: &craftkit_sdk::Node) -> Vec<String> {
    node.data
        .get("hidden_columns")
        .and_then(|columns| columns.as_array())
        .map(|columns| {
            columns
                .iter()
                .filter_map(|column| column.as_str().map(|column| column.to_string()))
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

/// The **row** a number says, if it is one.
fn row_of(value: Number, id: &str) -> Result<u64, EngineError> {
    let row = value.0;
    if !row.is_finite() || row < 1.0 || row.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a row is counted from one: {row} is not one"),
        });
    }
    Ok(row as u64)
}

/// The **column** a name says, if it is one.
fn column_of(name: &str, id: &str) -> Result<String, EngineError> {
    let column = name.trim();
    if column.is_empty() || !column.chars().all(|character| character.is_ascii_alphabetic()) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
        });
    }
    Ok(column.to_uppercase())
}

/// **Hide** a row.
///
/// The row is still there: it is a row that is not shown.
#[derive(Command)]
#[command(id = "grid.row.hide", area = "grid", layer = "grid", icon = "eye", mutates, undoable)]
pub struct HideRow {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// Which row, counting from one.
    pub row: Number,
}

impl HideRow {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.row.hide".to_string();
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
        let row = row_of(self.row, &id)?;
        let mut rows = hidden_rows(node);
        if rows.contains(&row) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the row {row} is already hidden"),
            });
        }
        rows.push(row);
        rows.sort_unstable();
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["hidden_rows"] = json!(rows);
        }
        Ok(Outcome::changed(format!("the row {row} is hidden")).with_data(json!({
            "sheet": sheet,
            "row": row,
        })))
    }
}

/// **Show** a row that was hidden.
#[derive(Command)]
#[command(id = "grid.row.show", area = "grid", layer = "grid", icon = "eye", mutates, undoable)]
pub struct ShowRow {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// Which row, counting from one.
    pub row: Number,
}

impl ShowRow {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.row.show".to_string();
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
        let row = row_of(self.row, &id)?;
        let mut rows = hidden_rows(node);
        if !rows.contains(&row) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the row {row} is not hidden"),
            });
        }
        rows.retain(|other| *other != row);
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["hidden_rows"] = json!(rows);
        }
        Ok(Outcome::changed(format!("the row {row} is shown")).with_data(json!({
            "sheet": sheet,
            "row": row,
        })))
    }
}

/// **Hide** a column.
#[derive(Command)]
#[command(id = "grid.column.hide", area = "grid", layer = "grid", icon = "eye", mutates, undoable)]
pub struct HideColumn {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// Which column: `A`, `B`, `AA`.
    pub column: Text,
}

impl HideColumn {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.column.hide".to_string();
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
        let column = column_of(&self.column.0, &id)?;
        let mut columns = hidden_columns(node);
        if columns.contains(&column) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the column {column} is already hidden"),
            });
        }
        columns.push(column.clone());
        columns.sort();
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["hidden_columns"] = json!(columns);
        }
        Ok(Outcome::changed(format!("the column {column} is hidden")).with_data(json!({
            "sheet": sheet,
            "column": column,
        })))
    }
}

/// **Show** a column that was hidden.
#[derive(Command)]
#[command(id = "grid.column.show", area = "grid", layer = "grid", icon = "eye", mutates, undoable)]
pub struct ShowColumn {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// Which column: `A`, `B`, `AA`.
    pub column: Text,
}

impl ShowColumn {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.column.show".to_string();
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
        };
        let column = column_of(&self.column.0, &id)?;
        let mut columns = hidden_columns(node);
        if !columns.contains(&column) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the column {column} is not hidden"),
            });
        }
        columns.retain(|other| other != &column);
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["hidden_columns"] = json!(columns);
        }
        Ok(Outcome::changed(format!("the column {column} is shown")).with_data(json!({
            "sheet": sheet,
            "column": column,
        })))
    }
}
