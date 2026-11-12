//! The **table** of a drawing: a grid of cells, and the tree's own
//! rules for the merging of them.
//!
//! # The tree's rules
//!
//! - a table is a **grid** of cells: rows and columns, and a cell may
//!   be **merged** with the cells beside it;
//! - a **merged** cell is written in its **anchor**: the cell that
//!   holds the merge; the cells it covers have no text of their own;
//! - **writing into a covered cell writes its anchor**;
//! - a merge of **one cell** is refused: a merge is at least two cells;
//! - the **texts** of the cells that are merged are **joined** into the
//!   anchor, with a space between them;
//! - a merge that **overlaps** another one **absorbs** it;
//! - a table keeps **at least one row**;
//! - a row is **inserted before** the row that is named, or at the end
//!   when none is named, and the merges **move with the rows**;
//! - a cell of a table that is written with a **formula** keeps the
//!   formula and the value; here a cell holds its text and nothing
//!   else.
//!
//! # What the port found
//!
//! - **The tree clamps the size of a table and this tree refuses it.**
//!   The tree's table takes at most two thousand rows and two hundred
//!   columns, and it says so by making the table smaller than it was
//!   asked for. Here a table that is asked for too much is **refused**,
//!   because a table that is not the one that was asked for is a lie.
//! - **The tree clamps the range that is merged and this tree refuses
//!   it**: the tree merges what fits of the range that is named; here a
//!   merge that goes past the edge of the table is **refused**.
//! - **The tree's table has a title, a header row and a style of its
//!   own**, and it may be broken into parts and linked to a file; here
//!   the table is a grid of cells and nothing else.
//! - **The tree's table is an entity of a closed kind** and here it is
//!   an entity of an **open** kind, like every other entity of this
//!   drawing.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The most **rows** a table may have: the tree's own bound.
const MOST_ROWS: usize = 2000;

/// The most **columns** a table may have: the tree's own bound.
const MOST_COLUMNS: usize = 200;

/// The **width** of a column when none is said: the tree's own.
const COLUMN_WIDTH: f64 = 2.5;

/// The **height** of a row when none is said: the tree's own.
const ROW_HEIGHT: f64 = 1.3;

/// A **cell** of a table: its text and the merge it holds.
#[derive(Clone, Debug, PartialEq)]
struct Cell {
    /// The text of the cell.
    text: String,
    /// The **merge** the cell holds: how many rows and columns it
    /// covers. It is the anchor when it is there.
    merged: Option<(usize, usize)>,
}

impl Cell {
    /// An empty cell.
    fn empty() -> Cell {
        Cell { text: String::new(), merged: None }
    }

    /// The cell as the **payload** holds it.
    fn to_json(&self) -> Json {
        json!({
            "text": self.text,
            "merged": match self.merged {
                Some((rows, columns)) => json!([rows, columns]),
                None => Json::Null,
            },
        })
    }

    /// The cell as the **payload** says it.
    fn from_json(value: &Json) -> Cell {
        Cell {
            text: value.get("text").and_then(|text| text.as_str()).unwrap_or_default().to_string(),
            merged: value.get("merged").and_then(|span| span.as_array()).and_then(|span| {
                let rows = span.first().and_then(|rows| rows.as_u64())?;
                let columns = span.get(1).and_then(|columns| columns.as_u64())?;
                Some((rows as usize, columns as usize))
            }),
        }
    }
}

/// The **cells** of a table, as the payload holds them.
fn cells_of(geometry: &Json) -> Vec<Vec<Cell>> {
    geometry
        .get("cells")
        .and_then(|cells| cells.as_array())
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_array()
                        .map(|cells| cells.iter().map(Cell::from_json).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **cells** as the payload holds them.
fn cells_into(cells: &[Vec<Cell>]) -> Json {
    Json::Array(cells.iter().map(|row| Json::Array(row.iter().map(Cell::to_json).collect())).collect())
}

/// The **merges** of a table: the anchor of each, and what it covers.
fn anchors(cells: &[Vec<Cell>]) -> Vec<(usize, usize, usize, usize)> {
    let mut out: Vec<(usize, usize, usize, usize)> = Vec::new();
    for (row, line) in cells.iter().enumerate() {
        for (column, cell) in line.iter().enumerate() {
            if let Some((rows, columns)) = cell.merged {
                out.push((row, column, rows.max(1), columns.max(1)));
            }
        }
    }
    out
}

/// The **merges** of a table, written back into its cells.
fn set_anchors(cells: &mut [Vec<Cell>], anchors: &[(usize, usize, usize, usize)]) {
    for row in cells.iter_mut() {
        for cell in row.iter_mut() {
            cell.merged = None;
        }
    }
    let rows = cells.len();
    let columns = cells.first().map(|row| row.len()).unwrap_or(0);
    for &(row, column, span_rows, span_columns) in anchors {
        let span_rows = span_rows.min(rows.saturating_sub(row));
        let span_columns = span_columns.min(columns.saturating_sub(column));
        if (span_rows > 1 || span_columns > 1)
            && let Some(cell) = cells.get_mut(row).and_then(|line| line.get_mut(column))
        {
            cell.merged = Some((span_rows, span_columns));
        }
    }
}

/// A **count**: a number of rows or columns, and it is a whole one.
fn count(value: Number, what: &str, command: &str) -> Result<usize, EngineError> {
    let number = value.0;
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("{what} is a whole number and not a negative one"),
        });
    }
    Ok(number as usize)
}

/// The **table** a node is, if it is one.
fn the_table<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no table `{node}`"),
        });
    };
    if found.kind != "cad.entity.table" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not a table", found.kind),
        });
    }
    Ok(found)
}

/// The **geometry** of a table: its shape and its cells.
fn geometry_of(node: &craftkit_sdk::Node) -> Json {
    node.data.get("geometry").cloned().unwrap_or(Json::Null)
}

/// Put the **geometry** of a table back.
fn put_geometry(session: &mut Session, table: u64, geometry: Json, command: &str) -> Result<(), EngineError> {
    let Some(node) = session.document_mut()?.node_mut(table) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no table `{table}`"),
        });
    };
    node.data["geometry"] = geometry;
    Ok(())
}

/// The **cell** a command names: the row and the column.
fn cell_at(
    geometry: &Json,
    cells: &[Vec<Cell>],
    row: usize,
    column: usize,
    command: &str,
) -> Result<(usize, usize), EngineError> {
    let rows = cells.len();
    let columns = geometry.get("columns").and_then(|columns| columns.as_u64()).unwrap_or(0) as usize;
    if row >= rows || column >= columns {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the cell ({row}, {column}) is outside the {rows}×{columns} table"),
        });
    }
    Ok((row, column))
}

commands! {
    /// **Add** a **table** to a block.
    ///
    /// The tree's own table: a grid of cells, at a point, with the tree's
    /// own default column width and row height.
    cad.table.add {
        area: "cad",
        layer: "cad",
        icon: "table",
        mutates,
        undoable,
        params: {
            /// The block the table stands in.

            block: NodeRef,
            /// What to call it.

            name: Text,
            /// Where its first corner is.

            at: Json,
            /// How many rows it has.

            rows: Number,
            /// How many columns it has.

            columns: Number,
        },
        run: {

            let id = "cad.table.add".to_string();
            let rows = count(self.rows, "the rows of a table", &id)?;
            let columns = count(self.columns, "the columns of a table", &id)?;
            if rows == 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a table has at least one row".to_string(),
                });
            }
            if columns == 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a table has at least one column".to_string(),
                });
            }
            // The tree **clamps** the size of a table and this tree
            // **refuses** it.
            if rows > MOST_ROWS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a table has at most {MOST_ROWS} rows"),
                });
            }
            if columns > MOST_COLUMNS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a table has at most {MOST_COLUMNS} columns"),
                });
            }
            let (x, y) = crate::point(&self.at, &id, "where the table is")?;
            let cells: Vec<Vec<Cell>> =
                (0..rows).map(|_| (0..columns).map(|_| Cell::empty()).collect()).collect();
            let geometry = json!({
                "at": { "x": x, "y": y },
                "rows": rows,
                "columns": columns,
                "col_widths": vec![COLUMN_WIDTH; columns],
                "row_heights": vec![ROW_HEIGHT; rows],
                "cells": cells_into(&cells),
            });
            let table =
                crate::add_entity(session, self.block.0, &self.name.0, "cad.entity.table", geometry, &id)?;
            Ok(Outcome::changed(format!("added the table `{}`", self.name.0.trim()))
                .with_data(json!({ "table": table, "rows": rows, "columns": columns })))

        }
    }
    /// **Write** the **text** of a cell.
    ///
    /// The tree's own rule: writing into a covered cell writes its anchor.
    cad.table.set {
        area: "cad",
        layer: "cad",
        icon: "text",
        mutates,
        undoable,
        params: {
            /// The table the cell is in.

            table: NodeRef,
            /// The row of the cell.

            row: Number,
            /// The column of the cell.

            column: Number,
            /// The text of the cell.

            text: Text,
        },
        run: {

            let id = "cad.table.set".to_string();
            let node = the_table(session, &id, self.table.0)?;
            let mut geometry = geometry_of(node);
            let mut cells = cells_of(&geometry);
            let row = count(self.row, "the row of a cell", &id)?;
            let column = count(self.column, "the column of a cell", &id)?;
            let (row, column) = cell_at(&geometry, &cells, row, column, &id)?;
            // Writing into a covered cell writes its **anchor**.
            let (row, column) = anchors(&cells)
                .into_iter()
                .find(|(anchor_row, anchor_column, rows, columns)| {
                    row >= *anchor_row
                        && row < *anchor_row + *rows
                        && column >= *anchor_column
                        && column < *anchor_column + *columns
                })
                .map(|(anchor_row, anchor_column, _, _)| (anchor_row, anchor_column))
                .unwrap_or((row, column));
            if let Some(cell) = cells.get_mut(row).and_then(|line| line.get_mut(column)) {
                cell.text = self.text.0.clone();
            }
            geometry["cells"] = cells_into(&cells);
            put_geometry(session, self.table.0, geometry, &id)?;
            Ok(Outcome::changed(format!("the cell ({row}, {column}) says `{}`", self.text.0))
                .with_data(json!({ "table": self.table.0, "row": row, "column": column, "text": self.text.0 })))

        }
    }
    /// **Insert** a **row** into a table.
    ///
    /// The tree's own rule: the row goes **before** the row that is named,
    /// or at the end when none is named, and the merges move with the rows.
    cad.table.insert.row {
        area: "cad",
        layer: "cad",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The table the row goes in.

            table: NodeRef,
            /// The row it goes before. It goes at the end when it is not said.

            row: Option<Number>,
        },
        run: {

            let id = "cad.table.insert.row".to_string();
            let node = the_table(session, &id, self.table.0)?;
            let mut geometry = geometry_of(node);
            let mut cells = cells_of(&geometry);
            if cells.len() >= MOST_ROWS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a table has at most {MOST_ROWS} rows"),
                });
            }
            let row = match self.row {
                Some(row) => count(row, "the row a row goes before", &id)?.min(cells.len()),
                None => cells.len(),
            };
            // The **merges** move with the rows.
            let moved: Vec<(usize, usize, usize, usize)> = anchors(&cells)
                .into_iter()
                .map(|(anchor_row, anchor_column, rows, columns)| {
                    if anchor_row >= row {
                        (anchor_row + 1, anchor_column, rows, columns)
                    } else if row < anchor_row + rows {
                        (anchor_row, anchor_column, rows + 1, columns)
                    } else {
                        (anchor_row, anchor_column, rows, columns)
                    }
                })
                .collect();
            let columns = cells.first().map(|line| line.len()).unwrap_or(0);
            cells.insert(row, vec![Cell::empty(); columns]);
            set_anchors(&mut cells, &moved);
            geometry["cells"] = cells_into(&cells);
            if let Some(heights) = geometry.get_mut("row_heights").and_then(|heights| heights.as_array_mut()) {
                heights.insert(row, json!(ROW_HEIGHT));
            }
            geometry["rows"] = json!(cells.len());
            put_geometry(session, self.table.0, geometry, &id)?;
            Ok(Outcome::changed(format!("the table has a row at {row}"))
                .with_data(json!({ "table": self.table.0, "row": row, "rows": cells.len() })))

        }
    }
    /// **Delete** a **row** of a table.
    ///
    /// The tree's own rule: a table keeps at least one row.
    cad.table.delete.row {
        area: "cad",
        layer: "cad",
        icon: "trash",
        mutates,
        undoable,
        params: {
            /// The table the row is in.

            table: NodeRef,
            /// The row that goes.

            row: Number,
        },
        run: {

            let id = "cad.table.delete.row".to_string();
            let node = the_table(session, &id, self.table.0)?;
            let mut geometry = geometry_of(node);
            let mut cells = cells_of(&geometry);
            let row = count(self.row, "the row that goes", &id)?;
            if row >= cells.len() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the table has no row {row}"),
                });
            }
            if cells.len() <= 1 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a table keeps at least one row".to_string(),
                });
            }
            // The **merges** move with the rows, and the merge of the row
            // that goes moves to the row below it.
            let mut moved: Vec<(usize, usize, usize, usize)> = Vec::new();
            for (anchor_row, anchor_column, rows, columns) in anchors(&cells) {
                if anchor_row == row {
                    if rows > 1 {
                        let text = cells
                            .get(anchor_row)
                            .and_then(|line| line.get(anchor_column))
                            .map(|cell| cell.text.clone())
                            .unwrap_or_default();
                        if let Some(cell) =
                            cells.get_mut(anchor_row + 1).and_then(|line| line.get_mut(anchor_column))
                        {
                            cell.text = text;
                        }
                        moved.push((anchor_row, anchor_column, rows - 1, columns));
                    }
                } else if anchor_row > row {
                    moved.push((anchor_row - 1, anchor_column, rows, columns));
                } else if row < anchor_row + rows {
                    moved.push((anchor_row, anchor_column, rows - 1, columns));
                } else {
                    moved.push((anchor_row, anchor_column, rows, columns));
                }
            }
            cells.remove(row);
            set_anchors(&mut cells, &moved);
            geometry["cells"] = cells_into(&cells);
            if let Some(heights) = geometry.get_mut("row_heights").and_then(|heights| heights.as_array_mut())
                && row < heights.len()
            {
                heights.remove(row);
            }
            geometry["rows"] = json!(cells.len());
            put_geometry(session, self.table.0, geometry, &id)?;
            Ok(Outcome::changed(format!("the table has no row {row}"))
                .with_data(json!({ "table": self.table.0, "rows": cells.len() })))

        }
    }
    /// **Merge** the cells of a table into one.
    ///
    /// The tree's own rules: a merge is **at least two cells**, the texts
    /// of the cells are **joined** into the anchor, and a merge that
    /// **overlaps** another one **absorbs** it.
    cad.table.merge {
        area: "cad",
        layer: "cad",
        icon: "merge",
        mutates,
        undoable,
        params: {
            /// The table the cells are in.

            table: NodeRef,
            /// The row of the first cell.

            row: Number,
            /// The column of the first cell.

            column: Number,
            /// How many rows the merge covers.

            rows: Number,
            /// How many columns the merge covers.

            columns: Number,
        },
        run: {

            let id = "cad.table.merge".to_string();
            let node = the_table(session, &id, self.table.0)?;
            let mut geometry = geometry_of(node);
            let mut cells = cells_of(&geometry);
            let row = count(self.row, "the row of the first cell", &id)?;
            let column = count(self.column, "the column of the first cell", &id)?;
            let (row, column) = cell_at(&geometry, &cells, row, column, &id)?;
            let rows = count(self.rows, "the rows of a merge", &id)?.max(1);
            let columns = count(self.columns, "the columns of a merge", &id)?.max(1);
            if rows == 1 && columns == 1 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "merge at least two cells".to_string(),
                });
            }
            // The tree **clamps** the range and this tree **refuses** it.
            if row + rows > cells.len() || column + columns > cells.first().map(|line| line.len()).unwrap_or(0) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the merge of {rows}×{columns} at ({row}, {column}) goes past the edge of the table"
                    ),
                });
            }
            // The merges that **overlap** the range are absorbed.
            let overlaps =
                |(anchor_row, anchor_column, anchor_rows, anchor_columns): &(usize, usize, usize, usize)| {
                    *anchor_row < row + rows
                        && row < *anchor_row + *anchor_rows
                        && *anchor_column < column + columns
                        && column < *anchor_column + *anchor_columns
                };
            let mut kept: Vec<(usize, usize, usize, usize)> =
                anchors(&cells).into_iter().filter(|anchor| !overlaps(anchor)).collect();
            // The **texts** of the cells that are merged are joined into the
            // anchor.
            let mut texts: Vec<String> = Vec::new();
            for line in row..row + rows {
                for index in column..column + columns {
                    if let Some(cell) = cells.get_mut(line).and_then(|cells| cells.get_mut(index))
                        && !cell.text.is_empty()
                    {
                        texts.push(std::mem::take(&mut cell.text));
                    }
                }
            }
            if let Some(cell) = cells.get_mut(row).and_then(|line| line.get_mut(column)) {
                cell.text = texts.join(" ");
            }
            kept.push((row, column, rows, columns));
            set_anchors(&mut cells, &kept);
            geometry["cells"] = cells_into(&cells);
            put_geometry(session, self.table.0, geometry, &id)?;
            Ok(Outcome::changed(format!("the cells at ({row}, {column}) are merged")).with_data(
                json!({ "table": self.table.0, "row": row, "column": column, "rows": rows, "columns": columns }),
            ))

        }
    }
    /// **Unmerge** the cells of a table.
    ///
    /// The tree's own rule: the merges that cover the cell are taken away.
    cad.table.unmerge {
        area: "cad",
        layer: "cad",
        icon: "unmerge",
        mutates,
        undoable,
        params: {
            /// The table the cells are in.

            table: NodeRef,
            /// The row of the cell.

            row: Number,
            /// The column of the cell.

            column: Number,
        },
        run: {

            let id = "cad.table.unmerge".to_string();
            let node = the_table(session, &id, self.table.0)?;
            let mut geometry = geometry_of(node);
            let mut cells = cells_of(&geometry);
            let row = count(self.row, "the row of the cell", &id)?;
            let column = count(self.column, "the column of the cell", &id)?;
            let (row, column) = cell_at(&geometry, &cells, row, column, &id)?;
            let kept: Vec<(usize, usize, usize, usize)> = anchors(&cells)
                .into_iter()
                .filter(|(anchor_row, anchor_column, rows, columns)| {
                    !(row >= *anchor_row
                        && row < *anchor_row + *rows
                        && column >= *anchor_column
                        && column < *anchor_column + *columns)
                })
                .collect();
            set_anchors(&mut cells, &kept);
            geometry["cells"] = cells_into(&cells);
            put_geometry(session, self.table.0, geometry, &id)?;
            Ok(Outcome::changed(format!("the cells at ({row}, {column}) are not merged any more"))
                .with_data(json!({ "table": self.table.0, "row": row, "column": column })))

        }
    }
}
