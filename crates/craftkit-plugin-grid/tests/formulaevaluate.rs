//! The **evaluation** of a formula that is not in a cell.
//!
//! The formula is worked out against the sheet as it is, and nothing
//! is changed.

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

/// A formula is **evaluated** and its value is given back.
#[test]
fn a_formula_is_evaluated() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let outcome = session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "=1+2" }))
        .expect("the formula is evaluated");
    assert_eq!(outcome.data["value"].as_f64(), Some(3.0), "the formula gives three");
    assert_eq!(outcome.data["type"].as_str(), Some("number"), "and it is a number");
}

/// A formula that reads the **sheet** is worked out against it.
#[test]
fn a_formula_reads_the_sheet() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 }))
        .expect("the cell is set");
    let outcome = session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "=A1*2" }))
        .expect("the formula is evaluated");
    assert_eq!(outcome.data["value"].as_f64(), Some(10.0), "the formula gives ten");
}

/// The formula may be written with or without its `=`.
#[test]
fn the_formula_may_have_no_equal_sign() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let outcome = session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "1+2" }))
        .expect("the formula is evaluated");
    assert_eq!(outcome.data["value"].as_f64(), Some(3.0), "the formula gives three");
}

/// **Nothing is changed**: a formula that is not in a cell is not in a
/// cell.
#[test]
fn nothing_is_changed() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "=1+2" }))
        .expect("the formula is evaluated");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["value"], json!(null), "the cell holds nothing");
    assert_eq!(outcome.data["formula"], json!(null), "and no formula");
}

/// A formula the tree **cannot parse** is an error, exactly as it is
/// when it is put in a cell.
#[test]
fn a_formula_that_cannot_be_read_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.formula.evaluate", &json!({ "sheet": sheet, "formula": "=1+" }))
        .expect_err("the formula is refused");
    assert!(!error.to_string().is_empty(), "the error says something: {error}");
}

/// A formula that reads **itself** is refused when it is put in a
/// cell: the tree's own rule, and so a cycle never reaches the
/// evaluation.
#[test]
fn a_formula_that_reads_itself_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=A1" }))
        .expect_err("the formula is refused");
    assert!(error.to_string().contains("reads itself"), "the error says so: {error}");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.formula.evaluate", &json!({ "sheet": workbook, "formula": "=1+2" }))
        .expect_err("the formula is refused");
    assert!(error.to_string().contains("not a sheet"), "the error says so: {error}");
}
