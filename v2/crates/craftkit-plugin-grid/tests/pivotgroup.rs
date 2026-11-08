//! The **grouping** of the items of a date field.
//!
//! A date field may be grouped by the year, the quarter, the month or
//! the day.

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

/// A sheet of three **spendings**: ann three on the first of January
/// of 2024, bob five on the fifteenth of March of 2024 and ann two on
/// the first of January of 2025.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    for (cell, value) in [("A1", json!("who")), ("B1", json!("amount")), ("C1", json!("when"))] {
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
            .expect("the cell is set");
    }
    for (row, who, amount, when) in
        [(2, "ann", 3, "2024-01-01"), (3, "bob", 5, "2024-03-15"), (4, "ann", 2, "2025-01-01")]
    {
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": format!("A{row}"), "value": who }))
            .expect("the cell is set");
        session
            .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": format!("B{row}"), "value": amount }))
            .expect("the cell is set");
        session
            .execute("grid.cell.date", &json!({ "sheet": sheet, "cell": format!("C{row}"), "date": when }))
            .expect("the date is set");
    }
    session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": "the pivot",
                "range": "A1:C4",
                "row": "C",
                "value": "B",
            }),
        )
        .expect("the pivot is made");
    sheet
}

/// The **keys** of the rows, in the order they are in.
fn keys_of(session: &mut Session, sheet: u64) -> Vec<String> {
    let outcome = session
        .execute("grid.pivot.read", &json!({ "sheet": sheet, "name": "the pivot" }))
        .expect("the pivot is read");
    outcome.data["rows"]
        .as_array()
        .expect("the rows are a list")
        .iter()
        .filter_map(|row| row["key"].as_str().map(|key| key.to_string()))
        .collect()
}

/// The items of a date field may be grouped by the **year**, the
/// **quarter**, the **month** or the **day**: the tree's own rules.
#[test]
fn the_items_of_a_date_field_may_be_grouped() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for (by, expected) in [
        ("years", vec!["2024", "2025"]),
        ("quarters", vec!["Qtr1"]),
        ("months", vec!["Jan", "Mar"]),
        ("days", vec!["1-Jan", "15-Mar"]),
    ] {
        session
            .execute(
                "grid.pivot.group",
                &json!({ "sheet": sheet, "name": "the pivot", "field": "C", "by": by }),
            )
            .expect("the items are grouped");
        let keys = keys_of(&mut session, sheet);
        assert_eq!(keys.len(), expected.len(), "the groups of {by}: {keys:?}");
        for key in &expected {
            assert!(keys.iter().any(|held| held == key), "the group {key} is there in {by}: {keys:?}");
        }
    }
}

/// A value that is **not a date** is left as it is.
#[test]
fn a_value_that_is_not_a_date_is_left_as_it_is() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "C3", "value": "someday" }))
        .expect("the cell is set");
    session
        .execute(
            "grid.pivot.group",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "C", "by": "years" }),
        )
        .expect("the items are grouped");
    let keys = keys_of(&mut session, sheet);
    assert!(keys.contains(&"someday".to_string()), "the text is left as it is: {keys:?}");
    assert!(keys.contains(&"2024".to_string()), "and the dates are grouped: {keys:?}");
}

/// A grouping that is **not one of the tree's** is refused by name,
/// and so is a field the pivot does not gather by.
#[test]
fn a_grouping_that_is_not_one_of_the_trees_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute(
            "grid.pivot.group",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "C", "by": "fortnights" }),
        )
        .expect_err("the grouping is refused");
    assert!(error.to_string().contains("fortnights"), "the refusal names the grouping: {error}");
    let error = session
        .execute(
            "grid.pivot.group",
            &json!({ "sheet": sheet, "name": "the pivot", "field": "B", "by": "years" }),
        )
        .expect_err("the field is refused");
    assert!(error.to_string().contains("neither"), "the refusal says why: {error}");
}
