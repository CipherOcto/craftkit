//! The **flash fill** of a column.
//!
//! The tree's flash fill: the column beside the cell is the source, the
//! rows that are filled are the **examples**, and what the examples say
//! is a **program** — literals, tokens of the source and the first
//! letter of a token — that is then run on the rows that are empty.

use craftkit_sdk::serde_json::{Value, json};
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

/// Put a **text** in a cell.
fn put(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// The **value** of a cell, as text.
fn text_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["value"].as_str()?.to_string())
}

/// The **format** of a cell.
fn format_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["format"].as_str()?.to_string())
}

/// The flash fill learns a **token** and writes the rows that are empty.
#[test]
fn a_token_is_learned_and_filled() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "John Smith");
    put(&mut session, sheet, "B1", "Smith, John");
    put(&mut session, sheet, "A2", "Ada Lovelace");
    session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" })).expect("the column is filled");
    assert_eq!(text_of(&session, sheet, "B2").as_deref(), Some("Lovelace, Ada"), "the second cell is filled");
}

/// The flash fill learns the **initials** of the source.
#[test]
fn the_initials_are_learned() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "jane doe");
    put(&mut session, sheet, "B1", "JD");
    put(&mut session, sheet, "A2", "alan turing");
    session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" })).expect("the column is filled");
    assert_eq!(text_of(&session, sheet, "B2").as_deref(), Some("AT"), "the initials are written");
}

/// The flash fill learns a **literal** between the tokens.
#[test]
fn a_literal_is_learned() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "alice@example.com");
    put(&mut session, sheet, "B1", "alice");
    put(&mut session, sheet, "A2", "bob@example.com");
    session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" })).expect("the column is filled");
    assert_eq!(text_of(&session, sheet, "B2").as_deref(), Some("bob"), "the first token is written");
}

/// The flash fill reads the **shown text** of a cell: a date is the date
/// it shows and not the number it holds.
#[test]
fn the_shown_text_is_read() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A1", "date": "2024-01-15" }))
        .expect("the date is set");
    put(&mut session, sheet, "B1", "15/01/2024");
    session
        .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": "A2", "date": "2023-12-03" }))
        .expect("the date is set");
    session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" })).expect("the column is filled");
    assert_eq!(
        text_of(&session, sheet, "B2").as_deref(),
        Some("03/12/2023"),
        "the date is written as the example wrote it"
    );
}

/// The fill takes the **format** of the top of the column it fills.
#[test]
fn the_format_of_the_column_is_taken() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "John Smith");
    put(&mut session, sheet, "B1", "Smith, John");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "B1", "format": "0" }))
        .expect("the cell is formatted");
    put(&mut session, sheet, "A2", "Ada Lovelace");
    session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" })).expect("the column is filled");
    assert_eq!(format_of(&session, sheet, "B2").as_deref(), Some("0"), "the format is taken");
}

/// The **first column** has nothing beside it to learn from.
#[test]
fn the_first_column_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "John Smith");
    put(&mut session, sheet, "A2", "Ada Lovelace");
    let outcome = session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "A2" }));
    assert!(outcome.is_err(), "the first column is refused");
}

/// A column with **no examples** has nothing to learn from.
#[test]
fn a_column_with_no_examples_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "John Smith");
    put(&mut session, sheet, "A2", "Ada Lovelace");
    let outcome = session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" }));
    assert!(outcome.is_err(), "the column with no examples is refused");
}

/// Examples that do not **share a program** are refused: the tree
/// refuses to guess.
#[test]
fn examples_that_share_no_program_are_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "abc");
    put(&mut session, sheet, "B1", "xyz");
    put(&mut session, sheet, "A2", "def");
    put(&mut session, sheet, "B2", "123");
    put(&mut session, sheet, "A3", "ghi");
    let outcome = session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B3" }));
    assert!(outcome.is_err(), "the examples that share no program are refused");
}

/// A column that is **already full** has nothing to fill.
#[test]
fn a_full_column_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "John Smith");
    put(&mut session, sheet, "B1", "Smith, John");
    put(&mut session, sheet, "A2", "Ada Lovelace");
    put(&mut session, sheet, "B2", "Lovelace, Ada");
    let outcome = session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" }));
    assert!(outcome.is_err(), "the full column is refused");
}

/// A cell that holds a **formula** is not empty: the fill does not
/// write over it.
#[test]
fn a_formula_cell_is_not_empty() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "John Smith");
    put(&mut session, sheet, "B1", "Smith, John");
    put(&mut session, sheet, "A2", "Ada Lovelace");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B2", "formula": "1+1" }))
        .expect("the formula is set");
    let outcome = session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" }));
    assert!(outcome.is_err(), "the column with nothing to fill is refused");
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = &node.data["cells"]["B2"];
    assert_eq!(entry["formula"].as_str(), Some("1+1"), "the formula is still there");
}

/// The **payload** of the outcome says what was written.
#[test]
fn the_outcome_says_what_was_written() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "John Smith");
    put(&mut session, sheet, "B1", "Smith, John");
    put(&mut session, sheet, "A2", "Ada Lovelace");
    let outcome = session
        .execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" }))
        .expect("the column is filled");
    let written = outcome.data["cells"].as_array().expect("the cells are named");
    assert_eq!(written.len(), 1, "one cell was written");
    assert_eq!(written[0]["cell"].as_str(), Some("B2"), "the cell is named");
    assert_eq!(written[0]["value"].as_str(), Some("Lovelace, Ada"), "and the value is given");
}

/// The value that is written is a **text**: the program writes text.
#[test]
fn the_written_value_is_a_text() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "John Smith");
    put(&mut session, sheet, "B1", "Smith, John");
    put(&mut session, sheet, "A2", "Ada Lovelace");
    session.execute("grid.flash.fill", &json!({ "sheet": sheet, "at": "B2" })).expect("the column is filled");
    let node = session.document().node(sheet).expect("the sheet is there");
    let value: &Value = &node.data["cells"]["B2"]["value"];
    assert!(value.is_string(), "the value is a text");
}
