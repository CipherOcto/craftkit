//! **Reading** a range of cells.

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

/// The **values** of a read, as the answer holds them.
fn values_of(session: &mut Session, sheet: u64, range: Option<&str>) -> Vec<Vec<Json>> {
    let mut params = json!({ "sheet": sheet });
    if let Some(range) = range {
        params["range"] = json!(range);
    }
    let outcome = session.execute("grid.range.read", &params).expect("the range is read");
    outcome.data["values"]
        .as_array()
        .expect("the values are a list")
        .iter()
        .map(|row| row.as_array().expect("a row is a list").clone())
        .collect()
}

/// A range of cells is **read**: the values, row by row.
#[test]
fn a_range_of_cells_is_read() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 7 }))
        .expect("the value is set");
    let values = values_of(&mut session, sheet, Some("A1:B2"));
    assert_eq!(values.len(), 2, "two rows");
    assert_eq!(values[0].len(), 2, "and two cells in the first");
    assert_eq!(values[0][0].as_str(), Some("red"), "the value is there");
    assert_eq!(values[0][1].as_f64(), Some(7.0), "and the next one");
    assert!(values[1][0].is_null(), "and an empty cell is nothing");
}

/// The value of a **formula** cell is computed.
#[test]
fn the_value_of_a_formula_cell_is_computed() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 2 }))
        .expect("the value is set");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=A1*3" }))
        .expect("the formula is set");
    let values = values_of(&mut session, sheet, Some("A1:B1"));
    assert_eq!(values[0][1].as_f64(), Some(6.0), "the formula is computed");
}

/// The **formulas** may be asked for instead.
#[test]
fn the_formulas_may_be_asked_for_instead() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=1+1" }))
        .expect("the formula is set");
    let outcome = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": "A1:A1", "formulas": true }))
        .expect("the range is read");
    let values = outcome.data["values"].as_array().expect("the values are a list");
    assert_eq!(values[0][0].as_str(), Some("=1+1"), "the formula is given");
}

/// Without a range the **used range** is read.
#[test]
fn without_a_range_the_used_range_is_read() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "C3", "value": "here" }))
        .expect("the value is set");
    let outcome = session.execute("grid.range.read", &json!({ "sheet": sheet })).expect("the range is read");
    assert_eq!(outcome.data["range"].as_str(), Some("C3"), "the used range is said");
    let values = outcome.data["values"].as_array().expect("the values are a list");
    assert_eq!(values.len(), 1, "one row");
}

/// A sheet that holds **nothing** has no used range.
#[test]
fn a_sheet_that_holds_nothing_has_no_used_range() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome = session.execute("grid.range.read", &json!({ "sheet": sheet })).expect("the range is read");
    let values = outcome.data["values"].as_array().expect("the values are a list");
    assert!(values.is_empty(), "and nothing is read");
}

/// A range that is **not a range** is refused.
#[test]
fn a_range_that_is_not_a_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": "A1:B2:C3" }))
        .expect_err("the reading is refused");
    let message = error.to_string();
    assert!(message.contains("not a range"), "it says why: {message}");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.range.read", &json!({ "sheet": workbook }))
        .expect_err("the reading is refused");
    let message = error.to_string();
    assert!(message.contains("not a sheet"), "it says why: {message}");
    let error =
        session.execute("grid.range.read", &json!({ "sheet": 9999 })).expect_err("the reading is refused");
    let message = error.to_string();
    assert!(message.contains("there is no sheet"), "and there is none: {message}");
}
