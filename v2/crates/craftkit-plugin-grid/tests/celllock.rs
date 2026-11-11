//! The **locked cell**: a cell that may not be changed while the sheet
//! is protected, and a cell that says it may be changed anyway.
//!
//! The tree's rules: a cell is locked unless its style says otherwise,
//! and the lock of a cell matters only when the sheet is protected.

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

/// The **lock** of a cell, as the sheet keeps it.
fn locked_of(session: &Session, sheet: u64, cell: &str) -> Option<bool> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell]["locked"].as_bool()
}

/// The button pressed twice does the **opposite**, and a cell that says
/// nothing follows the sheet — and the sheet's cells are locked.
#[test]
fn the_button_pressed_twice_does_the_opposite() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is unlocked");
    assert_eq!(outcome.data["locked"], json!(false), "the cell that followed the sheet is unlocked");
    assert_eq!(locked_of(&session, sheet, "A1"), Some(false), "and the sheet keeps it");
    session
        .execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is locked again");
    assert_eq!(locked_of(&session, sheet, "A1"), Some(true), "and the cell is locked");
}

/// The lock may be **named** as well.
#[test]
fn the_lock_may_be_named_as_well() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1", "locked": false }))
        .expect("the cell is unlocked");
    assert_eq!(locked_of(&session, sheet, "A1"), Some(false), "the cell is unlocked");
    session
        .execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1", "locked": false }))
        .expect("the cell is unlocked again");
    assert_eq!(locked_of(&session, sheet, "A1"), Some(false), "and it stays unlocked");
}

/// A **protected** sheet's cells may not be changed.
#[test]
fn a_protected_sheets_cells_may_not_be_changed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.protect", &json!({ "sheet": sheet, "format": true }))
        .expect("the sheet is protected");
    let outcome = session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }));
    assert!(outcome.is_err(), "the cell is locked: {outcome:?}");
}

/// A cell that says it is **unlocked** may be changed all the same.
#[test]
fn a_cell_that_says_it_is_unlocked_may_be_changed_all_the_same() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1", "locked": false }))
        .expect("the cell is unlocked");
    session
        .execute("grid.sheet.protect", &json!({ "sheet": sheet, "format": true }))
        .expect("the sheet is protected");
    let outcome = session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }));
    assert!(outcome.is_ok(), "the cell is unlocked: {outcome:?}");
}

/// The cell that is **not** named is still locked.
#[test]
fn the_cell_that_is_not_named_is_still_locked() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1", "locked": false }))
        .expect("the cell is unlocked");
    session
        .execute("grid.sheet.protect", &json!({ "sheet": sheet, "format": true }))
        .expect("the sheet is protected");
    let outcome = session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 1 }));
    assert!(outcome.is_err(), "the other cell is locked: {outcome:?}");
}

/// A sheet that is **not protected** may be changed.
#[test]
fn a_sheet_that_is_not_protected_may_be_changed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session.execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is locked");
    let outcome = session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }));
    assert!(outcome.is_ok(), "the lock matters only under protection: {outcome:?}");
}
