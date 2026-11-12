//! The **import** of a text into a sheet.

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

/// A **workbook**.
fn a_workbook(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    outcome.data["id"].as_u64().expect("the workbook has an id")
}

/// A **text** imported into a workbook, and the sheet it made.
fn an_import(session: &mut Session, workbook: u64, text: &str) -> (u64, u64) {
    let outcome = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": text }))
        .expect("the text is imported");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (sheet, outcome.data["cells"].as_u64().unwrap_or_default())
}

/// The **value** of a cell.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Option<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    let value = outcome.data["value"].clone();
    if value.is_null() { None } else { Some(value) }
}

/// **A text of values becomes a sheet of cells.**
#[test]
fn a_text_of_values_becomes_a_sheet() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let (sheet, cells) = an_import(&mut session, workbook, "who,amount\nann,3\nbob,5");
    assert_eq!(cells, 6, "six cells were filled");
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!("who")));
    assert_eq!(value_of(&mut session, sheet, "B1"), Some(json!("amount")));
    assert_eq!(value_of(&mut session, sheet, "A2"), Some(json!("ann")));
    assert_eq!(value_of(&mut session, sheet, "B2"), Some(json!(3.0)));
    assert_eq!(value_of(&mut session, sheet, "B3"), Some(json!(5.0)));
}

/// **A number is a number and the rest of it is text.**
#[test]
fn a_number_is_a_number_and_the_rest_is_text() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let (sheet, _) = an_import(&mut session, workbook, "3,3.5,three, 4 ");
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!(3.0)));
    assert_eq!(value_of(&mut session, sheet, "B1"), Some(json!(3.5)));
    assert_eq!(value_of(&mut session, sheet, "C1"), Some(json!("three")));
    assert_eq!(value_of(&mut session, sheet, "D1"), Some(json!(4.0)), "the spaces are around it");
}

/// **An empty field is nothing.**
#[test]
fn an_empty_field_is_nothing() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let (sheet, cells) = an_import(&mut session, workbook, "a,,c");
    assert_eq!(cells, 2, "the empty field is not a cell");
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!("a")));
    assert_eq!(value_of(&mut session, sheet, "B1"), None, "and the cell is empty");
    assert_eq!(value_of(&mut session, sheet, "C1"), Some(json!("c")));
}

/// **A quoted field may hold the delimiter, and a quote in it is
/// written twice.**
#[test]
fn a_quoted_field_may_hold_the_delimiter() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let (sheet, _) = an_import(&mut session, workbook, "\"a,b\",\"say \"\"hi\"\"\"");
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!("a,b")));
    assert_eq!(value_of(&mut session, sheet, "B1"), Some(json!("say \"hi\"")));
}

/// **Rows of different lengths are refused.**
#[test]
fn rows_of_different_lengths_are_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let failure = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": "a,b\nc" }))
        .expect_err("the text is refused");
    let message = failure.to_string();
    assert!(message.contains("the rows of a text are the same length"), "the refusal says why: {message}");
}

/// **A quoted field that is never closed is refused.**
#[test]
fn a_quoted_field_that_is_never_closed_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let failure = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": "\"a,b" }))
        .expect_err("the text is refused");
    let message = failure.to_string();
    assert!(message.contains("never closed"), "the refusal says why: {message}");
}

/// **A delimiter that is not one character is refused.**
#[test]
fn a_delimiter_that_is_not_one_character_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let failure = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": "a,b", "delimiter": ";;" }))
        .expect_err("the delimiter is refused");
    let message = failure.to_string();
    assert!(message.contains("a delimiter is one character"), "the refusal says why: {message}");
}

/// **The delimiter may be another character.**
#[test]
fn the_delimiter_may_be_another_character() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let outcome = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": "a;b", "delimiter": ";" }))
        .expect("the text is imported");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!("a")));
    assert_eq!(value_of(&mut session, sheet, "B1"), Some(json!("b")));
}

/// **The sheet is named, and the name may be given.**
#[test]
fn the_sheet_is_named() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let outcome = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": "a", "name": "the numbers" }))
        .expect("the text is imported");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    let node = session.document().node(sheet).expect("the sheet is in the document");
    assert_eq!(node.name, "the numbers");
}

/// **A field that reads like a formula is text.**
#[test]
fn a_field_that_reads_like_a_formula_is_text() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let (sheet, _) = an_import(&mut session, workbook, "=1+1");
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!("=1+1")));
    let node = session.document().node(sheet).expect("the sheet is in the document");
    let entry = node.data["cells"]["A1"].clone();
    assert!(entry.get("formula").is_none(), "and it is not a formula: {entry}");
}

/// **A text that holds nothing is refused.**
#[test]
fn a_text_that_holds_nothing_is_refused() {
    let mut session = session();
    let workbook = a_workbook(&mut session);
    let failure = session
        .execute("grid.import.csv", &json!({ "workbook": workbook, "text": "" }))
        .expect_err("the text is refused");
    let message = failure.to_string();
    assert!(message.contains("holds nothing to import"), "the refusal says why: {message}");
}
