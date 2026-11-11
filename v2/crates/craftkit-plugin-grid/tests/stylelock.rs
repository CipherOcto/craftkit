//! The **lock of a style**: the tree's protection, carried by the
//! style that the cells name.
//!
//! The tree's rule: the tree's cell is locked or unlocked by its
//! style, and here the style may say so too.

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

/// A style is put in the workbook, with a **lock**.
fn a_style(session: &mut Session, workbook: u64, name: &str, locked: bool) {
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": name, "format": "general" }))
        .expect("the style is added");
    session
        .execute("grid.style.lock", &json!({ "workbook": workbook, "name": name, "locked": locked }))
        .expect("the style is locked");
}

/// A cell names a style.
fn names(session: &mut Session, sheet: u64, cell: &str, style: &str) {
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": cell, "style": style }))
        .expect("the cell names the style");
}

/// A **value** is put in a cell.
fn a_value(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A style that says **unlocked** lets the cells that name it be
/// changed on a protected sheet.
#[test]
fn a_style_that_says_unlocked_lets_the_cells_be_changed() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "open", false);
    names(&mut session, sheet, "A1", "open");
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    a_value(&mut session, sheet, "A1", 1.0);
    assert!(session.document().node(sheet).is_some(), "the cell was changed");
}

/// A style that says **locked** locks the cells that name it.
#[test]
fn a_style_that_says_locked_locks_the_cells_that_name_it() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "shut", true);
    names(&mut session, sheet, "A1", "shut");
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let error = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.0 }))
        .expect_err("the cell is locked");
    assert!(error.to_string().contains("is locked"), "the refusal says so: {error}");
}

/// The lock of a style is **derived**: changing the style changes the
/// cells that name it.
#[test]
fn the_lock_of_a_style_is_derived() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "open", false);
    names(&mut session, sheet, "A1", "open");
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    a_value(&mut session, sheet, "A1", 1.0);
    // The style is locked now, and the cell that names it is locked
    // with it.
    session
        .execute("grid.style.lock", &json!({ "workbook": workbook, "name": "open", "locked": true }))
        .expect("the style is locked");
    let error = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 2.0 }))
        .expect_err("the cell is locked");
    assert!(error.to_string().contains("is locked"), "the refusal says so: {error}");
}

/// A cell that says of itself that it is **unlocked** says more than
/// the style it names.
#[test]
fn the_cell_says_more_than_the_style() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "shut", true);
    names(&mut session, sheet, "A1", "shut");
    session
        .execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1", "locked": false }))
        .expect("the cell is unlocked");
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    a_value(&mut session, sheet, "A1", 1.0);
    assert!(session.document().node(sheet).is_some(), "the cell was changed");
}

/// The **format** of a style does not take its **lock** away.
#[test]
fn the_format_of_a_style_does_not_take_its_lock_away() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "shut", true);
    names(&mut session, sheet, "A1", "shut");
    session
        .execute("grid.style.set", &json!({ "workbook": workbook, "name": "shut", "format": "0.00" }))
        .expect("the format is set");
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let error = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.0 }))
        .expect_err("the cell is locked");
    assert!(error.to_string().contains("is locked"), "the refusal says so: {error}");
}

/// A sheet that is **not** protected says nothing about the lock.
#[test]
fn a_sheet_that_is_not_protected_says_nothing() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    a_style(&mut session, workbook, "shut", true);
    names(&mut session, sheet, "A1", "shut");
    a_value(&mut session, sheet, "A1", 1.0);
    assert!(session.document().node(sheet).is_some(), "the cell was changed");
}

/// A style that is **not there** may not be locked.
#[test]
fn a_style_that_is_not_there_may_not_be_locked() {
    let mut session = session();
    let (workbook, _) = a_sheet(&mut session);
    let error = session
        .execute("grid.style.lock", &json!({ "workbook": workbook, "name": "nothing", "locked": true }))
        .expect_err("the style is refused");
    assert!(error.to_string().contains("there is no style `nothing`"), "the refusal says so: {error}");
}
