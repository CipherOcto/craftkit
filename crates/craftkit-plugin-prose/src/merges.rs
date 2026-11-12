//! The **merged cells** of a table.
//!
//! A table may have cells that are **one cell**: a cell that reaches
//! across the cells beside it or below it. The merge is a fact of the
//! table — it says which cells are one — and the **value** of the
//! merged cell is the value of the cell at its **top left**.
//!
//! The rules:
//!
//! - a merge of **one cell** is refused: one cell is not a merge;
//! - a merge that reaches **outside the table** is refused;
//! - a merge that **overlaps** another merge is refused: two cells
//!   that are one cell are not one cell;
//! - the cells the merge covers are **emptied** and the top left one
//!   keeps its value — the tree's rule, and the same here: a value
//!   that is covered is a value that is gone;
//! - a **row** that is the first row of a merge may not be taken out,
//!   and a **column** that is the first column of a merge may not be
//!   either: that cell holds the value of the merge, and the value
//!   would go with it;
//! - a row or a column that is **inside** a merge makes the merge
//!   smaller, and one that is **beside** it moves it.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

/// A **merge**: the cell at the top left and how far it reaches.
#[derive(Debug, Clone, PartialEq)]
pub struct Merge {
    /// The row the merge begins at, counting from nothing.
    pub row: usize,
    /// The column the merge begins at, counting from nothing.
    pub column: usize,
    /// How many rows it reaches down.
    pub rows: usize,
    /// How many columns it reaches across.
    pub columns: usize,
}

impl Merge {
    /// The merge, as the table keeps it.
    fn to_json(&self) -> Json {
        json!({
            "row": self.row,
            "column": self.column,
            "rows": self.rows,
            "columns": self.columns,
        })
    }

    /// The merge a piece of the table holds.
    fn from_json(value: &Json) -> Option<Merge> {
        let row = value.get("row")?.as_u64()?;
        let column = value.get("column")?.as_u64()?;
        let rows = value.get("rows")?.as_u64()?;
        let columns = value.get("columns")?.as_u64()?;
        Some(Merge {
            row: row as usize,
            column: column as usize,
            rows: rows as usize,
            columns: columns as usize,
        })
    }

    /// Whether the merge **covers** a cell.
    fn covers(&self, row: usize, column: usize) -> bool {
        row >= self.row
            && row < self.row + self.rows
            && column >= self.column
            && column < self.column + self.columns
    }

    /// Whether the merge **overlaps** another one.
    fn overlaps(&self, other: &Merge) -> bool {
        self.row < other.row + other.rows
            && other.row < self.row + self.rows
            && self.column < other.column + other.columns
            && other.column < self.column + self.columns
    }
}

/// The **merges** of a table, as it keeps them.
pub fn merges_of(node: &Node) -> Vec<Merge> {
    node.data
        .get("merges")
        .and_then(|merges| merges.as_array())
        .map(|merges| merges.iter().filter_map(Merge::from_json).collect())
        .unwrap_or_default()
}

/// The **merges** of a table, as the table keeps them.
pub fn merges_into(merges: &[Merge]) -> Json {
    Json::Array(merges.iter().map(|merge| merge.to_json()).collect())
}

/// The **merge** that covers a cell, if there is one.
pub fn merge_covering(merges: &[Merge], row: usize, column: usize) -> Option<usize> {
    merges.iter().position(|merge| merge.covers(row, column))
}

/// The merges after a **row** goes in at a place.
pub fn merges_after_row_added(merges: &[Merge], at: usize) -> Vec<Merge> {
    merges
        .iter()
        .map(|merge| {
            let mut merge = merge.clone();
            if merge.row >= at {
                merge.row += 1;
            } else if merge.row < at && at < merge.row + merge.rows {
                // The row goes **inside** the merge, and the merge
                // reaches over it.
                merge.rows += 1;
            }
            merge
        })
        .collect()
}

/// The merges after a **row** goes out at a place.
pub fn merges_after_row_removed(merges: &[Merge], row: usize) -> Vec<Merge> {
    let mut out: Vec<Merge> = Vec::new();
    for merge in merges {
        let mut merge = merge.clone();
        if merge.row > row {
            merge.row -= 1;
        } else if merge.row <= row && row < merge.row + merge.rows {
            merge.rows -= 1;
        }
        if merge.rows > 0 && merge.columns > 0 {
            out.push(merge);
        }
    }
    out
}

/// The merges after a **column** goes in at a place.
pub fn merges_after_column_added(merges: &[Merge], at: usize) -> Vec<Merge> {
    merges
        .iter()
        .map(|merge| {
            let mut merge = merge.clone();
            if merge.column >= at {
                merge.column += 1;
            } else if merge.column < at && at < merge.column + merge.columns {
                merge.columns += 1;
            }
            merge
        })
        .collect()
}

/// The merges after a **column** goes out at a place.
pub fn merges_after_column_removed(merges: &[Merge], column: usize) -> Vec<Merge> {
    let mut out: Vec<Merge> = Vec::new();
    for merge in merges {
        let mut merge = merge.clone();
        if merge.column > column {
            merge.column -= 1;
        } else if merge.column <= column && column < merge.column + merge.columns {
            merge.columns -= 1;
        }
        if merge.rows > 0 && merge.columns > 0 {
            out.push(merge);
        }
    }
    out
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

/// **Merge** cells of a table into one.
///
/// A change: the cells the merge covers are emptied, and the cell at
/// the top left keeps its value.
#[derive(Command)]
#[command(id = "prose.table.merge", area = "prose", layer = "prose", icon = "link", mutates, undoable)]
pub struct MergeCells {
    /// The table.
    pub table: NodeRef,
    /// The row the merge begins at, counting from nothing.
    pub row: Number,
    /// The column the merge begins at, counting from nothing.
    pub column: Number,
    /// How many rows it reaches down. It is one unless the file says
    /// another.
    pub rows: Option<Number>,
    /// How many columns it reaches across. It is one unless the file
    /// says another.
    pub columns: Option<Number>,
}

impl MergeCells {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.merge".to_string();
        let table = self.table.0;
        let row = index_of(&id, self.row, "row")?;
        let column = index_of(&id, self.column, "column")?;
        let rows = match &self.rows {
            None => 1usize,
            Some(rows) => index_of(&id, *rows, "number of rows")?,
        };
        let columns = match &self.columns {
            None => 1usize,
            Some(columns) => index_of(&id, *columns, "number of columns")?,
        };
        if rows == 0 || columns == 0 {
            return Err(EngineError::Failed {
                id,
                message: "a merge reaches over at least one row and one column".to_string(),
            });
        }
        if rows == 1 && columns == 1 {
            return Err(EngineError::Failed { id, message: "one cell is not a merge".to_string() });
        }
        let node = the_table(session, &id, table)?;
        let grid = crate::tables::rows_of(node, &id)?;
        let height = grid.len();
        let width = grid.first().map(|cells| cells.len()).unwrap_or(0);
        if row + rows > height || column + columns > width {
            return Err(EngineError::Failed {
                id,
                message: format!(
                    "the merge reaches outside the table: the table is {height} row(s) and {width} column(s) wide"
                ),
            });
        }
        let merges = merges_of(node);
        let merge = Merge { row, column, rows, columns };
        if merges.iter().any(|other| other.overlaps(&merge)) {
            return Err(EngineError::Failed {
                id,
                message: "the merge reaches over a cell that is already merged".to_string(),
            });
        }
        // The **cells** the merge covers are emptied, and the top
        // left one keeps its value.
        let mut grid = grid;
        for r in row..row + rows {
            for c in column..column + columns {
                if r == row && c == column {
                    continue;
                }
                if let Some(cells) = grid.get_mut(r)
                    && let Some(cell) = cells.get_mut(c)
                {
                    *cell = Json::Null;
                }
            }
        }
        let mut merges = merges;
        merges.push(merge);
        crate::tables::write_rows(session, &id, table, &grid)?;
        let Some(node) = session.document_mut()?.node_mut(table) else {
            return Err(EngineError::Failed { id, message: format!("there is no table `{table}`") });
        };
        node.data["merges"] = merges_into(&merges);
        Ok(Outcome::changed(format!("merged the cells of the table `{table}`")).with_data(json!({
            "table": table,
            "row": row,
            "column": column,
            "rows": rows,
            "columns": columns,
            "merges": merges.len(),
        })))
    }
}

/// **Split** a merged cell of a table.
///
/// A change: the merge is taken away. The cells it covered stay
/// empty — the values they held were taken when the merge was made.
#[derive(Command)]
#[command(id = "prose.table.split", area = "prose", layer = "prose", icon = "minus", mutates, undoable)]
pub struct SplitCell {
    /// The table.
    pub table: NodeRef,
    /// The row the cell is in, counting from nothing.
    pub row: Number,
    /// The column the cell is in, counting from nothing.
    pub column: Number,
}

impl SplitCell {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.split".to_string();
        let table = self.table.0;
        let row = index_of(&id, self.row, "row")?;
        let column = index_of(&id, self.column, "column")?;
        let node = the_table(session, &id, table)?;
        let mut merges = merges_of(node);
        let Some(at) = merge_covering(&merges, row, column) else {
            return Err(EngineError::Failed {
                id,
                message: format!("the cell {row},{column} of the table `{table}` is not merged"),
            });
        };
        let merge = merges.remove(at);
        let Some(node) = session.document_mut()?.node_mut(table) else {
            return Err(EngineError::Failed { id, message: format!("there is no table `{table}`") });
        };
        node.data["merges"] = merges_into(&merges);
        Ok(Outcome::changed(format!("split the merged cell of the table `{table}`")).with_data(json!({
            "table": table,
            "row": merge.row,
            "column": merge.column,
            "rows": merge.rows,
            "columns": merge.columns,
            "merges": merges.len(),
        })))
    }
}

/// **Read** the **merged cells** of a table.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "prose.table.merges", area = "prose", layer = "prose", icon = "list")]
pub struct ReadMerges {
    /// The table.
    pub table: NodeRef,
}

impl ReadMerges {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.table.merges".to_string();
        let table = self.table.0;
        let node = the_table(session, &id, table)?;
        let merges = merges_of(node);
        let count = merges.len();
        Ok(Outcome::new(format!("the table `{table}` has {count} merged cell(s)")).with_data(json!({
            "table": table,
            "merges": merges.iter().map(|merge| merge.to_json()).collect::<Vec<Json>>(),
            "count": count,
        })))
    }
}
