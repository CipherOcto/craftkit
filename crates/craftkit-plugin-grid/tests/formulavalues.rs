//! The formulas that become their **values**.
//!
//! The tree's rules: a formula that gives a number becomes that
//! number and keeps its format, and a formula that gives anything else
//! is left as it is.

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

/// A **workbook** with a sheet in it, and the sheet's id.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **cell** as the document holds it.
fn cell_of(session: &Session, sheet: u64, cell: &str) -> Json {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell].clone()
}

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
}

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
}

/// A formula that gives a **number** becomes that number.
#[test]
fn a_formula_that_gives_a_number_becomes_that_number() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=1+2");
    let outcome = session
        .execute("grid.formula.values", &json!({ "sheet": sheet, "range": "A1:A5" }))
        .expect("the formulas are converted");
    assert_eq!(outcome.data["converted"], json!(1), "one formula was converted: {outcome:?}");
    let cell = cell_of(&session, sheet, "A1");
    assert_eq!(cell["value"], json!(3.0), "the cell holds the number: {cell:?}");
    assert!(cell.get("formula").is_none(), "the cell holds no formula: {cell:?}");
}

/// The **format** of the cell is kept.
#[test]
fn the_format_of_the_cell_is_kept() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=1+2");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    session
        .execute("grid.formula.values", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the formulas are converted");
    let cell = cell_of(&session, sheet, "A1");
    assert_eq!(cell["format"], json!("0.00"), "the format is kept: {cell:?}");
}

/// A cell whose formula gives **text** is left as it is.
#[test]
fn a_formula_that_gives_text_is_left_alone() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=IF(1>0,\"yes\",\"no\")");
    let before = cell_of(&session, sheet, "A1");
    let outcome = session
        .execute("grid.formula.values", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the formulas are converted");
    assert_eq!(outcome.data["converted"], json!(0), "nothing was converted: {outcome:?}");
    assert_eq!(cell_of(&session, sheet, "A1"), before, "the cell is as it was");
}

/// A cell with **no formula** is left as it is.
#[test]
fn a_cell_with_no_formula_is_left_alone() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 12 }))
        .expect("the cell is set");
    let before = cell_of(&session, sheet, "A1");
    session
        .execute("grid.formula.values", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the formulas are converted");
    assert_eq!(cell_of(&session, sheet, "A1"), before, "the cell is as it was");
}

/// The value of a formula is the one the **formula** gives.
#[test]
fn the_value_is_the_one_the_formula_gives() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=2*3");
    formula(&mut session, sheet, "B1", "=A1*2");
    session
        .execute("grid.formula.values", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the formulas are converted");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(12.0), "the other formula counts the number");
}

/// A range that is **not a range** is refused.
#[test]
fn a_range_that_is_not_a_range_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.formula.values", &json!({ "sheet": sheet, "range": "here" }));
    assert!(outcome.is_err(), "the range that is not a range is refused");
}

/// A sheet that is **not there** is refused.
#[test]
fn a_sheet_that_is_not_there_is_refused() {
    let mut session = session();
    let outcome = session.execute("grid.formula.values", &json!({ "sheet": 999, "range": "A1" }));
    assert!(outcome.is_err(), "the sheet that is not there is refused");
}
