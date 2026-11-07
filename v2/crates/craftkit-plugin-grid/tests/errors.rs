//! The **errors** of a sheet.
//!
//! The cells whose formula comes out as an error, with the error and
//! the formula that gave it.

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
    let outcome = session
        .execute("grid.workbook.add", &json!({ "name": "the workbook" }))
        .expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "the sheet" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **errors** of a sheet, as the command reads them.
fn errors_of(session: &mut Session, sheet: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session.execute("grid.errors", &json!({ "sheet": sheet })).expect("the errors are read");
    outcome.data["errors"].clone()
}

/// A formula that **cannot be worked out** is an error, and the cell
/// that holds it says so.
#[test]
fn a_formula_that_fails_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 0 }))
        .expect("the cell is set");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=A1/0" }))
        .expect("the formula is set");
    let errors = errors_of(&mut session, sheet);
    assert_eq!(errors.as_array().map(|errors| errors.len()), Some(1), "there is one error: {errors}");
    assert_eq!(errors[0]["cell"], json!("B1"), "the cell is named");
    assert_eq!(errors[0]["error"], json!("#DIV/0!"), "and the error is the tree's");
    assert_eq!(errors[0]["formula"], json!("=A1/0"), "and the formula is given");
}

/// A cell with **no formula** is not an error, and a formula that
/// works out is not an error either.
#[test]
fn a_cell_that_is_fine_is_not_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 2 }))
        .expect("the cell is set");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=A1*3" }))
        .expect("the formula is set");
    let errors = errors_of(&mut session, sheet);
    assert_eq!(errors.as_array().map(|errors| errors.len()), Some(0), "there is no error: {errors}");
}

/// The errors are **derived**: they are worked out every time, and a
/// formula that is fixed is no longer an error.
#[test]
fn the_errors_are_derived() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 0 }))
        .expect("the cell is set");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=A1/0" }))
        .expect("the formula is set");
    assert_eq!(errors_of(&mut session, sheet).as_array().map(|e| e.len()), Some(1), "there is an error");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=A1/2" }))
        .expect("the formula is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 6 }))
        .expect("the cell is set");
    let errors = errors_of(&mut session, sheet);
    assert_eq!(errors.as_array().map(|errors| errors.len()), Some(0), "and now there is none: {errors}");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.errors", &json!({ "sheet": workbook }))
        .expect_err("the workbook is not a sheet");
    assert!(error.to_string().contains("not a sheet"), "the refusal says why: {error}");
}
