//! The **text of a column**, cut into columns.

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

/// The **values** of a range, as the answer holds them.
fn values_of(session: &mut Session, sheet: u64, range: &str) -> Vec<Json> {
    let outcome = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": range }))
        .expect("the range is read");
    outcome.data["values"].as_array().expect("the values are a list").clone()
}

/// The text of a column is **cut** into the columns beside it.
#[test]
fn the_text_of_a_column_is_cut_into_columns() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one,two,three"));
    session
        .execute("grid.text.columns", &json!({ "sheet": sheet, "column": "A", "delimiter": "," }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:C1");
    assert_eq!(values[0][0], json!("one"), "the first part is in the first column");
    assert_eq!(values[0][1], json!("two"), "the second part is in the second");
    assert_eq!(values[0][2], json!("three"), "and the third in the third");
}

/// A part that is a **number** is a number.
#[test]
fn a_part_that_is_a_number_is_a_number() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("12,thirty"));
    session
        .execute("grid.text.columns", &json!({ "sheet": sheet, "column": "A", "delimiter": "," }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:B1");
    assert_eq!(values[0][0], json!(12.0), "the twelve is a number");
    assert_eq!(values[0][1], json!("thirty"), "and the thirty is a text");
}

/// The parts may be written into another **column**.
#[test]
fn the_parts_may_be_written_into_another_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one,two"));
    session
        .execute(
            "grid.text.columns",
            &json!({ "sheet": sheet, "column": "A", "delimiter": ",", "destination": "C" }),
        )
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "C1:D1");
    assert_eq!(values[0][0], json!("one"), "the first part is in the third column");
    assert_eq!(values[0][1], json!("two"), "and the second in the fourth");
}

/// A **qualifier** marks a part that holds the delimiter itself.
#[test]
fn a_qualifier_marks_a_part_that_holds_the_delimiter() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("\"one,two\",three"));
    session
        .execute("grid.text.columns", &json!({ "sheet": sheet, "column": "A", "delimiter": "," }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:B1");
    assert_eq!(values[0][0], json!("one,two"), "the quoted part is one part");
    assert_eq!(values[0][1], json!("three"), "and the other is another");
}

/// A run of delimiters is **one** delimiter when the file says so.
#[test]
fn a_run_of_delimiters_is_one_delimiter_when_the_file_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one,,two"));
    session
        .execute(
            "grid.text.columns",
            &json!({ "sheet": sheet, "column": "A", "delimiter": ",", "treat_consecutive": true }),
        )
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:B1");
    assert_eq!(values[0][0], json!("one"), "the first part is one");
    assert_eq!(values[0][1], json!("two"), "and the empty part is gone");
}

/// A cell that holds **no text** is left alone.
#[test]
fn a_cell_that_holds_no_text_is_left_alone() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(7));
    session
        .execute("grid.text.columns", &json!({ "sheet": sheet, "column": "A", "delimiter": "," }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:B1");
    assert_eq!(values[0][0], json!(7.0), "the number is still there");
}

/// A delimiter that is more than one character is **refused**.
#[test]
fn a_delimiter_that_is_more_than_one_character_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one;;two"));
    let outcome =
        session.execute("grid.text.columns", &json!({ "sheet": sheet, "column": "A", "delimiter": ";;" }));
    assert!(outcome.is_err(), "a delimiter of two characters is refused");
}

/// A column that is **not a column** is refused.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome = session.execute("grid.text.columns", &json!({ "sheet": sheet, "column": "1" }));
    assert!(outcome.is_err(), "a number is not a column");
}
