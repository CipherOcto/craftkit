//! The **forecast sheet**: a new sheet that forecasts from a history.

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_grid::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the grid battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A **workbook** with a **sheet** in it, and their ids.
fn a_sheet(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// A **history** of a straight line: `y = 10 + 2t`, for ten points.
fn a_line(session: &mut Session, sheet: u64) {
    for t in 1..=10 {
        value(session, sheet, &format!("A{t}"), json!(t));
        value(session, sheet, &format!("B{t}"), json!(10.0 + 2.0 * f64::from(t)));
    }
}

/// The **cell**: what it holds.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// The **value** a cell holds or computes.
fn computed(session: &mut Session, sheet: u64, cell: &str) -> Json {
    cell_of(session, sheet, cell)["value"].clone()
}

/// The **formula** a cell holds.
fn formula_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    cell_of(session, sheet, cell)["formula"].as_str().unwrap_or_default().to_string()
}

/// The **forecast sheet** of a history, and its id.
fn forecast(session: &mut Session, sheet: u64, extra: Json) -> (u64, Json) {
    let mut params = json!({ "sheet": sheet, "timeline": "A1:A10", "values": "B1:B10" });
    if let (Some(target), Some(source)) = (params.as_object_mut(), extra.as_object()) {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
    let outcome = session.execute("grid.forecast.sheet", &params).expect("the forecast is made");
    let new_sheet = outcome.data["sheet"].as_u64().expect("the sheet has an id");
    (new_sheet, outcome.data.clone())
}

/// The forecast sheet holds the history and the forecast.
#[test]
fn the_forecast_sheet_holds_the_history_and_the_forecast() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let (new_sheet, data) = forecast(&mut session, sheet, json!({ "periods": 3 }));
    assert_eq!(data["name"].as_str(), Some("Forecast"), "the sheet is named for what it is");
    assert_eq!(data["history"].as_u64(), Some(10), "the history is ten points");
    assert_eq!(data["periods"].as_u64(), Some(3), "the forecast is three periods");
    // The history is copied: the timeline in the first column and the
    // values in the second.
    assert_eq!(computed(&mut session, new_sheet, "A2"), json!(1.0), "the timeline is copied");
    assert_eq!(computed(&mut session, new_sheet, "B2"), json!(12.0), "the values are copied");
    // The forecast is a formula over the copied history.
    let text = formula_of(&mut session, new_sheet, "C12");
    assert!(text.contains("FORECAST.ETS"), "the forecast is the model's: {text}");
}

/// The forecast of a straight line is the line.
#[test]
fn the_forecast_of_a_straight_line_is_the_line() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let (new_sheet, _) = forecast(&mut session, sheet, json!({ "periods": 3 }));
    for (row, expected) in [(12, 32.0), (13, 34.0), (14, 36.0)] {
        let value = computed(&mut session, new_sheet, &format!("C{row}"));
        let number = value.as_f64().unwrap_or(f64::NAN);
        assert!(
            (number - expected).abs() < 0.5,
            "the forecast of row {row} is {expected}, and it is {number}"
        );
    }
}

/// The **bounds** of the confidence interval are written when a
/// confidence is asked for.
#[test]
fn the_bounds_are_written_when_a_confidence_is_asked_for() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // A history with a **season** in it, so that the model is not exact
    // and the bounds are around it.
    let season = [3.0, -1.0, 4.0, -6.0];
    for t in 1..=10 {
        value(&mut session, sheet, &format!("A{t}"), json!(t));
        let y = 10.0 + 2.0 * f64::from(t) + season[((t - 1) % 4) as usize];
        value(&mut session, sheet, &format!("B{t}"), json!(y));
    }
    let (new_sheet, _) = forecast(&mut session, sheet, json!({ "periods": 2, "confidence": 0.95 }));
    let text = formula_of(&mut session, new_sheet, "D12");
    assert!(text.contains("FORECAST.ETS.CONFINT"), "the bound is the model's: {text}");
    // The lower bound is under the forecast and the upper one over it.
    let lower = computed(&mut session, new_sheet, "D12").as_f64().unwrap_or(f64::NAN);
    let upper = computed(&mut session, new_sheet, "E12").as_f64().unwrap_or(f64::NAN);
    let middle = computed(&mut session, new_sheet, "C12").as_f64().unwrap_or(f64::NAN);
    assert!(lower < middle && middle < upper, "the bounds are around the forecast: {lower} {middle} {upper}");
}

/// The **statistics** of the model are written when they are asked for.
#[test]
fn the_statistics_are_written_when_they_are_asked_for() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let (new_sheet, _) = forecast(&mut session, sheet, json!({ "periods": 1, "statistics": true }));
    let text = formula_of(&mut session, new_sheet, "F2");
    assert!(text.contains("FORECAST.ETS.STAT"), "the statistic is the model's: {text}");
}

/// A timeline that is **not** a constant step apart is refused: the
/// model has nothing to fit.
#[test]
fn a_timeline_that_is_not_a_step_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for (index, time) in [1.0, 2.0, 3.5, 5.0].iter().enumerate() {
        let row = index + 1;
        value(&mut session, sheet, &format!("A{row}"), json!(time));
        value(&mut session, sheet, &format!("B{row}"), json!(10.0));
    }
    let outcome = session
        .execute("grid.forecast.sheet", &json!({ "sheet": sheet, "timeline": "A1:A4", "values": "B1:B4" }))
        .expect_err("a timeline that is not a step is refused");
    let message = outcome.to_string();
    assert!(message.contains("constant step"), "the refusal says why: {message}");
}

/// A **header** in the first cell is a header, and not part of the
/// history.
#[test]
fn a_text_first_cell_is_a_header() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("Month"));
    value(&mut session, sheet, "B1", json!("Sales"));
    for t in 1..=6 {
        let row = t + 1;
        value(&mut session, sheet, &format!("A{row}"), json!(t));
        value(&mut session, sheet, &format!("B{row}"), json!(10.0 + 2.0 * f64::from(t)));
    }
    let outcome = session
        .execute(
            "grid.forecast.sheet",
            &json!({ "sheet": sheet, "timeline": "A1:A7", "values": "B1:B7", "periods": 2 }),
        )
        .expect("the forecast is made");
    let data = outcome.data.clone();
    assert_eq!(data["history"].as_u64(), Some(6), "the header is not part of the history");
    let new_sheet = data["sheet"].as_u64().expect("the sheet has an id");
    let header = cell_of(&mut session, new_sheet, "A1");
    assert_eq!(header["value"].as_str(), Some("Month"), "the header names the column");
}

/// The forecast runs **to an end**: the periods are worked out from the
/// steps of the timeline.
#[test]
fn the_forecast_runs_to_an_end() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let (new_sheet, data) = forecast(&mut session, sheet, json!({ "end": "13" }));
    assert_eq!(data["periods"].as_u64(), Some(3), "the forecast runs to the thirteenth point");
    // The last point of the forecast is the end.
    let value = computed(&mut session, new_sheet, "A14");
    assert_eq!(value, json!(13.0), "the forecast ends at the point that was named");
}

/// The forecast may start **before the last point**: the model is fit
/// to the points before it and the forecast runs over the actuals.
#[test]
fn the_forecast_may_start_before_the_last_point() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let (new_sheet, _) = forecast(&mut session, sheet, json!({ "start": "6", "periods": 2 }));
    // The model is fit to the points before the sixth.
    let text = formula_of(&mut session, new_sheet, "C7");
    assert!(text.contains("$A$2:$A$6"), "the model is fit to the points before the start: {text}");
    // And the forecast runs over the actuals from there: the sixth
    // point is forecast from the five before it, and the seventh from
    // the same model.
    for (cell, expected) in [("C7", 22.0), ("C8", 24.0)] {
        let value = computed(&mut session, new_sheet, cell);
        let number = value.as_f64().unwrap_or(f64::NAN);
        assert!((number - expected).abs() < 1.0, "the forecast in {cell} is {expected}, and it is {number}");
    }
}

/// A start that is **not a point** of the timeline is refused.
#[test]
fn a_start_that_is_not_a_point_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let outcome = session
        .execute(
            "grid.forecast.sheet",
            &json!({ "sheet": sheet, "timeline": "A1:A10", "values": "B1:B10", "start": "4.5" }),
        )
        .expect_err("a start that is not a point is refused");
    let message = outcome.to_string();
    assert!(message.contains("not a point"), "the refusal says why: {message}");
}

/// An end **before** the last point of the timeline is refused.
#[test]
fn an_end_before_the_last_point_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let outcome = session
        .execute(
            "grid.forecast.sheet",
            &json!({ "sheet": sheet, "timeline": "A1:A10", "values": "B1:B10", "end": "5" }),
        )
        .expect_err("an end before the last point is refused");
    let message = outcome.to_string();
    assert!(message.contains("after the last point"), "the refusal says why: {message}");
}

#[test]
fn the_forecast_sheet_is_a_table() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let (forecast_sheet, data) = forecast(&mut session, sheet, json!({}));
    assert_eq!(data["table"], "Forecast");
    // The table reaches over the whole of the forecast sheet.
    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": forecast_sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert_eq!(tables.len(), 1);
    assert_eq!(tables[0]["name"], "Forecast");
    assert_eq!(tables[0]["range"], "A1:C14");
}

#[test]
fn the_forecast_sheet_may_hold_a_chart() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let (_forecast_sheet, data) = forecast(&mut session, sheet, json!({ "chart": "column" }));
    let chart = data["chart"].as_u64().expect("the forecast has a chart");
    let outcome = session.execute("grid.chart.read", &json!({ "chart": chart })).expect("the chart is read");
    assert_eq!(outcome.data["kind"], "column");
    assert_eq!(outcome.data["range"], "A1:C14");
}

#[test]
fn the_forecast_sheet_may_have_no_table() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let (forecast_sheet, data) = forecast(&mut session, sheet, json!({ "table": false }));
    assert_eq!(data["table"], json!(null));
    let outcome =
        session.execute("grid.table.read", &json!({ "sheet": forecast_sheet })).expect("the tables are read");
    let tables = outcome.data["tables"].as_array().expect("the tables are a list");
    assert!(tables.is_empty(), "the sheet has no table");
}

#[test]
fn the_chart_of_the_forecast_sheet_is_line_or_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_line(&mut session, sheet);
    let outcome = session.execute(
        "grid.forecast.sheet",
        &json!({
            "sheet": sheet,
            "timeline": "A1:A10",
            "values": "B1:B10",
            "chart": "pie",
        }),
    );
    let error = outcome.expect_err("the forecast is refused").to_string();
    assert!(error.contains("line"), "the error names the kinds: {error}");
}
