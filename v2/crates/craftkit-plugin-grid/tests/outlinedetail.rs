//! The **detail** of a group of the outline: the rows of the group,
//! hidden and shown.
//!
//! The tree's rules: the group that holds the lines is the deepest of
//! those that hold them, the rows of the group are hidden, and the
//! summary row of the group is not.

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
    node.data["hidden_rows"]
        .as_array()
        .cloned()
        .expect("the sheet has the hidden rows")
        .iter()
        .filter_map(|row| row.as_u64())
        .collect()
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

/// The rows of the group are **hidden**, and its summary row is not.
#[test]
fn the_rows_of_the_group_are_hidden() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    let outcome = session
        .execute("grid.row.detail.hide", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the detail is hidden");
    assert_eq!(outcome.data["hidden"], json!([5, 6, 7, 8]), "the rows of the group are hidden");
    assert_eq!(hidden_rows(&session, sheet), vec![5, 6, 7, 8], "and they are hidden in the sheet");
}

/// The rows of the group are **shown** again.
#[test]
fn the_rows_of_the_group_are_shown_again() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    session
        .execute("grid.row.detail.hide", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the detail is hidden");
    session
        .execute("grid.row.detail.show", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the detail is shown");
    assert!(hidden_rows(&session, sheet).is_empty(), "every row is shown");
}

/// The group that holds the lines is the **deepest** of those.
#[test]
fn the_group_that_holds_the_lines_is_the_deepest() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "6:7" }))
        .expect("the rows are grouped again");
    let outcome = session
        .execute("grid.row.detail.hide", &json!({ "sheet": sheet, "rows": "6:7" }))
        .expect("the detail is hidden");
    assert_eq!(outcome.data["level"], json!(2), "the deeper group is the one that is hidden");
    assert_eq!(hidden_rows(&session, sheet), vec![6, 7], "and only its rows are hidden");
}

/// The **level** names the group when the rows are in more than one.
#[test]
fn the_level_names_the_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "6:7" }))
        .expect("the rows are grouped again");
    let outcome = session
        .execute("grid.row.detail.hide", &json!({ "sheet": sheet, "rows": "5:8", "level": 1 }))
        .expect("the detail is hidden");
    assert_eq!(outcome.data["level"], json!(1), "the group of the first level is the one");
    assert_eq!(hidden_rows(&session, sheet), vec![5, 6, 7, 8], "and all of its rows are hidden");
}

/// The group whose **summary row** is named is the one that is opened.
#[test]
fn the_group_whose_summary_row_is_named_is_the_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    let outcome = session
        .execute("grid.row.detail.hide", &json!({ "sheet": sheet, "rows": "9" }))
        .expect("the detail is hidden");
    assert_eq!(outcome.data["start"], json!(5), "the group is the one that holds the rows");
    assert_eq!(outcome.data["end"], json!(8), "the group is the one that holds the rows");
    assert_eq!(hidden_rows(&session, sheet), vec![5, 6, 7, 8], "and its rows are hidden");
}

/// An inner group that is **collapsed on its own** stays collapsed.
#[test]
fn an_inner_group_that_is_collapsed_on_its_own_stays_collapsed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "5:8" }))
        .expect("the rows are grouped");
    session
        .execute("grid.row.group", &json!({ "sheet": sheet, "rows": "6:7" }))
        .expect("the rows are grouped again");
    session
        .execute("grid.row.detail.hide", &json!({ "sheet": sheet, "rows": "6:7" }))
        .expect("the inner group is collapsed");
    assert_eq!(hidden_rows(&session, sheet), vec![6, 7], "the rows of the inner group are hidden");
    session
        .execute("grid.row.detail.show", &json!({ "sheet": sheet, "rows": "5:8", "level": 1 }))
        .expect("the detail is shown");
    assert_eq!(hidden_rows(&session, sheet), vec![6, 7], "the inner group is still collapsed");
}

/// The **columns** of a group are hidden and shown as well.
#[test]
fn the_columns_of_a_group_are_hidden_and_shown() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.column.group", &json!({ "sheet": sheet, "columns": "B:D" }))
        .expect("the columns are grouped");
    let outcome = session
        .execute("grid.column.detail.hide", &json!({ "sheet": sheet, "columns": "B:D" }))
        .expect("the detail is hidden");
    assert_eq!(outcome.data["hidden"], json!(["B", "C", "D"]), "the columns of the group are hidden");
    assert_eq!(
        hidden_columns(&session, sheet),
        vec!["B".to_string(), "C".to_string(), "D".to_string()],
        "and they are hidden in the sheet"
    );
    session
        .execute("grid.column.detail.show", &json!({ "sheet": sheet, "columns": "B:D" }))
        .expect("the detail is shown");
    assert!(hidden_columns(&session, sheet).is_empty(), "every column is shown");
}

/// Lines that are in **no group** are refused.
#[test]
fn lines_that_are_in_no_group_are_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.row.detail.hide", &json!({ "sheet": sheet, "rows": "5:8" }));
    assert!(outcome.is_err(), "there is no group of the rows: {outcome:?}");
    let outcome = session.execute("grid.column.detail.show", &json!({ "sheet": sheet, "columns": "B:D" }));
    assert!(outcome.is_err(), "there is no group of the columns: {outcome:?}");
}
