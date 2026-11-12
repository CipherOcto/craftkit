//! The **subtotals** of a pivot.
//!
//! The total of a group of the rows may be a line of its own, at the
//! top of the group or at the bottom of it.

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

/// The **rows** of the summary.
fn rows_of(session: &mut Session, sheet: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("grid.pivot.read", &json!({ "sheet": sheet, "name": "the pivot" }))
        .expect("the pivot is read");
    outcome.data["rows"].as_array().expect("the rows are a list").clone()
}

/// The **keys** of the rows, in the order they are in.
fn keys_of(session: &mut Session, sheet: u64) -> Vec<String> {
    rows_of(session, sheet).iter().filter_map(|row| row["key"].as_str().map(|key| key.to_string())).collect()
}

/// The subtotal of a group is a **line of its own**, with the name of
/// the group and the word "Total": the tree's own rule.
#[test]
fn the_subtotal_of_a_group_is_a_line_of_its_own() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.pivot.subtotals", &json!({ "sheet": sheet, "name": "the pivot", "mode": "bottom" }))
        .expect("the subtotals are placed");
    let keys = keys_of(&mut session, sheet);
    assert_eq!(
        keys,
        vec!["ann", "ann Total", "bob", "bob Total"],
        "the subtotal is under its group: {keys:?}"
    );
    let rows = rows_of(&mut session, sheet);
    let subtotal = rows.iter().find(|row| row["key"] == json!("ann Total")).expect("the subtotal is there");
    assert_eq!(subtotal["subtotal"], json!(true), "the line is a subtotal: {subtotal:?}");
    assert_eq!(subtotal["total"], json!(5.0), "and it is the total of the group: {subtotal:?}");
}

/// The subtotal may be at the **top** of its group: the tree's own
/// rule.
#[test]
fn the_subtotal_may_be_at_the_top_of_its_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.pivot.subtotals", &json!({ "sheet": sheet, "name": "the pivot", "mode": "top" }))
        .expect("the subtotals are placed");
    let keys = keys_of(&mut session, sheet);
    assert_eq!(
        keys,
        vec!["ann Total", "ann", "bob Total", "bob"],
        "the subtotal is over its group: {keys:?}"
    );
}

/// A pivot has **no subtotals** by default, and the mode that is not
/// one of the tree's is refused by name.
#[test]
fn a_pivot_has_no_subtotals_by_default() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let keys = keys_of(&mut session, sheet);
    assert_eq!(keys, vec!["ann", "bob"], "there are no subtotals: {keys:?}");
    let error = session
        .execute("grid.pivot.subtotals", &json!({ "sheet": sheet, "name": "the pivot", "mode": "middle" }))
        .expect_err("the mode is refused");
    assert!(error.to_string().contains("middle"), "the refusal names the mode: {error}");
}
