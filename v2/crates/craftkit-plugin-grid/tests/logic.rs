//! The **logic** of a formula.
//!
//! A formula may ask about the truth of things and may choose between
//! them.

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

/// Put a **formula** in a cell and read what it computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, text: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": text }))
        .expect("the formula is set");
    value_of(session, sheet, cell)
}

/// Whether every one of the truths, or one of them, is true.
#[test]
fn the_truths_are_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(computed(&mut session, sheet, "A1", "=AND(TRUE,1)"), json!(true), "and of truths");
    assert_eq!(computed(&mut session, sheet, "A2", "=AND(TRUE,0)"), json!(false), "and of a false one");
    assert_eq!(computed(&mut session, sheet, "A3", "=OR(FALSE,2)"), json!(true), "or of truths");
    assert_eq!(computed(&mut session, sheet, "A4", "=OR(FALSE,FALSE)"), json!(false), "or of false ones");
    assert_eq!(computed(&mut session, sheet, "A5", "=XOR(TRUE,TRUE)"), json!(false), "an even number");
    assert_eq!(computed(&mut session, sheet, "A6", "=XOR(TRUE,TRUE,TRUE)"), json!(true), "and an odd one");
    assert_eq!(computed(&mut session, sheet, "A7", "=NOT(FALSE)"), json!(true), "the reverse");
    assert_eq!(computed(&mut session, sheet, "A8", "=NOT(5)"), json!(false), "of a number");
}

/// A **range** of truths: its numbers and its booleans count, and its
/// texts and its empties do not.
#[test]
fn a_range_of_truths_is_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 0 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B3", "value": "x" }))
        .expect("the cell is set");
    assert_eq!(
        computed(&mut session, sheet, "A1", "=AND(B1:B3)"),
        json!(false),
        "the zero in the range is false"
    );
    assert_eq!(computed(&mut session, sheet, "A2", "=OR(B1:B3)"), json!(true), "and the one is true");
}

/// The **first** of a list of conditions that is true.
#[test]
fn the_first_condition_that_is_true_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=IFS(FALSE,\"a\",TRUE,\"b\")"),
        json!("b"),
        "the condition"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=IFERROR(1/0,9)"),
        json!(9.0),
        "and the fallback of an error"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=IFERROR(3,9)"), json!(3.0), "and of a value");
    assert_eq!(
        computed(&mut session, sheet, "A4", "=IFNA(NA(),1)"),
        json!(1.0),
        "and the fallback of a missing value"
    );
}

/// The **match** of a value in a list, and the **choice** by place.
#[test]
fn the_match_and_the_choice_are_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=SWITCH(2,1,\"one\",2,\"two\")"),
        json!("two"),
        "the match"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=SWITCH(3,1,\"one\",\"other\")"),
        json!("other"),
        "and the default"
    );
    assert_eq!(computed(&mut session, sheet, "A3", "=CHOOSE(2,\"a\",\"b\",\"c\")"), json!("b"), "the choice");
}

/// The tree's **errors**: a text that is not a truth, a list of
/// conditions with one missing its value, and a place that is not in
/// the list.
#[test]
fn the_trees_errors_are_the_errors() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    assert_eq!(
        computed(&mut session, sheet, "A1", "=AND(\"abc\")"),
        json!({ "error": "#VALUE!" }),
        "a text that is not a truth is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A2", "=IFS(FALSE,\"a\")"),
        json!({ "error": "#N/A" }),
        "and a condition without a value is missing"
    );
    assert_eq!(
        computed(&mut session, sheet, "A3", "=CHOOSE(4,\"a\",\"b\",\"c\")"),
        json!({ "error": "#VALUE!" }),
        "and a place that is not in the list is an error"
    );
    assert_eq!(
        computed(&mut session, sheet, "A4", "=SWITCH(3,1,\"one\")"),
        json!({ "error": "#N/A" }),
        "and a match that is not there is missing"
    );
}
