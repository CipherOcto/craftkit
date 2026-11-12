//! The **group** of a column.

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

/// The **groups** of the columns of a sheet, as the answer holds them.
fn groups_of(session: &mut Session, sheet: u64) -> Vec<Json> {
    let outcome =
        session.execute("grid.column.groups", &json!({ "sheet": sheet })).expect("the groups are read");
    outcome.data["groups"].as_array().cloned().expect("the groups are a list")
}

/// **Columns may be grouped.**
#[test]
fn columns_may_be_grouped() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "C:E" }))
        .expect("the columns are grouped");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups.len(), 1, "there is one group");
    assert_eq!(groups[0]["level"], json!(1), "and it is a group of the first level");
    assert_eq!(groups[0]["start"], json!("C"), "it starts at the third column");
    assert_eq!(groups[0]["end"], json!("E"), "and ends at the fifth");
}

/// The **summary column** of a group is the column after it.
#[test]
fn the_summary_column_of_a_group_is_the_column_after_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "C:E" }))
        .expect("the columns are grouped");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups[0]["summary"], json!("F"), "the summary is the sixth column");
}

/// The summary column may sit **before** the group.
#[test]
fn the_summary_column_may_sit_before_the_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.summary", &json!({ "sheet": sheet, "right": false }))
        .expect("the sheet says where its summaries sit");
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "C:E" }))
        .expect("the columns are grouped");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups[0]["summary"], json!("B"), "the summary is the second column");
}

/// The summaries of the **rows** and of the **columns** are placed apart.
#[test]
fn the_summaries_of_the_rows_and_of_the_columns_are_placed_apart() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.summary", &json!({ "sheet": sheet, "below": false, "right": true }))
        .expect("the sheet says where its summaries sit");
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "C:E" }))
        .expect("the columns are grouped");
    let rows = session.execute("grid.row.groups", &json!({ "sheet": sheet })).expect("the groups are read");
    let columns = groups_of(&mut session, sheet);
    assert_eq!(rows.data["groups"][0]["summary"], json!(4), "the summary row is the fourth row");
    assert_eq!(columns[0]["summary"], json!("F"), "and the summary column is the sixth column");
}

/// A group of columns may be **taken apart**.
#[test]
fn a_group_of_columns_may_be_taken_apart() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "C:E" }))
        .expect("the columns are grouped");
    session
        .execute("grid.column.ungroup", &json!({ "sheet": sheet, "columns": "C:E" }))
        .expect("the columns are ungrouped");
    let groups = groups_of(&mut session, sheet);
    assert!(groups.is_empty(), "there is no group left");
}

/// Taking a group off a column that has **none** is refused.
#[test]
fn taking_a_group_off_a_column_that_has_none_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let refused = session
        .execute("grid.column.ungroup", &json!({ "sheet": sheet, "columns": "C" }))
        .expect_err("the ungroup is refused");
    assert!(refused.to_string().contains("no group"), "the refusal says why");
}

/// A group that is **deeper than seven** is refused.
#[test]
fn a_group_of_columns_that_is_deeper_than_seven_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for _ in 0..7 {
        session
            .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "C" }))
            .expect("the column is grouped");
    }
    let refused = session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "C" }))
        .expect_err("the group is refused");
    assert!(refused.to_string().contains("deepest"), "the refusal says how deep a group may be");
}

/// A group whose columns are **all hidden** is collapsed.
#[test]
fn a_group_whose_columns_are_all_hidden_is_collapsed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "C:E" }))
        .expect("the columns are grouped");
    for column in ["C", "D", "E"] {
        session
            .execute("grid.column.hide", &json!({ "sheet": sheet, "column": column }))
            .expect("the column is hidden");
    }
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups[0]["collapsed"], json!(true), "the group is collapsed");
}

/// A range of columns that runs **backwards** is refused.
#[test]
fn a_range_of_columns_that_runs_backwards_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let refused = session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "E:C" }))
        .expect_err("the range is refused");
    assert!(refused.to_string().contains("backwards"), "the refusal says why");
}
