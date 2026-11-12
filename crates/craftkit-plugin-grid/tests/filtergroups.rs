//! The **groups** of a filter: the advanced filter.

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

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **word** in a cell.
fn a_word(session: &mut Session, sheet: u64, cell: &str, word: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": word }))
        .expect("the cell is set");
}

/// The **rows** a sheet's filter hides.
fn hidden_rows(session: &mut Session, sheet: u64) -> Vec<u64> {
    let outcome = session.execute("grid.filter.rows", &json!({ "sheet": sheet })).expect("the rows are read");
    outcome.data["rows"]
        .as_array()
        .map(|rows| rows.iter().filter_map(|row| row.as_u64()).collect())
        .unwrap_or_default()
}

/// The columns of a group are judged **together**: a row that matches
/// every criterion of the group is kept.
#[test]
fn the_columns_of_a_group_are_judged_together() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_number(&mut session, sheet, "A2", 10.0);
    a_number(&mut session, sheet, "A3", 1.0);
    a_word(&mut session, sheet, "B1", "small");
    a_word(&mut session, sheet, "B2", "big");
    a_word(&mut session, sheet, "B3", "big");
    session
        .execute(
            "grid.filter.groups.set",
            &json!({
                "sheet": sheet,
                "groups": [[
                    { "column": "A", "op": ">", "value": "5" },
                    { "column": "B", "op": "=", "value": "big" },
                ]],
            }),
        )
        .expect("the sheet is filtered");
    let hidden = hidden_rows(&mut session, sheet);
    assert_eq!(hidden, vec![1, 3], "the rows that do not match both are hidden");
}

/// The groups are judged **one by one**: a row that matches any group
/// is kept.
#[test]
fn the_groups_are_judged_one_by_one() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 10.0);
    a_number(&mut session, sheet, "A2", 1.0);
    a_number(&mut session, sheet, "A3", 7.0);
    session
        .execute(
            "grid.filter.groups.set",
            &json!({
                "sheet": sheet,
                "groups": [
                    [{ "column": "A", "op": ">", "value": "5" }],
                    [{ "column": "A", "op": "=", "value": "7" }],
                ],
            }),
        )
        .expect("the sheet is filtered");
    let hidden = hidden_rows(&mut session, sheet);
    assert_eq!(hidden, vec![2], "only the row that matches no group is hidden");
}

/// A group of **no criteria** is refused: it would match every row and
/// hide nothing.
#[test]
fn a_group_of_no_criteria_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let value = session
        .execute("grid.filter.groups.set", &json!({ "sheet": sheet, "groups": [[]] }))
        .expect_err("the filter is refused");
    assert!(value.to_string().contains("a criterion or more"), "the refusal says why, and it says {}", value);
}

/// A filter with **no group** is refused.
#[test]
fn a_filter_with_no_group_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let value = session
        .execute("grid.filter.groups.set", &json!({ "sheet": sheet, "groups": [] }))
        .expect_err("the filter is refused");
    assert!(
        value.to_string().contains("a group of criteria or more"),
        "the refusal says why, and it says {}",
        value
    );
}

/// An operation that is **not an operation** is refused.
#[test]
fn an_operation_that_is_not_an_operation_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let value = session
        .execute(
            "grid.filter.groups.set",
            &json!({
                "sheet": sheet,
                "groups": [[{ "column": "A", "op": "perhaps", "value": "5" }]],
            }),
        )
        .expect_err("the filter is refused");
    assert!(value.to_string().contains("no operation"), "the refusal says why, and it says {}", value);
}

/// A column that is **not a column** is refused.
#[test]
fn a_column_that_is_not_a_column_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let value = session
        .execute(
            "grid.filter.groups.set",
            &json!({
                "sheet": sheet,
                "groups": [[{ "column": "?", "op": "=", "value": "5" }]],
            }),
        )
        .expect_err("the filter is refused");
    assert!(value.to_string().contains("not a column"), "the refusal says why, and it says {}", value);
}

/// A comparison that needs a **number** and is given a word is
/// refused.
#[test]
fn a_comparison_that_needs_a_number_and_is_given_a_word_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let value = session
        .execute(
            "grid.filter.groups.set",
            &json!({
                "sheet": sheet,
                "groups": [[{ "column": "A", "op": ">", "value": "many" }]],
            }),
        )
        .expect_err("the filter is refused");
    assert!(value.to_string().contains("compares numbers"), "the refusal says why, and it says {}", value);
}

/// A **column filter** and a **group filter** are the same thing at
/// two sizes: a column filter is one group of one criterion.
#[test]
fn a_column_filter_is_one_group_of_one_criterion() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A2", 5.0);
    session
        .execute("grid.filter.set", &json!({ "sheet": sheet, "column": "A", "op": ">", "value": "4" }))
        .expect("the sheet is filtered");
    let by_column = hidden_rows(&mut session, sheet);
    session
        .execute(
            "grid.filter.groups.set",
            &json!({
                "sheet": sheet,
                "groups": [[{ "column": "A", "op": ">", "value": "4" }]],
            }),
        )
        .expect("the sheet is filtered by groups");
    let by_group = hidden_rows(&mut session, sheet);
    assert_eq!(by_column, by_group, "the two filters hide the same rows");
    assert_eq!(by_group, vec![1], "and it is the row that does not match");
}
