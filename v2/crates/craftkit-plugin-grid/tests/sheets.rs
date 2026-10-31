//! The **formula reads another sheet**.
//!
//! §148 found that a grid's formulas could read the sheet they were in
//! and no other, and that the family's formulas read across the sheets
//! of a workbook. This is the arc that brings that back.

use craftkit_sdk::serde_json::{Value, json};
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

/// A workbook with two sheets: `Sheet1` and `Sheet2`.
fn a_workbook() -> Session {
    let mut session = session();
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet2" }))
        .expect("the sheet is added");
    session
}

/// The **value** of a cell, as the calculation says it.
fn value(session: &mut Session, sheet: u64, cell: &str) -> Value {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// A formula reads a cell of **another sheet**.
#[test]
fn a_formula_reads_a_cell_of_another_sheet() {
    let mut session = a_workbook();
    session.execute("grid.cell.set", &json!({ "sheet": 3, "cell": "A1", "value": 7 })).expect("set");
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=Sheet2!A1" }))
        .expect("the formula is set");
    assert_eq!(value(&mut session, 2, "A1"), json!(7.0), "the cell of the other sheet is read");
}

/// A sheet's name may be **quoted**, and then it may hold anything.
#[test]
fn a_quoted_sheets_name_is_read() {
    let mut session = session();
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "My Sheet" }))
        .expect("the sheet is added");
    session.execute("grid.cell.set", &json!({ "sheet": 2, "cell": "A1", "value": 9 })).expect("set");
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "B1", "formula": "='My Sheet'!A1" }))
        .expect("the formula is set");
    assert_eq!(value(&mut session, 2, "B1"), json!(9.0), "the quoted sheet is read");
}

/// A **range** of another sheet is read, and a function may take it.
#[test]
fn a_range_of_another_sheet_is_read() {
    let mut session = a_workbook();
    session.execute("grid.cell.set", &json!({ "sheet": 3, "cell": "A1", "value": 1 })).expect("set");
    session.execute("grid.cell.set", &json!({ "sheet": 3, "cell": "A2", "value": 2 })).expect("set");
    session.execute("grid.cell.set", &json!({ "sheet": 3, "cell": "A3", "value": 3 })).expect("set");
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=SUM(Sheet2!A1:A3)" }))
        .expect("the formula is set");
    assert_eq!(value(&mut session, 2, "A1"), json!(6.0), "the range of the other sheet is read");
}

/// A sheet that is **not there** is an error and not a zero.
#[test]
fn a_sheet_that_is_not_there_is_an_error() {
    let mut session = a_workbook();
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=Nowhere!A1" }))
        .expect("the formula is set");
    let value = value(&mut session, 2, "A1");
    assert!(value.get("error").is_some(), "the cell is an error: {value}");
}

/// A **cycle** across the sheets is refused.
///
/// The cycle is a cycle of the workbook: `Sheet1!A1` reads `Sheet2!A1`
/// and `Sheet2!A1` reads `Sheet1!A1`, and the second one is a cell
/// that reads itself.
#[test]
fn a_cycle_across_the_sheets_is_refused() {
    let mut session = a_workbook();
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=Sheet2!A1" }))
        .expect("the first formula is set");
    let refused =
        session.execute("grid.formula.set", &json!({ "sheet": 3, "cell": "A1", "formula": "=Sheet1!A1" }));
    assert!(refused.is_err(), "the cycle is refused");
}

/// The **precedents** name the sheet they are in.
#[test]
fn the_precedents_name_the_sheet() {
    let mut session = a_workbook();
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=Sheet2!A1+1" }))
        .expect("the formula is set");
    let outcome = session
        .execute("grid.precedents", &json!({ "sheet": 2, "cell": "A1" }))
        .expect("the precedents are asked for");
    let precedents = outcome.data["precedents"].as_array().expect("the precedents are a list");
    assert_eq!(precedents.len(), 1, "there is one precedent: {precedents:?}");
    assert_eq!(precedents[0]["sheet"], json!("Sheet2"), "and it names the sheet");
    assert_eq!(precedents[0]["area"], json!("A1"), "and the area");
}

/// The **dependents** of a cell are the cells that read it, in any
/// sheet.
#[test]
fn the_dependents_of_a_cell_are_the_cells_that_read_it() {
    let mut session = a_workbook();
    session.execute("grid.cell.set", &json!({ "sheet": 3, "cell": "A1", "value": 5 })).expect("set");
    session
        .execute("grid.formula.set", &json!({ "sheet": 2, "cell": "A1", "formula": "=Sheet2!A1" }))
        .expect("the formula is set");
    let outcome = session
        .execute("grid.dependents", &json!({ "sheet": 3, "cell": "A1" }))
        .expect("the dependents are asked for");
    let dependents = outcome.data["dependents"].as_array().expect("the dependents are a list");
    assert_eq!(dependents.len(), 1, "there is one dependent: {dependents:?}");
    assert_eq!(dependents[0]["sheet"], json!("Sheet1"), "and it is in the other sheet");
    assert_eq!(dependents[0]["cell"], json!("A1"), "and it is the cell that reads it");
}
