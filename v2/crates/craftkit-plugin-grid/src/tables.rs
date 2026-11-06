//! The **tables** of a sheet.
//!
//! A table is a **named range of cells** whose first row names the
//! **fields** of the table: `amount`, `who`, `when`. Every column of
//! the range is a field, and the header cell of that column is its
//! name.
//!
//! Here a table is a **fact of the sheet**: the tables it holds, and
//! where each of them reaches. The rules that hold it together:
//!
//! - a table's **name** is not nothing, does not look like a **cell**,
//!   and is not another table's name on the same sheet;
//! - a table's **range** covers more than one cell, because a table
//!   of one cell is a cell;
//! - the **header** of a table is a row of **text**: a field is named
//!   by text, and a cell that holds nothing, a number, a boolean, an
//!   error or a formula does not name a field — so the table is
//!   **refused** and the refusal says which cell does not name one;
//! - two tables of a sheet do not **overlap**, because a cell that is
//!   in two tables is in none of them.
//!
//! The **fields** of a table are **derived**: they are the text of the
//! header cells, and a header cell that is edited changes the field.
//! A table does not keep a copy of its own fields, because a second
//! copy is a copy that can be wrong.
//!
//! # What the port found
//!
//! - **The tree's table is an object with a name, a range, a header
//!   row and a style.** The tree's style is a banded row and a total
//!   row and a colour. Here the table is the **name, the range and
//!   the fields**, because a style is a drawing and a drawing is not
//!   a fact of the sheet.
//! - **The tree's tables may be resized and renamed and their
//!   duplicates removed.** Here they may be resized and renamed, and
//!   the tree's removal of the duplicate rows is not here: it is a
//!   change of the cells, and a change of the cells is a change of
//!   the sheet.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Cells, Value};

/// A **table** of a sheet.
#[derive(Debug, Clone)]
pub struct Table {
    /// The name of the table.
    pub name: String,
    /// The range it reaches.
    pub range: Area,
    /// Whether the table has a **total row**: the row at the bottom
    /// of it that holds the totals of the columns.
    pub totals: bool,
}

impl Table {
    /// The table, as the sheet keeps it.
    fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        json!({
            "name": self.name,
            "range": self.range.to_text(),
            "totals": self.totals,
        })
    }
}

/// The **tables** of a sheet, as they are kept.
pub fn tables_of(node: &Node) -> Vec<Table> {
    let mut tables = Vec::new();
    let Some(list) = node.data.get("tables").and_then(|tables| tables.as_array()) else {
        return tables;
    };
    for table in list {
        let Some(name) = table.get("name").and_then(|name| name.as_str()) else { continue };
        let Some(range) = table.get("range").and_then(|range| range.as_str()) else { continue };
        let Some((_, area)) = crate::merges::parse_area(range) else { continue };
        let totals = table.get("totals").and_then(|totals| totals.as_bool()).unwrap_or(false);
        tables.push(Table { name: name.to_string(), range: area, totals });
    }
    tables
}

/// The **tables** of a sheet, as the sheet keeps them.
fn tables_into(tables: &[Table]) -> craftkit_sdk::serde_json::Value {
    json!(tables.iter().map(|table| table.to_json()).collect::<Vec<_>>())
}

/// Whether a name may be **a table's name**, and if it may not, why.
fn why_not_name(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("a table's name is not nothing".to_string());
    }
    if name.contains(':') || name.contains('!') {
        return Some(format!("`{name}` looks like a range, and a table's name is not a range"));
    }
    if CellRef::parse(name).is_some() {
        return Some(format!("`{name}` looks like a cell, and a table's name is not a cell"));
    }
    None
}

/// Whether two ranges **overlap**.
fn overlap(one: &Area, other: &Area) -> bool {
    one.start.col <= other.end.col
        && one.end.col >= other.start.col
        && one.start.row <= other.end.row
        && one.end.row >= other.start.row
}

/// The **text** of a header cell, and why it is not one.
fn header_text(cells: &Cells, cell: CellRef) -> Result<String, String> {
    let Some(entry) = cells.get(cell) else {
        return Err(format!("the cell {} holds nothing, and a field is named by text", cell.to_text()));
    };
    if entry.formula.is_some() {
        return Err(format!("the cell {} holds a formula, and a field is named by text", cell.to_text()));
    }
    match &entry.value {
        Some(Value::Text(text)) if !text.trim().is_empty() => Ok(text.trim().to_string()),
        Some(Value::Text(_)) => {
            Err(format!("the cell {} holds nothing, and a field is named by text", cell.to_text()))
        }
        Some(Value::Number(_)) => {
            Err(format!("the cell {} holds a number, and a field is named by text", cell.to_text()))
        }
        Some(Value::Bool(_)) => {
            Err(format!("the cell {} holds a boolean, and a field is named by text", cell.to_text()))
        }
        Some(Value::Error(_)) => {
            Err(format!("the cell {} holds an error, and a field is named by text", cell.to_text()))
        }
        Some(Value::Empty) | None => {
            Err(format!("the cell {} holds nothing, and a field is named by text", cell.to_text()))
        }
    }
}

/// The **fields** of a table: the text of its header cells.
fn fields_of(table: &Table, cells: &Cells) -> Result<Vec<(CellRef, String)>, String> {
    let mut fields = Vec::new();
    for col in table.range.start.col..=table.range.end.col {
        let cell = CellRef { col, row: table.range.start.row };
        fields.push((cell, header_text(cells, cell)?));
    }
    Ok(fields)
}

/// The **totals** of the columns of a table: the sum of the numbers
/// of the rows under the header.
///
/// A table without a total row has no totals. A table with one takes
/// its last row for the totals, and the rows above it are the data.
/// A column with no numbers in it has **no total**: the answer is
/// nothing and not a zero that means something else.
fn totals_of(
    table: &Table,
    fields: &[(CellRef, String)],
    cells: &Cells,
) -> Vec<craftkit_sdk::serde_json::Value> {
    if !table.totals {
        return Vec::new();
    }
    let last = table.range.end.row;
    let mut totals: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
    for (cell, name) in fields {
        let mut sum = 0.0;
        let mut numbers = 0usize;
        for row in table.range.start.row + 1..last {
            let Some(entry) = cells.get(CellRef { col: cell.col, row }) else { continue };
            let Some(Value::Number(number)) = &entry.value else { continue };
            sum += *number;
            numbers += 1;
        }
        let total = if numbers == 0 { craftkit_sdk::serde_json::Value::Null } else { json!(sum) };
        totals.push(json!({
            "name": name.clone(),
            "column": CellRef::column_text(cell.col),
            "total": total,
        }));
    }
    totals
}

/// The **sheet** a command is about, and whether it is a sheet.
fn sheet_of(session: &Session, id: &str, sheet: u64) -> Result<Node, EngineError> {
    let Some(node) = session.document().node(sheet) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
        });
    }
    Ok(node.clone())
}

/// The **table** of a sheet, and the failure if there is none.
fn table_of(tables: &[Table], name: &str, id: &str) -> Result<Table, EngineError> {
    tables.iter().find(|table| table.name == name).cloned().ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no table `{name}` on the sheet"),
    })
}

/// **Make** a table of a sheet.
///
/// The range's first row names the fields of the table, and every cell
/// of it must hold text.
#[derive(Command)]
#[command(id = "grid.table.create", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct CreateTable {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the table.
    pub name: Text,
    /// The range it reaches: `A1:C5`.
    pub range: Text,
}

impl CreateTable {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.create".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        if let Some(why) = why_not_name(&name) {
            return Err(EngineError::Failed { id: id.clone(), message: why });
        }
        let text = self.range.0.trim().to_string();
        let area = crate::fill::area_at(&text, &id)?;
        if area.count() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the range {text} is one cell, and a table of one cell is a cell"),
            });
        }
        let tables = tables_of(&node);
        if tables.iter().any(|table| table.name == name) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the sheet already has a table called `{name}`"),
            });
        }
        for table in &tables {
            if overlap(&table.range, &area) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the range {text} is in the table `{}`, and a cell that is in two tables is in none of them",
                        table.name
                    ),
                });
            }
        }
        // The **header**: a row of text, because a field is named by
        // text.
        let cells = crate::cells_of(&node);
        let table = Table { name: name.clone(), range: area, totals: false };
        let fields =
            fields_of(&table, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let mut tables = tables;
        tables.push(table);
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["tables"] = tables_into(&tables);
        }
        Ok(Outcome::changed(format!("the table `{name}` is made on {text}")).with_data(json!({
            "sheet": sheet,
            "name": name,
            "range": text,
            "fields": fields.iter().map(|(_, name)| name.clone()).collect::<Vec<_>>(),
        })))
    }
}

/// **Read** the tables of a sheet.
#[derive(Command)]
#[command(id = "grid.table.read", area = "grid", layer = "grid", icon = "grid")]
pub struct ReadTables {
    /// The sheet they are in.
    pub sheet: NodeRef,
}

impl ReadTables {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.read".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let tables = tables_of(&node);
        Ok(Outcome::new(format!("the sheet holds {} tables", tables.len())).with_data(json!({
            "sheet": sheet,
            "tables": tables.iter().map(|table| table.to_json()).collect::<Vec<_>>(),
        })))
    }
}

/// **Read** the fields of a table.
///
/// The fields are the text of the header cells, and a header cell that
/// no longer holds text is a refusal, because a field with no name is
/// not a field.
#[derive(Command)]
#[command(id = "grid.table.fields", area = "grid", layer = "grid", icon = "grid")]
pub struct ReadTableFields {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the table.
    pub name: Text,
}

impl ReadTableFields {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.fields".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let tables = tables_of(&node);
        let table = table_of(&tables, &name, &id)?;
        let cells = crate::cells_of(&node);
        let fields =
            fields_of(&table, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let rows = table.range.end.row.saturating_sub(table.range.start.row);
        Ok(Outcome::new(format!("the table `{name}` holds {} fields", fields.len())).with_data(json!({
            "sheet": sheet,
            "name": name,
            "range": table.range.to_text(),
            "rows": rows,
            "totals": table.totals,
            "sums": totals_of(&table, &fields, &cells),
            "fields": fields
                .iter()
                .map(|(cell, name)| json!({
                    "name": name,
                    "column": CellRef::column_text(cell.col),
                    "cell": cell.to_text(),
                }))
                .collect::<Vec<_>>(),
        })))
    }
}

/// **Say** whether a table has a **total row**.
///
/// The total row is the row at the bottom of the table that holds the
/// totals of the columns: the sum of the numbers of the rows above
/// it. The totals are **derived** — they are worked out from the
/// cells every time they are asked for — so a table that has one
/// cannot have a total that is out of date with the cells.
///
/// The rules:
///
/// - a table with a total row and **no rows** under its header has
///   nothing to total, and the total of nothing is nothing;
/// - a column whose cells hold no numbers has **no total**: the
///   answer is nothing and not a zero that means something else.
#[derive(Command)]
#[command(id = "grid.table.totals", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct SetTableTotals {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the table.
    pub name: Text,
    /// Whether the table has a total row.
    pub totals: Bool,
}

impl SetTableTotals {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.totals".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let mut tables = tables_of(&node);
        let Some(table) = tables.iter_mut().find(|table| table.name == name) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no table `{name}` on the sheet"),
            });
        };
        table.totals = self.totals.0;
        let tables = tables;
        let sheet_node = session.document_mut()?.node_mut(sheet);
        let Some(sheet_node) = sheet_node else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        sheet_node.data["tables"] = tables_into(&tables);
        Ok(Outcome::changed(if self.totals.0 {
            format!("the table `{name}` has a total row")
        } else {
            format!("the table `{name}` has no total row")
        })
        .with_data(json!({ "sheet": sheet, "name": name, "totals": self.totals.0 })))
    }
}

/// **Rename** a table of a sheet.
#[derive(Command)]
#[command(id = "grid.table.rename", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct RenameTable {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name it has now.
    pub name: Text,
    /// The name it is given.
    pub to: Text,
}

impl RenameTable {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.rename".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let to = self.to.0.trim().to_string();
        if let Some(why) = why_not_name(&to) {
            return Err(EngineError::Failed { id: id.clone(), message: why });
        }
        let mut tables = tables_of(&node);
        if !tables.iter().any(|table| table.name == name) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no table `{name}` on the sheet"),
            });
        }
        if tables.iter().any(|other| other.name == to) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the sheet already has a table called `{to}`"),
            });
        }
        if let Some(table) = tables.iter_mut().find(|table| table.name == name) {
            table.name = to.clone();
        }
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["tables"] = tables_into(&tables);
        }
        Ok(Outcome::changed(format!("the table `{name}` is called `{to}` now")).with_data(json!({
            "sheet": sheet,
            "name": to,
            "was": name,
        })))
    }
}

/// **Resize** a table of a sheet.
///
/// The new range must name its fields the same way: a row of text.
#[derive(Command)]
#[command(id = "grid.table.resize", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct ResizeTable {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the table.
    pub name: Text,
    /// The range it reaches now: `A1:C5`.
    pub range: Text,
}

impl ResizeTable {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.resize".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let text = self.range.0.trim().to_string();
        let area = crate::fill::area_at(&text, &id)?;
        if area.count() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the range {text} is one cell, and a table of one cell is a cell"),
            });
        }
        let mut tables = tables_of(&node);
        if !tables.iter().any(|table| table.name == name) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no table `{name}` on the sheet"),
            });
        }
        for other in &tables {
            if other.name == name {
                continue;
            }
            if overlap(&other.range, &area) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the range {text} is in the table `{}`, and a cell that is in two tables is in none of them",
                        other.name
                    ),
                });
            }
        }
        let Some(table) = tables.iter_mut().find(|table| table.name == name) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no table `{name}` on the sheet"),
            });
        };
        let resized = Table { name: name.clone(), range: area, totals: table.totals };
        let cells = crate::cells_of(&node);
        let fields =
            fields_of(&resized, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        table.range = area;
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["tables"] = tables_into(&tables);
        }
        Ok(Outcome::changed(format!("the table `{name}` reaches {text} now")).with_data(json!({
            "sheet": sheet,
            "name": name,
            "range": text,
            "fields": fields.iter().map(|(_, name)| name.clone()).collect::<Vec<_>>(),
        })))
    }
}

/// **Remove** a table of a sheet.
///
/// The cells of the table stay where they are: a table is a fact of
/// the sheet, and taking the fact away leaves the cells.
#[derive(Command)]
#[command(id = "grid.table.remove", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct RemoveTable {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the table.
    pub name: Text,
}

impl RemoveTable {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.remove".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let mut tables = tables_of(&node);
        let table = table_of(&tables, &name, &id)?;
        tables.retain(|other| other.name != name);
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["tables"] = tables_into(&tables);
        }
        Ok(Outcome::changed(format!("the table `{name}` is taken away")).with_data(json!({
            "sheet": sheet,
            "name": name,
            "range": table.range.to_text(),
        })))
    }
}
