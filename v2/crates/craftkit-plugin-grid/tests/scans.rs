//! The **crossings** of a sheet: every place the result goes past the
//! target.

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

/// A **formula** in a cell.
fn a_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// A sheet whose result is **twice** its input.
fn a_sheet(session: &mut Session) -> u64 {
    let (_, sheet) = a_workbook(session);
    a_formula(session, sheet, "A1", "=B1*2");
    sheet
}

/// The **crossings** of a scan.
fn crossings(
    session: &mut Session,
    sheet: u64,
    from: f64,
    to: f64,
    step: f64,
) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute(
            "grid.goal.scan",
            &json!({ "sheet": sheet, "input": "B1", "result": "A1", "target": 10.0, "from": from, "to": to, "step": step }),
        )
        .expect("the sheet is scanned");
    outcome.data["crossings"].as_array().expect("the crossings are a list").clone()
}

/// **The scan finds the place the result goes past the target.**
#[test]
fn the_scan_finds_the_crossing() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let found = crossings(&mut session, sheet, 0.0, 10.0, 1.0);
    assert_eq!(found.len(), 1, "there is one crossing");
    assert_eq!(found[0]["from"], json!(4.0));
    assert_eq!(found[0]["to"], json!(5.0));
    assert_eq!(found[0]["result_from"], json!(8.0));
    assert_eq!(found[0]["result_to"], json!(10.0));
}

/// **A sheet that moves one way and then the other is found.**
#[test]
fn a_sheet_that_turns_around_is_found() {
    let mut session = session();
    let (_, sheet) = a_workbook(&mut session);
    // The result is the square of the input less ten times the input
    // plus twenty-five: it goes down and then up, and it crosses the
    // target twice.
    a_formula(&mut session, sheet, "A1", "=B1*B1-10*B1+25");
    let found = crossings(&mut session, sheet, 0.0, 10.0, 1.0);
    assert_eq!(found.len(), 2, "there are two crossings: {found:?}");
}

/// **A scan that asks the sheet too many times is refused.**
#[test]
fn a_scan_that_is_too_long_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute(
            "grid.goal.scan",
            &json!({
                "sheet": sheet,
                "input": "B1",
                "result": "A1",
                "target": 10.0,
                "from": 0.0,
                "to": 100000.0,
                "step": 0.001,
            }),
        )
        .expect_err("the scan is refused");
    let message = failure.to_string();
    assert!(message.contains("a scan asks it at most"), "the refusal says why: {message}");
}

/// **A step that is not a number above zero is refused.**
#[test]
fn a_step_that_is_not_a_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute(
            "grid.goal.scan",
            &json!({
                "sheet": sheet,
                "input": "B1",
                "result": "A1",
                "target": 10.0,
                "from": 0.0,
                "to": 10.0,
                "step": 0.0,
            }),
        )
        .expect_err("the step is refused");
    let message = failure.to_string();
    assert!(message.contains("a step is a number above zero"), "the refusal says why: {message}");
}

/// **The sheet is not changed by the scan.**
#[test]
fn the_sheet_is_not_changed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    crossings(&mut session, sheet, 0.0, 10.0, 1.0);
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "B1" })).expect("the cell is read");
    assert!(outcome.data["value"].is_null(), "the input cell is as it was: {outcome:?}");
}
