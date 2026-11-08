//! The **grand totals** of a pivot.
//!
//! The grand total of the rows is the column at the right, and the
//! grand total of the columns is the line at the bottom.

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

/// A sheet of three **spendings**: ann three in 2024, bob five in 2024
/// and ann two in 2025.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    for (cell, value) in [
        ("A1", json!("who")),
        ("B1", json!("amount")),
        ("C1", json!("when")),
        ("A2", json!("ann")),
        ("B2", json!(3)),
        ("C2", json!(2024)),
        ("A3", json!("bob")),
        ("B3", json!(5)),
        ("C3", json!(2024)),
        ("A4", json!("ann")),
        ("B4", json!(2)),
        ("C4", json!(2025)),
    ] {
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
            .expect("the cell is set");
    }
    session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": "the pivot",
                "range": "A1:C4",
                "row": "A",
                "column": "C",
                "value": "B",
            }),
        )
        .expect("the pivot is made");
    sheet
}

/// The **summary** of the pivot.
fn of_the_pivot(session: &mut Session, sheet: u64) -> craftkit_sdk::serde_json::Value {
    let outcome = session
        .execute("grid.pivot.read", &json!({ "sheet": sheet, "name": "the pivot" }))
        .expect("the pivot is read");
    outcome.data.clone()
}

/// A pivot has the **grand totals** by default: the tree's own default.
#[test]
fn a_pivot_has_the_grand_totals_by_default() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let summary = of_the_pivot(&mut session, sheet);
    assert_eq!(summary["grand_totals_rows"], json!(true), "the rows are totalled: {summary:?}");
    assert_eq!(summary["grand_totals_cols"], json!(true), "the columns are totalled: {summary:?}");
    assert_eq!(
        summary["grand_total"]["total"],
        json!(10.0),
        "and the total of everything is the sum of it all: {summary:?}"
    );
    assert_eq!(
        summary["grand_total"]["columns"]["2024"],
        json!(8.0),
        "and the total of a column is the sum of it: {summary:?}"
    );
}

/// The grand total of the **rows** is the total of the row, and it is
/// not there when the totals are turned off.
#[test]
fn the_grand_total_of_the_rows_is_the_total_of_the_row() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let summary = of_the_pivot(&mut session, sheet);
    let rows = summary["rows"].as_array().expect("the rows are a list");
    let ann = rows.iter().find(|row| row["key"] == json!("ann")).expect("ann is there");
    assert_eq!(ann["total"], json!(5.0), "the total of ann is the sum of her spendings: {ann:?}");
    session
        .execute("grid.pivot.grandtotals", &json!({ "sheet": sheet, "name": "the pivot", "mode": "columns" }))
        .expect("the totals are placed");
    let summary = of_the_pivot(&mut session, sheet);
    let rows = summary["rows"].as_array().expect("the rows are a list");
    let ann = rows.iter().find(|row| row["key"] == json!("ann")).expect("ann is there");
    assert!(ann.get("total").is_none(), "and the total of the row is not there: {ann:?}");
    assert_eq!(summary["grand_total"]["total"], json!(10.0), "and the grand total still is");
}

/// The mode that is **not one of the tree's** is refused by name.
#[test]
fn a_mode_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.pivot.grandtotals", &json!({ "sheet": sheet, "name": "the pivot", "mode": "some" }))
        .expect_err("the mode is refused");
    assert!(error.to_string().contains("some"), "the refusal names the mode: {error}");
    let error = session
        .execute("grid.pivot.grandtotals", &json!({ "sheet": sheet, "name": "nothing", "mode": "on" }))
        .expect_err("the pivot is refused");
    assert!(error.to_string().contains("nothing"), "the refusal names the pivot: {error}");
}

/// The mode **off** takes both totals away.
#[test]
fn the_mode_off_takes_both_totals_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.pivot.grandtotals", &json!({ "sheet": sheet, "name": "the pivot", "mode": "off" }))
        .expect("the totals are placed");
    let summary = of_the_pivot(&mut session, sheet);
    assert_eq!(summary["grand_totals_rows"], json!(false), "the rows are not totalled: {summary:?}");
    assert_eq!(summary["grand_totals_cols"], json!(false), "the columns are not totalled: {summary:?}");
    assert!(summary["grand_total"].is_null(), "and there is no grand total: {summary:?}");
}
