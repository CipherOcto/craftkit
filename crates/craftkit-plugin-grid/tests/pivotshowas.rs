//! The way the **values** of a pivot are shown.
//!
//! The values may be a share of a total, a running total, or the place
//! of a value among its siblings.

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

/// A sheet of four **spendings**: ann three in 2024, bob five in 2024,
/// cid two in 2025 and ann one in 2025.
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
        ("A4", json!("cid")),
        ("B4", json!(2)),
        ("C4", json!(2025)),
        ("A5", json!("ann")),
        ("B5", json!(1)),
        ("C5", json!(2025)),
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
                "range": "A1:C5",
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

/// The **rows** of the summary.
fn rows_of(session: &mut Session, sheet: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    of_the_pivot(session, sheet)["rows"].as_array().expect("the rows are a list").clone()
}

/// The values of a pivot may be the **share of the grand total**: the
/// tree's own rule, and the share is a fraction.
#[test]
fn the_values_may_be_the_share_of_the_grand_total() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.showas",
            &json!({ "sheet": sheet, "name": "the pivot", "show_as": "percentOfGrandTotal" }),
        )
        .expect("the values are shown");
    let rows = rows_of(&mut session, sheet);
    // ann has four of the eleven, bob five and cid two.
    let ann = rows.iter().find(|row| row["key"] == json!("ann")).expect("ann is there");
    assert_eq!(ann["values"]["2024"], json!(3.0 / 11.0), "the share of ann: {ann:?}");
    let bob = rows.iter().find(|row| row["key"] == json!("bob")).expect("bob is there");
    assert_eq!(bob["values"]["2024"], json!(5.0 / 11.0), "the share of bob: {bob:?}");
    let summary = of_the_pivot(&mut session, sheet);
    assert_eq!(summary["show_as"], json!("percentOfGrandTotal"), "the way is said: {summary:?}");
}

/// The values of a pivot may be the **share of the total of the row**,
/// of the **total of the column**, a **running total** or the
/// **place** of the value among its siblings.
#[test]
fn the_values_may_be_shown_in_the_other_ways() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    // The share of the row: ann's total is four, and she has three of
    // them in 2024.
    session
        .execute(
            "grid.pivot.showas",
            &json!({ "sheet": sheet, "name": "the pivot", "show_as": "percentOfRowTotal" }),
        )
        .expect("the values are shown");
    let rows = rows_of(&mut session, sheet);
    let ann = rows.iter().find(|row| row["key"] == json!("ann")).expect("ann is there");
    assert_eq!(ann["values"]["2024"], json!(3.0 / 4.0), "the share of the row: {ann:?}");
    assert_eq!(ann["total"], json!(1.0), "and the total of the row is the whole of it: {ann:?}");
    // The place of the value: ann is the second of the three.
    session
        .execute("grid.pivot.showas", &json!({ "sheet": sheet, "name": "the pivot", "show_as": "rank" }))
        .expect("the values are shown");
    let rows = rows_of(&mut session, sheet);
    let ann = rows.iter().find(|row| row["key"] == json!("ann")).expect("ann is there");
    assert_eq!(ann["values"]["2024"], json!(2.0), "the place of ann: {ann:?}");
    let bob = rows.iter().find(|row| row["key"] == json!("bob")).expect("bob is there");
    assert_eq!(bob["values"]["2024"], json!(1.0), "and bob is the first: {bob:?}");
}

/// A way of showing that is **not one of the tree's** is refused by
/// name.
#[test]
fn a_way_of_showing_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.pivot.showas", &json!({ "sheet": sheet, "name": "the pivot", "show_as": "perCent" }))
        .expect_err("the way is refused");
    assert!(error.to_string().contains("perCent"), "the refusal names the way: {error}");
}
