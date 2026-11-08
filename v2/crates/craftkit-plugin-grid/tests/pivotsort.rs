//! The **order** of the items of a pivot.
//!
//! The items may be in the tree's own order, or from the last to the
//! first, or in the order of the source.

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

/// A sheet of four **spendings**: bob five in 2024, ann three in 2024,
/// cid two in 2025 and ann one in 2025.
///
/// The order of the source is bob, ann and cid — and it is not the
/// order of the tree.
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
        ("A2", json!("bob")),
        ("B2", json!(5)),
        ("C2", json!(2024)),
        ("A3", json!("ann")),
        ("B3", json!(3)),
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

/// The **keys** of the rows, in the order they are in.
fn keys_of(session: &mut Session, sheet: u64) -> Vec<String> {
    let summary = of_the_pivot(session, sheet);
    summary["rows"]
        .as_array()
        .expect("the rows are a list")
        .iter()
        .filter_map(|row| row["key"].as_str().map(|key| key.to_string()))
        .collect()
}

/// The items of a field may be **from the last to the first**: the
/// tree's own order.
#[test]
fn the_items_may_be_from_the_last_to_the_first() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert_eq!(keys_of(&mut session, sheet), vec!["ann", "bob", "cid"], "the tree's own order is the first");
    session
        .execute(
            "grid.pivot.sort",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "A", "order": "desc" }),
        )
        .expect("the order is said");
    assert_eq!(keys_of(&mut session, sheet), vec!["cid", "bob", "ann"], "and the other way round");
    let summary = of_the_pivot(&mut session, sheet);
    assert_eq!(summary["sort"]["field"], json!("A"), "the order is said: {summary:?}");
    assert_eq!(summary["sort"]["order"], json!("desc"), "and the order of it: {summary:?}");
}

/// The order of the **source** is the order the items are in where
/// they came from.
#[test]
fn the_order_of_the_source_is_the_order_they_come_in() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.sort",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "A", "order": "none" }),
        )
        .expect("the order is said");
    let summary = of_the_pivot(&mut session, sheet);
    assert_eq!(summary["sort"]["order"], json!("none"), "the order is the source: {summary:?}");
    assert_eq!(
        keys_of(&mut session, sheet),
        vec!["bob", "ann", "cid"],
        "and the items are in the order they come in: {summary:?}"
    );
}

/// A field that the pivot does **not** gather by is refused by name,
/// and so is an order that is not one of the tree's.
#[test]
fn a_field_that_the_pivot_does_not_gather_by_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.pivot.sort",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "B", "order": "asc" }),
        )
        .expect_err("the field is refused");
    assert!(error.to_string().contains("neither"), "the refusal says why: {error}");
    let error = session
        .execute(
            "grid.pivot.sort",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "A", "order": "sideways" }),
        )
        .expect_err("the order is refused");
    assert!(error.to_string().contains("sideways"), "the refusal names the order: {error}");
}
