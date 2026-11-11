//! The **summary** of the scenarios: a sheet of its own that says what
//! every scenario does to the book.
//!
//! The tree's own `data.scenarioSummary`: the title, the current values
//! and the values of every scenario, the changing cells and the result
//! cells, and a note that says what the current values are.

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

/// A **scenario** on the sheet.
fn a_scenario(
    session: &mut Session,
    workbook: u64,
    sheet: u64,
    name: &str,
    changing: &[&str],
    values: &[f64],
) {
    session
        .execute(
            "grid.scenario.add",
            &json!({
                "workbook": workbook,
                "sheet": sheet,
                "name": name,
                "changing": changing,
                "values": values,
            }),
        )
        .expect("the scenario is added");
}

/// The **cell** of a sheet, as the document carries it.
fn cell_of(session: &Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell].clone()
}

/// The summary says the **changing cells**, the **current values** and
/// the **values of every scenario**.
#[test]
fn the_summary_says_the_changing_cells_and_the_values() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_scenario(&mut session, workbook, sheet, "base", &["A1"], &[10.0]);
    a_scenario(&mut session, workbook, sheet, "best", &["A1"], &[20.0]);
    let outcome = session
        .execute("grid.scenario.summary", &json!({ "workbook": workbook, "sheet": sheet }))
        .expect("the summary is made");
    assert_eq!(outcome.data["sheet"], json!("Scenario Summary"), "the sheet is named");
    assert_eq!(outcome.data["scenarios"], json!(2), "there are two scenarios");
    assert_eq!(outcome.data["changing_cells"], json!(1), "and one changing cell");
    let summary = outcome.data["id"].as_u64().expect("the sheet has an id");
    // The tree's own layout: the title in `B2`, the headings in `C3`
    // and `A4`, and the changing cell in the row below.
    assert_eq!(cell_of(&session, summary, "B2")["value"], json!("Scenario Summary"), "the title is there");
    assert_eq!(cell_of(&session, summary, "C3")["value"], json!("Current Values:"), "the heading is there");
    assert_eq!(cell_of(&session, summary, "D3")["value"], json!("base"), "and the first scenario's name");
    assert_eq!(cell_of(&session, summary, "E3")["value"], json!("best"), "and the second one's");
    assert_eq!(cell_of(&session, summary, "A4")["value"], json!("Changing Cells:"), "the heading is there");
    assert_eq!(cell_of(&session, summary, "B5")["value"], json!("$A$1"), "the changing cell is named");
    assert_eq!(cell_of(&session, summary, "C5")["value"], json!(10.0), "the current value is there");
    assert_eq!(cell_of(&session, summary, "D5")["value"], json!(10.0), "and the first scenario's value");
    assert_eq!(cell_of(&session, summary, "E5")["value"], json!(20.0), "and the second one's");
}

/// The summary says the **results**: the tree's own rule, and the
/// results of a scenario are computed by a trial.
#[test]
fn the_summary_says_the_results() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_formula(&mut session, sheet, "B1", "=A1*2");
    a_scenario(&mut session, workbook, sheet, "base", &["A1"], &[10.0]);
    a_scenario(&mut session, workbook, sheet, "best", &["A1"], &[20.0]);
    let outcome = session
        .execute(
            "grid.scenario.summary",
            &json!({ "workbook": workbook, "sheet": sheet, "result_cells": "B1" }),
        )
        .expect("the summary is made");
    assert_eq!(outcome.data["result_cells"], json!(1), "there is one result cell");
    let summary = outcome.data["id"].as_u64().expect("the sheet has an id");
    assert_eq!(cell_of(&session, summary, "B6")["value"], json!("Result Cells:"), "the heading is there");
    assert_eq!(cell_of(&session, summary, "B7")["value"], json!("$B$1"), "the result cell is named");
    assert_eq!(cell_of(&session, summary, "C7")["value"], json!(20.0), "the current result is there");
    assert_eq!(cell_of(&session, summary, "D7")["value"], json!(20.0), "and the first scenario's result");
    assert_eq!(cell_of(&session, summary, "E7")["value"], json!(40.0), "and the second one's");
}

/// A sheet with **no scenario** has no summary.
#[test]
fn a_sheet_with_no_scenario_has_no_summary() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.scenario.summary", &json!({ "workbook": workbook, "sheet": sheet }))
        .expect_err("the summary is refused");
    assert!(error.to_string().contains("no scenarios on this sheet"), "the refusal says so: {error}");
}

/// The name of the summary is **taken** when there is one already.
#[test]
fn the_name_of_the_summary_is_taken_when_there_is_one() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_scenario(&mut session, workbook, sheet, "base", &["A1"], &[10.0]);
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Scenario Summary" }))
        .expect("the sheet is added");
    let outcome = session
        .execute("grid.scenario.summary", &json!({ "workbook": workbook, "sheet": sheet }))
        .expect("the summary is made");
    assert_eq!(outcome.data["sheet"], json!("Scenario Summary 2"), "the name is not taken twice");
}
