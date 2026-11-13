//! The **forecast** of a line.

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

/// The **numbers** of a set of cells.
fn numbers(session: &mut Session, sheet: u64, cells: &[&str]) {
    for (index, cell) in cells.iter().enumerate() {
        value(session, sheet, cell, json!(index as f64 + 1.0));
    }
}

/// The tree's `FORECAST.LINEAR` is the tree's `FORECAST` under a new
/// name, and it predicts the same way.
#[test]
fn the_forecast_of_a_line_is_the_same_under_either_name() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // The tree's own example: the ys and the xs.
    value(&mut session, sheet, "A1", json!(6.0));
    value(&mut session, sheet, "A2", json!(7.0));
    value(&mut session, sheet, "A3", json!(9.0));
    value(&mut session, sheet, "A4", json!(15.0));
    value(&mut session, sheet, "A5", json!(21.0));
    numbers(&mut session, sheet, &["B1", "B2", "B3", "B4", "B5"]);
    for (index, cell) in ["C1", "C2", "C3", "C4", "C5"].iter().enumerate() {
        value(&mut session, sheet, cell, json!([20.0, 28.0, 31.0, 38.0, 40.0][index]));
    }
    formula(&mut session, sheet, "E1", "=FORECAST.LINEAR(30,A1:A5,C1:C5)");
    let linear = value_of(&mut session, sheet, "E1");
    formula(&mut session, sheet, "E2", "=FORECAST(30,A1:A5,C1:C5)");
    let old = value_of(&mut session, sheet, "E2");
    assert_eq!(linear, old, "the two names are one function");
    let number = linear.as_f64().expect("the forecast is a number");
    assert!((number - 10.607253).abs() < 1e-6, "the forecast is the tree's: {number}");
}

/// A line that does not rise is a **division by nothing**.
#[test]
fn a_line_that_does_not_rise_is_a_division_by_nothing() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for (index, cell) in ["A1", "A2", "A3"].iter().enumerate() {
        value(&mut session, sheet, cell, json!(index as f64 + 1.0));
    }
    for (index, cell) in ["B1", "B2", "B3"].iter().enumerate() {
        value(&mut session, sheet, cell, json!([5.0, 5.0, 5.0][index]));
    }
    formula(&mut session, sheet, "D1", "=FORECAST.LINEAR(10,A1:A3,B1:B3)");
    assert_eq!(
        value_of(&mut session, sheet, "D1")["error"],
        Json::String("#DIV/0!".to_string()),
        "the xs do not move"
    );
}
