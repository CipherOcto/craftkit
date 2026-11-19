//! A formula may carry the **`@`** of an implicit intersection.

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

/// The `@` of a **column** is the cell of the column in the row of
/// the formula.
#[test]
fn the_at_of_a_column_is_the_cell_in_the_row_of_the_formula() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(3));
    let answer = computed(&mut session, sheet, "C2", "=@A1:A3");
    assert_eq!(answer, json!(2.0), "the cell in the row of the formula");
}

/// The `@` of a **row** is the cell of the row in the column of the
/// formula.
#[test]
fn the_at_of_a_row_is_the_cell_in_the_column_of_the_formula() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "B1", json!(2));
    value(&mut session, sheet, "C1", json!(3));
    let answer = computed(&mut session, sheet, "B3", "=@A1:C1");
    assert_eq!(answer, json!(2.0), "the cell in the column of the formula");
}

/// The `@` of a **cell** is the cell itself.
#[test]
fn the_at_of_a_cell_is_the_cell() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(7));
    let answer = computed(&mut session, sheet, "C2", "=@A1");
    assert_eq!(answer, json!(7.0), "the cell is the cell");
}

/// An area that does **not** meet the cell of the formula is
/// `#VALUE!`: the tree does not guess.
#[test]
fn an_area_that_does_not_meet_the_cell_is_a_value_error() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "B2", json!(2));
    let answer = computed(&mut session, sheet, "C3", "=@A1:B2");
    assert_eq!(answer, json!({ "error": "#VALUE!" }), "the area does not meet the cell");
}

/// The `@` is written as it was written.
#[test]
fn the_at_is_written_as_it_was_written() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "C2", "formula": "=@A1:A3" }))
        .expect("the formula is set");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C2" })).expect("the cell is read");
    assert_eq!(outcome.data["formula"], json!("=@A1:A3"), "the sign is as it was written");
}

/// The `@` is **derived**: it is the cell the formula is in that says
/// which cell it is — and so the same formula in another cell is
/// another cell.
#[test]
fn the_at_is_derived() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(3));
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "C2", "formula": "=@A1:A3" }))
        .expect("the formula is set");
    session
        .execute("grid.range.copy", &json!({ "sheet": sheet, "range": "C2" }))
        .expect("the formula is copied");
    session
        .execute("grid.range.paste", &json!({ "sheet": sheet, "at": "C3" }))
        .expect("the formula is pasted");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C3" })).expect("the cell is read");
    assert_eq!(outcome.data["value"], json!(3.0), "the same formula in another cell is another cell");
}

/// A file writes the intersection as `_xlfn.SINGLE(x)`, and the tree
/// reads it as the sign it stands for.
#[test]
fn the_file_s_way_of_writing_the_intersection_is_read() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "A3", json!(3));
    let answer = computed(&mut session, sheet, "C2", "=_xlfn.SINGLE(A1:A3)");
    assert_eq!(answer, json!(2.0), "the file's way is the tree's way");
}
