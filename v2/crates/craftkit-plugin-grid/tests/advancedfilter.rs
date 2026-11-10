//! The **advanced filter** of a sheet: the tree's own advanced filter.

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
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// A **cell** that holds a text.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// A **cell** that holds a number.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **value** of a cell.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// A sheet with a **list** of three rows and a **criteria** range.
fn a_list(session: &mut Session) -> (u64, u64) {
    let (workbook, sheet) = a_workbook(session);
    a_text(session, sheet, "A1", "Name");
    a_text(session, sheet, "B1", "Price");
    a_text(session, sheet, "A2", "apple");
    a_number(session, sheet, "B2", 10.0);
    a_text(session, sheet, "A3", "pear");
    a_number(session, sheet, "B3", 20.0);
    a_text(session, sheet, "A4", "apricot");
    a_number(session, sheet, "B4", 30.0);
    // The **criteria**: the headings, and one row of criteria.
    a_text(session, sheet, "D1", "Name");
    a_text(session, sheet, "E1", "Price");
    (workbook, sheet)
}

/// The **rows** the advanced filter keeps.
fn kept(session: &mut Session, sheet: u64, criteria: &str) -> Vec<u64> {
    let outcome = session
        .execute("grid.filter.advanced", &json!({ "sheet": sheet, "range": "A1:B4", "criteria": criteria }))
        .expect("the list is filtered");
    outcome.data["rows"]
        .as_array()
        .map(|rows| rows.iter().filter_map(|row| row.as_u64()).collect())
        .unwrap_or_default()
}

/// The rows that **match** the criteria are kept: the tree's own rule.
#[test]
fn the_rows_that_match_are_kept() {
    let mut session = session();
    let (_workbook, sheet) = a_list(&mut session);
    a_text(&mut session, sheet, "D2", "apple");
    // The rows of this tree are counted from **zero**: the apple is in
    // the second row of the sheet, and that is the row `1`.
    assert_eq!(kept(&mut session, sheet, "D1:E2"), vec![1], "the row that matches is kept");
}

/// The **columns** of a criteria row are judged **together**: the
/// tree's own rule.
#[test]
fn the_columns_of_a_criteria_row_are_judged_together() {
    let mut session = session();
    let (_workbook, sheet) = a_list(&mut session);
    a_text(&mut session, sheet, "D2", "ap");
    a_text(&mut session, sheet, "E2", ">15");
    // The apple at ten does not pass the price, the pear does not
    // begin with "ap", and the apricot at thirty does both: so only
    // the apricot is kept.
    assert_eq!(kept(&mut session, sheet, "D1:E2"), vec![3], "one row matches both");
}

/// The **rows** of the criteria are judged **one by one**: the tree's
/// own rule.
#[test]
fn the_rows_of_the_criteria_are_judged_one_by_one() {
    let mut session = session();
    let (_workbook, sheet) = a_list(&mut session);
    a_text(&mut session, sheet, "D2", "apple");
    a_text(&mut session, sheet, "D3", "pear");
    assert_eq!(kept(&mut session, sheet, "D1:E3"), vec![1, 2], "either row is kept");
}

/// A **text criterion begins with** the text: the tree's own rule for
/// an advanced filter, and it is not the rule of a counting function.
#[test]
fn a_text_criterion_begins_with() {
    let mut session = session();
    let (_workbook, sheet) = a_list(&mut session);
    a_text(&mut session, sheet, "D2", "ap");
    assert_eq!(kept(&mut session, sheet, "D1:E2"), vec![1, 3], "the rows that begin with it are kept");
}

/// A criterion whose **label** is not a heading of the list is
/// refused: the tree's own rule.
#[test]
fn a_criterion_with_a_label_that_is_not_a_heading_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_list(&mut session);
    a_text(&mut session, sheet, "D2", "apple");
    a_text(&mut session, sheet, "F1", "Colour");
    a_text(&mut session, sheet, "F2", "red");
    let error = session
        .execute("grid.filter.advanced", &json!({ "sheet": sheet, "range": "A1:B4", "criteria": "F1:F2" }))
        .expect_err("the filter is refused");
    assert!(error.to_string().contains("does not match a column"), "the error says why: {error}");
}

/// The **copy**: the heading row and the rows that are kept are
/// written to the range the caller names: the tree's own rule.
#[test]
fn the_rows_that_match_are_copied() {
    let mut session = session();
    let (_workbook, sheet) = a_list(&mut session);
    a_text(&mut session, sheet, "D2", "pear");
    let outcome = session
        .execute(
            "grid.filter.advanced",
            &json!({
                "sheet": sheet,
                "range": "A1:B4",
                "criteria": "D1:E2",
                "copy_to": "G1",
            }),
        )
        .expect("the rows are copied");
    assert_eq!(outcome.data["matched"], json!(1), "one row matched");
    // The **heading** row is copied too.
    assert_eq!(value_of(&mut session, sheet, "G1"), json!("Name"), "the heading is copied");
    assert_eq!(value_of(&mut session, sheet, "H1"), json!("Price"), "and the rest of it");
    assert_eq!(value_of(&mut session, sheet, "G2"), json!("pear"), "and the row that matched");
    assert_eq!(value_of(&mut session, sheet, "H2"), json!(20.0), "with its numbers");
}

/// The range the rows are copied to may not **overlap** the list: the
/// tree's own rule.
#[test]
fn a_destination_that_overlaps_the_list_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_list(&mut session);
    a_text(&mut session, sheet, "D2", "apple");
    let error = session
        .execute(
            "grid.filter.advanced",
            &json!({
                "sheet": sheet,
                "range": "A1:B4",
                "criteria": "D1:E2",
                "copy_to": "A1",
            }),
        )
        .expect_err("the copy is refused");
    assert!(error.to_string().contains("overlaps"), "the error says why: {error}");
}

/// **Only the first** row of a set of the same values is kept: the
/// tree's own rule.
#[test]
fn only_the_first_of_a_set_of_the_same_values_is_kept() {
    let mut session = session();
    let (_workbook, sheet) = a_list(&mut session);
    a_text(&mut session, sheet, "A5", "apple");
    a_number(&mut session, sheet, "B5", 10.0);
    a_text(&mut session, sheet, "D2", "apple");
    let outcome = session
        .execute(
            "grid.filter.advanced",
            &json!({
                "sheet": sheet,
                "range": "A1:B5",
                "criteria": "D1:E2",
                "unique": true,
            }),
        )
        .expect("the list is filtered");
    assert_eq!(outcome.data["rows"], json!([1]), "only the first row is kept");
}
