//! The **`TEXT`** of a formula.
//!
//! The tree's `TEXT` writes a value with a format code inside a
//! formula, and the code may be in a cell.

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

/// A workbook with one sheet, and the sheet's id.
fn a_sheet(session: &mut Session) -> u64 {
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **text** a cell shows.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell has a text");
    outcome.data["text"].as_str().unwrap_or_default().to_string()
}

/// Put a formula in a cell.
fn set_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// A `TEXT` writes a number with a **format code**.
#[test]
fn a_text_writes_a_number_with_a_format_code() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 })).expect("set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1, \"0.00\")");
    assert_eq!(text_of(&mut session, sheet, "B1"), "5.00", "the number is written with the code");
}

/// A `TEXT` of a **percentage** multiplies the value by a hundred,
/// as the format does.
#[test]
fn a_text_of_a_percentage_multiplies_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 0.5 })).expect("set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1, \"0%\")");
    assert_eq!(text_of(&mut session, sheet, "B1"), "50%", "the percentage is written");
}

/// A `TEXT` of a **date** writes the date.
#[test]
fn a_text_of_a_date_writes_the_date() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "2026-10-26" }))
        .expect("the date is set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1, \"yyyy-mm-dd\")");
    assert_eq!(text_of(&mut session, sheet, "B1"), "2026-10-26", "the date is written");
}

/// The **code** may be in a cell: the tree's second argument is
/// evaluated.
#[test]
fn the_code_may_be_in_a_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 })).expect("set");
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A2", "value": "0.00" })).expect("set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1, A2)");
    assert_eq!(text_of(&mut session, sheet, "B1"), "5.00", "the code is read from the cell");
}

/// A code that this tree does **not** have is an error, and the error
/// says which code it was.
#[test]
fn a_code_this_tree_does_not_have_is_an_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 })).expect("set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1, \"dd/mm/yy\")");
    assert!(
        text_of(&mut session, sheet, "B1").contains("there is no format"),
        "the error says why: {}",
        text_of(&mut session, sheet, "B1")
    );
}

/// The number formats are a **grammar** and not a list: a code with
/// four decimals is a code this tree has.
#[test]
fn a_code_with_four_decimals_is_a_code_this_tree_has() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 })).expect("set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1, \"0.0000\")");
    assert_eq!(text_of(&mut session, sheet, "B1"), "5.0000", "the code is a code");
}

/// A `TEXT` with the **wrong number of arguments** is an error.
#[test]
fn a_text_with_the_wrong_arguments_is_an_error() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 })).expect("set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1)");
    assert_eq!(text_of(&mut session, sheet, "B1"), "#VALUE!", "the error is the tree's");
}

/// A `TEXT` of a **text** is the text: a format does not change a
/// text.
#[test]
fn a_text_of_a_text_is_the_text() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "hello" }))
        .expect("set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1, \"0.00\")");
    assert_eq!(text_of(&mut session, sheet, "B1"), "hello", "the text is the text");
}

/// A `TEXT` of a **general** format is the value as it is.
#[test]
fn a_text_of_a_general_format_is_the_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5.5 })).expect("set");
    set_formula(&mut session, sheet, "B1", "=TEXT(A1, \"general\")");
    assert_eq!(text_of(&mut session, sheet, "B1"), "5.5", "the value is the value");
}
