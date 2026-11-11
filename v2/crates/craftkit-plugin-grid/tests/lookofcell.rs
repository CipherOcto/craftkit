//! The **look** of a cell: the alignment, the font, the fill and the
//! lock that a cell is read back with.
//!
//! The tree's cell carries all of them in its **style**, and the
//! tree's `cell.get` says the style; here the cell carries them
//! itself, and the read says each of them.

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

/// The **cell** as it is read.
fn cell(session: &mut Session, sheet: u64, cell: &str) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// The **alignment** of a cell is read back.
#[test]
fn the_alignment_of_a_cell_is_read_back() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.align",
            &json!({ "sheet": sheet, "cell": "A1", "horizontal": "center", "wrap": true }),
        )
        .expect("the cell is aligned");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["align"]["horizontal"].as_str(), Some("center"), "the alignment is read back");
    assert_eq!(read["align"]["wrap"].as_bool(), Some(true), "and the wrap");
    // The tree's rule: an indented cell lies **left**, whatever it
    // was told before.
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "indent": 2 }))
        .expect("the cell is indented");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["align"]["indent"].as_u64(), Some(2), "the indent is read back");
    assert_eq!(read["align"]["horizontal"].as_str(), Some("left"), "and the cell lies left");
}

/// The **font** of a cell is read back.
#[test]
fn the_font_of_a_cell_is_read_back() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.font", &json!({ "sheet": sheet, "cell": "A1", "name": "Iowan", "size": 14.0 }))
        .expect("the font is set");
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the font is bold");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["font"]["name"].as_str(), Some("Iowan"), "the face is read back");
    assert_eq!(read["font"]["size"].as_f64(), Some(14.0), "and the size");
    assert_eq!(read["font"]["bold"].as_bool(), Some(true), "and the bold");
}

/// The **fill** of a cell is read back.
#[test]
fn the_fill_of_a_cell_is_read_back() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.fill", &json!({ "sheet": sheet, "cell": "A1", "colour": "#ff0000" }))
        .expect("the fill is set");
    let read = cell(&mut session, sheet, "A1");
    assert_eq!(read["fill"].as_str(), Some("#ff0000"), "the fill is read back");
}

/// A cell that says nothing has **no** look.
#[test]
fn a_cell_that_says_nothing_has_no_look() {
    let mut session = session();
    let (_, sheet) = a_sheet(&mut session);
    let read = cell(&mut session, sheet, "A1");
    assert!(read["align"].is_null(), "there is no alignment");
    assert!(read["font"].is_null(), "there is no font");
    assert!(read["fill"].is_null(), "there is no fill");
    assert_eq!(read["locked"].as_bool(), Some(false), "and the sheet is not protected, so nothing is locked");
}

/// The **lock** that is read back is the lock that holds: the cell's
/// own word, then the style it names, and then the sheet.
#[test]
fn the_lock_that_is_read_back_is_the_lock_that_holds() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    // A cell that says it is unlocked is not locked.
    session
        .execute("grid.cell.lock", &json!({ "sheet": sheet, "cell": "A1", "locked": false }))
        .expect("the cell is unlocked");
    // And a style that says so is the lock of the cells that name it.
    // Both are said before the sheet is protected, because a protected
    // sheet does not let the formatting change.
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "open", "format": "general" }))
        .expect("the style is added");
    session
        .execute("grid.style.lock", &json!({ "workbook": workbook, "name": "open", "locked": false }))
        .expect("the style is unlocked");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "B1", "style": "open" }))
        .expect("the cell names the style");
    // The sheet is protected.
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    assert_eq!(cell(&mut session, sheet, "A1")["locked"].as_bool(), Some(false), "the cell says so");
    assert_eq!(cell(&mut session, sheet, "B1")["locked"].as_bool(), Some(false), "and the style says so");
    assert_eq!(
        cell(&mut session, sheet, "A2")["locked"].as_bool(),
        Some(true),
        "and the cell that says nothing is locked"
    );
}
