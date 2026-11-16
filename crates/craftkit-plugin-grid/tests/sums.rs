//! The **sum** of a range of numbers, written in a cell.

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
        .expect("the value is set");
}

/// The **value** of a cell, as the answer holds it.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// The **formula** of a cell, as the answer holds it.
fn formula_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["formula"].clone()
}

/// The **run of numbers above** a cell is summed.
#[test]
fn the_run_of_numbers_above_a_cell_is_summed() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    let outcome = session
        .execute("grid.formula.sum", &json!({ "sheet": sheet, "cell": "A3" }))
        .expect("the sum is written");
    assert_eq!(outcome.data["formula"], json!("SUM(A1:A2)"), "the formula is the sum of the run");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(3.0), "and the value is the sum");
}

/// The **range** may be named.
#[test]
fn the_range_may_be_named() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B2", json!(10));
    value(&mut session, sheet, "B3", json!(20));
    session
        .execute("grid.formula.sum", &json!({ "sheet": sheet, "cell": "C1", "range": "B2:B3" }))
        .expect("the sum is written");
    assert_eq!(formula_of(&mut session, sheet, "C1"), json!("SUM(B2:B3)"), "the range is the one named");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(30.0), "and the value is the sum");
}

/// The run of numbers **beside** the cell is summed when there is none
/// above it.
#[test]
fn the_run_of_numbers_beside_the_cell_is_summed() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(3));
    value(&mut session, sheet, "B1", json!(4));
    session
        .execute("grid.formula.sum", &json!({ "sheet": sheet, "cell": "C1" }))
        .expect("the sum is written");
    assert_eq!(formula_of(&mut session, sheet, "C1"), json!("SUM(A1:B1)"), "the run is the one beside");
    assert_eq!(value_of(&mut session, sheet, "C1"), json!(7.0), "and the value is the sum");
}

/// A run of numbers ends where a cell that holds **no number** begins.
#[test]
fn a_run_of_numbers_ends_where_a_cell_that_holds_no_number_begins() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!("two"));
    value(&mut session, sheet, "A3", json!(3));
    session
        .execute("grid.formula.sum", &json!({ "sheet": sheet, "cell": "A4" }))
        .expect("the sum is written");
    assert_eq!(formula_of(&mut session, sheet, "A4"), json!("SUM(A3)"), "the run is the one below");
}

/// A cell that holds a **formula** counts when the formula gives a
/// number.
#[test]
fn a_cell_that_holds_a_formula_counts_when_the_formula_gives_a_number() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(5));
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A2", "formula": "A1 * 2" }))
        .expect("the formula is set");
    session
        .execute("grid.formula.sum", &json!({ "sheet": sheet, "cell": "A3" }))
        .expect("the sum is written");
    assert_eq!(formula_of(&mut session, sheet, "A3"), json!("SUM(A1:A2)"), "the run reaches the formula");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(15.0), "and the value is the sum");
}

/// The **way** of the sum may be another one.
#[test]
fn the_way_of_the_sum_may_be_another_one() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    value(&mut session, sheet, "A2", json!(4));
    session
        .execute("grid.formula.sum", &json!({ "sheet": sheet, "cell": "A3", "function": "average" }))
        .expect("the sum is written");
    assert_eq!(formula_of(&mut session, sheet, "A3"), json!("AVERAGE(A1:A2)"), "the way is the one named");
    assert_eq!(value_of(&mut session, sheet, "A3"), json!(3.0), "and the value is the average");
}

/// A way that is **not one of the tree's** is refused.
#[test]
fn a_way_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    let outcome =
        session.execute("grid.formula.sum", &json!({ "sheet": sheet, "cell": "A2", "function": "median" }));
    assert!(outcome.is_err(), "a way of summing is one of the tree's");
}

/// A cell with **no numbers** around it is refused.
#[test]
fn a_cell_with_no_numbers_around_it_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome = session.execute("grid.formula.sum", &json!({ "sheet": sheet, "cell": "A1" }));
    assert!(outcome.is_err(), "there is nothing to sum");
}
