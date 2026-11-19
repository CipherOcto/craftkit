//! A formula may name a **whole column** or a **whole row**.

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

/// A **sheet** with a workbook above it, and their ids.
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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// The **value** a formula computes.
fn computed(session: &mut Session, sheet: u64, cell: &str, formula: &str) -> Json {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["value"].clone()
}

/// A whole **column** is the cells of the column the sheet holds.
#[test]
fn a_whole_column_is_the_cells_of_the_column() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "B2", json!(2));
    value(&mut session, sheet, "B3", json!(4));
    value(&mut session, sheet, "C1", json!(100));
    let answer = computed(&mut session, sheet, "E1", "=SUM(B:B)");
    assert_eq!(answer, json!(7.0), "the whole column is summed");
    let answer = computed(&mut session, sheet, "E2", "=COUNT(B:B)");
    assert_eq!(answer, json!(3.0), "and counted");
    let answer = computed(&mut session, sheet, "E3", "=COUNTA(B:B)");
    assert_eq!(answer, json!(3.0), "and the values of it are counted");
}

/// A whole **row** is the cells of the row the sheet holds.
#[test]
fn a_whole_row_is_the_cells_of_the_row() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "B1", json!(2));
    value(&mut session, sheet, "C1", json!(4));
    value(&mut session, sheet, "A2", json!(100));
    // The formula is in a row that the whole rows do not reach: a whole
    // row that holds the formula reads itself, and the tree refuses it.
    let answer = computed(&mut session, sheet, "E5", "=SUM(1:1)");
    assert_eq!(answer, json!(7.0), "the whole row is summed");
    let answer = computed(&mut session, sheet, "E6", "=SUM(1:2)");
    assert_eq!(answer, json!(107.0), "and two whole rows are two whole rows");
}

/// A whole line reaches as far as the sheet holds anything, and no
/// further.
#[test]
fn a_whole_line_reaches_as_far_as_the_sheet_holds() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "B3", json!(2));
    let answer = computed(&mut session, sheet, "E1", "=SUM(B:B)");
    assert_eq!(answer, json!(3.0), "the blank row between them is read as nothing");
    // A cell **below** the line is read by it, because the line reaches
    // as far as the sheet holds.
    value(&mut session, sheet, "B10", json!(4));
    let answer = computed(&mut session, sheet, "E2", "=SUM(B:B)");
    assert_eq!(answer, json!(7.0), "and the line reaches the last cell of the sheet");
}

/// A whole line in **another sheet** is that sheet's line.
#[test]
fn a_whole_line_in_another_sheet_is_that_sheets_line() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the sheet is added");
    let other = outcome.data["id"].as_u64().expect("the sheet has an id");
    value(&mut session, other, "B1", json!(5));
    let answer = computed(&mut session, sheet, "E1", "=SUM(Sheet2!B:B)");
    assert_eq!(answer, json!(5.0), "the line of the other sheet is read");
}

/// A line that reads **itself** is refused.
#[test]
fn a_line_that_reads_itself_is_refused() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B1", json!(1));
    let refused = session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B2", "formula": "=SUM(B:B)" }))
        .expect_err("the formula is refused");
    assert!(refused.to_string().contains("reads itself"), "the refusal says why: {refused}");
}

/// A **mixed** range is not a range, and the formula that holds it is
/// refused.
#[test]
fn a_mixed_range_is_refused() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let refused = session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=SUM(A1:B)" }))
        .expect_err("the formula is refused");
    assert!(
        refused.to_string().contains("two cells, two whole columns or two whole rows"),
        "the refusal says why: {refused}"
    );
}

/// A whole line is written as it was written.
#[test]
fn a_whole_line_is_written_as_it_was_written() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=SUM(B:B)" }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=SUM(B:B)"), "the formula is as it was written");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A5", "formula": "=SUM(1:3)" }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A5" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=SUM(1:3)"), "and a whole row is as it was written");
}

/// A whole line **moves** with the line it names when the formula is
/// copied, and an **anchored** line does not.
#[test]
fn a_whole_line_moves_with_the_formula() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=SUM(B:B)" }))
        .expect("the formula is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "A1" }))
        .expect("the formula is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C1" }))
        .expect("the formula is pasted");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C1" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=SUM(D:D)"), "the whole column moves with the cell");
}
