//! The **forecast sheet**: a new sheet that forecasts from a history.
//!
//! A history is a **timeline** — the times of the points — and the
//! **values** at those times. A forecast sheet holds a copy of the
//! history and, beside it, the **forecast**: the value of a model of
//! the history at each point after it, and the **bounds** of the
//! confidence interval around it.
//!
//! The forecast is **live**: the sheet holds formulas over the copied
//! history, so an edit of the history is an edit of the forecast. The
//! model is the tree's own — `FORECAST.ETS` and `FORECAST.ETS.CONFINT`
//! (§ the ETS module) — and the sheet says so in its formulas.
//!
//! The rules:
//!
//! - the timeline and the values are each **one row or one column**,
//!   and they have the **same number of cells**;
//! - a **text** first cell is a **header**: it names the column and is
//!   not part of the history;
//! - the timeline holds **numbers** — a date is a number — and the
//!   points are **sorted** by it;
//! - the timeline must have **two points a constant step apart**, or
//!   the model has nothing to fit;
//! - the forecast runs **after** the history, and its points are the
//!   steps of the timeline.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Text};

use crate::formula::{Area, CellRef, Value};

/// The **most** rows of history a forecast may be made from.
const MAX_HISTORY: usize = 10_000;

/// The **most** periods a forecast may run for.
const MAX_PERIODS: usize = 10_000;

/// The **most** work a forecast may do: the history times the periods.
///
/// Every forecast row holds formulas that refit the model over the
/// whole history, so this bounds the work of one sheet.
const MAX_WORK: u64 = 500_000;

/// The **cells** of a range that is one row or one column.
fn line_cells(area: &Area, id: &str) -> Result<Vec<CellRef>, EngineError> {
    if area.start.col == area.end.col {
        return Ok((area.start.row..=area.end.row).map(|row| CellRef { col: area.start.col, row }).collect());
    }
    if area.start.row == area.end.row {
        return Ok((area.start.col..=area.end.col).map(|col| CellRef { col, row: area.start.row }).collect());
    }
    Err(EngineError::Failed {
        id: id.to_string(),
        message: "the timeline and the values are each one row or one column".to_string(),
    })
}

/// The **most** periods for that much history.
fn max_periods(history: usize) -> u32 {
    let rows = u64::try_from(history).unwrap_or(u64::MAX).max(1);
    u32::try_from(MAX_WORK / rows).unwrap_or(u32::MAX).clamp(1, MAX_PERIODS as u32)
}

/// The **periods** a forecast runs for, by default: a third of the
/// history.
fn default_periods(history: usize) -> u32 {
    u32::try_from(history.div_ceil(3)).unwrap_or(u32::MAX).clamp(1, max_periods(history))
}

/// A **time** a caller gives: a number, or a date.
fn a_time(text: &str, system: crate::dates::System) -> Option<f64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(number) = text.parse::<f64>() {
        return number.is_finite().then_some(number);
    }
    let (date, _) = crate::dates::parse(text, system)?;
    crate::dates::to_serial(date.year, date.month, date.day, system)
}

/// Whether two points of a timeline are the **same** point.
fn same_point(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9 * left.abs().max(right.abs()).max(1.0)
}

/// The **name** of the forecast sheet: the first one that is free.
fn a_free_name(taken: &[String]) -> String {
    for number in 1..1000 {
        let name = if number == 1 { "Forecast".to_string() } else { format!("Forecast{number}") };
        if !taken.iter().any(|other| other.eq_ignore_ascii_case(&name)) {
            return name;
        }
    }
    "Forecast".to_string()
}

commands! {
    /// **Forecast** from a history, into a **new sheet**.
    ///
    /// The new sheet holds a copy of the history and, beside it, the
    /// forecast: the value of the model at each point after the
    /// history, and the bounds of the confidence interval around it.
    ///
    /// The forecast is **live**: the sheet holds formulas over the
    /// copied history, so an edit of the history is an edit of the
    /// forecast.
    grid.forecast.sheet {
        area: "grid",
        layer: "grid",
        icon: "chart",
        mutates,
        undoable,
        params: {
            /// The sheet the history is on.
            sheet: NodeRef,
            /// The range of the **timeline**: the times of the points.
            /// One row or one column.
            timeline: Text,
            /// The range of the **values**: what was measured at those
            /// times. One row or one column.
            values: Text,
            /// How many **periods** to forecast. Without it, a third of
            /// the history.
            periods: Option<Number>,
            /// The **confidence** level: `0.95` or `95`. Without it,
            /// there are no bounds.
            confidence: Option<Number>,
            /// The length of the **season**: `0` (none) or a number
            /// from 2 to 8760. Without it, the model finds the season.
            seasonality: Option<Number>,
            /// How the **gaps** are filled: `interpolate` or `zeros`.
            fill: Option<Text>,
            /// How the points of the **same time** are gathered:
            /// `average`, `count`, `counta`, `max`, `median`, `min` or
            /// `sum`.
            aggregate: Option<Text>,
            /// Whether the **statistics** of the model are written.
            statistics: Option<Bool>,
            /// Where the forecast **starts**: a point of the timeline, as a
            /// number or as a date, before its last point. The model is
            /// then fit to the points before it and the forecast runs
            /// over the actuals from there. Without it, the forecast
            /// runs after the history.
            start: Option<Text>,
            /// Where the forecast **ends**: a point of the timeline, as a
            /// number or as a date, after its last point. The forecast
            /// runs that far.
            end: Option<Text>,
        },
        run: {
            let id = "grid.forecast.sheet".to_string();
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
                    message: format!("the node `{sheet}` is not a sheet"),
                });
            }
            // The **ranges**: the timeline and the values.
            let timeline_area = crate::fill::area_at(&self.timeline.0, &id)?;
            let values_area = crate::fill::area_at(&self.values.0, &id)?;
            let timeline_cells = line_cells(&timeline_area, &id)?;
            let value_cells = line_cells(&values_area, &id)?;
            if timeline_cells.len() != value_cells.len() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the timeline and the values need the same number of cells".to_string(),
                });
            }
            if timeline_cells.len() > MAX_HISTORY + 1 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("at most {MAX_HISTORY} rows of history"),
                });
            }
            let cells = crate::cells_of(&node);
            // A **text** first cell is the header: it names the column.
            let text_of = |cell: CellRef| -> Option<String> {
                match cells.get(cell).and_then(|entry| entry.value.clone()) {
                    Some(Value::Text(text)) => Some(text),
                    _ => None,
                }
            };
            let header = text_of(timeline_cells[0]).is_some() || text_of(value_cells[0]).is_some();
            let name_of = |cell: CellRef, fallback: &str| -> String {
                match text_of(cell) {
                    Some(text) if header && !text.is_empty() => text,
                    _ => fallback.to_string(),
                }
            };
            let (timeline_name, values_name) =
                (name_of(timeline_cells[0], "Timeline"), name_of(value_cells[0], "Values"));
            let skip = usize::from(header);
            // The **history**: the points, sorted by the timeline.
            let mut rows: Vec<(f64, Option<f64>)> = Vec::new();
            for (timeline_cell, value_cell) in timeline_cells.iter().zip(&value_cells).skip(skip) {
                let time = match cells.get(*timeline_cell).and_then(|entry| entry.value.clone()) {
                    Some(Value::Number(number)) => number,
                    Some(Value::Empty) | None => continue,
                    Some(_) => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the timeline must hold numbers or dates ({} isn't one)",
                                timeline_cell.to_text()
                            ),
                        });
                    }
                };
                let value = match cells.get(*value_cell).and_then(|entry| entry.value.clone()) {
                    Some(Value::Number(number)) => Some(number),
                    _ => None,
                };
                rows.push((time, value));
            }
            if rows.len() < 2 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the timeline needs at least two points a constant step apart".to_string(),
                });
            }
            if !rows.iter().any(|(_, value)| value.is_some()) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "there are no values to forecast".to_string(),
                });
            }
            rows.sort_by(|left, right| left.0.total_cmp(&right.0));
            let times: Vec<f64> = rows.iter().map(|row| row.0).collect();
            let system = crate::system_of(session, sheet);
            let Some(axis) = crate::ets::detect_axis(system, &times) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "the timeline needs at least two points a constant step apart".to_string(),
                });
            };
            // The **options** of the model.
            let seasonality = match self.seasonality {
                None => 1,
                Some(number) => {
                    let value = number.0;
                    if value < 0.0 || value > 8760.0 || (value != 0.0 && value < 2.0) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the season is 0 (none) or a number from 2 to 8760".to_string(),
                        });
                    }
                    value.trunc() as u32
                }
            };
            let fill = match self.fill.as_ref().map(|fill| fill.0.trim().to_ascii_lowercase()) {
                None => 1,
                Some(ref text) if text.is_empty() => 1,
                Some(ref text) if text == "interpolate" || text == "interpolation" => 1,
                Some(ref text) if text == "zeros" || text == "zero" => 0,
                Some(ref text) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the gaps are filled with `interpolate` or `zeros`, not `{text}`"),
                    });
                }
            };
            let aggregate = match self.aggregate.as_ref().map(|text| text.0.trim().to_ascii_lowercase()) {
                None => None,
                Some(ref text) if text.is_empty() || text == "average" => None,
                Some(ref text) => match text.as_str() {
                    "count" => Some(2),
                    "counta" => Some(3),
                    "max" => Some(4),
                    "median" => Some(5),
                    "min" => Some(6),
                    "sum" => Some(7),
                    _ => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the points of the same time are gathered by `average`, `count`, `counta`, `max`, `median`, `min` or `sum`, not `{text}`"),
                        });
                    }
                },
            };
            let confidence = match self.confidence {
                None => None,
                Some(number) => {
                    let value = number.0;
                    let value = if value >= 1.0 { value / 100.0 } else { value };
                    if !(value > 0.0 && value < 1.0) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the confidence is between 0 and 1 (or 0 and 100%)".to_string(),
                        });
                    }
                    Some(value)
                }
            };
            // The **last** point of the history.
            let last = times.last().copied().unwrap_or(f64::NAN);
            // Where the forecast **starts**: the index of the first
            // forecast point, and the points before it are the ones the
            // model is fit to.
            let fit = match self.start.as_ref().map(|text| a_time(&text.0, system)) {
                None => rows.len(),
                Some(None) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the start of the forecast is not a number or a date".to_string(),
                    });
                }
                Some(Some(start)) => {
                    let Some(index) = times.iter().position(|time| same_point(*time, start)) else {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the start of the forecast is not a point of the timeline".to_string(),
                        });
                    };
                    if index + 1 >= times.len() {
                        // The last point of the history: the forecast
                        // runs after the history.
                        rows.len()
                    } else {
                        let values =
                            rows.iter().take(index).filter(|(_, value)| value.is_some()).count();
                        if values < 2 {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: "the start of the forecast needs at least two values before it to fit the model".to_string(),
                            });
                        }
                        index
                    }
                }
            };
            // The **periods**: how far the forecast runs.
            let cap = max_periods(rows.len());
            let periods = match self.end.as_ref().map(|text| a_time(&text.0, system)) {
                None => match self.periods {
                    Some(number) => {
                        let value = number.0;
                        if !value.is_finite() || value < 1.0 || value.ceil() > f64::from(cap) {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("the periods are from 1 to {cap} for {} rows of history", rows.len()),
                            });
                        }
                        value.ceil() as u32
                    }
                    None => default_periods(rows.len()),
                },
                Some(None) => {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the end of the forecast is not a number or a date".to_string(),
                    });
                }
                Some(Some(end)) => {
                    let steps = axis.steps_between(system, last, end).ok_or_else(|| EngineError::Failed {
                        id: id.clone(),
                        message: "the end of the forecast is not on the timeline".to_string(),
                    })?;
                    if steps < 1.0 {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: "the end of the forecast must be after the last point of the timeline".to_string(),
                        });
                    }
                    let steps = steps.ceil();
                    if steps > f64::from(cap) {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("at most {cap} forecast steps for this much history"),
                        });
                    }
                    steps as u32
                }
            };
            // The **future**: the points after the last one.
            let mut future: Vec<f64> = Vec::with_capacity(periods as usize);
            for step in 1..=periods {
                let Some(point) = axis.after_last(system, last, step) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: "the forecast runs past the last point of the timeline".to_string(),
                    });
                };
                future.push(point);
            }
            // The **sheet**: a new one, named for what it is.
            let workbook = session
                .document()
                .walk()
                .find(|node| node.children.iter().any(|child| child.id == sheet))
                .map(|node| node.id);
            let Some(workbook) = workbook else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{sheet}` is not in a workbook"),
                });
            };
            let taken: Vec<String> = session
                .document()
                .node(workbook)
                .map(|node| {
                    node.children
                        .iter()
                        .filter(|child| child.kind == "grid.sheet")
                        .map(|child| child.name.clone())
                        .collect()
                })
                .unwrap_or_default();
            let name = a_free_name(&taken);
            // The **columns** of the sheet: the timeline, the values, the
            // forecast and the bounds.
            let mut headers = vec![timeline_name.clone(), values_name.clone()];
            headers.push(format!("Forecast({values_name})"));
            if confidence.is_some() {
                headers.push(format!("Lower Confidence Bound({values_name})"));
                headers.push(format!("Upper Confidence Bound({values_name})"));
            }
            let width = headers.len() as u32;
            // The **formulas**: the model over the copied history.
            let history = rows.len() as u32;
            // The **last point the model is fit to**: the point before
            // the forecast starts.
            let fit_last = fit as u32 + 1;
            let mut tail = format!("{seasonality},{fill}");
            if let Some(aggregate) = aggregate {
                tail.push_str(&format!(",{aggregate}"));
            }
            let timeline_range = format!("$A$2:$A${fit_last}");
            let values_range = format!("$B$2:$B${fit_last}");
            let forecast_of = |row: u32| -> String {
                format!("FORECAST.ETS(A{row},{values_range},{timeline_range},{tail})")
            };
            let interval_of = |row: u32, level: f64| -> String {
                format!(
                    "FORECAST.ETS.CONFINT(A{row},{values_range},{timeline_range},{level},{tail})"
                )
            };
            // The **new sheet**.
            let new_sheet = session.document_mut()?.add_node_with_under(
                Some(workbook),
                name.clone(),
                "grid.sheet",
                None,
                json!({
                    "cells": {},
                    "view": crate::view::SheetView::default().to_json(),
                    "zoom": 100,
                }),
            );
            let mut new_cells = crate::formula::Cells::new();
            for (column, header) in headers.iter().enumerate() {
                new_cells.set_value(
                    CellRef { col: column as u32, row: 0 },
                    Value::Text(header.clone()),
                );
            }
            // The **formats** of the history, carried over.
            let timeline_format = cells.format(timeline_cells.get(skip).copied().unwrap_or(timeline_cells[0]))
                .map(|format| format.to_string());
            let values_format = cells.format(value_cells.get(skip).copied().unwrap_or(value_cells[0]))
                .map(|format| format.to_string());
            for (index, (time, value)) in rows.iter().enumerate() {
                let row = index as u32 + 1;
                new_cells.set_value(CellRef { col: 0, row }, Value::Number(*time));
                new_cells.set_format(CellRef { col: 0, row }, timeline_format.clone());
                if let Some(value) = value {
                    new_cells.set_value(CellRef { col: 1, row }, Value::Number(*value));
                    new_cells.set_format(CellRef { col: 1, row }, values_format.clone());
                }
            }
            // The **join**: the forecast starts at the last point the
            // model is fit to, so the two lines meet.
            let join = fit_last - 1;
            new_cells.set_formula(CellRef { col: 2, row: join }, format!("=B{fit_last}"));
            new_cells.set_format(CellRef { col: 2, row: join }, values_format.clone());
            if confidence.is_some() {
                new_cells.set_formula(CellRef { col: 3, row: join }, format!("=C{fit_last}"));
                new_cells.set_formula(CellRef { col: 4, row: join }, format!("=C{fit_last}"));
                new_cells.set_format(CellRef { col: 3, row: join }, values_format.clone());
                new_cells.set_format(CellRef { col: 4, row: join }, values_format.clone());
            }
            // The **forecast**: a row for each point of the forecast —
            // the actuals that follow the start, and then the points
            // after the history. The rows of the cells are counted from
            // zero and the rows of the formulas from one.
            for row in (fit as u32 + 1)..=(history + periods) {
                let number = row + 1;
                if row > history {
                    let Some(point) = future.get((row - history - 1) as usize) else { continue };
                    new_cells.set_value(CellRef { col: 0, row }, Value::Number(*point));
                    new_cells.set_format(CellRef { col: 0, row }, timeline_format.clone());
                }
                new_cells.set_formula(CellRef { col: 2, row }, format!("={}", forecast_of(number)));
                new_cells.set_format(CellRef { col: 2, row }, values_format.clone());
                if let Some(level) = confidence {
                    let interval = interval_of(number, level);
                    new_cells.set_formula(CellRef { col: 3, row }, format!("=C{number}-{interval}"));
                    new_cells.set_formula(CellRef { col: 4, row }, format!("=C{number}+{interval}"));
                    new_cells.set_format(CellRef { col: 3, row }, values_format.clone());
                    new_cells.set_format(CellRef { col: 4, row }, values_format.clone());
                }
            }
            // The **statistics** of the model, when they are asked for.
            if self.statistics.map(|statistics| statistics.0).unwrap_or(false) {
                let column = width + 1;
                new_cells.set_value(CellRef { col: column, row: 0 }, Value::Text("Statistic".to_string()));
                new_cells.set_value(CellRef { col: column + 1, row: 0 }, Value::Text("Value".to_string()));
                for (index, label) in
                    ["Alpha", "Beta", "Gamma", "MASE", "SMAPE", "MAE", "RMSE"].iter().enumerate()
                {
                    let row = index as u32 + 1;
                    new_cells.set_value(CellRef { col: column, row }, Value::Text((*label).to_string()));
                    new_cells.set_formula(
                        CellRef { col: column + 1, row },
                        format!("=FORECAST.ETS.STAT({values_range},{timeline_range},{},{tail})", index + 1),
                    );
                }
            }
            let json_cells = crate::cells_into(&new_cells);
            if let Some(node) = session.document_mut()?.node_mut(new_sheet) {
                node.data["cells"] = json_cells;
            }
            Ok(Outcome::changed(format!("forecast {} rows into the sheet `{name}`", history)).with_data(json!({
                "sheet": new_sheet,
                "name": name,
                "history": history,
                "periods": periods,
                "columns": width,
            })))

        }
    }
}
