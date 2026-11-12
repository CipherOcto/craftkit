//! The **items** of a pivot that are kept.
//!
//! The rows whose field is not selected are left out of the pivot.

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

/// The rows whose **field** is not selected are left out of the pivot.
#[test]
fn the_rows_whose_field_is_not_selected_are_left_out() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.filter",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "C", "selected": [2024] }),
        )
        .expect("the items are chosen");
    let summary = of_the_pivot(&mut session, sheet);
    let rows = summary["rows"].as_array().expect("the rows are a list");
    assert_eq!(rows.len(), 2, "both groups are there: {summary:?}");
    let ann = rows.iter().find(|row| row["key"] == json!("ann")).expect("ann is there");
    assert_eq!(ann["total"], json!(3.0), "and ann has only the spending of 2024: {ann:?}");
    assert_eq!(summary["columns"], json!([2024.0]), "and there is one column: {summary:?}");
    assert_eq!(summary["filter"]["field"], json!("C"), "the filter is said: {summary:?}");
}

/// The items are the labels **as they are written**, and no list at
/// all keeps every item.
#[test]
fn no_list_at_all_keeps_every_item() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.filter",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "C", "selected": [2024] }),
        )
        .expect("the items are chosen");
    session
        .execute("grid.pivot.filter", &json!({ "sheet": sheet, "name": "the pivot", "field": "C" }))
        .expect("every item is kept");
    let summary = of_the_pivot(&mut session, sheet);
    let rows = summary["rows"].as_array().expect("the rows are a list");
    assert_eq!(rows.len(), 2, "both groups are there: {summary:?}");
    assert!(summary["filter"].is_null(), "and there is no filter: {summary:?}");
}

/// A field that is **not a column of the range** is refused by name,
/// and so is a list that is empty.
#[test]
fn a_field_that_is_not_a_column_of_the_range_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.pivot.filter",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "Z", "selected": [2024] }),
        )
        .expect_err("the field is refused");
    assert!(error.to_string().contains("not in the range"), "the refusal says why: {error}");
    let error = session
        .execute(
            "grid.pivot.filter",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "C", "selected": [] }),
        )
        .expect_err("the list is refused");
    assert!(error.to_string().contains("at least one item"), "the refusal says why: {error}");
}

/// The **labels** of the items are the way they are written, and a
/// blank is the blank.
#[test]
fn the_labels_of_the_items_are_the_way_they_are_written() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.filter",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "A", "selected": ["ann"] }),
        )
        .expect("the items are chosen");
    let summary = of_the_pivot(&mut session, sheet);
    let rows = summary["rows"].as_array().expect("the rows are a list");
    assert_eq!(rows.len(), 1, "only ann is there: {summary:?}");
    assert_eq!(
        summary["filter"]["selected"][0],
        json!("ann"),
        "and the label is as it is written: {summary:?}"
    );
}
