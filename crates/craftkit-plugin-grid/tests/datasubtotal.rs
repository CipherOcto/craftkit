//! The **subtotal** of a group of rows.
//!
//! A total row goes under each group of the rows, and a grand total
//! goes at the end.

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

/// A **sheet** with a group column and a column of numbers.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    for (cell, value) in [
        ("A1", json!("Group")),
        ("B1", json!("Value")),
        ("A2", json!("East")),
        ("B2", json!(10)),
        ("A3", json!("East")),
        ("B3", json!(20)),
        ("A4", json!("West")),
        ("B4", json!(5)),
    ] {
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
            .expect("the cell is set");
    }
    sheet
}

/// The **text** a cell shows.
fn text_of(session: &mut Session, sheet: u64, cell: &str) -> String {
    let outcome = session
        .execute("grid.cell.text", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the cell is read");
    outcome.data["text"].as_str().unwrap_or_default().to_string()
}

/// A total row goes under each **group** of the rows, and a grand
/// total goes at the end.
#[test]
fn a_total_row_goes_under_each_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.data.subtotal", &json!({ "sheet": sheet, "range": "A1:B4", "group_by": "A" }))
        .expect("the subtotals are put in");
    assert_eq!(outcome.data["groups"], json!(2), "there are two groups: {outcome:?}");
    assert_eq!(text_of(&mut session, sheet, "A4"), "East Total", "the first total row is under its group");
    assert_eq!(text_of(&mut session, sheet, "B4"), "30", "and it totals the group");
    assert_eq!(text_of(&mut session, sheet, "A6"), "West Total", "the second total row is under its group");
    assert_eq!(text_of(&mut session, sheet, "B6"), "5", "and it totals the group");
    assert_eq!(text_of(&mut session, sheet, "A7"), "Grand Total", "the grand total is at the end");
    assert_eq!(text_of(&mut session, sheet, "B7"), "35", "and it totals the detail");
}

/// The rows of each group are **grouped**, and the summary of the
/// sheet goes below.
#[test]
fn the_rows_of_each_group_are_grouped() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.data.subtotal", &json!({ "sheet": sheet, "range": "A1:B4", "group_by": "A" }))
        .expect("the subtotals are put in");
    let outcome =
        session.execute("grid.row.groups", &json!({ "sheet": sheet })).expect("the groups are read");
    let groups = outcome.data["groups"].as_array().cloned().unwrap_or_default();
    assert_eq!(groups.len(), 2, "there are two groups of rows: {outcome:?}");
    let outcome =
        session.execute("grid.sheet.summary", &json!({ "sheet": sheet })).expect("the summary is read");
    assert_eq!(outcome.data["below"], json!(true), "the summary goes below: {outcome:?}");
}

/// The way of totalling that is **not one of the tree's** is refused by
/// name, and so is a range that is not a range.
#[test]
fn a_way_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.data.subtotal",
            &json!({ "sheet": sheet, "range": "A1:B4", "group_by": "A", "function": "median" }),
        )
        .expect_err("the way is refused");
    assert!(error.to_string().contains("median"), "the refusal names the way: {error}");
    let error = session
        .execute("grid.data.subtotal", &json!({ "sheet": sheet, "range": "not a range", "group_by": "A" }))
        .expect_err("the range is refused");
    assert!(error.to_string().contains("not a range of cells"), "the refusal says why: {error}");
}

/// The way of totalling is the tree's own: the **count** counts the
/// rows, and the **average** divides them.
#[test]
fn the_way_of_totalling_is_the_trees_own() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.data.subtotal",
            &json!({ "sheet": sheet, "range": "A1:B4", "group_by": "A", "function": "count" }),
        )
        .expect("the subtotals are put in");
    assert_eq!(text_of(&mut session, sheet, "A4"), "East Count", "the count is counted");
    assert_eq!(text_of(&mut session, sheet, "B4"), "2", "and it counts the rows of the group");
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.data.subtotal",
            &json!({ "sheet": sheet, "range": "A1:B4", "group_by": "A", "function": "average" }),
        )
        .expect("the subtotals are put in");
    assert_eq!(text_of(&mut session, sheet, "B4"), "15", "the average is of the group");
}
