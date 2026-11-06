//! The **error** of a reference that is gone.

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

/// A **workbook** with a **sheet** in it.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// A **number** in a cell.
fn set_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the number is set");
}

/// A **formula** in a cell.
fn set_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// What a cell **holds**.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// A formula may **hold an error**.
#[test]
fn a_formula_may_hold_an_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=#REF!");
    assert_eq!(cell_of(&mut session, sheet, "A1")["formula"], json!("=#REF!"), "the formula holds the error");
}

/// The **value** of a formula that holds an error is the error.
#[test]
fn the_value_of_a_formula_that_holds_an_error_is_the_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=#REF!");
    assert_eq!(
        cell_of(&mut session, sheet, "A1")["value"],
        json!({ "error": "#REF!" }),
        "the value is the error the tree writes"
    );
}

/// An error in a formula makes the **whole** formula an error.
#[test]
fn an_error_in_a_formula_makes_the_whole_formula_an_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_number(&mut session, sheet, "B1", 2.0);
    set_formula(&mut session, sheet, "A1", "=B1+#REF!");
    assert_eq!(
        cell_of(&mut session, sheet, "A1")["value"],
        json!({ "error": "#REF!" }),
        "the error is the value of the formula"
    );
}

/// A **function** with an error in it is an error.
#[test]
fn a_function_with_an_error_in_it_is_an_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_number(&mut session, sheet, "B1", 2.0);
    set_formula(&mut session, sheet, "A1", "=SUM(B1:B3, #REF!)");
    assert_eq!(
        cell_of(&mut session, sheet, "A1")["value"],
        json!({ "error": "#REF!" }),
        "the call is the error"
    );
}

/// A **range** that loses its first row names the error.
#[test]
fn a_range_that_loses_its_first_row_names_the_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "C5", "=A1:B2");
    session.execute("grid.row.delete", &json!({ "sheet": sheet, "row": 1 })).expect("the row is deleted");
    assert_eq!(cell_of(&mut session, sheet, "C4")["formula"], json!("=#REF!"), "the range names the error");
}

/// A **range** that loses a row in the middle shrinks.
#[test]
fn a_range_that_loses_a_row_in_the_middle_shrinks() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "C1", "=SUM(A1:A4)");
    session.execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2 })).expect("the row is deleted");
    assert_eq!(cell_of(&mut session, sheet, "C1")["formula"], json!("=SUM(A1:A3)"), "the range shrank");
}

/// A **column** that is deleted names the error too.
#[test]
fn a_column_that_is_deleted_names_the_error_too() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_formula(&mut session, sheet, "A1", "=B1");
    session
        .execute("grid.column.delete", &json!({ "sheet": sheet, "column": "B" }))
        .expect("the column is deleted");
    assert_eq!(cell_of(&mut session, sheet, "A1")["formula"], json!("=#REF!"), "the formula names the error");
}

/// The error of a **reference** is the tree's own spelling.
#[test]
fn the_error_of_a_reference_is_the_trees_own_spelling() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    set_number(&mut session, sheet, "A2", 2.0);
    set_formula(&mut session, sheet, "B1", "=A2");
    session.execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2 })).expect("the row is deleted");
    let cell = cell_of(&mut session, sheet, "B1");
    assert_eq!(cell["value"], json!({ "error": "#REF!" }), "the error is spelled the tree's way");
    assert_eq!(cell["type"], json!("error"), "and the cell says it holds an error");
}
