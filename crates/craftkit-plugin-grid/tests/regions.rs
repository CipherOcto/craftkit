//! The **region** of a cell.

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
        .expect("the value is set");
}

/// The **region** of a cell, as the answer holds it.
fn region_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome = session
        .execute("grid.cell.region", &json!({ "sheet": sheet, "cell": cell }))
        .expect("the region is read");
    outcome.data["region"].clone()
}

/// The block of filled cells around a cell is its **region**.
#[test]
fn the_block_of_filled_cells_around_a_cell_is_its_region() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    for cell in ["A1", "B1", "A2", "B2"] {
        value(&mut session, sheet, cell, json!(1));
    }
    assert_eq!(region_of(&mut session, sheet, "A1"), json!("A1:B2"), "the region is the block");
    assert_eq!(region_of(&mut session, sheet, "B2"), json!("A1:B2"), "and it is the same block");
}

/// A cell that holds **nothing** is a region of one cell.
#[test]
fn a_cell_that_holds_nothing_is_a_region_of_one_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    assert_eq!(region_of(&mut session, sheet, "C3"), json!("C3"), "the cell is alone");
}

/// The region does not grow past the **empty cells** around it.
#[test]
fn the_region_does_not_grow_past_the_empty_cells_around_it() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!(1));
    value(&mut session, sheet, "A2", json!(2));
    value(&mut session, sheet, "C1", json!(3));
    assert_eq!(region_of(&mut session, sheet, "A1"), json!("A1:A2"), "the block is the column");
}

/// A cell that holds a **formula** is filled.
#[test]
fn a_cell_that_holds_a_formula_is_filled() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "1 + 1" }))
        .expect("the formula is set");
    value(&mut session, sheet, "B1", json!(3));
    assert_eq!(region_of(&mut session, sheet, "A1"), json!("A1:B1"), "the formula is part of the block");
}

/// The region **grows** while the line next to it holds something.
#[test]
fn the_region_grows_while_the_line_next_to_it_holds_something() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "B2", json!(1));
    value(&mut session, sheet, "B3", json!(2));
    value(&mut session, sheet, "C2", json!(3));
    value(&mut session, sheet, "A1", json!(4));
    assert_eq!(region_of(&mut session, sheet, "B2"), json!("A1:C3"), "the region grew around the cell");
}
