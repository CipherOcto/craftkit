//! The **pivot** of a sheet.
//!
//! A pivot **summarises** a range of cells: the rows are gathered by
//! the value of one **field**, and the numbers of another field are
//! put together — summed, averaged, or the largest or the smallest of
//! them.
//!
//! Here a pivot is a **fact of the sheet** — the range it reads, the
//! fields it gathers by and the way it puts the numbers together — and
//! the **summary is derived**: it is worked out every time it is asked
//! for, so it cannot be out of date.
//!
//! The rules that hold it together:
//!
//! - the **row field** and the **value field** are columns of the
//!   range, and the range has a **header**: the header cell of a
//!   field names the field, and a cell that holds nothing or a number
//!   does not name a field — so the pivot is **refused** and the
//!   refusal says which cell;
//! - the **way of putting together** is one of the five: `sum`,
//!   `average`, `min`, `max` and `count` — and a way that is not one
//!   of them is refused;
//! - a cell of the **value field** that holds **text** is not a
//!   number, and a number cannot be put together with a text — so the
//!   summary is refused and the refusal says which cell;
//! - a group of rows that holds **no number** has **no value**: the
//!   answer is nothing, and nothing is written as nothing and not as
//!   a zero.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Cells, Value};

/// The **way** the numbers of a pivot are put together.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Aggregate {
    /// The sum of them.
    Sum,
    /// Their average.
    Average,
    /// The smallest of them.
    Min,
    /// The largest of them.
    Max,
    /// How many of them there are.
    Count,
}

impl Aggregate {
    /// The way, from its name.
    pub fn parse(text: &str) -> Option<Aggregate> {
        match text.trim().to_ascii_lowercase().as_str() {
            "sum" => Some(Aggregate::Sum),
            "average" | "avg" | "mean" => Some(Aggregate::Average),
            "min" | "minimum" => Some(Aggregate::Min),
            "max" | "maximum" => Some(Aggregate::Max),
            "count" => Some(Aggregate::Count),
            _ => None,
        }
    }

    /// The name of the way.
    pub fn to_text(self) -> &'static str {
        match self {
            Aggregate::Sum => "sum",
            Aggregate::Average => "average",
            Aggregate::Min => "min",
            Aggregate::Max => "max",
            Aggregate::Count => "count",
        }
    }
}

/// A **group** of a pivot: the key of the group, and the columns of it
/// with the numbers they hold.
type Group = (Value, Vec<(Value, Vec<Value>)>);

/// A **pivot** of a sheet.
#[derive(Debug, Clone)]
pub struct Pivot {
    /// The name of the pivot.
    pub name: String,
    /// The range it reads.
    pub range: Area,
    /// The column it gathers the rows by.
    pub row_field: u32,
    /// The column it spreads the groups over, if it spreads them.
    pub column_field: Option<u32>,
    /// The column it puts together, if it puts one together.
    pub value_field: Option<u32>,
    /// The way it puts them together.
    pub aggregate: Aggregate,
}

impl Pivot {
    /// The pivot, as the sheet keeps it.
    fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        json!({
            "name": self.name,
            "range": self.range.to_text(),
            "row": CellRef::column_text(self.row_field),
            "column": self.column_field.map(CellRef::column_text),
            "value": self.value_field.map(CellRef::column_text),
            "aggregate": self.aggregate.to_text(),
        })
    }
}

/// The **pivots** of a sheet, as they are kept.
pub fn pivots_of(node: &Node) -> Vec<Pivot> {
    let mut pivots = Vec::new();
    let Some(list) = node.data.get("pivots").and_then(|pivots| pivots.as_array()) else {
        return pivots;
    };
    for pivot in list {
        let Some(name) = pivot.get("name").and_then(|name| name.as_str()) else { continue };
        let Some(range) = pivot.get("range").and_then(|range| range.as_str()) else { continue };
        let Some((_, area)) = crate::merges::parse_area(range) else { continue };
        let Some(row) = pivot.get("row").and_then(|row| row.as_str()).and_then(CellRef::column_index) else {
            continue;
        };
        let column = pivot.get("column").and_then(|column| column.as_str()).and_then(CellRef::column_index);
        let value = pivot.get("value").and_then(|value| value.as_str()).and_then(CellRef::column_index);
        let aggregate = pivot
            .get("aggregate")
            .and_then(|aggregate| aggregate.as_str())
            .and_then(Aggregate::parse)
            .unwrap_or(Aggregate::Sum);
        pivots.push(Pivot {
            name: name.to_string(),
            range: area,
            row_field: row,
            column_field: column,
            value_field: value,
            aggregate,
        });
    }
    pivots
}

/// The **pivots** of a sheet, as the sheet keeps them.
fn pivots_into(pivots: &[Pivot]) -> craftkit_sdk::serde_json::Value {
    json!(pivots.iter().map(|pivot| pivot.to_json()).collect::<Vec<_>>())
}

/// The **order** of two values, the tree's way.
///
/// The numbers come before the text, and the text before the booleans,
/// and the empty cells last.
fn compare(left: &Value, right: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    match (left, right) {
        (Value::Empty, Value::Empty) => Ordering::Equal,
        (Value::Empty, _) => Ordering::Greater,
        (_, Value::Empty) => Ordering::Less,
        (Value::Number(one), Value::Number(other)) => one.partial_cmp(other).unwrap_or(Ordering::Equal),
        (Value::Text(one), Value::Text(other)) => one.cmp(other),
        (Value::Bool(one), Value::Bool(other)) => one.cmp(other),
        (Value::Error(one), Value::Error(other)) => one.as_str().cmp(other.as_str()),
        _ => Ordering::Equal,
    }
}

/// The **key** of a cell: the value, or nothing.
fn key_of(value: &Value) -> craftkit_sdk::serde_json::Value {
    match value {
        Value::Empty => json!("(blank)"),
        Value::Text(text) => json!(text),
        Value::Number(number) => json!(number),
        Value::Bool(flag) => json!(flag),
        Value::Error(message) => json!(message),
    }
}

/// The **text** of a key: the way it is written in a spread.
fn key_text(value: &Value) -> String {
    match value {
        Value::Empty => "(blank)".to_string(),
        Value::Text(text) => text.clone(),
        Value::Number(number) => {
            if number.fract() == 0.0 && number.abs() < 9.0e15 {
                format!("{}", *number as i64)
            } else {
                format!("{number}")
            }
        }
        Value::Bool(flag) => flag.to_string(),
        Value::Error(message) => message.clone(),
    }
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

/// The **text** of a header cell, and why it is not one.
fn header_text(cells: &Cells, cell: CellRef) -> Result<String, String> {
    let Some(entry) = cells.get(cell) else {
        return Err(format!("the cell {} holds nothing, and a field is named by its header", cell.to_text()));
    };
    if entry.formula.is_some() {
        return Err(format!(
            "the cell {} holds a formula, and a field is named by its header",
            cell.to_text()
        ));
    }
    match &entry.value {
        Some(Value::Text(text)) if !text.trim().is_empty() => Ok(text.trim().to_string()),
        _ => {
            Err(format!("the cell {} does not hold text, and a field is named by its header", cell.to_text()))
        }
    }
}

/// The **name** of a field: the text of its header cell.
fn field_name(cells: &Cells, area: &Area, column: u32) -> Result<String, String> {
    let cell = CellRef { col: column, row: area.start.row };
    header_text(cells, cell)
}

/// The **pivot** of a sheet, and the failure if there is none.
fn pivot_of(pivots: &[Pivot], name: &str, id: &str) -> Result<Pivot, EngineError> {
    pivots.iter().find(|pivot| pivot.name == name).cloned().ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no pivot `{name}` on the sheet"),
    })
}

/// **Make** a pivot of a sheet.
///
/// The range has a header row, and the fields are named by it.
#[derive(Command)]
#[command(id = "grid.pivot.create", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct CreatePivot {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the pivot.
    pub name: Text,
    /// The range it reads: `A1:C5`.
    pub range: Text,
    /// The column it gathers the rows by, as `A`.
    pub row: Text,
    /// The column it spreads the groups over, as `C`.
    pub column: Option<Text>,
    /// The column it puts together, as `B`.
    pub value: Option<Text>,
    /// The way it puts them together: `sum`, `average`, `min`, `max` or
    /// `count`.
    pub aggregate: Option<Text>,
}

impl CreatePivot {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.pivot.create".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a pivot's name is not nothing".to_string(),
            });
        }
        let text = self.range.0.trim().to_string();
        let area = crate::fill::area_at(&text, &id)?;
        if area.count() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the range {text} is one cell, and a pivot reads more than one cell"),
            });
        }
        let row_field = self.row.0.trim().to_ascii_uppercase();
        let Some(row_column) = CellRef::column_index(&row_field) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{}` is not a column", self.row.0.trim()),
            });
        };
        if row_column < area.start.col || row_column > area.end.col {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the column {} is not in the range {text}",
                    CellRef::column_text(row_column)
                ),
            });
        }
        let column_field = match &self.column {
            Some(column) => {
                let text = column.0.trim().to_ascii_uppercase();
                let Some(column) = CellRef::column_index(&text) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{}` is not a column", column.0.trim()),
                    });
                };
                if column < area.start.col || column > area.end.col {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the column {} is not in the range {text}",
                            CellRef::column_text(column)
                        ),
                    });
                }
                Some(column)
            }
            None => None,
        };
        let value_field = match &self.value {
            Some(value) => {
                let text = value.0.trim().to_ascii_uppercase();
                let Some(column) = CellRef::column_index(&text) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{}` is not a column", value.0.trim()),
                    });
                };
                if column < area.start.col || column > area.end.col {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the column {} is not in the range {text}",
                            CellRef::column_text(column)
                        ),
                    });
                }
                Some(column)
            }
            None => None,
        };
        let aggregate = match &self.aggregate {
            Some(aggregate) => {
                let Some(aggregate) = Aggregate::parse(&aggregate.0) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "`{}` is not a way of putting numbers together: sum, average, min, max or count",
                            aggregate.0.trim()
                        ),
                    });
                };
                aggregate
            }
            None => Aggregate::Sum,
        };
        if aggregate != Aggregate::Count && value_field.is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the way of putting numbers together is `{}`, and it needs the column it puts together",
                    aggregate.to_text()
                ),
            });
        }
        // The **header**: the fields are named by it.
        let cells = crate::cells_of(&node);
        let row_name = field_name(&cells, &area, row_column)
            .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        if let Some(column) = column_field {
            field_name(&cells, &area, column)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        }
        if let Some(value) = value_field {
            field_name(&cells, &area, value)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
        }
        let mut pivots = pivots_of(&node);
        if pivots.iter().any(|pivot| pivot.name == name) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the sheet already has a pivot called `{name}`"),
            });
        }
        let pivot = Pivot {
            name: name.clone(),
            range: area,
            row_field: row_column,
            column_field,
            value_field,
            aggregate,
        };
        pivots.push(pivot);
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["pivots"] = pivots_into(&pivots);
        }
        Ok(Outcome::changed(format!("the pivot `{name}` is made on {text}")).with_data(json!({
            "sheet": sheet,
            "name": name,
            "range": text,
            "row": row_name,
            "aggregate": aggregate.to_text(),
        })))
    }
}

/// **Read** the summary of a pivot.
///
/// The summary is derived: it is worked out now, from the cells of the
/// sheet as they are.
#[derive(Command)]
#[command(id = "grid.pivot.read", area = "grid", layer = "grid", icon = "grid")]
pub struct ReadPivot {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the pivot.
    pub name: Text,
}

impl ReadPivot {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.pivot.read".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let pivots = pivots_of(&node);
        let pivot = pivot_of(&pivots, &name, &id)?;
        let cells = crate::cells_of(&node);
        let book = crate::book_of(session.document());
        let sheet_name = node.name.clone();

        // The **value** of a cell: what it holds, or what its formula
        // computes.
        let value_at = |cell: CellRef| -> Value {
            match cells.get(cell) {
                Some(entry) => match (&entry.value, &entry.formula) {
                    (Some(value), _) => value.clone(),
                    (None, Some(_)) => book.value_of(&sheet_name, cell),
                    (None, None) => Value::Empty,
                },
                None => Value::Empty,
            }
        };

        // The **groups**: the rows gathered by the key of the row
        // field, and the columns gathered by the key of the column
        // field.
        let mut groups: Vec<Group> = Vec::new();
        for row in pivot.range.start.row + 1..=pivot.range.end.row {
            let key = value_at(CellRef { col: pivot.row_field, row });
            let column_key = match pivot.column_field {
                Some(column) => value_at(CellRef { col: column, row }),
                None => Value::Empty,
            };
            let number = match pivot.value_field {
                Some(column) => {
                    let cell = CellRef { col: column, row };
                    let value = value_at(cell);
                    match value {
                        Value::Empty => Value::Empty,
                        Value::Number(number) => Value::Number(number),
                        Value::Text(_) => {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!(
                                    "the cell {} holds text, and a number cannot be put together with a text",
                                    cell.to_text()
                                ),
                            });
                        }
                        Value::Bool(_) | Value::Error(_) => {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!(
                                    "the cell {} does not hold a number, and a number cannot be put together with it",
                                    cell.to_text()
                                ),
                            });
                        }
                    }
                }
                None => Value::Empty,
            };
            // The **group** the row belongs to, and the **column** of
            // that group it belongs to.
            let index = match groups.iter().position(|(name, _)| *name == key) {
                Some(index) => index,
                None => {
                    groups.push((key.clone(), Vec::new()));
                    groups.len() - 1
                }
            };
            let group = &mut groups[index].1;
            match group.iter().position(|(name, _)| *name == column_key) {
                Some(position) => group[position].1.push(number),
                None => group.push((column_key, vec![number])),
            }
        }
        let mut keys: Vec<Value> = groups.iter().map(|(key, _)| key.clone()).collect();
        keys.sort_by(compare);
        let mut columns: Vec<Value> = Vec::new();
        if pivot.column_field.is_some() {
            for (_, group) in &groups {
                for (column_key, _) in group {
                    if !columns.contains(column_key) {
                        columns.push(column_key.clone());
                    }
                }
            }
            columns.sort_by(compare);
        }

        // The **numbers** of a group, put together.
        let put_together = |numbers: &[Value]| -> Option<f64> {
            if pivot.aggregate == Aggregate::Count {
                let count = numbers.iter().filter(|value| !matches!(value, Value::Empty)).count();
                return Some(count as f64);
            }
            let values: Vec<f64> = numbers.iter().filter_map(|value| value.as_number()).collect();
            if values.is_empty() {
                return None;
            }
            Some(match pivot.aggregate {
                Aggregate::Sum => values.iter().sum(),
                Aggregate::Average => values.iter().sum::<f64>() / values.len() as f64,
                Aggregate::Min => values.iter().copied().fold(f64::INFINITY, f64::min),
                Aggregate::Max => values.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                Aggregate::Count => values.len() as f64,
            })
        };

        // The **summary**: a row for every group, and a column for
        // every column of the spread.
        let mut rows = Vec::new();
        for key in &keys {
            let Some((_, group)) = groups.iter().find(|(name, _)| name == key) else { continue };
            let mut row = craftkit_sdk::serde_json::Map::new();
            row.insert("key".to_string(), key_of(key));
            if pivot.column_field.is_some() {
                let mut values = craftkit_sdk::serde_json::Map::new();
                let mut total_numbers: Vec<Value> = Vec::new();
                for column in &columns {
                    let numbers: Vec<Value> = match group.iter().find(|(name, _)| name == column) {
                        Some((_, numbers)) => numbers.clone(),
                        None => Vec::new(),
                    };
                    total_numbers.extend(numbers.iter().cloned());
                    let value = match put_together(&numbers) {
                        Some(number) => json!(number),
                        None => json!(null),
                    };
                    values.insert(key_text(column), value);
                }
                let total = match put_together(&total_numbers) {
                    Some(number) => json!(number),
                    None => json!(null),
                };
                row.insert("values".to_string(), craftkit_sdk::serde_json::Value::Object(values));
                row.insert("total".to_string(), total);
            } else {
                let numbers: Vec<Value> = match group.first() {
                    Some((_, numbers)) => numbers.clone(),
                    None => Vec::new(),
                };
                let value = match put_together(&numbers) {
                    Some(number) => json!(number),
                    None => json!(null),
                };
                row.insert("value".to_string(), value);
            }
            rows.push(craftkit_sdk::serde_json::Value::Object(row));
        }

        Ok(Outcome::new(format!("the pivot `{name}` gathers {} groups", rows.len())).with_data(json!({
            "sheet": sheet,
            "name": name,
            "aggregate": pivot.aggregate.to_text(),
            "columns": columns.iter().map(key_of).collect::<Vec<_>>(),
            "rows": rows,
        })))
    }
}

/// **Read** the rows that stand behind a **group** of a pivot.
///
/// The group is the key of the row field, and the rows are the rows
/// of the range that hold it. The detail is **derived**: it is worked
/// out from the range every time it is asked for, and it cannot be
/// out of date.
#[derive(Command)]
#[command(id = "grid.pivot.detail", area = "grid", layer = "grid", icon = "grid")]
pub struct ReadPivotDetail {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the pivot.
    pub name: Text,
    /// The group: the key of the row field.
    pub group: Text,
}

impl ReadPivotDetail {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.pivot.detail".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let pivots = pivots_of(&node);
        let pivot = pivot_of(&pivots, &name, &id)?;
        let cells = crate::cells_of(&node);
        let book = crate::book_of(session.document());
        let sheet_name = node.name.clone();
        let group = self.group.0.trim().to_string();

        // The **value** of a cell: what it holds, or what its formula
        // computes.
        let value_at = |cell: CellRef| -> Value {
            match cells.get(cell) {
                Some(entry) => match (&entry.value, &entry.formula) {
                    (Some(value), _) => value.clone(),
                    (None, Some(_)) => book.value_of(&sheet_name, cell),
                    (None, None) => Value::Empty,
                },
                None => Value::Empty,
            }
        };

        // The **rows** that stand behind the group.
        let mut rows: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
        for row in pivot.range.start.row + 1..=pivot.range.end.row {
            let key = value_at(CellRef { col: pivot.row_field, row });
            if key_text(&key) != group {
                continue;
            }
            let column_key = match pivot.column_field {
                Some(column) => value_at(CellRef { col: column, row }),
                None => Value::Empty,
            };
            // The **number** of the row: the cell of the value field
            // holds a number, or nothing.
            let value = match pivot.value_field {
                Some(column) => {
                    let cell = CellRef { col: column, row };
                    let value = value_at(cell);
                    match value {
                        Value::Empty => Value::Empty,
                        Value::Number(number) => Value::Number(number),
                        Value::Text(_) => {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!(
                                    "the cell {} holds text, and a number cannot be put together with a text",
                                    cell.to_text()
                                ),
                            });
                        }
                        Value::Bool(_) | Value::Error(_) => {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!(
                                    "the cell {} does not hold a number, and a number cannot be put together with it",
                                    cell.to_text()
                                ),
                            });
                        }
                    }
                }
                None => Value::Empty,
            };
            let mut held = craftkit_sdk::serde_json::Map::new();
            held.insert("row".to_string(), json!(row + 1));
            held.insert("key".to_string(), key_of(&key));
            held.insert("column".to_string(), key_of(&column_key));
            held.insert("value".to_string(), value.to_json());
            rows.push(craftkit_sdk::serde_json::Value::Object(held));
        }
        if rows.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no group `{group}` in the pivot `{name}`"),
            });
        }
        let count = rows.len();
        Ok(Outcome::new(format!("the group `{group}` stands for {count} row(s)")).with_data(json!({
            "sheet": sheet,
            "name": name,
            "group": group,
            "rows": rows,
        })))
    }
}

/// **Take away** a pivot of a sheet.
///
/// The cells it read stay where they are: a pivot is a fact of the
/// sheet, and taking the fact away leaves the cells.
#[derive(Command)]
#[command(id = "grid.pivot.remove", area = "grid", layer = "grid", icon = "grid", mutates, undoable)]
pub struct RemovePivot {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The name of the pivot.
    pub name: Text,
}

impl RemovePivot {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.pivot.remove".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let name = self.name.0.trim().to_string();
        let mut pivots = pivots_of(&node);
        let pivot = pivot_of(&pivots, &name, &id)?;
        pivots.retain(|other| other.name != name);
        if let Some(node) = session.document_mut()?.node_mut(sheet) {
            node.data["pivots"] = pivots_into(&pivots);
        }
        Ok(Outcome::changed(format!("the pivot `{name}` is taken away")).with_data(json!({
            "sheet": sheet,
            "name": name,
            "range": pivot.range.to_text(),
        })))
    }
}
