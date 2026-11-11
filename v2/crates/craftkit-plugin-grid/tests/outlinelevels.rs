//! The **levels** of the outline: the buttons that show a sheet down
//! to a level.
//!
//! The tree's rules: a row of a group that is deeper than the level
//! asked for is hidden, a row of a group that is not deeper is shown,
//! and a row that is in no group keeps its own hidden state.

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
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **rows** of a sheet that are hidden.
fn hidden_rows(session: &Session, sheet: u64) -> Vec<u64> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["hidden_rows"].as_array().cloned().expect("the sheet has the hidden rows").iter().filter_map(|row| row.as_u64()).collect()
}

/// The **columns** of a sheet that are hidden.
fn hidden_columns(session: &Session, sheet: u64) -> Vec<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["hidden_columns"]
        .as_array()
        .cloned()
        .expect("the sheet has the hidden columns")
        .iter()
        .filter_map(|column| column.as_str().map(|column| column.to_string()))
        .collect()
}

/// The **groups** of the rows of a sheet.
fn groups_of(session: &mut Session, sheet: u64) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("grid.row.groups", &json!({ "sheet": sheet })).expect("the groups are read");
    outcome.data["groups"].as_array().cloned().expect("the groups are a list")
}

/// A level of **one** shows the top-level summaries only.
#[test]
fn a_level_of_one_shows_the_top_level_summaries_only() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" })).expect("the rows are grouped");
    session.execute("grid.outline.levels", &json!({ "sheet": sheet, "rows": 1 })).expect("the level is shown");
    assert_eq!(hidden_rows(&session, sheet), vec![5, 6, 7, 8], "the rows of the group are hidden");
    let groups = groups_of(&mut session, sheet);
    assert_eq!(groups[0]["collapsed"], json!(true), "the group is collapsed: {groups:?}");
}

/// A deeper level shows **more** of the detail.
#[test]
fn a_deeper_level_shows_more_of_the_detail() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" })).expect("the rows are grouped");
    session.execute("grid.row.group", &json!({ "sheet": sheet, "rows": "6:7" })).expect("the rows are grouped again");
    session.execute("grid.outline.levels", &json!({ "sheet": sheet, "rows": 2 })).expect("the level is shown");
    assert_eq!(hidden_rows(&session, sheet), vec![6, 7], "the deeper rows are hidden");
    session.execute("grid.outline.levels", &json!({ "sheet": sheet, "rows": 1 })).expect("the level is shown");
    assert_eq!(hidden_rows(&session, sheet), vec![5, 6, 7, 8], "the whole group is hidden");
}

/// The **columns** are shown to a level as well.
#[test]
fn the_columns_are_shown_to_a_level() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "B:C" }))
        .expect("the columns are grouped");
    session.execute("grid.outline.levels", &json!({ "sheet": sheet, "cols": 1 })).expect("the level is shown");
    assert_eq!(hidden_columns(&session, sheet), vec!["B".to_string(), "C".to_string()], "the columns are hidden");
    session.execute("grid.outline.levels", &json!({ "sheet": sheet, "cols": 2 })).expect("the level is shown");
    assert!(hidden_columns(&session, sheet).is_empty(), "the columns are shown again");
}

/// A row that is in **no group** keeps its own hidden state.
#[test]
fn a_row_in_no_group_keeps_its_own_hidden_state() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 2 })).expect("the row is hidden");
    session.execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" })).expect("the rows are grouped");
    session.execute("grid.outline.levels", &json!({ "sheet": sheet, "rows": 1 })).expect("the level is shown");
    assert_eq!(hidden_rows(&session, sheet), vec![2, 5, 6, 7, 8], "the row of no group is still hidden");
}

/// A level that is **not one to eight** is refused.
#[test]
fn a_level_that_is_not_one_to_eight_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    for level in [0.0, 9.0, -1.0] {
        let outcome = session.execute("grid.outline.levels", &json!({ "sheet": sheet, "rows": level }));
        assert!(outcome.is_err(), "the level {level} is refused: {outcome:?}");
    }
}

/// A level of **eight** shows everything.
#[test]
fn a_level_of_eight_shows_everything() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" })).expect("the rows are grouped");
    session.execute("grid.outline.levels", &json!({ "sheet": sheet, "rows": 8 })).expect("the level is shown");
    assert!(hidden_rows(&session, sheet).is_empty(), "every row is shown");
}

/// A command with **no level** is refused.
#[test]
fn a_command_with_no_level_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.outline.levels", &json!({ "sheet": sheet }));
    assert!(outcome.is_err(), "there is no level to show: {outcome:?}");
}
