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
//! The **elements** of a chart are the tree's own: the **title**, the
//! **legend**, the **data labels**, the **gridlines**, the **titles of
//! the axes** and the **smooth** line. They are in the payload of the
//! chart, and the tree's own rules say what they are when a chart is
//! made:
//!
//! - the **legend** is at the **bottom**, and at the **right** for a
//!   **pie**;
//! - the **data labels** are **off**;
//! - the **gridlines** are **on**, and **off** for a **pie**;
//! - the **titles of the axes** are **none**;
//! - the line is **not smooth**.
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

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::{Area, CellRef, Value};

/// The **kinds** of chart there are.
pub const KINDS: [&str; 4] = ["column", "bar", "line", "pie"];

/// The **positions** of the legend of a chart: the tree's own.
pub const LEGENDS: [&str; 5] = ["none", "bottom", "top", "left", "right"];

/// The **elements** of a chart that may be turned on and off: the
/// tree's own.
pub const ELEMENTS: [&str; 6] = ["title", "legend", "datalabels", "gridlines", "axistitles", "smooth"];

/// The position of the legend, from its name.
pub fn legend_of(text: &str) -> Option<&'static str> {
    let wanted = text.trim().to_ascii_lowercase();
    LEGENDS.iter().find(|legend| **legend == wanted).copied()
}

/// The element of a chart, from its name.
///
/// The tree's own names are camel-cased — `dataLabels`, `axisTitles` —
/// and the names of this tree are not: the case is not what a name
/// is.
pub fn element_of(text: &str) -> Option<&'static str> {
    let wanted = text.trim().to_ascii_lowercase();
    ELEMENTS.iter().find(|element| **element == wanted).copied()
}

/// The legend of a chart, if it has one: the tree's own default is the
/// bottom, and the right for a pie.
pub fn legend_of_chart(chart: &Node, kind: &str) -> String {
    match chart.data.get("legend").and_then(|legend| legend.as_str()) {
        Some(legend) => legend.to_string(),
        None => {
            if kind == "pie" {
                "right".to_string()
            } else {
                "bottom".to_string()
            }
        }
    }
}

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
pub(crate) fn sheet_of(session: &Session, id: &str, chart: u64) -> Result<Node, EngineError> {
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

commands! {
    /// **Add** a chart to a sheet.
    ///
    /// The chart reads a range: the first row names the series, the first
    /// column holds the categories and the rest are the values.
    grid.chart.add {
        area: "grid",
        layer: "grid",
        icon: "chart",
        mutates,
        undoable,
        params: {
            /// The sheet the chart is in.

            sheet: NodeRef,
            /// The range the chart reads: `A1:C10`.

            range: Text,
            /// The kind of chart: `column`, `bar`, `line` or `pie`.

            kind: Option<Text>,
            /// The title of the chart.

            title: Option<Text>,
            /// Where the legend is: `none`, `bottom`, `top`, `left` or `right`.

            legend: Option<Text>,
            /// Whether the data labels are shown.

            data_labels: Option<Bool>,
            /// Whether the gridlines are shown.

            gridlines: Option<Bool>,
            /// The title of the axis of the categories.

            x_title: Option<Text>,
            /// The title of the axis of the values.

            y_title: Option<Text>,
        },
        run: {

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
            // The **legend**: the tree's own default is the bottom, and
            // the right for a pie.
            let legend = match &self.legend {
                Some(legend) => match legend_of(&legend.0) {
                    Some(legend) => legend.to_string(),
                    None => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "`{}` is not a place for the legend: it is one of {}",
                                legend.0.trim(),
                                LEGENDS.join(", ")
                            ),
                        });
                    }
                },
                None => {
                    if kind == "pie" {
                        "right".to_string()
                    } else {
                        "bottom".to_string()
                    }
                }
            };
            let name = title.clone().unwrap_or_else(|| "the chart".to_string());
            let chart = session.document_mut()?.add_node_with_under(
                Some(sheet),
                name,
                "grid.chart",
                None,
                json!({
                    "range": area.to_text(),
                    "kind": kind.clone(),
                    "title": title,
                    "legend": legend,
                    "dataLabels": self.data_labels.map(|on| on.0).unwrap_or(false),
                    "gridlines": self.gridlines.map(|on| on.0).unwrap_or(kind != "pie"),
                    "xTitle": self.x_title.as_ref().map(|title| title.0.trim().to_string()),
                    "yTitle": self.y_title.as_ref().map(|title| title.0.trim().to_string()),
                    "smooth": false,
                }),
            );
            Ok(Outcome::changed(format!("the chart reads {}", area.to_text()))
                .with_data(json!({ "chart": chart, "range": area.to_text(), "kind": kind })))

        }
    }
    /// **Read** a chart: what it reads and what it shows.
    ///
    /// The data is **derived**: it is worked out from the cells of the
    /// sheet, every time it is asked for.
    grid.chart.read {
        area: "grid",
        layer: "grid",
        icon: "chart",
        params: {
            /// The chart.

            chart: NodeRef,
        },
        run: {

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
            let kind = kind_of_chart(&node);
            Ok(Outcome::new(format!("the chart `{chart}` reads a range")).with_data(json!({
                "chart": chart,
                "range": match area {
                    Some(area) => json!(area.to_text()),
                    None => Json::Null,
                },
                "kind": kind.clone(),
                "title": title_of(&node),
                "legend": legend_of_chart(&node, &kind),
                "dataLabels": element_on(&node, "dataLabels"),
                "gridlines": element_on(&node, "gridlines"),
                "xTitle": node.data.get("xTitle").cloned().unwrap_or(Json::Null),
                "yTitle": node.data.get("yTitle").cloned().unwrap_or(Json::Null),
                "smooth": element_on(&node, "smooth"),
                "data": data,
            })))

        }
    }
    /// **Change** a chart: its range, its kind or its title.
    ///
    /// What is not said stays as it was.
    grid.chart.set {
        area: "grid",
        layer: "grid",
        icon: "chart",
        mutates,
        undoable,
        params: {
            /// The chart.

            chart: NodeRef,
            /// The range the chart reads.

            range: Option<Text>,
            /// The kind of chart.

            kind: Option<Text>,
            /// The title of the chart.

            title: Option<Text>,
            /// Where the legend is: `none`, `bottom`, `top`, `left` or `right`.

            legend: Option<Text>,
            /// Whether the data labels are shown.

            data_labels: Option<Bool>,
            /// Whether the gridlines are shown.

            gridlines: Option<Bool>,
            /// The title of the axis of the categories.

            x_title: Option<Text>,
            /// The title of the axis of the values.

            y_title: Option<Text>,
        },
        run: {

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
            // The **legend**.
            let mut legend = legend_of_chart(&node, &kind);
            if let Some(text) = &self.legend {
                let Some(place) = legend_of(&text.0) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "`{}` is not a place for the legend: it is one of {}",
                            text.0.trim(),
                            LEGENDS.join(", ")
                        ),
                    });
                };
                legend = place.to_string();
            }
            if let Some(node) = session.document_mut()?.node_mut(chart) {
                node.data["range"] = match &range {
                    Some(range) => json!(range),
                    None => Json::Null,
                };
                node.data["kind"] = json!(kind.clone());
                node.data["title"] = title.clone();
                node.data["legend"] = json!(legend.clone());
                if let Some(on) = self.data_labels {
                    node.data["dataLabels"] = json!(on.0);
                }
                if let Some(on) = self.gridlines {
                    node.data["gridlines"] = json!(on.0);
                }
                if let Some(text) = &self.x_title {
                    let text = text.0.trim().to_string();
                    node.data["xTitle"] = if text.is_empty() { Json::Null } else { json!(text) };
                }
                if let Some(text) = &self.y_title {
                    let text = text.0.trim().to_string();
                    node.data["yTitle"] = if text.is_empty() { Json::Null } else { json!(text) };
                }
            }
            Ok(Outcome::changed(format!("the chart `{chart}` is set")).with_data(json!({
                "chart": chart,
                "range": range,
                "kind": kind,
            })))

        }
    }
    /// **Turn** an **element** of a chart on or off.
    ///
    /// The tree's own: the elements of a chart are the **title**, the
    /// **legend**, the **data labels**, the **gridlines**, the **titles of
    /// the axes** and the **smooth** line — and an element that is not one
    /// of them is refused.
    ///
    /// The rules:
    ///
    /// - the element is turned **on** unless it is said to be off;
    /// - the **title** is the title the chart has, or `Chart Title` when
    ///   it has none, and an empty title when the title is turned off;
    /// - the **legend** is `none` when it is turned off, and it is put
    ///   where it is said to be when it is turned on;
    /// - the **titles of the axes** are the titles the chart has, or
    ///   `Axis Title` when it has none, and they are taken away when they
    ///   are turned off.
    grid.chart.element {
        area: "grid",
        layer: "grid",
        icon: "chart",
        mutates,
        undoable,
        params: {
            /// The chart.

            chart: NodeRef,
            /// The element: `title`, `legend`, `dataLabels`, `gridlines`,
            /// `axisTitles` or `smooth`.

            element: Text,
            /// Whether the element is on; it is on unless it is said to be
            /// off.

            on: Option<Bool>,
            /// Where the legend is: `bottom`, `top`, `left` or `right`.

            position: Option<Text>,
        },
        run: {

            let id = "grid.chart.element".to_string();
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
            let Some(element) = element_of(&self.element.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not an element of a chart: it is one of {}",
                        self.element.0.trim(),
                        ELEMENTS.join(", ")
                    ),
                });
            };
            let on = self.on.map(|on| on.0).unwrap_or(true);
            let kind = kind_of_chart(&node);
            // The **legend**: where it is put.
            let mut legend = legend_of_chart(&node, &kind);
            if element == "legend"
                && on
                && let Some(text) = &self.position
            {
                let Some(place) = legend_of(&text.0) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "`{}` is not a place for the legend: it is one of {}",
                            text.0.trim(),
                            LEGENDS.join(", ")
                        ),
                    });
                };
                legend = place.to_string();
            }
            if element == "legend" && !on {
                legend = "none".to_string();
            }
            // The **title**: the one it has, or the tree's own.
            let title = if element == "title" {
                if on {
                    node.data
                        .get("title")
                        .and_then(|title| title.as_str())
                        .map(|title| title.to_string())
                        .filter(|title| !title.is_empty())
                        .unwrap_or_else(|| "Chart Title".to_string())
                } else {
                    String::new()
                }
            } else {
                node.data
                    .get("title")
                    .and_then(|title| title.as_str())
                    .map(|title| title.to_string())
                    .unwrap_or_default()
            };
            // The **titles of the axes**.
            let axis = |name: &str| {
                node.data
                    .get(name)
                    .and_then(|title| title.as_str())
                    .map(|title| title.to_string())
                    .filter(|title| !title.is_empty())
            };
            let (x_title, y_title) = if element == "axistitles" {
                if on {
                    (
                        axis("xTitle").or_else(|| Some("Axis Title".to_string())),
                        axis("yTitle").or_else(|| Some("Axis Title".to_string())),
                    )
                } else {
                    (None, None)
                }
            } else {
                (axis("xTitle"), axis("yTitle"))
            };
            if let Some(node) = session.document_mut()?.node_mut(chart) {
                node.data["legend"] = json!(legend.clone());
                if element == "title" {
                    node.data["title"] = if title.is_empty() { Json::Null } else { json!(title.clone()) };
                }
                if element == "datalabels" {
                    node.data["dataLabels"] = json!(on);
                }
                if element == "gridlines" {
                    node.data["gridlines"] = json!(on);
                }
                if element == "smooth" {
                    node.data["smooth"] = json!(on);
                }
                if element == "axistitles" {
                    node.data["xTitle"] = match &x_title {
                        Some(title) => json!(title),
                        None => Json::Null,
                    };
                    node.data["yTitle"] = match &y_title {
                        Some(title) => json!(title),
                        None => Json::Null,
                    };
                }
            }
            Ok(Outcome::changed(format!("the element `{element}` of the chart `{chart}` is set")).with_data(
                json!({
                    "chart": chart,
                    "element": element,
                    "on": on,
                    "legend": legend,
                }),
            ))

        }
    }
    /// **Take** a chart away.
    grid.chart.remove {
        area: "grid",
        layer: "grid",
        icon: "chart",
        mutates,
        undoable,
        params: {
            /// The chart.

            chart: NodeRef,
        },
        run: {

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
}

/// Whether an **element** of a chart is on: the tree's own default is
/// that the data labels and the smooth line are off, and that the
/// gridlines are on.
pub fn element_on(chart: &Node, key: &str) -> bool {
    match chart.data.get(key) {
        Some(value) => value.as_bool().unwrap_or(false),
        None => key == "gridlines",
    }
}
