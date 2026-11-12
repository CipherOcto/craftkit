//! The **sorting** of a range.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// Put a **formula** in a cell.
fn formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **values** of a range, as the answer holds them.
fn values_of(session: &mut Session, sheet: u64, range: &str) -> Vec<Json> {
    let outcome = session
        .execute("grid.range.read", &json!({ "sheet": sheet, "range": range }))
        .expect("the range is read");
    outcome.data["values"].as_array().expect("the values are a list").clone()
}

/// The rows are put in **order**.
#[test]
fn the_rows_are_put_in_order() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(3));
    value(&mut session, sheet, "A2", json!(1));
    value(&mut session, sheet, "A3", json!(2));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], json!(1.0), "the one is first");
    assert_eq!(values[2][0], json!(3.0), "and the three is last");
}

/// The order may be **descending**.
#[test]
fn the_order_may_be_descending() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(3));
    value(&mut session, sheet, "A2", json!(1));
    value(&mut session, sheet, "A3", json!(2));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A", "descending": true }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], json!(3.0), "the three is first");
    assert_eq!(values[2][0], json!(1.0), "and the one is last");
}

/// The **empty** cells are last, whatever the order is.
#[test]
fn the_empty_cells_are_last_whatever_the_order_is() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(3));
    value(&mut session, sheet, "A2", json!(1));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[1][0], json!(3.0), "the three is under the one");
    assert_eq!(values[2][0], Json::Null, "and the empty cell is last");
}

/// The text is compared **without case**.
#[test]
fn the_text_is_compared_without_case() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("b"));
    value(&mut session, sheet, "A2", json!("A"));
    value(&mut session, sheet, "A3", json!("c"));
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], Json::String("A".to_string()), "the A is first");
    assert_eq!(values[2][0], Json::String("c".to_string()), "and the c is last");
}

/// A **header** is guessed and stays where it is.
#[test]
fn a_header_is_guessed_and_stays_where_it_is() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("name"));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(1));
    let outcome = session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A3", "column": "A" }))
        .expect("the range is sorted");
    assert_eq!(outcome.data["header"], Json::Bool(true), "the first row is a header");
    let values = values_of(&mut session, sheet, "A1:A3");
    assert_eq!(values[0][0], Json::String("name".to_string()), "the header stays");
    assert_eq!(values[1][0], json!(1.0), "and the rows are in order");
}

/// A **formula** moves with its row, and its relative references move
/// with it.
#[test]
fn a_formula_moves_with_its_row() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(5));
    value(&mut session, sheet, "A2", json!(1));
    formula(&mut session, sheet, "B1", "=A1*2");
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:B2", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:B2");
    assert_eq!(values[1][0], json!(5.0), "the five moved down");
    assert_eq!(values[1][1], json!(10.0), "and the formula still reads the row it is on");
}

/// An **anchored** reference does not move.
#[test]
fn an_anchored_reference_does_not_move() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(5));
    value(&mut session, sheet, "A2", json!(1));
    formula(&mut session, sheet, "B1", "=$A$1*2");
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:B2", "column": "A" }))
        .expect("the range is sorted");
    let values = values_of(&mut session, sheet, "A1:B2");
    assert_eq!(values[1][1], json!(2.0), "the formula still reads the first cell");
}

/// The **note** and the **link** travel with the row.
#[test]
fn the_note_and_the_link_travel_with_the_row() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    value(&mut session, sheet, "A2", json!(1));
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "about the two" }))
        .expect("the note is written");
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "somewhere" }))
        .expect("the link is written");
    session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A2", "column": "A" }))
        .expect("the range is sorted");
    let outcome = session
        .execute("grid.cell.note.get", &json!({ "sheet": sheet, "cell": "A2" }))
        .expect("the note is read");
    assert_eq!(outcome.data["note"]["text"], Json::String("about the two".to_string()), "the note moved");
    let outcome = session
        .execute("grid.cell.link.get", &json!({ "sheet": sheet, "cell": "A2" }))
        .expect("the link is read");
    assert_eq!(outcome.data["link"]["target"], Json::String("somewhere".to_string()), "and the link");
}

/// A column that is **not in the range** is refused.
#[test]
fn a_column_that_is_not_in_the_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    let error = session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:A2", "column": "C" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("not in the range"), "it says why: {error}");
}

/// A range with a **merge** in it is refused.
#[test]
fn a_range_with_a_merge_in_it_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(2));
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A2:B2" }))
        .expect("the merge is set");
    let error = session
        .execute("grid.sort", &json!({ "sheet": sheet, "range": "A1:B2", "column": "A" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("merge"), "it says why: {error}");
}
