//! The **text** of a cell: the tree's text format and its text
//! section.
//!
//! The tree's rules: the code `@` writes the text of a cell, and a
//! format may end with a section that is the text's own.

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
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, value: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **text** a cell is written with.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell is read");
    outcome.data["text"].as_str().expect("the cell has text").to_string()
}

/// Put a **format** on a cell.
fn format(session: &mut Session, sheet: u64, cell: &str, code: &str) -> Result<(), String> {
    match session.execute("grid.cell.format", &json!({ "sheet": sheet, "cell": cell, "format": code })) {
        Ok(_) => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

/// The text format writes a **text** as it is.
#[test]
fn the_text_format_writes_a_text_as_it_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "hello");
    format(&mut session, sheet, "A1", "@").expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "hello", "the text is written as it is");
}

/// The text format writes a **number** as its own text.
#[test]
fn the_text_format_writes_a_number_as_its_own_text() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1234.5);
    format(&mut session, sheet, "A1", "@").expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "1234.5", "the number is written as its own text");
}

/// The name **text** is the tree's own, and it says the code `@`.
#[test]
fn the_name_text_says_the_code() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "hello");
    format(&mut session, sheet, "A1", "text").expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "hello", "the text is written as it is");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["format"].as_str(), Some("@"), "and the code is the one the name says");
}

/// The **fourth** section is the text's own.
#[test]
fn the_fourth_section_is_the_texts_own() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    a_text(&mut session, sheet, "A2", "hello");
    format(&mut session, sheet, "A1", "0.00;0.00;0.00;@").expect("the format is set");
    format(&mut session, sheet, "A2", "0.00;0.00;0.00;@").expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.00", "the number is written by the first section");
    assert_eq!(text_of(&mut session, sheet, "A2"), "hello", "and the text by the fourth");
}

/// The **last** section is the text's own when it is a text section.
#[test]
fn the_last_section_is_the_texts_own_when_it_is_a_text_section() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 5.0);
    a_text(&mut session, sheet, "A2", "hello");
    format(&mut session, sheet, "A1", "0.00;@").expect("the format is set");
    format(&mut session, sheet, "A2", "0.00;@").expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "5.00", "the number is written by the first section");
    assert_eq!(text_of(&mut session, sheet, "A2"), "hello", "and the text by the last");
}

/// The **literals** around the `@` are written as they are.
#[test]
fn the_literals_around_the_placeholder_are_written() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "hello");
    format(&mut session, sheet, "A1", "\"kg \"@").expect("the format is set");
    assert_eq!(text_of(&mut session, sheet, "A1"), "kg hello", "the literal is written before the text");
}

/// A code with a **fifth** section is refused.
#[test]
fn a_code_with_a_fifth_section_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = format(&mut session, sheet, "A1", "0;0;0;@;0").expect_err("the format is refused");
    assert!(error.contains("is not a format this tree has"), "the error says so: {error}");
}

/// A fourth section that is **not** a text section is refused.
#[test]
fn a_fourth_section_that_is_not_a_text_section_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = format(&mut session, sheet, "A1", "0;0;0;0").expect_err("the format is refused");
    assert!(error.contains("is not a format this tree has"), "the error says so: {error}");
}

/// A section with an `@` and a **digit** in it is not a text section.
#[test]
fn a_section_with_a_digit_in_it_is_not_a_text_section() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = format(&mut session, sheet, "A1", "0.00@").expect_err("the format is refused");
    assert!(error.contains("is not a format this tree has"), "the error says so: {error}");
}
