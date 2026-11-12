//! **The choice** of a value.

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

/// The **value** a formula computes.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session.execute("grid.calc", &json!({ "sheet": sheet })).expect("the sheet is calculated");
    outcome.data["cells"][cell].clone()
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

/// A condition that **holds** takes the first branch.
#[test]
fn a_condition_that_holds_takes_the_first_branch() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(5));
    formula(&mut session, sheet, "B1", "=IF(A1>2,\"big\",\"small\")");
    assert_eq!(value_of(&mut session, sheet, "B1"), Json::String("big".to_string()), "the first branch");
}

/// A condition that **does not hold** takes the second branch.
#[test]
fn a_condition_that_does_not_hold_takes_the_second_branch() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    formula(&mut session, sheet, "B1", "=IF(A1>2,\"big\",\"small\")");
    assert_eq!(value_of(&mut session, sheet, "B1"), Json::String("small".to_string()), "the second branch");
}

/// The branch that is **not taken** is not computed.
#[test]
fn the_branch_that_is_not_taken_is_not_computed() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(0));
    formula(&mut session, sheet, "B1", "=IF(A1=0,\"\",1/A1)");
    assert_eq!(value_of(&mut session, sheet, "B1"), Json::String(String::new()), "and no division by zero");
}

/// A formula of **one argument** is the truth of its condition.
#[test]
fn a_formula_of_one_argument_is_the_truth_of_its_condition() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "B1", "=IF(3>2)");
    assert_eq!(value_of(&mut session, sheet, "B1"), Json::Bool(true), "the condition holds");
    formula(&mut session, sheet, "C1", "=IF(1>2)");
    assert_eq!(value_of(&mut session, sheet, "C1"), Json::Bool(false), "and it does not");
}

/// A formula of **two arguments** has `FALSE` for the false branch.
#[test]
fn a_formula_of_two_arguments_has_false_for_the_false_branch() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    formula(&mut session, sheet, "B1", "=IF(A1>2,\"big\")");
    assert_eq!(value_of(&mut session, sheet, "B1"), Json::Bool(false), "the false answer is false");
}

/// A formula of **more than three** arguments is an error.
#[test]
fn a_formula_of_more_than_three_arguments_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "B1", "=IF(TRUE,1,2,3)");
    let value = value_of(&mut session, sheet, "B1");
    assert_eq!(value["error"], Json::String("#VALUE!".to_string()), "it is an error");
}

/// A condition that is **text** that is not a boolean is an error.
#[test]
fn a_condition_that_is_text_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "B1", "=IF(\"maybe\",1,2)");
    let value = value_of(&mut session, sheet, "B1");
    assert_eq!(value["error"], Json::String("#VALUE!".to_string()), "it is an error");
}

/// A condition that is an **error** is that error.
#[test]
fn a_condition_that_is_an_error_is_that_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "B1", "=IF(1/0,1,2)");
    let value = value_of(&mut session, sheet, "B1");
    assert_eq!(value["error"], Json::String("#DIV/0!".to_string()), "the error is the error");
}

/// An **empty cell** as the condition is false, and a **zero** is false.
#[test]
fn an_empty_cell_as_the_condition_is_false() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "B1", "=IF(A1,1,2)");
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(2.0), "an empty cell is false");
    value(&mut session, sheet, "A1", json!(0));
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(2.0), "and a zero is false");
    value(&mut session, sheet, "A1", json!(3));
    assert_eq!(value_of(&mut session, sheet, "B1"), json!(1.0), "and a number that is not zero is true");
}
