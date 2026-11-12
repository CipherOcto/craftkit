//! The **autofit** of a row and of a column.
//!
//! A column as wide as its text, and a row as tall as its text: the
//! tree's autofit, with the measure this tree has.

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

/// **Text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// The **width** of a column, as the sheet holds it.
fn width_of(session: &Session, sheet: u64, column: &str) -> Option<f64> {
    session
        .document()
        .node(sheet)
        .and_then(|node| node.data.get("columns"))
        .and_then(|columns| columns.get(column))
        .and_then(|width| width.as_f64())
}

/// The **height** of a row, as the sheet holds it.
fn height_of(session: &Session, sheet: u64, row: u64) -> Option<f64> {
    session
        .document()
        .node(sheet)
        .and_then(|node| node.data.get("rows"))
        .and_then(|rows| rows.get(row.to_string()))
        .and_then(|height| height.as_f64())
}

/// A column is made **as wide as its text**.
#[test]
fn a_column_is_made_as_wide_as_its_text() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "hello");
    let outcome = session
        .execute("grid.column.autofit", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is fitted");
    assert_eq!(outcome.data["fitted"], json!(true), "the column is fitted");
    assert_eq!(outcome.data["width"], json!(6.0), "the text is five characters and there is a room of one");
    assert_eq!(width_of(&session, sheet, "A"), Some(6.0), "and the sheet holds the width");
}

/// The **widest line** of the column wins: the text of a cell may be
/// more than one line long.
#[test]
fn the_widest_line_of_the_column_wins() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "a\nbcd");
    a_text(&mut session, sheet, "A2", "xy");
    let outcome = session
        .execute("grid.column.autofit", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is fitted");
    assert_eq!(outcome.data["width"], json!(4.0), "the longest line is three characters long");
}

/// A column with **no text** is left as it is.
#[test]
fn a_column_with_no_text_is_left_alone() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "B1", "hello");
    let outcome = session
        .execute("grid.column.autofit", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is read");
    assert_eq!(outcome.data["fitted"], json!(false), "the column holds no text and is not fitted");
    assert_eq!(width_of(&session, sheet, "A"), None, "and it has no width");
}

/// A cell **under a merge** is not measured: the merge is the width of
/// the column, and not its text.
#[test]
fn a_cell_under_a_merge_is_not_measured() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "hello");
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A2:B2" }))
        .expect("the cells are merged");
    let outcome = session
        .execute("grid.column.autofit", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is fitted");
    assert_eq!(outcome.data["width"], json!(6.0), "the text of the merged cell is not measured");
}

/// A row is made **as tall as its text**: a line is as high as the
/// default height of a row.
#[test]
fn a_row_is_made_as_tall_as_its_text() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "one\ntwo\nthree");
    let outcome =
        session.execute("grid.row.autofit", &json!({ "sheet": sheet, "row": 1 })).expect("the row is fitted");
    assert_eq!(outcome.data["fitted"], json!(true), "the row is fitted");
    assert_eq!(outcome.data["height"], json!(45.0), "three lines of fifteen points");
    assert_eq!(height_of(&session, sheet, 1), Some(45.0), "and the sheet holds the height");
}

/// A row with **no text** is left as it is.
#[test]
fn a_row_with_no_text_is_left_alone() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "A1", "hello");
    let outcome =
        session.execute("grid.row.autofit", &json!({ "sheet": sheet, "row": 2 })).expect("the row is read");
    assert_eq!(outcome.data["fitted"], json!(false), "the row holds no text and is not fitted");
    assert_eq!(height_of(&session, sheet, 2), None, "and it has no height");
}

/// A **column** that is not a column is refused, and so is a **row**
/// that is not counted from one.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.column.autofit", &json!({ "sheet": sheet, "column": "A1" }))
        .expect_err("the column is refused");
    assert!(error.to_string().contains("not a column"), "the refusal says why: {error}");
    let error = session
        .execute("grid.row.autofit", &json!({ "sheet": sheet, "row": 0 }))
        .expect_err("the row is refused");
    assert!(error.to_string().contains("counted from one"), "the refusal says why: {error}");
}
