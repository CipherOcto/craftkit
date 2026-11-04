//! The **joining** and the **finding** of texts.

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

/// The texts are **joined**.
#[test]
fn the_texts_are_joined() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=CONCATENATE(\"hello\",\" \",\"world\")");
    assert_eq!(
        value_of(&mut session, sheet, "A1"),
        Json::String("hello world".to_string()),
        "the texts are joined"
    );
}

/// The texts are joined with a **delimiter**, and the empty ones may
/// be left out.
#[test]
fn the_texts_are_joined_with_a_delimiter() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=TEXTJOIN(\", \",TRUE,\"a\",\"\",\"b\")");
    assert_eq!(
        value_of(&mut session, sheet, "A1"),
        Json::String("a, b".to_string()),
        "the empty one is left out"
    );
    formula(&mut session, sheet, "A2", "=TEXTJOIN(\", \",FALSE,\"a\",\"\",\"b\")");
    assert_eq!(value_of(&mut session, sheet, "A2"), Json::String("a, , b".to_string()), "and here it is not");
}

/// A text is **found** in another, and the place is counted from one.
#[test]
fn a_text_is_found_in_another() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=FIND(\"lo\",\"hello\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(4.0), "the fourth character");
    formula(&mut session, sheet, "A2", "=FIND(\"l\",\"hello\",4)");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(4.0), "and the search may begin later");
}

/// The **search** folds the case and takes the wildcards.
#[test]
fn the_search_folds_the_case_and_takes_the_wildcards() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=SEARCH(\"LO\",\"hello\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), json!(4.0), "the case is not the case");
    formula(&mut session, sheet, "A2", "=SEARCH(\"h*o\",\"hello\")");
    assert_eq!(value_of(&mut session, sheet, "A2"), json!(1.0), "and the wildcards are the tree's");
}

/// A text that is **not there** is not found.
#[test]
fn a_text_that_is_not_there_is_not_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=FIND(\"z\",\"hello\")");
    let value = value_of(&mut session, sheet, "A1");
    assert_eq!(value["error"], Json::String("#VALUE!".to_string()), "it is an error");
}

/// A part of a text is **changed**.
#[test]
fn a_part_of_a_text_is_changed() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=SUBSTITUTE(\"hello\",\"l\",\"L\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String("heLlo".to_string()), "the first one");
    formula(&mut session, sheet, "A2", "=SUBSTITUTE(\"hello\",\"l\",\"L\",2)");
    assert_eq!(value_of(&mut session, sheet, "A2"), Json::String("helLo".to_string()), "and the second");
}

/// A part of a text is **replaced**.
#[test]
fn a_part_of_a_text_is_replaced() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=REPLACE(\"hello\",2,3,\"XY\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String("hXYo".to_string()), "the three characters");
}

/// The text of a value that is **not a text** is nothing.
#[test]
fn the_text_of_a_value_that_is_not_a_text_is_nothing() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=T(7)");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String(String::new()), "a number is not a text");
    formula(&mut session, sheet, "A2", "=T(\"x\")");
    assert_eq!(value_of(&mut session, sheet, "A2"), Json::String("x".to_string()), "and a text is");
}

/// The **control characters** are cleaned away.
#[test]
fn the_control_characters_are_cleaned_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    formula(&mut session, sheet, "A1", "=CLEAN(\"a\tb\")");
    assert_eq!(value_of(&mut session, sheet, "A1"), Json::String("ab".to_string()), "the tab is gone");
}
