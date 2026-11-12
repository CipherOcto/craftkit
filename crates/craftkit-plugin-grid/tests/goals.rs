//! The **goal** of a sheet: the input that gives a result.

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

/// The **goal** a sheet reaches.
fn goal(session: &mut Session, sheet: u64, target: f64, from: f64, to: f64) -> f64 {
    let outcome = session
        .execute(
            "grid.goal",
            &json!({ "sheet": sheet, "input": "A1", "result": "B1", "target": target, "from": from, "to": to }),
        )
        .expect("the goal is found");
    outcome.data["value"].as_f64().expect("the goal is a number")
}

/// The goal is the input that gives the **result**.
#[test]
fn the_goal_is_the_input_that_gives_the_result() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    a_formula(&mut session, sheet, "B1", "=A1*2");
    let found = goal(&mut session, sheet, 10.0, 0.0, 100.0);
    assert!((found - 5.0).abs() < 1e-6, "the goal is five, and it says {}", found);
}

/// The sheet is **not changed** by a goal seek.
#[test]
fn the_sheet_is_not_changed_by_a_goal_seek() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 7.0);
    a_formula(&mut session, sheet, "B1", "=A1*2");
    let _ = goal(&mut session, sheet, 10.0, 0.0, 100.0);
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["value"], json!(7.0), "the input is as it was");
}

/// A target the sheet does **not reach** between the ends is refused.
#[test]
fn a_target_the_sheet_does_not_reach_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    a_formula(&mut session, sheet, "B1", "=A1*2");
    let value = session
        .execute(
            "grid.goal",
            &json!({ "sheet": sheet, "input": "A1", "result": "B1", "target": 1000.0, "from": 0.0, "to": 100.0 }),
        )
        .expect_err("the goal is refused");
    assert!(value.to_string().contains("does not reach"), "the refusal says why, and it says {}", value);
}

/// The goal may be **at an end** of the range.
#[test]
fn the_goal_may_be_at_an_end_of_the_range() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    a_formula(&mut session, sheet, "B1", "=A1*2");
    let found = goal(&mut session, sheet, 0.0, 0.0, 100.0);
    assert!((found - 0.0).abs() < 1e-9, "the goal is at the start, and it says {}", found);
}

/// A range whose ends are the **same point** is refused.
#[test]
fn a_range_whose_ends_are_the_same_point_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    a_formula(&mut session, sheet, "B1", "=A1*2");
    let value = session
        .execute(
            "grid.goal",
            &json!({ "sheet": sheet, "input": "A1", "result": "B1", "target": 10.0, "from": 5.0, "to": 5.0 }),
        )
        .expect_err("the goal is refused");
    assert!(value.to_string().contains("two ends"), "the refusal says why, and it says {}", value);
}

/// The **input** and the **result** may not be the same cell.
#[test]
fn the_input_and_the_result_may_not_be_the_same_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    let value = session
        .execute(
            "grid.goal",
            &json!({ "sheet": sheet, "input": "A1", "result": "A1", "target": 10.0, "from": 0.0, "to": 100.0 }),
        )
        .expect_err("the goal is refused");
    assert!(value.to_string().contains("same cell"), "the refusal says why, and it says {}", value);
}

/// A result that is **not a number** is refused.
#[test]
fn a_result_that_is_not_a_number_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 0.0);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "C1", "value": "text" }))
        .expect("the cell is set");
    a_formula(&mut session, sheet, "B1", "=C1");
    let value = session
        .execute(
            "grid.goal",
            &json!({ "sheet": sheet, "input": "A1", "result": "B1", "target": 10.0, "from": 0.0, "to": 100.0 }),
        )
        .expect_err("the goal is refused");
    assert!(value.to_string().contains("the sheet says"), "the refusal says why, and it says {}", value);
}
