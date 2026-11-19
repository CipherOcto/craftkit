//! Two areas may be **intersected**: a space between them.

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

/// Two areas are cut down to the **cells they share**.
#[test]
fn two_areas_are_cut_down_to_the_cells_they_share() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B2", json!(1));
    value(&mut session, sheet, "C2", json!(2));
    value(&mut session, sheet, "B3", json!(4));
    value(&mut session, sheet, "C3", json!(8));
    let answer = computed(&mut session, sheet, "E1", "=SUM(A1:C3 B2:D4)");
    assert_eq!(answer, json!(15.0), "the four cells that are in both");
}

/// Two areas that do **not** meet are `#VALUE!`: there are no cells
/// that are in both.
#[test]
fn two_areas_that_do_not_meet_are_a_value_error() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let answer = computed(&mut session, sheet, "E1", "=SUM(A1:B5 D2:E3)");
    assert_eq!(answer, json!({ "error": "#VALUE!" }), "the areas do not meet");
}

/// Two areas of **two sheets** do not meet either.
#[test]
fn two_areas_of_two_sheets_do_not_meet() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the sheet is added");
    let other = outcome.data["id"].as_u64().expect("the sheet has an id");
    value(&mut session, sheet, "B2", json!(1));
    value(&mut session, other, "C2", json!(2));
    let answer = computed(&mut session, sheet, "E1", "=SUM(A1:B5 Sheet2!A1:C3)");
    assert_eq!(
        answer,
        json!({ "error": "an intersection is of two areas of one sheet" }),
        "the areas are of two sheets"
    );
}

/// The intersection is written as it was written.
#[test]
fn the_intersection_is_written_as_it_was_written() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "E1", "formula": "=SUM(A1:B5 B2:C3)" }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "E1" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=SUM(A1:B5 B2:C3)"), "the space is as it was written");
}

/// A space between a reference and an **operator** is not an
/// intersection: `=A1 + B1` is a sum.
#[test]
fn a_space_before_an_operator_is_not_an_intersection() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(3));
    value(&mut session, sheet, "B1", json!(4));
    let answer = computed(&mut session, sheet, "D1", "=A1 + B1");
    assert_eq!(answer, json!(7.0), "the sum of the two cells");
    let answer = computed(&mut session, sheet, "D2", "=SUM(A1 , B1)");
    assert_eq!(answer, json!(7.0), "and the sum of the two cells in a call");
}
