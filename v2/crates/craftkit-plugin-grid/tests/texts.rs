//! The **shaping** of a text.

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

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **first** characters of a text are taken.
#[test]
fn the_first_characters_of_a_text_are_taken() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=LEFT(\"hello world\",5)");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String("hello".to_string()), "five characters");
    formula(&mut session, sheet, "A2", "=LEFT(\"hello\")");
    assert_eq!(value_of(&mut session, sheet, "A2"), Json::String("h".to_string()), "and one without a count");
}

/// The **last** characters of a text are taken.
#[test]
fn the_last_characters_of_a_text_are_taken() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=RIGHT(\"hello world\",5)");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String("world".to_string()), "five characters");
    formula(&mut session, sheet, "A2", "=RIGHT(\"hi\",9)");
    assert_eq!(value_of(&mut session, sheet, "A2"), Json::String("hi".to_string()), "and the whole text");
}

/// The characters from the **start** are taken, and the start is
/// counted from one.
#[test]
fn the_characters_from_the_start_are_taken() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=MID(\"hello world\",7,5)");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String("world".to_string()), "from the seventh");
    formula(&mut session, sheet, "A2", "=MID(\"hello world\",7)");
    assert_eq!(
        value_of(&mut session, sheet, "A2"),
        Json::String(String::new()),
        "and nothing without a count"
    );
}

/// A start that is **not one** is an error.
#[test]
fn a_start_that_is_not_one_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=MID(\"hello\",0,2)");
    let value = value_of(&mut session, sheet, "A1");
    assert_eq!(value["error"], Json::String("#VALUE!".to_string()), "it is an error");
}

/// The characters of a text are **counted**.
#[test]
fn the_characters_of_a_text_are_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=LEN(\"hello\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(5.0), "five characters");
}

/// A text is **trimmed** of the spaces that are not between words.
#[test]
fn a_text_is_trimmed_of_the_spaces_that_are_not_between_words() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=TRIM(\"  hello   world  \")");
    assert_eq!(
        value_of(&mut session, sheet, "A1"),
        Json::String("hello world".to_string()),
        "the spaces are one"
    );
}

/// A text is **folded** into a case.
#[test]
fn a_text_is_folded_into_a_case() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=UPPER(\"hello\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String("HELLO".to_string()), "the upper case");
    formula(&mut session, sheet, "A2", "=LOWER(\"HeLLo\")");
    assert_eq!(value_of(&mut session, sheet, "A2"), Json::String("hello".to_string()), "the lower case");
    formula(&mut session, sheet, "A3", "=PROPER(\"hello world\")");
    assert_eq!(value_of(&mut session, sheet, "A3"), Json::String("Hello World".to_string()), "the words");
}

/// The text of a **number** is the text it is written as.
#[test]
fn the_text_of_a_number_is_the_text_it_is_written_as() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=LEFT(12345,2)");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String("12".to_string()), "the number is a text");
    formula(&mut session, sheet, "A2", "=LEN(TRUE)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(4.0), "and the boolean is four characters");
}

/// A count that is **negative** is an error.
#[test]
fn a_count_that_is_negative_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=LEFT(\"hello\",-1)");
    let value = value_of(&mut session, sheet, "A1");
    assert_eq!(value["error"], Json::String("#VALUE!".to_string()), "it is an error");
}

/// A value that is an **error** is that error.
#[test]
fn a_value_that_is_an_error_is_that_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=LEN(1/0)");
    let value = value_of(&mut session, sheet, "A1");
    assert_eq!(value["error"], Json::String("#DIV/0!".to_string()), "the error is the error");
}
