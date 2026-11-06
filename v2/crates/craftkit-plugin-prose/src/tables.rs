//! The **cells** of a table.
//!
//! A table is a **grid**: rows of cells, and every row as long as
//! every other row. A cell holds a **value** — a number, a text, a
//! nothing — and not a tree: a cell that holds blocks is a different
//! thing, and this battery does not pretend to have one.
//!
//! The grid may be **read** a cell at a time, **written** a cell at a
//! time, and it may be **changed** by a row or a column going in or
//! coming out.
//!
//! # What the port found
//!
//! - **The tree's table is a tree**: every cell holds paragraphs, and
//!   a cell has an identity of its own that a row or a column insert
//!   moves about. Here a cell is a **value in a row**, and a cell is
//!   found by its **row and its column** — there is no cell identity
//!   to keep, and a row that goes in pushes the rows below it down.
//! - **The tree's table is a grid and says so**: a row of one length
//!   beside a row of another is not a table. Here the same, and a
//!   table whose rows are not the same length is **refused** rather
//!   than tidied up: a table that is nearly a grid is a table whose
//!   cells cannot be addressed.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// Whether the **first row** of a table is a **header row**.
///
/// A header row says what the columns are, and it is the row a
/// screen reader reads to know the names of the columns. A table
/// that does not say is a table that does not say.
pub fn has_a_header(node: &Node) -> bool {
    node.data.get("header").and_then(|header| header.as_bool()).unwrap_or(false)
}

/// The **alt text** of a table: what it says about itself.
///
/// A table that says nothing has no alt text, and the answer is
/// nothing and not an empty text that means the same thing.
pub fn alt_of(table: &Node) -> Option<String> {
    table
        .data
        .get("alt")
        .and_then(|alt| alt.as_str())
        .map(|alt| alt.to_string())
        .filter(|alt| !alt.trim().is_empty())
}

/// **Say** what a table is.
#[derive(Command)]
#[command(id = "prose.table.alt", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetTableAlt {
    /// The table.
    pub table: NodeRef,
    /// The text that stands for it.
    ///
    /// An empty text takes the alt text away.
    pub alt: Text,
}

impl SetTableAlt {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.alt".to_string();
        let table = self.table.0;
        the_table(session, &id, table)?;
        let alt = self.alt.0.trim().to_string();
        let node = session.document_mut()?.node_mut(table);
        let Some(node) = node else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{table}`"),
            });
        };
        if alt.is_empty() {
            // An **empty** value takes the fact away.
            if let Some(map) = node.data.as_object_mut() {
                map.remove("alt");
            }
        } else {
            node.data["alt"] = json!(alt.clone());
        }
        Ok(Outcome::changed(if alt.is_empty() {
            format!("the table `{table}` says nothing about itself")
        } else {
            format!("the table `{table}` says what it is")
        })
        .with_data(json!({ "table": table, "alt": alt })))
    }
}

/// The **rows** of a table: how many there are.
pub fn row_count(node: &Node) -> usize {
    node.data.get("rows").and_then(|rows| rows.as_array()).map(|rows| rows.len()).unwrap_or(0)
}

/// **Say** whether the first row of a table is a header row.
#[derive(Command)]
#[command(id = "prose.table.header", area = "prose", layer = "prose", icon = "list", mutates, undoable)]
pub struct SetHeader {
    /// The table.
    pub table: NodeRef,
    /// Whether the first row is a header row.
    pub header: Bool,
}

impl SetHeader {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.header".to_string();
        let table = self.table.0;
        the_table(session, &id, table)?;
        if let Some(node) = session.document_mut()?.node_mut(table) {
            node.data["header"] = json!(self.header.0);
        }
        Ok(Outcome::changed(if self.header.0 {
            format!("the first row of the table `{table}` is a header row")
        } else {
            format!("the first row of the table `{table}` is not a header row")
        })
        .with_data(json!({ "table": table, "header": self.header.0 })))
    }
}

/// The **rows** of a table: a grid, or an error.
///
/// Every row must be as long as every other row: a table that is not
/// a grid has no cell at some place, and a cell that is not there
/// cannot be read or written.
fn rows_of(node: &Node, id: &str) -> Result<Vec<Vec<Json>>, EngineError> {
    let rows = node.data.get("rows").and_then(|rows| rows.as_array()).cloned().unwrap_or_default();
    let mut grid: Vec<Vec<Json>> = Vec::with_capacity(rows.len());
    let mut width: Option<usize> = None;
    for row in rows {
        let Some(cells) = row.as_array() else {
            return Err(EngineError::Failed {
                id: id.to_string(),
                message: "a row of a table is a list of cells".to_string(),
            });
        };
        match width {
            None => width = Some(cells.len()),
            Some(width) if width != cells.len() => {
                return Err(EngineError::Failed {
                    id: id.to_string(),
                    message: "the rows of a table are the same length".to_string(),
                });
            }
            Some(_) => {}
        }
        grid.push(cells.clone());
    }
    Ok(grid)
}

/// The **table** a node is, if it is one.
fn the_table<'a>(session: &'a Session, id: &str, node: u64) -> Result<&'a Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no table `{node}`"),
        });
    };
    if found.kind != "prose.block.table" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{node}` is a `{}` and not a table", found.kind),
        });
    }
    Ok(found)
}

/// An **index** a command is about: a whole number, counting from
/// nothing.
fn index_of(id: &str, number: Number, what: &str) -> Result<usize, EngineError> {
    if !number.0.is_finite() || number.0 < 0.0 || number.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a {what} is a whole number"),
        });
    }
    Ok(number.0 as usize)
}

/// Put a table's **rows** back into a table.
fn write_rows(session: &mut Session, id: &str, table: u64, rows: &[Vec<Json>]) -> Result<(), EngineError> {
    let Some(node) = session.document_mut()?.node_mut(table) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no table `{table}`"),
        });
    };
    node.data["rows"] = Json::Array(rows.iter().map(|row| Json::Array(row.clone())).collect());
    Ok(())
}

/// **Read** a cell of a table.
#[derive(Command)]
#[command(id = "prose.table.cell.get", area = "prose", layer = "prose", icon = "list")]
pub struct GetCell {
    /// The table.
    pub table: NodeRef,
    /// The row the cell is in, counting from nothing.
    pub row: Number,
    /// The column the cell is in, counting from nothing.
    pub column: Number,
}

impl GetCell {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.cell.get".to_string();
        let table = self.table.0;
        let row = index_of(&id, self.row, "row")?;
        let column = index_of(&id, self.column, "column")?;
        let node = the_table(session, &id, table)?;
        let rows = rows_of(node, &id)?;
        let Some(cells) = rows.get(row) else {
            return Err(EngineError::Failed {
                id,
                message: format!("the table `{table}` has no row `{row}`"),
            });
        };
        let Some(cell) = cells.get(column) else {
            return Err(EngineError::Failed {
                id,
                message: format!("the row `{row}` of the table `{table}` has no column `{column}`"),
            });
        };
        Ok(Outcome::new(format!("the cell {row},{column} of the table `{table}`"))
            .with_data(json!({ "table": table, "row": row, "column": column, "value": cell.clone() })))
    }
}

/// **Set** a cell of a table.
#[derive(Command)]
#[command(id = "prose.table.cell.set", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetCell {
    /// The table.
    pub table: NodeRef,
    /// The row the cell is in, counting from nothing.
    pub row: Number,
    /// The column the cell is in, counting from nothing.
    pub column: Number,
    /// What the cell holds.
    pub value: Json,
}

impl SetCell {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.cell.set".to_string();
        let table = self.table.0;
        let row = index_of(&id, self.row, "row")?;
        let column = index_of(&id, self.column, "column")?;
        let node = the_table(session, &id, table)?;
        let mut rows = rows_of(node, &id)?;
        let Some(cells) = rows.get_mut(row) else {
            return Err(EngineError::Failed {
                id,
                message: format!("the table `{table}` has no row `{row}`"),
            });
        };
        if column >= cells.len() {
            return Err(EngineError::Failed {
                id,
                message: format!("the row `{row}` of the table `{table}` has no column `{column}`"),
            });
        }
        cells[column] = self.value.clone();
        write_rows(session, &id, table, &rows)?;
        Ok(Outcome::changed(format!("set the cell {row},{column} of the table `{table}`"))
            .with_data(json!({ "table": table, "row": row, "column": column })))
    }
}

/// Put a **row** in a table.
#[derive(Command)]
#[command(id = "prose.table.row.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddRow {
    /// The table.
    pub table: NodeRef,
    /// Where the row goes: the rows below it move down. The end of the
    /// table when it is not said.
    pub at: Option<Number>,
}

impl AddRow {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.row.add".to_string();
        let table = self.table.0;
        let node = the_table(session, &id, table)?;
        let mut rows = rows_of(node, &id)?;
        let width = rows.first().map(|row| row.len()).unwrap_or(0);
        let at = match &self.at {
            Some(at) => index_of(&id, *at, "row")?,
            None => rows.len(),
        };
        if at > rows.len() {
            return Err(EngineError::Failed {
                id,
                message: format!("the table `{table}` has no row `{at}`"),
            });
        }
        let empty: Vec<Json> = (0..width).map(|_| Json::Null).collect();
        rows.insert(at, empty);
        write_rows(session, &id, table, &rows)?;
        Ok(Outcome::changed(format!("put a row in the table `{table}`"))
            .with_data(json!({ "table": table, "at": at, "rows": rows.len() })))
    }
}

/// Take a **row** out of a table.
#[derive(Command)]
#[command(id = "prose.table.row.remove", area = "prose", layer = "prose", icon = "minus", mutates, undoable)]
pub struct RemoveRow {
    /// The table.
    pub table: NodeRef,
    /// The row that goes.
    pub row: Number,
}

impl RemoveRow {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.row.remove".to_string();
        let table = self.table.0;
        let row = index_of(&id, self.row, "row")?;
        let node = the_table(session, &id, table)?;
        let mut rows = rows_of(node, &id)?;
        if row >= rows.len() {
            return Err(EngineError::Failed {
                id,
                message: format!("the table `{table}` has no row `{row}`"),
            });
        }
        rows.remove(row);
        write_rows(session, &id, table, &rows)?;
        Ok(Outcome::changed(format!("took a row out of the table `{table}`"))
            .with_data(json!({ "table": table, "row": row, "rows": rows.len() })))
    }
}

/// Put a **column** in a table.
#[derive(Command)]
#[command(id = "prose.table.column.add", area = "prose", layer = "prose", icon = "plus", mutates, undoable)]
pub struct AddColumn {
    /// The table.
    pub table: NodeRef,
    /// Where the column goes: the columns beside it move right. The
    /// end of the table when it is not said.
    pub at: Option<Number>,
}

impl AddColumn {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.column.add".to_string();
        let table = self.table.0;
        let node = the_table(session, &id, table)?;
        let mut rows = rows_of(node, &id)?;
        let width = rows.first().map(|row| row.len()).unwrap_or(0);
        let at = match &self.at {
            Some(at) => index_of(&id, *at, "column")?,
            None => width,
        };
        if at > width {
            return Err(EngineError::Failed {
                id,
                message: format!("the table `{table}` has no column `{at}`"),
            });
        }
        for row in rows.iter_mut() {
            row.insert(at, Json::Null);
        }
        write_rows(session, &id, table, &rows)?;
        Ok(Outcome::changed(format!("put a column in the table `{table}`"))
            .with_data(json!({ "table": table, "at": at, "width": width + 1 })))
    }
}

/// Take a **column** out of a table.
#[derive(Command)]
#[command(
    id = "prose.table.column.remove",
    area = "prose",
    layer = "prose",
    icon = "minus",
    mutates,
    undoable
)]
pub struct RemoveColumn {
    /// The table.
    pub table: NodeRef,
    /// The column that goes.
    pub column: Number,
}

impl RemoveColumn {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.column.remove".to_string();
        let table = self.table.0;
        let column = index_of(&id, self.column, "column")?;
        let node = the_table(session, &id, table)?;
        let mut rows = rows_of(node, &id)?;
        let width = rows.first().map(|row| row.len()).unwrap_or(0);
        if column >= width {
            return Err(EngineError::Failed {
                id,
                message: format!("the table `{table}` has no column `{column}`"),
            });
        }
        for row in rows.iter_mut() {
            row.remove(column);
        }
        write_rows(session, &id, table, &rows)?;
        Ok(Outcome::changed(format!("took a column out of the table `{table}`"))
            .with_data(json!({ "table": table, "column": column, "width": width - 1 })))
    }
}
