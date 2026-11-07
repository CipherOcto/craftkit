//! The **numbers stored as text** of a sheet.
//!
//! The cells whose text would be a number if it were typed, and the
//! command that turns them into numbers.

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

/// Put a **text** in a cell.
fn text(session: &mut Session, sheet: u64, cell: &str, value: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the text is set");
}

/// The **numbers stored as text** of a sheet, as the command reads
/// them.
fn stored_of(session: &mut Session, sheet: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.numbers.stored.as.text", &json!({ "sheet": sheet }))
        .expect("the numbers are read");
    outcome.data["numbers"].clone()
}

/// A **text** that is a number is a number stored as text, and the
/// command says which number it stands for.
#[test]
fn a_text_that_is_a_number_is_stored_as_text() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    text(&mut session, sheet, "A1", "42");
    text(&mut session, sheet, "A2", "-3.5");
    let stored = stored_of(&mut session, sheet);
    let numbers = stored.as_array().expect("the numbers are a list");
    assert_eq!(numbers.len(), 2, "there are two: {stored}");
    assert_eq!(numbers[0]["cell"], json!("A1"), "the cell is named");
    assert_eq!(numbers[0]["number"], json!(42.0), "and the number it stands for");
    assert_eq!(numbers[1]["number"], json!(-3.5), "and the other one too");
}

/// A **number** is a number, a **text** that is not a number is a
/// text, and a **formula** is neither.
#[test]
fn a_number_and_a_formula_are_not_stored_as_text() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 42 }))
        .expect("the cell is set");
    text(&mut session, sheet, "A2", "the beach");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A3", "formula": "=1+1" }))
        .expect("the formula is set");
    let stored = stored_of(&mut session, sheet);
    assert_eq!(stored.as_array().map(|numbers| numbers.len()), Some(0), "there is none: {stored}");
}

/// A number in **accounting parentheses** is a number: `(5)` stands for
/// `-5`, and `$(5)` for `(-5)`.
#[test]
fn an_accounting_parenthesis_is_a_number() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    text(&mut session, sheet, "A1", "(5)");
    text(&mut session, sheet, "A2", "$(5)");
    text(&mut session, sheet, "A3", "(-5)");
    let stored = stored_of(&mut session, sheet);
    let numbers = stored.as_array().expect("the numbers are a list");
    assert_eq!(numbers.len(), 2, "the two that are numbers: {stored}");
    assert_eq!(numbers[0]["number"], json!(-5.0), "`(5)` is `-5`");
    assert_eq!(numbers[1]["number"], json!(-5.0), "and `$(5)` is `(-5)`");
}

/// The list is **derived**: it is worked out every time, and a cell
/// that is turned into a number is not in it any more.
#[test]
fn the_numbers_are_derived() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    text(&mut session, sheet, "A1", "42");
    assert_eq!(stored_of(&mut session, sheet).as_array().map(|n| n.len()), Some(1), "there is one");
    session.execute("grid.numbers.convert", &json!({ "sheet": sheet })).expect("the numbers are converted");
    let stored = stored_of(&mut session, sheet);
    assert_eq!(stored.as_array().map(|n| n.len()), Some(0), "and now there is none: {stored}");
}

/// The **conversion** turns the text into the number, and the cell
/// keeps its **format**.
#[test]
fn the_conversion_turns_the_text_into_a_number() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "B1", "format": "0.00" }))
        .expect("the format is set");
    text(&mut session, sheet, "B1", "42");
    let outcome = session
        .execute("grid.numbers.convert", &json!({ "sheet": sheet }))
        .expect("the numbers are converted");
    assert_eq!(outcome.data["count"], json!(1), "one cell is converted: {outcome:?}");
    let cell =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "B1" })).expect("the cell is read");
    assert_eq!(cell.data["value"], json!(42.0), "the cell holds the number: {cell:?}");
    assert_eq!(cell.data["format"], json!("0.00"), "and it keeps its format");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.numbers.stored.as.text", &json!({ "sheet": workbook }))
        .expect_err("the workbook is not a sheet");
    assert!(error.to_string().contains("not a sheet"), "the refusal says why: {error}");
}
