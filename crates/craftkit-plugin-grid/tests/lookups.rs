//! The **finding** of a value in a range.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// A **table** of three rows: a number and a number beside it.
fn a_table(session: &mut Session, sheet: u64) {
    value(session, sheet, "A1", json!(1));
    value(session, sheet, "B1", json!(10));
    value(session, sheet, "A2", json!(2));
    value(session, sheet, "B2", json!(20));
    value(session, sheet, "A3", json!(3));
    value(session, sheet, "B3", json!(30));
}

/// The **place** of a value in a range is found.
#[test]
fn the_place_of_a_value_in_a_range_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=MATCH(2,A1:A3)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(2.0), "the second place");
}

/// The **wildcards** are the tree's, and the text is compared without
/// case.
#[test]
fn the_wildcards_are_the_trees() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("apple"));
    value(&mut session, sheet, "A2", json!("banana"));
    value(&mut session, sheet, "A3", json!("cherry"));
    formula(&mut session, sheet, "D1", "=MATCH(\"B*\",A1:A3,0)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(2.0), "the banana is the second");
    formula(&mut session, sheet, "D2", "=MATCH(\"CHERRY\",A1:A3,0)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(3.0), "and the case is not the case");
}

/// A value that is **not there** is not found.
#[test]
fn a_value_that_is_not_there_is_not_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=MATCH(99,A1:A3,0)");
    let value = value_of(&mut session, sheet, "D1");
    assert_eq!(value["error"], Json::String("#N/A".to_string()), "it is not found");
}

/// The **approximate** search is refused.
#[test]
fn the_approximate_search_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=MATCH(2.5,A1:A3,1)");
    let value = value_of(&mut session, sheet, "D1");
    assert!(
        value["error"].as_str().is_some_and(|message| message.contains("not supported yet")),
        "it says why: {value}"
    );
}

/// The value of a **column** of the row that holds the needle is
/// found.
#[test]
fn the_value_of_a_column_of_the_row_that_holds_the_needle_is_found() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=VLOOKUP(2,A1:B3,2,FALSE)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(20.0), "the twenty");
    formula(&mut session, sheet, "D2", "=VLOOKUP(3,A1:B3,2,FALSE)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(30.0), "and the thirty");
}

/// A lookup that does not say that it is **exact** is refused, because
/// the tree's default is the approximate search.
#[test]
fn a_lookup_that_does_not_say_that_it_is_exact_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=VLOOKUP(2,A1:B3,2)");
    let value = value_of(&mut session, sheet, "D1");
    assert!(
        value["error"].as_str().is_some_and(|message| message.contains("not supported yet")),
        "it says why: {value}"
    );
}

/// A column that is **not there** is an error.
#[test]
fn a_column_that_is_not_there_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=VLOOKUP(2,A1:B3,3,FALSE)");
    let value = value_of(&mut session, sheet, "D1");
    assert_eq!(value["error"], Json::String("#REF!".to_string()), "it is an error");
}

/// The value at a **place** is taken.
#[test]
fn the_value_at_a_place_is_taken() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=INDEX(A1:B3,2,2)");
    assert_eq!(value_of(&mut session, sheet, "D1"), json!(20.0), "the twenty");
    formula(&mut session, sheet, "D2", "=INDEX(A1:B3,3,1)");
    assert_eq!(value_of(&mut session, sheet, "D2"), json!(3.0), "and the three");
}

/// A place that is **not there** is an error, and a whole row is an
/// array — and this tree has no arrays.
#[test]
fn a_place_that_is_not_there_is_an_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=INDEX(A1:B3,4,1)");
    let value = value_of(&mut session, sheet, "D1");
    assert_eq!(value["error"], Json::String("#REF!".to_string()), "it is an error");
    formula(&mut session, sheet, "D2", "=INDEX(A1:B3,1)");
    let value = value_of(&mut session, sheet, "D2");
    assert!(
        value["error"].as_str().is_some_and(|message| message.contains("not supported yet")),
        "and a whole row is an array: {value}"
    );
}

/// A needle that is an **error** is that error.
#[test]
fn a_needle_that_is_an_error_is_that_error() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    a_table(&mut session, sheet);
    formula(&mut session, sheet, "D1", "=MATCH(1/0,A1:A3,0)");
    let value = value_of(&mut session, sheet, "D1");
    assert_eq!(value["error"], Json::String("#DIV/0!".to_string()), "the error is the error");
}
