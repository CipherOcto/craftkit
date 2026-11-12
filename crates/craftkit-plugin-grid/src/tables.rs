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

use crate::formula::{Area, Book, CellRef, Cells, Value};

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
    /// The **way** the columns are totalled in: `sum`, `average`,
    /// `count`, `count-numbers`, `max`, `min`, `standard-deviation`,
    /// `variance` or `none`.
    pub way: String,
    /// The **ways of the columns that have one of their own**: the
    /// index of the column in the range and the way it is totalled
    /// in. A column that is not here is totalled in the way of the
    /// table.
    pub ways: Vec<(u32, String)>,
    /// The **label** of the total row: the text the total row shows
    /// in its first cell.
    pub label: Option<String>,
}

impl Table {
    /// The table, as the sheet keeps it.
    fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        json!({
            "name": self.name,
            "range": self.range.to_text(),
            "totals": self.totals,
            "way": self.way.clone(),
            "label": self.label.clone().map(|label| json!(label)).unwrap_or(craftkit_sdk::serde_json::Value::Null),
            "ways": self
                .ways
                .iter()
                .map(|(column, way)| json!({ "column": column, "way": way.clone() }))
                .collect::<Vec<_>>(),
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
        let way = table.get("way").and_then(|way| way.as_str()).unwrap_or("sum").to_string();
        let ways = table
            .get("ways")
            .and_then(|ways| ways.as_array())
            .map(|ways| {
                ways.iter()
                    .filter_map(|entry| {
                        let column = entry.get("column")?.as_u64()?;
                        let way = entry.get("way")?.as_str()?;
                        Some((column as u32, way.to_string()))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let label = table.get("label").and_then(|label| label.as_str()).map(|label| label.to_string());
        tables.push(Table { name: name.to_string(), range: area, totals, way, ways, label });
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
pub(crate) fn fields_of(table: &Table, cells: &Cells) -> Result<Vec<(CellRef, String)>, String> {
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
    book: &Book,
    sheet: &str,
) -> Vec<craftkit_sdk::serde_json::Value> {
    if !table.totals {
        return Vec::new();
    }
    let last = table.range.end.row;
    let mut totals: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
    for (cell, name) in fields {
        let mut numbers: Vec<f64> = Vec::new();
        let mut filled = 0usize;
        for row in table.range.start.row + 1..last {
            let at = CellRef { col: cell.col, row };
            let value = match cells.get(at) {
                Some(entry) => match (&entry.value, &entry.formula) {
                    (Some(value), _) => value.clone(),
                    (None, Some(_)) => book.value_of(sheet, at),
                    (None, None) => Value::Empty,
                },
                None => Value::Empty,
            };
            if matches!(value, Value::Empty) {
                continue;
            }
            filled += 1;
            if let Some(number) = value.as_number() {
                numbers.push(number);
            }
        }
        let index = cell.col.saturating_sub(table.range.start.col);
        let total = match total_of(way_of(table, index), &numbers, filled) {
            Some(total) => json!(total),
            None => craftkit_sdk::serde_json::Value::Null,
        };
        totals.push(json!({
            "name": name.clone(),
            "column": CellRef::column_text(cell.col),
            "total": total,
        }));
    }
    totals
}

/// The **ways** a table may total in: the tree's own list.
const WAYS: [&str; 9] =
    ["sum", "average", "count", "count-numbers", "max", "min", "standard-deviation", "variance", "none"];

/// The **total** of a column, in the way the table says.
///
/// The numbers are the ones the cells hold, and the count is the
/// number of the cells that hold something at all. A column with
/// nothing to total has **no total**: the answer is nothing and not
/// a zero that means something else.
fn total_of(way: &str, numbers: &[f64], filled: usize) -> Option<f64> {
    match way {
        "sum" => {
            if numbers.is_empty() {
                None
            } else {
                Some(numbers.iter().sum())
            }
        }
        "average" => {
            if numbers.is_empty() {
                None
            } else {
                Some(numbers.iter().sum::<f64>() / numbers.len() as f64)
            }
        }
        "count" => Some(filled as f64),
        "count-numbers" => Some(numbers.len() as f64),
        "max" => numbers.iter().copied().fold(None, |largest: Option<f64>, number| {
            Some(largest.map_or(number, |largest| largest.max(number)))
        }),
        "min" => numbers.iter().copied().fold(None, |smallest: Option<f64>, number| {
            Some(smallest.map_or(number, |smallest| smallest.min(number)))
        }),
        "variance" => variance(numbers),
        "standard-deviation" => variance(numbers).map(|variance| variance.sqrt()),
        _ => None,
    }
}

/// The **variance** of the numbers: the sample variance, with the
/// divisor being one less than the number of them.
fn variance(numbers: &[f64]) -> Option<f64> {
    if numbers.len() < 2 {
        return None;
    }
    let mean = numbers.iter().sum::<f64>() / numbers.len() as f64;
    let sum = numbers.iter().map(|number| (number - mean) * (number - mean)).sum::<f64>();
    Some(sum / (numbers.len() as f64 - 1.0))
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

/// The **way** a column of a table is totalled in: the way the column
/// has of its own, and the way of the table when it has none.
fn way_of(table: &Table, index: u32) -> &str {
    table
        .ways
        .iter()
        .find(|(column, _)| *column == index)
        .map(|(_, way)| way.as_str())
        .unwrap_or(table.way.as_str())
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
        let table = Table {
            name: name.clone(),
            range: area,
            totals: false,
            label: None,
            way: "sum".to_string(),
            ways: Vec::new(),
        };
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
        let book = crate::book_of(session.document());
        let sheet_name = node.name.clone();
        let rows = table.range.end.row.saturating_sub(table.range.start.row);
        Ok(Outcome::new(format!("the table `{name}` holds {} fields", fields.len())).with_data(json!({
            "sheet": sheet,
            "name": name,
            "range": table.range.to_text(),
            "rows": rows,
            "totals": table.totals,
            "way": table.way.clone(),
            "sums": totals_of(&table, &fields, &cells, &book, &sheet_name),
            "fields": fields
                .iter()
                .map(|(cell, name)| json!({
                    "name": name,
                    "column": CellRef::column_text(cell.col),
                    "cell": cell.to_text(),
                    "way": way_of(&table, cell.col.saturating_sub(table.range.start.col)),
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
    ///
    /// Leave it out and the table keeps the total row it has.
    pub totals: Option<Bool>,
    /// The **way** the columns are totalled in: `sum`, `average`,
    /// `count`, `count-numbers`, `max`, `min`, `standard-deviation`,
    /// `variance` or `none`.
    ///
    /// Leave it out and the table keeps the way it has.
    pub way: Option<Text>,
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
        if let Some(totals) = &self.totals {
            let was = table.totals;
            table.totals = totals.0;
            // A table that gains a total row shows a **label** in it:
            // the text the total row has in its first cell.
            if table.totals && !was && table.label.is_none() {
                table.label = Some("Total".to_string());
            }
        }
        if let Some(way) = &self.way {
            let name = way.0.trim().to_ascii_lowercase();
            if !WAYS.contains(&name.as_str()) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{name}` is not a way to total: the ways are {}", WAYS.join(", ")),
                });
            }
            table.way = name;
        }
        let totals = table.totals;
        let way = table.way.clone();
        let tables = tables;
        let sheet_node = session.document_mut()?.node_mut(sheet);
        let Some(sheet_node) = sheet_node else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        sheet_node.data["tables"] = tables_into(&tables);
        Ok(Outcome::changed(if totals {
            format!("the table `{name}` has a total row, and it totals in `{way}`")
        } else {
            format!("the table `{name}` has no total row")
        })
        .with_data(json!({ "sheet": sheet, "name": name, "totals": totals, "way": way })))
    }
}

/// **Say** the way a **column** of a table is totalled in.
///
/// A column may be totalled in a way of its own, and the way of the
/// table is the way of the columns that have none of their own.
///
/// The rules:
///
/// - a column that is **not a column** of the table is refused, and a
///   name that **two columns share** is refused too, because a name
///   that is two columns does not say which one is meant;
/// - a way that is **not a way** is refused, and the refusal names
///   the ways there are;
/// - an **empty** way takes the column's own way away, and the column
///   is totalled in the way of the table again.
#[derive(Command)]
#[command(id = "grid.table.way", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct SetTableColumnWay {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the table.
    pub name: Text,
    /// The **column** of the table: the name of its field.
    pub column: Text,
    /// The way the column is totalled in: `sum`, `average`, `count`,
    /// `count-numbers`, `max`, `min`, `standard-deviation`, `variance`
    /// or `none`.
    ///
    /// An empty way takes the column's own way away.
    pub way: Text,
}

impl SetTableColumnWay {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.way".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let mut tables = tables_of(&node);
        let table = table_of(&tables, &name, &id)?;
        let cells = crate::cells_of(&node);
        let fields =
            fields_of(&table, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        let column = self.column.0.trim().to_string();
        let matches = fields
            .iter()
            .enumerate()
            .filter(|(_, (_, field))| *field == column)
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        let Some(index) = matches.first().copied() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the table `{name}` has no column called `{column}`"),
            });
        };
        if matches.len() > 1 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the table `{name}` has {} columns called `{column}`, and a name that is two columns does not say which one",
                    matches.len()
                ),
            });
        }
        let way = self.way.0.trim().to_ascii_lowercase();
        if !way.is_empty() && !WAYS.contains(&way.as_str()) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{way}` is not a way to total: the ways are {}", WAYS.join(", ")),
            });
        }
        let Some(table) = tables.iter_mut().find(|table| table.name == name) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no table `{name}` on the sheet"),
            });
        };
        table.ways.retain(|(held, _)| *held != index as u32);
        if !way.is_empty() {
            table.ways.push((index as u32, way.clone()));
        }
        let way = way_of(table, index as u32).to_string();
        let tables = tables;
        let sheet_node = session.document_mut()?.node_mut(sheet);
        let Some(sheet_node) = sheet_node else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        sheet_node.data["tables"] = tables_into(&tables);
        Ok(Outcome::changed(format!("the column `{column}` of the table `{name}` is totalled in `{way}`"))
            .with_data(json!({
                "sheet": sheet,
                "name": name,
                "column": column,
                "way": way,
            })))
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
        let columns = area.end.col.saturating_sub(area.start.col) + 1;
        let ways = table.ways.iter().filter(|(column, _)| *column < columns).cloned().collect::<Vec<_>>();
        let resized = Table {
            name: name.clone(),
            range: area,
            totals: table.totals,
            label: table.label.clone(),
            way: table.way.clone(),
            ways: ways.clone(),
        };
        let cells = crate::cells_of(&node);
        let fields =
            fields_of(&resized, &cells).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        table.range = area;
        table.ways = ways;
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

/// **Say** the **label** of the **total row** of a table.
///
/// The label is the text the total row shows in its first cell — the
/// word that says what the numbers below it are.
///
/// The rules:
///
/// - a table that is **not there** is refused;
/// - a table with **no total row** has no label, and saying one is
///   refused: the row the label would sit in is not there;
/// - an **empty** label takes the label away, and the total row
///   shows nothing in its first cell;
/// - the label is read as it is.
#[derive(Command)]
#[command(id = "grid.table.label", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct SetTableLabel {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the table.
    pub name: Text,
    /// The label of the total row.
    ///
    /// Leave it out and the label is read. An empty label takes the
    /// label away.
    pub label: Option<Text>,
}

impl SetTableLabel {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.table.label".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let mut tables = tables_of(&node);
        let table = table_of(&tables, &name, &id)?;
        if let Some(label) = &self.label {
            if !table.totals {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the table `{name}` has no total row, and a label sits in one"),
                });
            }
            let label = label.0.trim().to_string();
            let table =
                tables.iter_mut().find(|table| table.name == name).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no table `{name}` on the sheet"),
                })?;
            table.label = if label.is_empty() { None } else { Some(label) };
        }
        let label = tables.iter().find(|table| table.name == name).and_then(|table| table.label.clone());
        let sheet_node = session.document_mut()?.node_mut(sheet);
        let Some(sheet_node) = sheet_node else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        sheet_node.data["tables"] = tables_into(&tables);
        Ok(Outcome::changed(match &label {
            Some(label) => format!("the total row of the table `{name}` says `{label}`"),
            None => format!("the total row of the table `{name}` says nothing"),
        })
        .with_data(json!({
            "sheet": sheet,
            "name": name,
            "label": label.clone().map(|label| json!(label)).unwrap_or(craftkit_sdk::serde_json::Value::Null),
        })))
    }
}
