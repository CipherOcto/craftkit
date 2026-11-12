//! The **what-if** of a sheet.

use craftkit_sdk::serde_json::json;
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

/// A **workbook** with a **sheet** in it.
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **formula** in a cell.
fn a_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **table** a what-if gives.
fn table(
    session: &mut Session,
    sheet: u64,
    input: &str,
    result: &str,
    values: Vec<f64>,
) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute(
            "grid.whatif",
            &json!({ "sheet": sheet, "input": input, "result": result, "values": values }),
        )
        .expect("the sheet is asked");
    outcome.data["table"].as_array().cloned().expect("the table is a list")
}

/// A what-if says what the sheet would say for each value.
#[test]
fn a_what_if_says_what_the_sheet_would_say() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 2.0);
    a_formula(&mut session, sheet, "B1", "=A1*10");
    let table = table(&mut session, sheet, "A1", "B1", vec![1.0, 2.0, 3.0]);
    assert_eq!(table.len(), 3, "there are three answers");
    assert_eq!(table[0]["value"], json!(1.0), "the first value is one");
    assert_eq!(table[0]["result"], json!(10.0), "and the sheet says ten");
    assert_eq!(table[2]["result"], json!(30.0), "and the last says thirty");
}

/// The sheet is **not changed** by a what-if.
#[test]
fn the_sheet_is_not_changed_by_a_what_if() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 2.0);
    a_formula(&mut session, sheet, "B1", "=A1*10");
    let _ = table(&mut session, sheet, "A1", "B1", vec![1.0, 2.0, 3.0]);
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["value"], json!(2.0), "the input is as it was");
}

/// The **input** and the **result** may not be the same cell.
#[test]
fn the_input_and_the_result_may_not_be_the_same_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 2.0);
    let value = session
        .execute("grid.whatif", &json!({ "sheet": sheet, "input": "A1", "result": "A1", "values": [1.0] }))
        .expect_err("the what-if is refused");
    assert!(value.to_string().contains("same cell"), "the refusal says why, and it says {}", value);
}

/// A cell that is **not a cell** is refused.
#[test]
fn a_cell_that_is_not_a_cell_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let value = session
        .execute("grid.whatif", &json!({ "sheet": sheet, "input": "here", "result": "B1", "values": [1.0] }))
        .expect_err("the what-if is refused");
    assert!(value.to_string().contains("not a cell"), "the refusal says why, and it says {}", value);
}

/// A value that is **not a number** is refused.
#[test]
fn a_value_that_is_not_a_number_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 2.0);
    let value = session
        .execute("grid.whatif", &json!({ "sheet": sheet, "input": "A1", "result": "B1", "values": ["many"] }))
        .expect_err("the what-if is refused");
    assert!(value.to_string().contains("numbers"), "the refusal says why, and it says {}", value);
}

/// A what-if with **no value** is refused.
#[test]
fn a_what_if_with_no_value_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 2.0);
    let value = session
        .execute("grid.whatif", &json!({ "sheet": sheet, "input": "A1", "result": "B1", "values": [] }))
        .expect_err("the what-if is refused");
    assert!(value.to_string().contains("a value or more"), "the refusal says why, and it says {}", value);
}

/// The **result** may be a cell that has nothing in it.
#[test]
fn the_result_may_be_a_cell_that_has_nothing_in_it() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 2.0);
    let table = table(&mut session, sheet, "A1", "B1", vec![1.0]);
    assert_eq!(table.len(), 1, "there is one answer");
    assert_eq!(table[0]["result"], json!(null), "and the empty cell says nothing");
}
