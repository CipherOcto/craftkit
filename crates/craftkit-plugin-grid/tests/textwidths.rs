//! The **text of a column**, cut at fixed widths.

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

/// The text of a column is **cut** at the widths that are given.
#[test]
fn the_text_of_a_column_is_cut_at_fixed_widths() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("onetwothree"));
    session
        .execute("grid.text.widths", &json!({ "sheet": sheet, "column": "A", "widths": "3,6" }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:C1");
    assert_eq!(values[0][0], json!("one"), "the first part is the first three characters");
    assert_eq!(values[0][1], json!("two"), "the second part is the next three");
    assert_eq!(values[0][2], json!("three"), "and the last part is what is left");
}

/// A part that is a **number** is a number.
#[test]
fn a_part_that_is_a_number_is_a_number() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("12thirty"));
    session
        .execute("grid.text.widths", &json!({ "sheet": sheet, "column": "A", "widths": "2" }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:B1");
    assert_eq!(values[0][0], json!(12.0), "the twelve is a number");
    assert_eq!(values[0][1], json!("thirty"), "and the thirty is a text");
}

/// The widths are a **set**: the order does not matter, and a width
/// twice is one width.
#[test]
fn the_widths_are_a_set() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("onetwothree"));
    session
        .execute("grid.text.widths", &json!({ "sheet": sheet, "column": "A", "widths": "6,3,6" }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:C1");
    assert_eq!(values[0][0], json!("one"), "the first part is the first three characters");
    assert_eq!(values[0][1], json!("two"), "the second part is the next three");
    assert_eq!(values[0][2], json!("three"), "and the last part is what is left");
}

/// A width **past the end** of a text cuts nothing there.
#[test]
fn a_width_past_the_end_of_a_text_cuts_nothing() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("abc"));
    session
        .execute("grid.text.widths", &json!({ "sheet": sheet, "column": "A", "widths": "2,9" }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:B1");
    assert_eq!(values[0][0], json!("ab"), "the first part is the first two characters");
    assert_eq!(values[0][1], json!("c"), "and what is left is the last part");
}

/// The parts may be written into another **column**.
#[test]
fn the_parts_may_be_written_into_another_column() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("onetwo"));
    session
        .execute(
            "grid.text.widths",
            &json!({ "sheet": sheet, "column": "A", "widths": "3", "destination": "C" }),
        )
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "C1:D1");
    assert_eq!(values[0][0], json!("one"), "the first part is in the third column");
    assert_eq!(values[0][1], json!("two"), "and the second in the fourth");
}

/// A cell that holds **no text** is left alone.
#[test]
fn a_cell_that_holds_no_text_is_left_alone() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(7));
    session
        .execute("grid.text.widths", &json!({ "sheet": sheet, "column": "A", "widths": "3" }))
        .expect("the text is cut");
    let values = values_of(&mut session, sheet, "A1:B1");
    assert_eq!(values[0][0], json!(7.0), "the number is still there");
}

/// A width of **no characters** is refused.
#[test]
fn a_width_of_no_characters_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.text.widths", &json!({ "sheet": sheet, "column": "A", "widths": "0" }));
    assert!(outcome.is_err(), "a width of no characters is refused");
}

/// A width that is **not a number** is refused.
#[test]
fn a_width_that_is_not_a_number_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.text.widths", &json!({ "sheet": sheet, "column": "A", "widths": "3,wide" }));
    assert!(outcome.is_err(), "a word is not a width");
}

/// A column that is **not a column** is refused.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.text.widths", &json!({ "sheet": sheet, "column": "1", "widths": "3" }));
    assert!(outcome.is_err(), "a number is not a column");
}
