//! The **look** of a cell, copied to other cells.

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

/// The **cell** as the answer holds it.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// The look of a cell is **copied** onto a range of cells.
#[test]
fn the_look_of_a_cell_is_copied_onto_a_range() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1", "on": true }))
        .expect("the font is bold");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    session
        .execute("grid.cell.look.copy", &json!({ "sheet": sheet, "from": "A1", "to": "B1:C1" }))
        .expect("the look is copied");
    for cell in ["B1", "C1"] {
        let read = cell_of(&mut session, sheet, cell);
        assert_eq!(read["font"]["bold"], json!(true), "the {cell} is bold");
        assert_eq!(read["format"], json!("0.00"), "the {cell} has the format");
    }
}

/// What a cell **holds** is not part of its look.
#[test]
fn what_a_cell_holds_is_not_part_of_its_look() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1", "on": true }))
        .expect("the font is bold");
    value(&mut session, sheet, "B1", json!(7));
    session
        .execute("grid.cell.look.copy", &json!({ "sheet": sheet, "from": "A1", "to": "B1" }))
        .expect("the look is copied");
    let read = cell_of(&mut session, sheet, "B1");
    assert_eq!(read["value"], json!(7.0), "the seven is still there");
    assert_eq!(read["font"]["bold"], json!(true), "and the cell is bold");
}

/// The **borders** of the cell are part of its look.
#[test]
fn the_borders_of_the_cell_are_part_of_its_look() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "A1", "edge": "top", "style": "thin" }),
        )
        .expect("the border is set");
    session
        .execute("grid.cell.look.copy", &json!({ "sheet": sheet, "from": "A1", "to": "B1" }))
        .expect("the look is copied");
    let outcome = session
        .execute("grid.cell.border.get", &json!({ "sheet": sheet, "cell": "B1" }))
        .expect("the borders are read");
    assert_eq!(
        outcome.data["borders"]["top"]["style"],
        json!("thin"),
        "the border of the first cell is the border of the second"
    );
}

/// A look with **nothing in it** takes the look of the cell away.
#[test]
fn a_look_with_nothing_in_it_takes_the_look_of_the_cell_away() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "B1", "on": true }))
        .expect("the font is bold");
    session
        .execute("grid.cell.look.copy", &json!({ "sheet": sheet, "from": "A1", "to": "B1" }))
        .expect("the look is copied");
    let read = cell_of(&mut session, sheet, "B1");
    assert_eq!(read["font"]["bold"], Json::Null, "the cell is not bold any more");
}

/// A cell that is **covered by a merge** is refused.
#[test]
fn a_cell_that_is_covered_by_a_merge_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "B1:C1" }))
        .expect("the cells are merged");
    let outcome =
        session.execute("grid.cell.look.copy", &json!({ "sheet": sheet, "from": "A1", "to": "B1:C1" }));
    assert!(outcome.is_err(), "the look would be drawn under another cell");
}

/// A **locked** cell is refused.
#[test]
fn a_locked_cell_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let outcome =
        session.execute("grid.cell.look.copy", &json!({ "sheet": sheet, "from": "A1", "to": "B1" }));
    assert!(outcome.is_err(), "a locked cell is not written into");
}

/// A cell that is **not there** is refused: there is no look to take.
#[test]
fn a_cell_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.cell.look.copy", &json!({ "sheet": sheet, "from": "Z9", "to": "B1" }));
    assert!(outcome.is_err(), "there is no cell to take a look from");
}
