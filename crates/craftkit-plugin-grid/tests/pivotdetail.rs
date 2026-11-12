//! The **detail** of a group of a pivot.
//!
//! The rows that stand behind a group: the tree's `data.showDetail`.

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

/// **Text** in a cell.
fn a_text(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A sheet of three **spendings**: ann three in 2024, bob five in 2024
/// and ann two in 2025.
fn a_sheet(session: &mut Session) -> u64 {
    let (_, sheet) = a_workbook(session);
    a_text(session, sheet, "A1", "who");
    a_text(session, sheet, "B1", "amount");
    a_text(session, sheet, "C1", "when");
    a_text(session, sheet, "A2", "ann");
    a_number(session, sheet, "B2", 3.0);
    a_number(session, sheet, "C2", 2024.0);
    a_text(session, sheet, "A3", "bob");
    a_number(session, sheet, "B3", 5.0);
    a_number(session, sheet, "C3", 2024.0);
    a_text(session, sheet, "A4", "ann");
    a_number(session, sheet, "B4", 2.0);
    a_number(session, sheet, "C4", 2025.0);
    sheet
}

/// The **rows** that stand behind a group.
fn detail(
    session: &mut Session,
    sheet: u64,
    name: &str,
    group: &str,
) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("grid.pivot.detail", &json!({ "sheet": sheet, "name": name, "group": group }))
        .expect("the detail is read");
    outcome.data["rows"].as_array().expect("the rows are a list").clone()
}

/// The detail of a group is the **rows** that stand behind it.
#[test]
fn the_detail_of_a_group_is_the_rows_behind_it() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let rows = detail(&mut session, sheet, "by who", "ann");
    assert_eq!(rows.len(), 2, "the group of ann stands for two rows");
    assert_eq!(rows[0]["row"], json!(2), "the first of them is the second row of the sheet");
    assert_eq!(rows[0]["key"], json!("ann"), "and it holds the key of the group");
    assert_eq!(rows[0]["value"], json!(3.0), "and the number it puts together");
    assert_eq!(rows[1]["row"], json!(4), "the second of them is the fourth row");
    assert_eq!(rows[1]["value"], json!(2.0), "and it holds its own number");
}

/// The detail is **derived**: it follows the cells.
#[test]
fn the_detail_follows_the_cells() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    assert_eq!(detail(&mut session, sheet, "by who", "ann").len(), 2, "the group of ann stands for two rows");
    // The cell of the third row is changed, and the group follows it.
    a_text(&mut session, sheet, "A3", "ann");
    let rows = detail(&mut session, sheet, "by who", "ann");
    assert_eq!(rows.len(), 3, "the group of ann stands for three rows now");
    assert_eq!(rows[1]["row"], json!(3), "and the second of them is the third row");
    assert_eq!(rows[1]["value"], json!(5.0), "and it holds its own number");
}

/// A pivot with a **column field** says which column each row is in.
#[test]
fn a_pivot_with_a_column_field_says_the_column_of_each_row() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": "by who and when",
                "range": "A1:C4",
                "row": "A",
                "column": "C",
                "value": "B",
            }),
        )
        .expect("the pivot is made");
    let rows = detail(&mut session, sheet, "by who and when", "ann");
    assert_eq!(rows.len(), 2, "the group of ann stands for two rows");
    assert_eq!(rows[0]["column"], json!(2024.0), "the first of them is in the year 2024");
    assert_eq!(rows[1]["column"], json!(2025.0), "and the second one in the year 2025");
}

/// A group that is **not** one of the pivot's is refused.
#[test]
fn a_group_that_is_not_one_of_the_pivots_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let error = session
        .execute("grid.pivot.detail", &json!({ "sheet": sheet, "name": "by who", "group": "cat" }))
        .expect_err("the group is refused");
    assert!(error.to_string().contains("no group"), "the refusal says why: {error}");
    // A pivot that is not there is refused too.
    let error = session
        .execute("grid.pivot.detail", &json!({ "sheet": sheet, "name": "by whom", "group": "ann" }))
        .expect_err("the pivot is refused");
    assert!(error.to_string().contains("no pivot"), "the refusal says why: {error}");
}

/// A cell of the **value field** that holds **text** is refused: a
/// number cannot be put together with a text.
#[test]
fn a_value_that_is_text_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_text(&mut session, sheet, "B4", "much");
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let error = session
        .execute("grid.pivot.detail", &json!({ "sheet": sheet, "name": "by who", "group": "ann" }))
        .expect_err("the value is refused");
    assert!(error.to_string().contains("holds text"), "the refusal says why: {error}");
}
