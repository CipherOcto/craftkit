//! The **outline** of a sheet: the groups of its rows.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// The **groups** of a sheet, as the answer holds them.
fn groups_of(session: &mut Session, sheet: u64) -> Vec<Json> {
    let outcome =
        session.execute("grid.row.groups", &json!({ "sheet": sheet })).expect("the groups are read");
    outcome.data["groups"].as_array().cloned().expect("the groups are a list")
}

/// **Rows may be grouped.**
#[test]
fn rows_may_be_grouped() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups.len(), 1, "there is one group");
    assert_eq!(groups[0]["level"], json!(1), "and it is a group of the first level");
    assert_eq!(groups[0]["start"], json!(5), "it starts at the fifth row");
    assert_eq!(groups[0]["end"], json!(8), "and ends at the eighth");
}

/// The **summary row** of a group is the row after it.
#[test]
fn the_summary_row_of_a_group_is_the_row_after_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups[0]["summary"], json!(9), "the summary is the ninth row");
    assert_eq!(groups[0]["has_summary"], json!(true), "and the group has one");
}

/// The summary row may sit **before** the group.
#[test]
fn the_summary_row_may_sit_before_the_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.summary", &json!({ "sheet": sheet, "below": false }))
        .expect("the sheet says where its summaries sit");
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups[0]["summary"], json!(4), "the summary is the fourth row");
}

/// A **gap** ends a group.
#[test]
fn a_gap_ends_a_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:6" }))
        .expect("the rows are grouped");
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "9:10" }))
        .expect("the rows are grouped");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups.len(), 2, "there are two groups");
    assert_eq!(groups[0]["end"], json!(6), "the first ends at the sixth row");
    assert_eq!(groups[1]["start"], json!(9), "and the second starts at the ninth");
}

/// Rows may be **nested** in a group of their own.
#[test]
fn rows_may_be_nested_in_a_group_of_their_own() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "6:7" }))
        .expect("the rows are grouped again");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups.len(), 2, "there are two groups");
    assert_eq!(groups[0]["level"], json!(2), "the inner group is deeper");
    assert_eq!(groups[0]["start"], json!(6), "it starts at the sixth row");
    assert_eq!(groups[0]["end"], json!(7), "and ends at the seventh");
    assert_eq!(groups[1]["level"], json!(1), "and the outer group is the first level");
    assert_eq!(groups[1]["end"], json!(8), "and ends at the eighth");
}

/// A group may be **taken apart**.
#[test]
fn a_group_may_be_taken_apart() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    session
        .execute("grid.row.ungroup", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are ungrouped");
    let groups = groups_of(&mut session, sheet);
    assert!(groups.is_empty(), "there is no group left");
}

/// Taking a group off a row that has **none** is refused.
#[test]
fn taking_a_group_off_a_row_that_has_none_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let refused = session
        .execute("grid.row.ungroup", &json!({ "sheet": sheet, "rows": "5" }))
        .expect_err("the ungroup is refused");
    assert!(refused.to_string().contains("no group"), "the refusal says why");
}

/// A group that is **deeper than seven** is refused.
#[test]
fn a_group_that_is_deeper_than_seven_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for _ in 0..7 {
        session
            .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5" }))
            .expect("the row is grouped");
    }
    let refused = session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5" }))
        .expect_err("the group is refused");
    assert!(refused.to_string().contains("deepest"), "the refusal says how deep a group may be");
}

/// A group whose rows are **all hidden** is collapsed.
#[test]
fn a_group_whose_rows_are_all_hidden_is_collapsed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    for row in 5..=8 {
        session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": row })).expect("the row is hidden");
    }
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups[0]["collapsed"], json!(true), "the group is collapsed");
}

/// A range that runs **backwards** is refused.
#[test]
fn a_range_that_runs_backwards_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let refused = session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "8:5" }))
        .expect_err("the range is refused");
    assert!(refused.to_string().contains("backwards"), "the refusal says why");
}
