//! The **chart** of a sheet.
//!
//! A chart is a **picture of a range**: it reads the range of a sheet
//! and shows it as bars, as a line or as a pie. The range is the
//! fact — the chart does not hold numbers of its own — and everything
//! the chart **shows** is **derived**: the categories, the series and
//! the values are worked out from the cells every time they are asked
//! for, so a chart cannot be out of date with the sheet it reads.
//!
//! The shape of the data is the tree's shape:
//!
//! - the **first row** of the range names the **series** — the
//!   columns, one after the first;
//! - the **first column** of the range holds the **categories** —
//!   what each row is about;
//! - the rest are the **values**.
//!
//! The rules:
//!
//! - a range of **one row** or **one column** is refused: a chart
//!   needs a name and something to show;
//! - a **kind** that is not one of the four — `column`, `bar`,
//!   `line` and `pie` — is refused;
//! - a cell of a **value** that holds text is **not a number**, and a
//!   value that is not a number is **nothing**: it is not a zero, and
//!   nothing is written as nothing.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Value};

/// The **kinds** of chart there are.
pub const KINDS: [&str; 4] = ["column", "bar", "line", "pie"];

/// The kind of chart, from its name.
pub fn kind_of(text: &str) -> Option<&'static str> {
    let wanted = text.trim().to_ascii_lowercase();
    KINDS.iter().find(|kind| **kind == wanted).copied()
}

/// The **area** a chart reads.
pub fn area_of(chart: &Node) -> Option<Area> {
    let text = chart.data.get("range").and_then(|range| range.as_str())?;
    crate::merges::parse_area(text).map(|(_, area)| area)
}

/// The **kind** of a chart.
pub fn kind_of_chart(chart: &Node) -> String {
    chart.data.get("kind").and_then(|kind| kind.as_str()).unwrap_or("column").to_string()
}

/// The **title** of a chart, if it has one.
pub fn title_of(chart: &Node) -> Json {
    chart.data.get("title").cloned().unwrap_or(Json::Null)
}

/// A **series** of a chart: its name and its values.
#[derive(Debug, Clone)]
pub struct Series {
    /// The name of the series: the cell of the header row.
    pub name: String,
    /// The values of the series, one for each category.
    pub values: Vec<Option<f64>>,
}

/// The **data** of a chart: its categories and its series.
///
/// The categories are the cells of the first column, and the series are
/// the columns after it.
pub fn data_of(sheet: &Node, area: &Area) -> (Vec<String>, Vec<Series>) {
    let cells = crate::cells_of(sheet);
    let text_of = |cell: CellRef| -> String {
        match cells.get(cell).and_then(|cell| cell.value.clone()) {
            Some(Value::Text(text)) => text,
            Some(Value::Number(number)) => {
                if number.fract() == 0.0 && number.abs() < 1e15 {
                    format!("{}", number as i64)
                } else {
                    format!("{number}")
                }
            }
            Some(Value::Bool(value)) => value.to_string(),
            Some(Value::Error(message)) => message,
            None | Some(Value::Empty) => String::new(),
        }
    };
    let number_of = |cell: CellRef| -> Option<f64> {
        match cells.get(cell).and_then(|cell| cell.value.clone()) {
            Some(Value::Number(number)) => Some(number),
            _ => None,
        }
    };
    let categories: Vec<String> = (area.start.row + 1..=area.end.row)
        .map(|row| text_of(CellRef { col: area.start.col, row }))
        .collect();
    let mut series: Vec<Series> = Vec::new();
    for column in area.start.col + 1..=area.end.col {
        let name = text_of(CellRef { col: column, row: area.start.row });
        let values: Vec<Option<f64>> =
            (area.start.row + 1..=area.end.row).map(|row| number_of(CellRef { col: column, row })).collect();
        series.push(Series { name, values });
    }
    (categories, series)
}

/// The **data** of a chart, as it is reported.
pub fn data_to_json(sheet: &Node, area: &Area) -> Json {
    let (categories, series) = data_of(sheet, area);
    let series: Vec<Json> = series
        .iter()
        .map(|series| {
            json!({
                "name": series.name,
                "values": series.values.iter().map(|value| match value {
                    Some(value) => json!(value),
                    None => Json::Null,
                }).collect::<Vec<Json>>(),
            })
        })
        .collect();
    json!({ "categories": categories, "series": series })
}

/// The **sheet** a chart is in.
fn sheet_of(session: &Session, id: &str, chart: u64) -> Result<Node, EngineError> {
    let Some(node) = session.document().node(chart) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no chart `{chart}`"),
        });
    };
    if node.kind != "grid.chart" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{chart}` is a `{}` and not a chart", node.kind),
        });
    }
    let Some(sheet) = session
        .document()
        .walk()
        .find(|node| node.kind == "grid.sheet" && node.children.iter().any(|child| child.id == chart))
    else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the chart `{chart}` is in no sheet"),
        });
    };
    Ok(sheet.clone())
}

/// **Add** a chart to a sheet.
///
/// The chart reads a range: the first row names the series, the first
/// column holds the categories and the rest are the values.
#[derive(Command)]
#[command(id = "grid.chart.add", area = "grid", layer = "grid", icon = "chart", mutates, undoable)]
pub struct AddChart {
    /// The sheet the chart is in.
    pub sheet: NodeRef,
    /// The range the chart reads: `A1:C10`.
    pub range: Text,
    /// The kind of chart: `column`, `bar`, `line` or `pie`.
    pub kind: Option<Text>,
    /// The title of the chart.
    pub title: Option<Text>,
}

impl AddChart {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.chart.add".to_string();
        let sheet = self.sheet.0;
        let Some(node) = session.document().node(sheet).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        if node.kind != "grid.sheet" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
            });
        }
        let area = crate::fill::area_at(self.range.0.trim(), &id)?;
        if area.start.row == area.end.row || area.start.col == area.end.col {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the range {} is one row or one column, and a chart needs a name and something to show",
                    self.range.0.trim()
                ),
            });
        }
        let kind = match &self.kind {
            Some(kind) => match kind_of(&kind.0) {
                Some(kind) => kind.to_string(),
                None => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "`{}` is not a kind of chart: it is one of {}",
                            kind.0.trim(),
                            KINDS.join(", ")
                        ),
                    });
                }
            },
            None => "column".to_string(),
        };
        let title =
            self.title.as_ref().map(|title| title.0.trim().to_string()).filter(|title| !title.is_empty());
        let name = title.clone().unwrap_or_else(|| "the chart".to_string());
        let chart = session.document_mut()?.add_node_with_under(
            Some(sheet),
            name,
            "grid.chart",
            None,
            json!({ "range": area.to_text(), "kind": kind.clone(), "title": title }),
        );
        Ok(Outcome::changed(format!("the chart reads {}", area.to_text()))
            .with_data(json!({ "chart": chart, "range": area.to_text(), "kind": kind })))
    }
}

/// **Read** a chart: what it reads and what it shows.
///
/// The data is **derived**: it is worked out from the cells of the
/// sheet, every time it is asked for.
#[derive(Command)]
#[command(id = "grid.chart.read", area = "grid", layer = "grid", icon = "chart")]
pub struct ReadChart {
    /// The chart.
    pub chart: NodeRef,
}

impl ReadChart {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.chart.read".to_string();
        let chart = self.chart.0;
        let Some(node) = session.document().node(chart).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no chart `{chart}`"),
            });
        };
        if node.kind != "grid.chart" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{chart}` is a `{}` and not a chart", node.kind),
            });
        }
        let sheet = sheet_of(session, &id, chart)?;
        let area = area_of(&node);
        let data = match area {
            Some(area) => data_to_json(&sheet, &area),
            None => json!({ "categories": Vec::<Json>::new(), "series": Vec::<Json>::new() }),
        };
        Ok(Outcome::new(format!("the chart `{chart}` reads a range")).with_data(json!({
            "chart": chart,
            "range": match area {
                Some(area) => json!(area.to_text()),
                None => Json::Null,
            },
            "kind": kind_of_chart(&node),
            "title": title_of(&node),
            "data": data,
        })))
    }
}

/// **Change** a chart: its range, its kind or its title.
///
/// What is not said stays as it was.
#[derive(Command)]
#[command(id = "grid.chart.set", area = "grid", layer = "grid", icon = "chart", mutates, undoable)]
pub struct SetChart {
    /// The chart.
    pub chart: NodeRef,
    /// The range the chart reads.
    pub range: Option<Text>,
    /// The kind of chart.
    pub kind: Option<Text>,
    /// The title of the chart.
    pub title: Option<Text>,
}

impl SetChart {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.chart.set".to_string();
        let chart = self.chart.0;
        let Some(node) = session.document().node(chart).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no chart `{chart}`"),
            });
        };
        if node.kind != "grid.chart" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{chart}` is a `{}` and not a chart", node.kind),
            });
        }
        let mut range = area_of(&node).map(|area| area.to_text());
        let mut kind = kind_of_chart(&node);
        let mut title = title_of(&node);
        if let Some(text) = &self.range {
            let area = crate::fill::area_at(text.0.trim(), &id)?;
            if area.start.row == area.end.row || area.start.col == area.end.col {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the range {} is one row or one column, and a chart needs a name and something to show",
                        text.0.trim()
                    ),
                });
            }
            range = Some(area.to_text());
        }
        if let Some(text) = &self.kind {
            let Some(new_kind) = kind_of(&text.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not a kind of chart: it is one of {}",
                        text.0.trim(),
                        KINDS.join(", ")
                    ),
                });
            };
            kind = new_kind.to_string();
        }
        if let Some(text) = &self.title {
            let text = text.0.trim().to_string();
            title = if text.is_empty() { Json::Null } else { json!(text) };
        }
        if let Some(node) = session.document_mut()?.node_mut(chart) {
            node.data["range"] = match &range {
                Some(range) => json!(range),
                None => Json::Null,
            };
            node.data["kind"] = json!(kind.clone());
            node.data["title"] = title.clone();
        }
        Ok(Outcome::changed(format!("the chart `{chart}` is set")).with_data(json!({
            "chart": chart,
            "range": range,
            "kind": kind,
        })))
    }
}

/// **Take** a chart away.
#[derive(Command)]
#[command(id = "grid.chart.remove", area = "grid", layer = "grid", icon = "chart", mutates, undoable)]
pub struct RemoveChart {
    /// The chart.
    pub chart: NodeRef,
}

impl RemoveChart {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.chart.remove".to_string();
        let chart = self.chart.0;
        let Some(node) = session.document().node(chart).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no chart `{chart}`"),
            });
        };
        if node.kind != "grid.chart" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{chart}` is a `{}` and not a chart", node.kind),
            });
        }
        session.document_mut()?.remove_node(chart);
        Ok(Outcome::changed(format!("the chart `{chart}` is taken away"))
            .with_data(json!({ "chart": chart })))
    }
}
