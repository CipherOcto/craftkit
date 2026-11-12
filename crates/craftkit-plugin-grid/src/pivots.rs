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

use craftkit_derive::commands;

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

/// The **short names** of the months: the tree's own, and the names the
/// pivot writes them with.
const MONTHS: [&str; 12] =
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// The **text** of the key of a row: the name of the group as it is
/// written.
fn text_of_key(key: &craftkit_sdk::serde_json::Value) -> String {
    match key {
        craftkit_sdk::serde_json::Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// The **subtotals** of a pivot: the total of a group of the rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subtotals {
    /// There are none.
    None,
    /// The subtotal is at the **top** of its group.
    Top,
    /// The subtotal is at the **bottom** of its group.
    Bottom,
}

impl Subtotals {
    /// The way a name says, if it is **one of the tree's**.
    ///
    /// The tree's own modes: `none`, `top` and `bottom`.
    pub fn parse(name: &str) -> Option<Subtotals> {
        match name.trim().to_ascii_lowercase().as_str() {
            "none" => Some(Subtotals::None),
            "top" => Some(Subtotals::Top),
            "bottom" => Some(Subtotals::Bottom),
            _ => None,
        }
    }

    /// The name of the way.
    pub fn to_text(self) -> &'static str {
        match self {
            Subtotals::None => "none",
            Subtotals::Top => "top",
            Subtotals::Bottom => "bottom",
        }
    }
}

/// The way the values of a pivot are **shown**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowAs {
    /// The value itself.
    Normal,
    /// The share of the **grand total**.
    PercentOfGrandTotal,
    /// The share of the total of the **column**.
    PercentOfColumnTotal,
    /// The share of the total of the **row**.
    PercentOfRowTotal,
    /// The sum of the values up to this one.
    RunningTotal,
    /// The place of the value among its **siblings**: the largest is
    /// one.
    Rank,
}

impl ShowAs {
    /// The way a name says, if it is **one of the tree's**.
    ///
    /// The tree's own names: `normal` and `none` are one way,
    /// `percentOfGrandTotal`, `percentOfColumnTotal` and
    /// `percentOfRowTotal` are the shares, and `runningTotal` and
    /// `rank` are the last two.
    pub fn parse(name: &str) -> Option<ShowAs> {
        match name.trim().to_ascii_lowercase().as_str() {
            "normal" | "none" => Some(ShowAs::Normal),
            "percentofgrandtotal" => Some(ShowAs::PercentOfGrandTotal),
            "percentofcolumntotal" => Some(ShowAs::PercentOfColumnTotal),
            "percentofrowtotal" => Some(ShowAs::PercentOfRowTotal),
            "runningtotal" => Some(ShowAs::RunningTotal),
            "rank" => Some(ShowAs::Rank),
            _ => None,
        }
    }

    /// The name of the way.
    pub fn to_text(self) -> &'static str {
        match self {
            ShowAs::Normal => "normal",
            ShowAs::PercentOfGrandTotal => "percentOfGrandTotal",
            ShowAs::PercentOfColumnTotal => "percentOfColumnTotal",
            ShowAs::PercentOfRowTotal => "percentOfRowTotal",
            ShowAs::RunningTotal => "runningTotal",
            ShowAs::Rank => "rank",
        }
    }
}

/// The **raw** rows of a summary: the key of the group, the values of
/// every column and the total of the row.
type Raw = (craftkit_sdk::serde_json::Value, Vec<(String, Option<f64>)>, Option<f64>);

/// The **grouping** of the items of a date field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateGroup {
    /// The **year**: `2024`.
    Years,
    /// The **quarter**: `Qtr1`.
    Quarters,
    /// The **month**: `Jan`.
    Months,
    /// The **day**: `5-Jan`.
    Days,
}

impl DateGroup {
    /// The grouping a name says, if it is **one of the tree's**.
    pub fn parse(name: &str) -> Option<DateGroup> {
        match name.trim().to_ascii_lowercase().as_str() {
            "years" | "year" => Some(DateGroup::Years),
            "quarters" | "quarter" => Some(DateGroup::Quarters),
            "months" | "month" => Some(DateGroup::Months),
            "days" | "day" => Some(DateGroup::Days),
            _ => None,
        }
    }

    /// The name of the grouping.
    pub fn to_text(self) -> &'static str {
        match self {
            DateGroup::Years => "years",
            DateGroup::Quarters => "quarters",
            DateGroup::Months => "months",
            DateGroup::Days => "days",
        }
    }
}

/// The **order** of the items of a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    /// From the first to the last.
    Asc,
    /// From the last to the first.
    Desc,
    /// The order of the **source**: the order the items are in where
    /// they came from.
    None,
}

impl Sort {
    /// The order a name says, if it is **one of the tree's**.
    ///
    /// The tree's own names: `asc`, `ascending` and `a-z` are one
    /// order, `desc`, `descending` and `z-a` are another, and `none`,
    /// `manual` and `source` are the order of the source.
    pub fn parse(name: &str) -> Option<Sort> {
        match name.trim().to_ascii_lowercase().as_str() {
            "asc" | "ascending" | "a-z" => Some(Sort::Asc),
            "desc" | "descending" | "z-a" => Some(Sort::Desc),
            "none" | "manual" | "source" => Some(Sort::None),
            _ => None,
        }
    }

    /// The name of the order.
    pub fn to_text(self) -> &'static str {
        match self {
            Sort::Asc => "asc",
            Sort::Desc => "desc",
            Sort::None => "none",
        }
    }
}

/// A **filter** of a pivot: the field it filters by and the items it
/// keeps.
#[derive(Debug, Clone)]
pub struct Filter {
    /// The column it filters by.
    pub field: u32,
    /// The **items** it keeps, as they are written.
    pub selected: Vec<String>,
}

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
    /// Whether the **grand total of the rows** is there: the column at
    /// the right that totals the rows.
    pub grand_totals_rows: bool,
    /// Whether the **grand total of the columns** is there: the line at
    /// the bottom that totals the columns.
    pub grand_totals_cols: bool,
    /// The **filter** of the pivot: the rows whose field is not
    /// selected are left out of it.
    pub filter: Option<Filter>,
    /// The **order** of the items of a field: the field and the order.
    pub sort: Option<(u32, Sort)>,
    /// The **grouping** of a date field: the field and the grouping.
    pub group: Option<(u32, DateGroup)>,
    /// The way the values are **shown**.
    pub show_as: ShowAs,
    /// Where the **subtotals** of the groups are.
    pub subtotals: Subtotals,
}

/// The **pivot**, as the sheet keeps it.
pub(crate) fn pivot_json(pivot: &Pivot) -> craftkit_sdk::serde_json::Value {
    pivot.to_json()
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
            "grand_totals_rows": self.grand_totals_rows,
            "grand_totals_cols": self.grand_totals_cols,
            "filter": match &self.filter {
                Some(filter) => json!({
                    "field": CellRef::column_text(filter.field),
                    "selected": filter.selected.clone(),
                }),
                None => craftkit_sdk::serde_json::Value::Null,
            },
            "sort": match &self.sort {
                Some((field, order)) => {
                    json!({ "field": CellRef::column_text(*field), "order": order.to_text() })
                }
                None => craftkit_sdk::serde_json::Value::Null,
            },
            "group": match &self.group {
                Some((field, group)) => {
                    json!({ "field": CellRef::column_text(*field), "by": group.to_text() })
                }
                None => craftkit_sdk::serde_json::Value::Null,
            },
            "show_as": self.show_as.to_text(),
            "subtotals": self.subtotals.to_text(),
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
        // The tree's own defaults: a pivot has the grand totals of the
        // rows and of the columns.
        let grand_totals_rows =
            pivot.get("grand_totals_rows").and_then(|held| held.as_bool()).unwrap_or(true);
        let grand_totals_cols =
            pivot.get("grand_totals_cols").and_then(|held| held.as_bool()).unwrap_or(true);
        let filter = pivot.get("filter").and_then(|filter| filter.as_object()).and_then(|filter| {
            let field =
                filter.get("field").and_then(|field| field.as_str()).and_then(CellRef::column_index)?;
            let selected: Vec<String> =
                filter.get("selected").and_then(|selected| selected.as_array()).map(|selected| {
                    selected.iter().filter_map(|item| item.as_str().map(|item| item.to_string())).collect()
                })?;
            Some(Filter { field, selected })
        });
        let sort = pivot.get("sort").and_then(|sort| sort.as_object()).and_then(|sort| {
            let field = sort.get("field").and_then(|field| field.as_str()).and_then(CellRef::column_index)?;
            let order = sort.get("order").and_then(|order| order.as_str()).and_then(Sort::parse)?;
            Some((field, order))
        });
        let group = pivot.get("group").and_then(|group| group.as_object()).and_then(|group| {
            let field =
                group.get("field").and_then(|field| field.as_str()).and_then(CellRef::column_index)?;
            let by = group.get("by").and_then(|by| by.as_str()).and_then(DateGroup::parse)?;
            Some((field, by))
        });
        let show_as = pivot
            .get("show_as")
            .and_then(|show| show.as_str())
            .and_then(ShowAs::parse)
            .unwrap_or(ShowAs::Normal);
        let subtotals = pivot
            .get("subtotals")
            .and_then(|subtotals| subtotals.as_str())
            .and_then(Subtotals::parse)
            .unwrap_or(Subtotals::None);
        pivots.push(Pivot {
            name: name.to_string(),
            range: area,
            row_field: row,
            column_field: column,
            value_field: value,
            aggregate,
            grand_totals_rows,
            grand_totals_cols,
            filter,
            sort,
            group,
            show_as,
            subtotals,
        });
    }
    pivots
}

/// The **pivots** of a sheet, as the sheet keeps them.
fn pivots_into(pivots: &[Pivot]) -> craftkit_sdk::serde_json::Value {
    json!(pivots.iter().map(|pivot| pivot.to_json()).collect::<Vec<_>>())
}

/// The **label** of the group a value belongs to, when the pivot
/// groups a date field.
///
/// The tree's own rules: a value that is a number and a date is
/// grouped — by the year, the quarter, the month or the day — and a
/// value that is not a date is left as it is.
fn grouped_key(pivot: &Pivot, value: Value, system: crate::dates::System) -> Option<String> {
    let (field, group) = pivot.group?;
    if field != pivot.row_field {
        return None;
    }
    let number = value.as_number()?;
    let date = crate::dates::from_serial(number, system)?;
    let month = MONTHS[(date.month as usize).saturating_sub(1).min(11)];
    Some(match group {
        DateGroup::Years => date.year.to_string(),
        DateGroup::Quarters => format!("Qtr{}", date.month.saturating_sub(1) / 3 + 1),
        DateGroup::Months => month.to_string(),
        DateGroup::Days => format!("{}-{month}", date.day),
    })
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
pub(crate) fn sheet_of(session: &Session, id: &str, sheet: u64) -> Result<Node, EngineError> {
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
pub(crate) fn header_text(cells: &Cells, cell: CellRef) -> Result<String, String> {
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
pub(crate) fn pivot_of(pivots: &[Pivot], name: &str, id: &str) -> Result<Pivot, EngineError> {
    pivots.iter().find(|pivot| pivot.name == name).cloned().ok_or_else(|| EngineError::Failed {
        id: id.to_string(),
        message: format!("there is no pivot `{name}` on the sheet"),
    })
}

commands! {
    /// **Make** a pivot of a sheet.
    ///
    /// The range has a header row, and the fields are named by it.
    grid.pivot.create {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
            /// The range it reads: `A1:C5`.

            range: Text,
            /// The column it gathers the rows by, as `A`.

            row: Text,
            /// The column it spreads the groups over, as `C`.

            column: Option<Text>,
            /// The column it puts together, as `B`.

            value: Option<Text>,
            /// The way it puts them together: `sum`, `average`, `min`, `max` or
            /// `count`.

            aggregate: Option<Text>,
        },
        run: {

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
                // The tree's own defaults.
                grand_totals_rows: true,
                grand_totals_cols: true,
                filter: None,
                sort: None,
                group: None,
                show_as: ShowAs::Normal,
                subtotals: Subtotals::None,
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
    /// **Say** where the **grand totals** of a pivot are.
    ///
    /// The tree's own modes: `off`, `on`, `rows` and `columns`. The grand
    /// total of the **rows** is the column at the right that totals the
    /// rows, and the grand total of the **columns** is the line at the
    /// bottom that totals the columns.
    grid.pivot.grandtotals {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
            /// Where the totals are.
            ///
            /// `on` — the default — `off`, `rows` or `columns`.

            mode: Text,
        },
        run: {

            let id = "grid.pivot.grandtotals".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let name = self.name.0.trim().to_string();
            let mut pivots = pivots_of(&node);
            let Some(pivot) = pivots.iter_mut().find(|pivot| pivot.name == name) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` has no pivot called `{name}`"),
                });
            };
            let mode = self.mode.0.trim().to_ascii_lowercase();
            let (rows, columns) = match mode.as_str() {
                "off" => (false, false),
                "on" => (true, true),
                "rows" => (true, false),
                "columns" | "cols" => (false, true),
                _ => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the mode `{mode}` is not one of this tree's: it is one of off, on, rows, columns"
                        ),
                    });
                }
            };
            pivot.grand_totals_rows = rows;
            pivot.grand_totals_cols = columns;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["pivots"] = pivots_into(&pivots);
            }
            Ok(Outcome::changed(format!("the grand totals of the pivot `{name}` are placed")).with_data(
                json!({ "sheet": sheet, "name": name, "grand_totals_rows": rows, "grand_totals_cols": columns }),
            ))

        }
    }
    /// **Say** which **items** of a field a pivot keeps.
    ///
    /// The tree's own rules: the field is a column of the range the pivot
    /// reads, the items are the labels as they are written, an empty list
    /// is refused, and no list at all keeps every item.
    grid.pivot.filter {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
            /// The column it filters by: `C`.

            field: Text,
            /// The items it keeps.
            ///
            /// Without it, every item is kept.

            selected: Option<craftkit_sdk::serde_json::Value>,
        },
        run: {

            let id = "grid.pivot.filter".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let name = self.name.0.trim().to_string();
            let mut pivots = pivots_of(&node);
            let Some(pivot) = pivots.iter_mut().find(|pivot| pivot.name == name) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` has no pivot called `{name}`"),
                });
            };
            let field = self.field.0.trim().to_string();
            let Some(field) = CellRef::column_index(&field) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{field}` is not a column"),
                });
            };
            if field < pivot.range.start.col || field > pivot.range.end.col {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the column {} is not in the range {} of the pivot",
                        CellRef::column_text(field),
                        pivot.range.to_text()
                    ),
                });
            }
            // The **items**: a list of labels, one label, or nothing at
            // all — and a list that is empty is refused.
            let selected: Option<Vec<String>> = match &self.selected {
                None | Some(craftkit_sdk::serde_json::Value::Null) => None,
                Some(craftkit_sdk::serde_json::Value::String(item)) => Some(vec![item.clone()]),
                Some(craftkit_sdk::serde_json::Value::Array(items)) => {
                    if items.is_empty() {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "at least one item must be selected".to_string(),
                        });
                    }
                    Some(
                        items
                            .iter()
                            .map(|item| match item {
                                craftkit_sdk::serde_json::Value::String(text) => text.clone(),
                                craftkit_sdk::serde_json::Value::Null => "(blank)".to_string(),
                                other => other.to_string(),
                            })
                            .collect(),
                    )
                }
                Some(_) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the items that are kept are a list of labels".to_string(),
                    });
                }
            };
            pivot.filter = selected.map(|selected| Filter { field, selected });
            let filter = pivot.filter.clone();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["pivots"] = pivots_into(&pivots);
            }
            Ok(Outcome::changed(format!("the items of the pivot `{name}` are chosen")).with_data(json!({
                "sheet": sheet,
                "name": name,
                "filter": match &filter {
                    Some(filter) => json!({
                        "field": CellRef::column_text(filter.field),
                        "selected": filter.selected.clone(),
                    }),
                    None => craftkit_sdk::serde_json::Value::Null,
                },
            })))

        }
    }
    /// **Say** in what **order** the items of a field are.
    ///
    /// The tree's own orders: `asc`, `desc` and `none` — the last of them
    /// is the order of the source. The field is one of the two the pivot
    /// gathers by, and an order that is not one of the tree's is refused
    /// by name.
    grid.pivot.sort {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
            /// The column it orders: `A`.

            field: Text,
            /// The order it is in.
            ///
            /// `asc` — the default — `desc` or `none`.

            order: Option<Text>,
        },
        run: {

            let id = "grid.pivot.sort".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let name = self.name.0.trim().to_string();
            let mut pivots = pivots_of(&node);
            let Some(pivot) = pivots.iter_mut().find(|pivot| pivot.name == name) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` has no pivot called `{name}`"),
                });
            };
            let field = self.field.0.trim().to_string();
            let Some(field) = CellRef::column_index(&field) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{field}` is not a column"),
                });
            };
            if field != pivot.row_field && Some(field) != pivot.column_field {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the pivot gathers by {} and {}, and `{field}` is neither",
                        CellRef::column_text(pivot.row_field),
                        match pivot.column_field {
                            Some(column) => CellRef::column_text(column),
                            None => "nothing else".to_string(),
                        }
                    ),
                });
            }
            let order =
                self.order.as_ref().map(|order| order.0.trim().to_string()).unwrap_or_else(|| "asc".to_string());
            let Some(order) = Sort::parse(&order) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the order `{order}` is not one of this tree's: it is one of asc, desc, none"
                    ),
                });
            };
            pivot.sort = Some((field, order));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["pivots"] = pivots_into(&pivots);
            }
            Ok(Outcome::changed(format!("the items of the pivot `{name}` are in order")).with_data(json!({
                "sheet": sheet,
                "name": name,
                "field": CellRef::column_text(field),
                "order": order.to_text(),
            })))

        }
    }
    /// **Say** how the items of a **date field** are grouped.
    ///
    /// The tree's own groupings: `years`, `quarters`, `months` and `days`,
    /// and a grouping that is not one of them is refused by name. A value
    /// that is not a date is left as it is.
    grid.pivot.group {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
            /// The column it groups: `C`.

            field: Text,
            /// How the items are grouped.
            ///
            /// `years`, `quarters`, `months` or `days`.

            by: Text,
        },
        run: {

            let id = "grid.pivot.group".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let name = self.name.0.trim().to_string();
            let mut pivots = pivots_of(&node);
            let Some(pivot) = pivots.iter_mut().find(|pivot| pivot.name == name) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` has no pivot called `{name}`"),
                });
            };
            let field = self.field.0.trim().to_string();
            let Some(field) = CellRef::column_index(&field) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("`{field}` is not a column"),
                });
            };
            if field != pivot.row_field && Some(field) != pivot.column_field {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the pivot gathers by {} and {}, and `{field}` is neither",
                        CellRef::column_text(pivot.row_field),
                        match pivot.column_field {
                            Some(column) => CellRef::column_text(column),
                            None => "nothing else".to_string(),
                        }
                    ),
                });
            }
            let by = self.by.0.trim().to_string();
            let Some(group) = DateGroup::parse(&by) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the grouping `{by}` is not one of this tree's: it is one of years, quarters, months, days"
                    ),
                });
            };
            pivot.group = Some((field, group));
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["pivots"] = pivots_into(&pivots);
            }
            Ok(Outcome::changed(format!("the items of the pivot `{name}` are grouped")).with_data(json!({
                "sheet": sheet,
                "name": name,
                "field": CellRef::column_text(field),
                "by": group.to_text(),
            })))

        }
    }
    /// **Say** how the values of a pivot are **shown**.
    ///
    /// The tree's own ways: `normal`, `percentOfGrandTotal`,
    /// `percentOfColumnTotal`, `percentOfRowTotal`, `runningTotal` and
    /// `rank` — and a way that is not one of them is refused by name.
    grid.pivot.showas {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
            /// The way the values are shown.
            ///
            /// `normal` — the default — `percentOfGrandTotal`,
            /// `percentOfColumnTotal`, `percentOfRowTotal`, `runningTotal` or
            /// `rank`.

            show_as: Text,
        },
        run: {

            let id = "grid.pivot.showas".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let name = self.name.0.trim().to_string();
            let mut pivots = pivots_of(&node);
            let Some(pivot) = pivots.iter_mut().find(|pivot| pivot.name == name) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` has no pivot called `{name}`"),
                });
            };
            let show = self.show_as.0.trim().to_string();
            let Some(show) = ShowAs::parse(&show) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the way `{show}` is not one of this tree's: it is one of normal, percentOfGrandTotal, percentOfColumnTotal, percentOfRowTotal, runningTotal, rank"
                    ),
                });
            };
            pivot.show_as = show;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["pivots"] = pivots_into(&pivots);
            }
            Ok(Outcome::changed(format!("the values of the pivot `{name}` are shown")).with_data(json!({
                "sheet": sheet,
                "name": name,
                "show_as": show.to_text(),
            })))

        }
    }
    /// **Say** where the **subtotals** of a pivot are.
    ///
    /// The tree's own modes: `none`, `top` and `bottom`, and a mode that
    /// is not one of them is refused by name.
    grid.pivot.subtotals {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
            /// Where the subtotals are.
            ///
            /// `none` — the default — `top` or `bottom`.

            mode: Text,
        },
        run: {

            let id = "grid.pivot.subtotals".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let name = self.name.0.trim().to_string();
            let mut pivots = pivots_of(&node);
            let Some(pivot) = pivots.iter_mut().find(|pivot| pivot.name == name) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` has no pivot called `{name}`"),
                });
            };
            let mode = self.mode.0.trim().to_string();
            let Some(mode) = Subtotals::parse(&mode) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the mode `{mode}` is not one of this tree's: it is one of none, top, bottom"
                    ),
                });
            };
            pivot.subtotals = mode;
            let placed = pivot.subtotals;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["pivots"] = pivots_into(&pivots);
            }
            Ok(Outcome::changed(format!("the subtotals of the pivot `{name}` are placed"))
                .with_data(json!({ "sheet": sheet, "name": name, "subtotals": placed.to_text() })))

        }
    }
    /// **Read** the summary of a pivot.
    ///
    /// The summary is derived: it is worked out now, from the cells of the
    /// sheet as they are.
    grid.pivot.read {
        area: "grid",
        layer: "grid",
        icon: "grid",
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
        },
        run: {

            let id = "grid.pivot.read".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let name = self.name.0.trim().to_string();
            let pivots = pivots_of(&node);
            let pivot = pivot_of(&pivots, &name, &id)?;
            let cells = crate::cells_of(&node);
            let book = crate::book_of(session.document());
            let sheet_name = node.name.clone();
            let system = crate::system_of(session, sheet);

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
                // The **filter**: a row whose field is not selected is left
                // out of the pivot. The tree's own rule.
                if let Some(filter) = &pivot.filter {
                    let held = key_text(&value_at(CellRef { col: filter.field, row }));
                    if !filter.selected.contains(&held) {
                        continue;
                    }
                }
                let key = match grouped_key(&pivot, value_at(CellRef { col: pivot.row_field, row }), system) {
                    Some(label) => Value::Text(label),
                    None => value_at(CellRef { col: pivot.row_field, row }),
                };
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
            // The **order** of the items of the field: the tree's own
            // three orders, and the order of the source when the pivot says
            // so.
            let order_of = |field: u32| -> Option<Sort> {
                pivot.sort.filter(|(sorted, _)| *sorted == field).map(|(_, order)| order)
            };
            match order_of(pivot.row_field) {
                Some(Sort::None) => {}
                Some(Sort::Desc) => keys.sort_by(|left, right| compare(right, left)),
                _ => keys.sort_by(compare),
            }
            let mut columns: Vec<Value> = Vec::new();
            if pivot.column_field.is_some() {
                for (_, group) in &groups {
                    for (column_key, _) in group {
                        if !columns.contains(column_key) {
                            columns.push(column_key.clone());
                        }
                    }
                }
                match order_of(pivot.column_field.unwrap_or_default()) {
                    Some(Sort::None) => {}
                    Some(Sort::Desc) => columns.sort_by(|left, right| compare(right, left)),
                    _ => columns.sort_by(compare),
                }
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
            //
            // The **raw** values first: the way the values are shown is
            // worked out from them.
            let mut raw: Vec<Raw> = Vec::new();
            for key in &keys {
                let Some((_, group)) = groups.iter().find(|(name, _)| name == key) else { continue };
                if pivot.column_field.is_some() {
                    let mut values: Vec<(String, Option<f64>)> = Vec::new();
                    let mut total_numbers: Vec<Value> = Vec::new();
                    for column in &columns {
                        let numbers: Vec<Value> = match group.iter().find(|(name, _)| name == column) {
                            Some((_, numbers)) => numbers.clone(),
                            None => Vec::new(),
                        };
                        total_numbers.extend(numbers.iter().cloned());
                        values.push((key_text(column), put_together(&numbers)));
                    }
                    let total = put_together(&total_numbers);
                    raw.push((key_of(key), values, total));
                } else {
                    let numbers: Vec<Value> = match group.first() {
                        Some((_, numbers)) => numbers.clone(),
                        None => Vec::new(),
                    };
                    raw.push((key_of(key), Vec::new(), put_together(&numbers)));
                }
            }

            // The **totals of the columns** and the **total of everything**:
            // the tree's own, and they are worked out from the raw numbers
            // whether they are shown or not.
            let mut column_totals: Vec<(String, Option<f64>)> = Vec::new();
            let mut all: Vec<Value> = Vec::new();
            for column in &columns {
                let numbers: Vec<Value> = groups
                    .iter()
                    .flat_map(|(_, group)| {
                        group
                            .iter()
                            .find(|(name, _)| name == column)
                            .map(|(_, numbers)| numbers.clone())
                            .unwrap_or_default()
                    })
                    .collect();
                all.extend(numbers.iter().cloned());
                column_totals.push((key_text(column), put_together(&numbers)));
            }
            let everything = put_together(&all);

            // The **way the values are shown**: the tree's own rules.
            let show = pivot.show_as;
            let ratio = |value: Option<f64>, total: Option<f64>| -> craftkit_sdk::serde_json::Value {
                match (value, total) {
                    (Some(_), Some(0.0)) => json!("#DIV/0!"),
                    (Some(value), Some(total)) => json!(value / total),
                    (Some(value), None) => json!(value),
                    (None, _) => craftkit_sdk::serde_json::Value::Null,
                }
            };
            // The **running** sums, one for every column and one for the
            // totals of the rows.
            let mut running: Vec<f64> = vec![0.0; column_totals.len()];
            let mut running_total = 0.0;
            let mut rows = Vec::new();
            for (key, values, total) in &raw {
                let mut row = craftkit_sdk::serde_json::Map::new();
                row.insert("key".to_string(), key.clone());
                let mut shown_values = craftkit_sdk::serde_json::Map::new();
                for (at, (column, value)) in values.iter().enumerate() {
                    let shown_value = match show {
                        ShowAs::Normal => match value {
                            Some(number) => json!(number),
                            None => craftkit_sdk::serde_json::Value::Null,
                        },
                        ShowAs::PercentOfGrandTotal => ratio(*value, everything),
                        ShowAs::PercentOfColumnTotal => {
                            ratio(*value, column_totals.get(at).and_then(|(_, total)| *total))
                        }
                        ShowAs::PercentOfRowTotal => ratio(*value, *total),
                        ShowAs::RunningTotal => {
                            running[at] += value.unwrap_or(0.0);
                            json!(running[at])
                        }
                        ShowAs::Rank => {
                            let me = value.unwrap_or(0.0);
                            let rank = 1 + raw
                                .iter()
                                .filter(|(_, values, _)| {
                                    values.get(at).and_then(|(_, value)| *value).is_some_and(|other| other > me)
                                })
                                .count();
                            json!(rank as f64)
                        }
                    };
                    shown_values.insert(column.clone(), shown_value);
                }
                // The **total of the row**, shown the way the tree shows it:
                // the total of a row is a cell of the grand column, and the
                // grand column is the whole of the pivot.
                let shown_total = match show {
                    ShowAs::Normal => match total {
                        Some(number) => json!(number),
                        None => craftkit_sdk::serde_json::Value::Null,
                    },
                    ShowAs::PercentOfGrandTotal | ShowAs::PercentOfColumnTotal => ratio(*total, everything),
                    ShowAs::PercentOfRowTotal => ratio(*total, *total),
                    ShowAs::RunningTotal => {
                        running_total += total.unwrap_or(0.0);
                        json!(running_total)
                    }
                    ShowAs::Rank => {
                        let me = total.unwrap_or(0.0);
                        let rank = 1 + raw.iter().filter(|(_, _, other)| other.unwrap_or(0.0) > me).count();
                        json!(rank as f64)
                    }
                };
                let values_of_the_row = shown_values.clone();
                let total_of_the_row = shown_total.clone();
                if pivot.column_field.is_some() {
                    row.insert("values".to_string(), craftkit_sdk::serde_json::Value::Object(shown_values));
                    if pivot.grand_totals_rows {
                        row.insert("total".to_string(), shown_total);
                    }
                } else {
                    row.insert("value".to_string(), shown_total);
                }
                // The **subtotal** of the group: the tree's own line, with
                // the name of group and the word "Total".
                let subtotal = match pivot.subtotals {
                    Subtotals::None => None,
                    Subtotals::Top | Subtotals::Bottom => {
                        let mut line = craftkit_sdk::serde_json::Map::new();
                        line.insert("key".to_string(), json!(format!("{} Total", text_of_key(key))));
                        line.insert("subtotal".to_string(), json!(true));
                        if pivot.column_field.is_some() {
                            line.insert(
                                "values".to_string(),
                                craftkit_sdk::serde_json::Value::Object(values_of_the_row.clone()),
                            );
                            if pivot.grand_totals_rows {
                                line.insert("total".to_string(), total_of_the_row.clone());
                            }
                        } else {
                            line.insert("value".to_string(), total_of_the_row.clone());
                        }
                        Some(craftkit_sdk::serde_json::Value::Object(line))
                    }
                };
                if let Some(subtotal) = subtotal {
                    if pivot.subtotals == Subtotals::Top {
                        rows.push(subtotal);
                        rows.push(craftkit_sdk::serde_json::Value::Object(row));
                        continue;
                    }
                    rows.push(craftkit_sdk::serde_json::Value::Object(row));
                    rows.push(subtotal);
                    continue;
                }
                rows.push(craftkit_sdk::serde_json::Value::Object(row));
            }

            // The **grand total of the columns**: the line at the bottom
            // that totals the columns, and the total of everything.
            let grand_total = if pivot.grand_totals_cols {
                let mut totals = craftkit_sdk::serde_json::Map::new();
                for (column, value) in &column_totals {
                    let value = match value {
                        Some(number) => json!(number),
                        None => craftkit_sdk::serde_json::Value::Null,
                    };
                    totals.insert(column.clone(), value);
                }
                let total = match everything {
                    Some(number) => json!(number),
                    None => craftkit_sdk::serde_json::Value::Null,
                };
                json!({ "columns": craftkit_sdk::serde_json::Value::Object(totals), "total": total })
            } else {
                craftkit_sdk::serde_json::Value::Null
            };

            Ok(Outcome::new(format!("the pivot `{name}` gathers {} groups", rows.len())).with_data(json!({
                "sheet": sheet,
                "name": name,
                "aggregate": pivot.aggregate.to_text(),
                "show_as": show.to_text(),
                "subtotals": pivot.subtotals.to_text(),
                "grand_totals_rows": pivot.grand_totals_rows,
                "grand_totals_cols": pivot.grand_totals_cols,
                "filter": match &pivot.filter {
                    Some(filter) => json!({
                        "field": CellRef::column_text(filter.field),
                        "selected": filter.selected.clone(),
                    }),
                    None => craftkit_sdk::serde_json::Value::Null,
                },
                "sort": match &pivot.sort {
                    Some((field, order)) => {
                        json!({ "field": CellRef::column_text(*field), "order": order.to_text() })
                    }
                    None => craftkit_sdk::serde_json::Value::Null,
                },
                "group": match &pivot.group {
                    Some((field, group)) => {
                        json!({ "field": CellRef::column_text(*field), "by": group.to_text() })
                    }
                    None => craftkit_sdk::serde_json::Value::Null,
                },
                "grand_total": grand_total,
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
    grid.pivot.detail {
        area: "grid",
        layer: "grid",
        icon: "grid",
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
            /// The group: the key of the row field.

            group: Text,
        },
        run: {

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
    grid.pivot.remove {
        area: "grid",
        layer: "grid",
        icon: "grid",
        mutates,
        undoable,
        params: {
            /// The sheet it is in.

            sheet: NodeRef,
            /// The name of the pivot.

            name: Text,
        },
        run: {

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
}
