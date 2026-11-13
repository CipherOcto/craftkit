//! The **model of a series in time**.

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

/// A **workbook** with a sheet in it, and their ids.
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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// The **number** a formula computes.
fn number_of(session: &mut Session, sheet: u64, cell: &str) -> f64 {
    let value = value_of(session, sheet, cell);
    value.as_f64().unwrap_or_else(|| panic!("{cell} is a number: {value}"))
}

/// A series of **values** down a column, and its timeline beside it.
fn series(session: &mut Session, sheet: u64, ys: &[f64], ts: &[f64]) {
    for (index, value) in ys.iter().enumerate() {
        value_cell(session, sheet, &format!("A{}", index + 1), *value);
        value_cell(session, sheet, &format!("B{}", index + 1), ts[index]);
    }
}

/// Put a **value** in a cell.
fn value_cell(session: &mut Session, sheet: u64, cell: &str, number: f64) {
    value(session, sheet, cell, json!(number));
}

/// A **straight line**: `y = 3 + 2t` over twenty points.
fn straight_line(session: &mut Session, sheet: u64) {
    let ys: Vec<f64> = (1..=20).map(|t| 3.0 + 2.0 * t as f64).collect();
    let ts: Vec<f64> = (1..=20).map(|t| t as f64).collect();
    series(session, sheet, &ys, &ts);
}

/// A **seasonal** series: a straight line and a four-step season.
fn seasonal(session: &mut Session, sheet: u64) {
    let pattern = [4.0, -2.0, 7.0, -9.0];
    let ys: Vec<f64> = (1..=24).map(|t| 3.0 + 2.0 * t as f64 + pattern[(t - 1) % 4]).collect();
    let ts: Vec<f64> = (1..=24).map(|t| t as f64).collect();
    series(session, sheet, &ys, &ts);
}

/// A straight line is forecast **exactly**.
#[test]
fn a_straight_line_is_forecast_exactly() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    straight_line(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=FORECAST.ETS(25,A1:A20,B1:B20)");
    assert!((number_of(&mut session, sheet, "D1") - 53.0).abs() < 1e-9, "the forecast");
    formula(&mut session, sheet, "D2", "=FORECAST.ETS(20.5,A1:A20,B1:B20,0)");
    assert!((number_of(&mut session, sheet, "D2") - 44.0).abs() < 1e-9, "half a step");
    formula(&mut session, sheet, "D3", "=FORECAST.ETS.SEASONALITY(A1:A20,B1:B20)");
    assert_eq!(value_of(&mut session, sheet, "D3"), json!(0.0), "no season");
    formula(&mut session, sheet, "D4", "=FORECAST.ETS.CONFINT(25,A1:A20,B1:B20)");
    assert!(number_of(&mut session, sheet, "D4").abs() < 1e-9, "no error at all");
    formula(&mut session, sheet, "D5", "=FORECAST.ETS.STAT(A1:A20,B1:B20,8)");
    assert_eq!(value_of(&mut session, sheet, "D5"), json!(1.0), "the step of the timeline");
    formula(&mut session, sheet, "D6", "=FORECAST.ETS.STAT(A1:A20,B1:B20,6)");
    assert!(number_of(&mut session, sheet, "D6").abs() < 1e-9, "no error at all");
}

/// The **season** of a series is found, and the model continues it.
#[test]
fn the_season_of_a_series_is_found_and_the_model_continues_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    seasonal(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=FORECAST.ETS.SEASONALITY(A1:A24,B1:B24)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(4.0), "a season of four");
    formula(&mut session, sheet, "D2", "=FORECAST.ETS(27,A1:A24,B1:B24)");
    assert!((number_of(&mut session, sheet, "D2") - 64.0).abs() < 1e-7, "the third place of the pattern");
    formula(&mut session, sheet, "D3", "=FORECAST.ETS(28,A1:A24,B1:B24,4)");
    assert!((number_of(&mut session, sheet, "D3") - 50.0).abs() < 1e-7, "a season of four, said so");
}

/// The **order** of the timeline does not matter.
#[test]
fn the_order_of_the_timeline_does_not_matter() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let pattern = [4.0, -2.0, 7.0, -9.0];
    let ys: Vec<f64> = (1..=24).map(|t| 3.0 + 2.0 * t as f64 + pattern[(t - 1) % 4]).collect();
    let ts: Vec<f64> = (1..=24).map(|t| t as f64).collect();
    let mut reversed_y = ys.clone();
    let mut reversed_t = ts.clone();
    reversed_y.reverse();
    reversed_t.reverse();
    series(&mut session, sheet, &reversed_y, &reversed_t);
    formula(&mut session, sheet, "D1", "=FORECAST.ETS(27,A1:A24,B1:B24)");
    assert!((number_of(&mut session, sheet, "D1") - 64.0).abs() < 1e-7, "the same forecast");
}

/// The number of **days** in a month of a year.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        _ => 0,
    }
}

/// The **months** of a calendar are a timeline too, and a point that is
/// missing is filled in, and two points of the same time stamp are
/// **aggregated**.
#[test]
fn the_months_of_a_calendar_are_a_timeline_too() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // The first of each month, from January 2024 to December 2025.
    let mut times: Vec<f64> = Vec::new();
    let mut serial = 45292.0;
    for index in 0..24 {
        times.push(serial);
        let (year, month) = (2024 + index / 12, (index % 12) as u32 + 1);
        serial += f64::from(days_in_month(year, month));
    }
    for (index, &time) in times.iter().enumerate() {
        value_cell(&mut session, sheet, &format!("B{}", index + 1), time);
        value_cell(&mut session, sheet, &format!("A{}", index + 1), 100.0 + 5.0 * index as f64);
    }
    formula(&mut session, sheet, "D1", "=FORECAST.ETS(46023,A1:A24,B1:B24,0)");
    assert!((number_of(&mut session, sheet, "D1") - 220.0).abs() < 1e-9, "the first of January 2026");
    formula(&mut session, sheet, "D2", "=FORECAST.ETS(46082,A1:A24,B1:B24,0)");
    assert!((number_of(&mut session, sheet, "D2") - 230.0).abs() < 1e-9, "the first of March 2026");
}

/// A point that is **missing** is filled in — or filled with zeros, when
/// the completion says so.
#[test]
fn a_missing_point_is_filled_in() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let times = [1.0, 2.0, 4.0, 5.0, 6.0, 8.0, 9.0, 10.0];
    for (index, &time) in times.iter().enumerate() {
        value_cell(&mut session, sheet, &format!("B{}", index + 1), time);
        value_cell(&mut session, sheet, &format!("A{}", index + 1), 10.0 * time);
    }
    formula(&mut session, sheet, "D1", "=FORECAST.ETS(12,A1:A8,B1:B8,0)");
    assert!((number_of(&mut session, sheet, "D1") - 120.0).abs() < 1e-9, "the gap is filled in");
    formula(&mut session, sheet, "D2", "=FORECAST.ETS(12,A1:A8,B1:B8,0,0)");
    assert!((number_of(&mut session, sheet, "D2") - 120.0).abs() > 1e-9, "the gap is filled with zeros");
}

/// Two points of the same time stamp are **averaged** — or summed, when
/// the aggregation says so.
#[test]
fn two_points_of_one_time_stamp_are_aggregated() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let times = [1.0, 2.0, 2.0, 3.0, 4.0, 5.0];
    let values = [2.0, 3.0, 5.0, 6.0, 8.0, 10.0];
    for (index, (&time, &number)) in times.iter().zip(values.iter()).enumerate() {
        value_cell(&mut session, sheet, &format!("B{}", index + 1), time);
        value_cell(&mut session, sheet, &format!("A{}", index + 1), number);
    }
    formula(&mut session, sheet, "D1", "=FORECAST.ETS(6,A1:A6,B1:B6,0)");
    assert!((number_of(&mut session, sheet, "D1") - 12.0).abs() < 1e-9, "the two points are averaged");
    formula(&mut session, sheet, "D2", "=FORECAST.ETS.STAT(A1:A6,B1:B6,8)");
    assert!((number_of(&mut session, sheet, "D2") - 1.0).abs() < 1e-12, "the step of the timeline");
    formula(&mut session, sheet, "D3", "=FORECAST.ETS(6,A1:A6,B1:B6,0,1,7)");
    assert!((number_of(&mut session, sheet, "D3") - 12.0).abs() > 1e-9, "the two points are summed");
}

/// The two sets of numbers must be of the **same length**.
#[test]
fn the_two_sets_of_numbers_must_be_of_the_same_length() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    straight_line(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=FORECAST.ETS(25,A1:A20,B1:B10)");
    assert_eq!(
        value_of(&mut session, sheet, "D1")["error"],
        Json::String("#N/A".to_string()),
        "not the same length"
    );
}

/// A value that is not a number is **not a value**.
#[test]
fn a_value_that_is_not_a_number_is_not_a_value() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    straight_line(&mut session, sheet);
    value(&mut session, sheet, "A5", json!("a word"));
    formula(&mut session, sheet, "D1", "=FORECAST.ETS(25,A1:A20,B1:B20)");
    assert_eq!(
        value_of(&mut session, sheet, "D1")["error"],
        Json::String("#VALUE!".to_string()),
        "a word is not a number"
    );
}
