//! The **exporting** of a sheet as a text of values.

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

/// The **text** of the export, as the command gives it.
fn text_of(session: &mut Session, sheet: u64, range: &str) -> String {
    let outcome = session
        .execute("grid.export.csv", &json!({ "sheet": sheet, "range": range }))
        .expect("the sheet is exported");
    outcome.data["text"].as_str().expect("the text is a text").to_string()
}

/// The cells of a range are written as a **text of values**.
#[test]
fn the_cells_are_written_as_a_text_of_values() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    value(&mut session, sheet, "B1", json!(3));
    value(&mut session, sheet, "A2", json!("bob"));
    value(&mut session, sheet, "B2", json!(4));
    assert_eq!(text_of(&mut session, sheet, "A1:B2"), "ann,3\nbob,4", "the rows are the lines");
}

/// A field that holds the **delimiter** is quoted.
#[test]
fn a_field_that_holds_the_delimiter_is_quoted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one,two"));
    value(&mut session, sheet, "B1", json!("three"));
    assert_eq!(text_of(&mut session, sheet, "A1:B1"), "\"one,two\",three", "the field is quoted");
}

/// A quote inside a field is **doubled**.
#[test]
fn a_quote_inside_a_field_is_doubled() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("the \"word\""));
    assert_eq!(text_of(&mut session, sheet, "A1:A1"), "\"the \"\"word\"\"\"", "the quote is doubled");
}

/// The **delimiter** may be another character.
#[test]
fn the_delimiter_may_be_another_character() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    value(&mut session, sheet, "B1", json!(3));
    let outcome = session
        .execute("grid.export.csv", &json!({ "sheet": sheet, "range": "A1:B1", "delimiter": ";" }))
        .expect("the sheet is exported");
    assert_eq!(outcome.data["text"], json!("ann;3"), "the fields are separated by the semicolon");
}

/// An empty cell is an **empty field**.
#[test]
fn an_empty_cell_is_an_empty_field() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    assert_eq!(text_of(&mut session, sheet, "A1:B1"), "ann,", "the field is empty");
}

/// A delimiter of more than one character is **refused**.
#[test]
fn a_delimiter_of_more_than_one_character_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    let outcome =
        session.execute("grid.export.csv", &json!({ "sheet": sheet, "range": "A1:B1", "delimiter": ";;" }));
    assert!(outcome.is_err(), "a delimiter is one character");
}
