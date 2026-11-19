//! The **counting of the values** that are not nothing.

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

/// A **sheet** with a workbook above it, and their ids.
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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, formula: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// Every value of a range is counted: a number, a text and a boolean.
#[test]
fn every_value_of_a_range_is_counted() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!("two"));
    value(&mut session, sheet, "A3", json!(true));
    let answer = computed(&mut session, sheet, "C1", "=COUNTA(A1:A3)");
    assert_eq!(answer, json!(3.0), "the number, the text and the boolean are counted");
}

/// An **error** in a range is a value, and it is counted.
#[test]
fn an_error_in_a_range_is_counted() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A2", "formula": "=1/0" }))
        .expect("the formula is set");
    let answer = computed(&mut session, sheet, "C1", "=COUNTA(A1:A2)");
    assert_eq!(answer, json!(2.0), "the number and the error are counted");
}

/// A cell that holds **nothing** is not a value, and it is not counted.
#[test]
fn a_cell_that_holds_nothing_is_not_counted() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A3", json!("three"));
    let answer = computed(&mut session, sheet, "C1", "=COUNTA(A1:A3)");
    assert_eq!(answer, json!(2.0), "the empty cell is not counted");
    let answer = computed(&mut session, sheet, "C2", "=COUNTA(A2)");
    assert_eq!(answer, json!(0.0), "and an empty cell on its own is not counted");
}

/// Two values are **two** values, and each of them is counted once.
#[test]
fn two_values_are_two_values() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let answer = computed(&mut session, sheet, "C1", "=COUNTA(1,2)");
    assert_eq!(answer, json!(2.0), "the two numbers are counted once each");
    let answer = computed(&mut session, sheet, "C2", "=COUNTA(1,\"\")");
    assert_eq!(answer, json!(2.0), "and an empty text is a value");
}

/// The **number** of a cell is counted, and the count of a range of
/// numbers is the count of them.
#[test]
fn the_count_and_the_count_of_the_numbers_agree() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!("three"));
    let answer = computed(&mut session, sheet, "C1", "=COUNT(A1:A3)");
    assert_eq!(answer, json!(2.0), "the count counts the numbers");
    let answer = computed(&mut session, sheet, "C2", "=COUNTA(A1:A3)");
    assert_eq!(answer, json!(3.0), "and the count of the values counts them all");
}
