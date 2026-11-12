//! The **chart** and the **name** that move.

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
fn a_sheet(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// A **chart** on a range, and its id.
fn add_chart(session: &mut Session, sheet: u64, range: &str) -> u64 {
    let outcome = session
        .execute("grid.chart.add", &json!({ "sheet": sheet, "range": range }))
        .expect("the chart is added");
    outcome.data["chart"].as_u64().expect("the chart has an id")
}

/// A **name** of the workbook that points at a range of the sheet.
fn set_name(session: &mut Session, workbook: u64, name: &str, refers_to: &str) {
    session
        .execute("grid.name.set", &json!({ "workbook": workbook, "name": name, "refers_to": refers_to }))
        .expect("the name is set");
}

/// What a **name** stands for.
fn name_of(session: &mut Session, workbook: u64, name: &str) -> Json {
    let outcome = session
        .execute("grid.name.get", &json!({ "workbook": workbook, "name": name }))
        .expect("the name is read");
    outcome.data.clone()
}

/// A **name** moves with the rows.
#[test]
fn a_name_moves_with_the_rows() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    set_name(&mut session, workbook, "theArea", "Sheet1!A1:B4");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    assert_eq!(
        name_of(&mut session, workbook, "theArea")["refers_to"],
        json!("Sheet1!A2:B5"),
        "the area moved down"
    );
}

/// A **name** moves with the columns.
#[test]
fn a_name_moves_with_the_columns() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    set_name(&mut session, workbook, "theArea", "Sheet1!A1:B4");
    session
        .execute("grid.column.insert", &json!({ "sheet": sheet, "column": "A" }))
        .expect("the column is inserted");
    assert_eq!(
        name_of(&mut session, workbook, "theArea")["refers_to"],
        json!("Sheet1!B1:C4"),
        "the area moved right"
    );
}

/// A **name** that points at a deleted row refuses.
#[test]
fn a_name_that_points_at_a_deleted_row_refuses() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    set_name(&mut session, workbook, "theArea", "Sheet1!A1:B4");
    let refused = session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2 }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("points at a row that is deleted"), "the refusal says why");
}

/// A **name** that points at rows that are gone goes away.
#[test]
fn a_name_that_points_at_rows_that_are_gone_goes_away() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    set_name(&mut session, workbook, "theArea", "Sheet1!A2:B4");
    session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2, "count": 3 }))
        .expect("the rows are deleted");
    let refused = session
        .execute("grid.name.get", &json!({ "workbook": workbook, "name": "theArea" }))
        .expect_err("the name is gone");
    assert!(refused.to_string().contains("name"), "the refusal says there is no name");
}

/// A **chart** that reads a deleted row refuses.
#[test]
fn a_chart_that_reads_a_deleted_row_refuses() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    add_chart(&mut session, sheet, "A1:C5");
    let refused = session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2 }))
        .expect_err("the delete is refused");
    assert!(refused.to_string().contains("the chart reads"), "the refusal says why");
}

/// A **chart** that reads rows that are gone goes away.
#[test]
fn a_chart_that_reads_rows_that_are_gone_goes_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let chart = add_chart(&mut session, sheet, "A2:C4");
    session
        .execute("grid.row.delete", &json!({ "sheet": sheet, "row": 2, "count": 3 }))
        .expect("the rows are deleted");
    let refused =
        session.execute("grid.chart.read", &json!({ "chart": chart })).expect_err("the chart is gone");
    assert!(refused.to_string().contains("chart"), "the refusal says there is no chart");
}

/// A **chart** of another sheet is left alone.
#[test]
fn a_chart_of_another_sheet_is_left_alone() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the sheet is added");
    let other = outcome.data["id"].as_u64().expect("the sheet has an id");
    let chart = add_chart(&mut session, other, "A1:C5");
    session.execute("grid.row.insert", &json!({ "sheet": sheet, "row": 1 })).expect("the row is inserted");
    let outcome = session.execute("grid.chart.read", &json!({ "chart": chart })).expect("the chart is read");
    assert_eq!(
        outcome.data["range"],
        json!("A1:C5"),
        "the rows of the other sheet are not the rows that were inserted"
    );
}
