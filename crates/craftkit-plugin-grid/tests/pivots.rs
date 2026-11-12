//! The **pivot** of a sheet: the rows gathered by a field and the
//! numbers put together.

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

/// A **text** in a cell.
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

/// The **rows** of a summary.
fn rows(session: &mut Session, sheet: u64, name: &str) -> Vec<craftkit_sdk::serde_json::Value> {
    let outcome = session
        .execute("grid.pivot.read", &json!({ "sheet": sheet, "name": name }))
        .expect("the pivot is read");
    outcome.data["rows"].as_array().expect("the rows are a list").clone()
}

/// **A pivot gathers the rows and sums the numbers.**
#[test]
fn a_pivot_sums_the_numbers_of_a_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    let rows = rows(&mut session, sheet, "by who");
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["key"], "ann");
    assert_eq!(rows[0]["value"], 5.0);
    assert_eq!(rows[1]["key"], "bob");
    assert_eq!(rows[1]["value"], 5.0);
}

/// **A pivot with a column field spreads the groups over the columns.**
#[test]
fn a_pivot_spreads_the_groups_over_the_columns() {
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
    let outcome = session
        .execute("grid.pivot.read", &json!({ "sheet": sheet, "name": "by who and when" }))
        .expect("the pivot is read");
    let columns = outcome.data["columns"].as_array().expect("the columns are a list");
    assert_eq!(columns.len(), 2);
    assert_eq!(columns[0], 2024.0);
    assert_eq!(columns[1], 2025.0);
    let rows = rows(&mut session, sheet, "by who and when");
    assert_eq!(rows[0]["key"], "ann");
    assert_eq!(rows[0]["values"]["2024"], 3.0);
    assert_eq!(rows[0]["values"]["2025"], 2.0);
    assert_eq!(rows[0]["total"], 5.0);
    assert_eq!(rows[1]["key"], "bob");
    assert_eq!(rows[1]["values"]["2024"], 5.0);
    assert!(rows[1]["values"]["2025"].is_null(), "a group with no number has no value");
}

/// **The average is the average of the group.**
#[test]
fn a_pivot_averages_the_numbers_of_a_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": "by who",
                "range": "A1:C4",
                "row": "A",
                "value": "B",
                "aggregate": "average",
            }),
        )
        .expect("the pivot is made");
    let rows = rows(&mut session, sheet, "by who");
    assert_eq!(rows[0]["key"], "ann");
    assert_eq!(rows[0]["value"], 2.5);
}

/// **The count counts the rows of the group.**
#[test]
fn a_pivot_counts_the_rows_of_a_group() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": "by who",
                "range": "A1:C4",
                "row": "A",
                "value": "B",
                "aggregate": "count",
            }),
        )
        .expect("the pivot is made");
    let rows = rows(&mut session, sheet, "by who");
    assert_eq!(rows[0]["key"], "ann");
    assert_eq!(rows[0]["value"], 2.0);
}

/// **A group with no number has no value.**
#[test]
fn a_group_with_no_number_has_no_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the cell is emptied");
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "B4" }))
        .expect("the cell is emptied");
    let rows = rows(&mut session, sheet, "by who");
    assert_eq!(rows[0]["key"], "ann");
    assert!(rows[0]["value"].is_null(), "a group with no number has no value");
    assert_eq!(rows[1]["key"], "bob");
    assert_eq!(rows[1]["value"], 5.0);
}

/// **A header that does not name a field is refused.**
#[test]
fn a_header_that_does_not_name_a_field_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 7 }))
        .expect("the cell is set");
    let failure = session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect_err("the pivot is refused");
    let message = failure.to_string();
    assert!(message.contains("B1"), "the refusal says which cell: {message}");
    assert!(message.contains("a field is named by its header"), "the refusal says why: {message}");
}

/// **A value that is not a number is refused.**
#[test]
fn a_value_that_is_not_a_number_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    a_text(&mut session, sheet, "B2", "three");
    let failure = session
        .execute("grid.pivot.read", &json!({ "sheet": sheet, "name": "by who" }))
        .expect_err("the summary is refused");
    let message = failure.to_string();
    assert!(message.contains("B2"), "the refusal says which cell: {message}");
}

/// **A way of putting numbers together that is not one of the five is
/// refused.**
#[test]
fn an_aggregate_that_is_not_one_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let failure = session
        .execute(
            "grid.pivot.create",
            &json!({
                "sheet": sheet,
                "name": "by who",
                "range": "A1:C4",
                "row": "A",
                "value": "B",
                "aggregate": "median",
            }),
        )
        .expect_err("the pivot is refused");
    let message = failure.to_string();
    assert!(message.contains("sum, average, min, max or count"), "the refusal says why: {message}");
}

/// **The summary is derived: a number that is changed is in the
/// summary.**
#[test]
fn the_summary_is_derived_from_the_cells() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    a_number(&mut session, sheet, "B2", 30.0);
    let rows = rows(&mut session, sheet, "by who");
    assert_eq!(rows[0]["value"], 32.0);
}

/// **A pivot is taken away and the cells stay.**
#[test]
fn a_pivot_is_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute(
            "grid.pivot.create",
            &json!({ "sheet": sheet, "name": "by who", "range": "A1:C4", "row": "A", "value": "B" }),
        )
        .expect("the pivot is made");
    session
        .execute("grid.pivot.remove", &json!({ "sheet": sheet, "name": "by who" }))
        .expect("the pivot is taken away");
    let failure = session
        .execute("grid.pivot.read", &json!({ "sheet": sheet, "name": "by who" }))
        .expect_err("there is no pivot now");
    let message = failure.to_string();
    assert!(message.contains("there is no pivot"), "the refusal says why: {message}");
}
