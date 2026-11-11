//! The **edit ranges** of a protected sheet: the ranges that stay
//! editable while the sheet is protected.
//!
//! The tree's rules: the ranges may only be set while the sheet is
//! not protected, and a cell in one of them may be changed even when
//! the sheet is.

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

/// The **edit ranges** of a sheet, as the sheet keeps them.
fn ranges_of(session: &mut Session, sheet: u64) -> Vec<String> {
    let outcome =
        session.execute("grid.sheet.edit.read", &json!({ "sheet": sheet })).expect("the ranges are read");
    outcome.data["ranges"]
        .as_array()
        .cloned()
        .expect("the ranges are a list")
        .iter()
        .filter_map(|range| range.as_str().map(|range| range.to_string()))
        .collect()
}

/// A cell in an edit range may be **changed** while the sheet is
/// protected.
#[test]
fn a_cell_in_an_edit_range_may_be_changed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.edit.set", &json!({ "sheet": sheet, "ranges": ["A1:B2"] }))
        .expect("the ranges are set");
    assert_eq!(ranges_of(&mut session, sheet), vec!["A1:B2".to_string()], "the sheet keeps the range");
    session
        .execute("grid.sheet.protect", &json!({ "sheet": sheet, "format": true }))
        .expect("the sheet is protected");
    let outcome = session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }));
    assert!(outcome.is_ok(), "the cell is in the range: {outcome:?}");
}

/// A cell that is **not** in a range may not be changed.
#[test]
fn a_cell_that_is_not_in_a_range_may_not_be_changed() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.edit.set", &json!({ "sheet": sheet, "ranges": ["A1:B2"] }))
        .expect("the ranges are set");
    session
        .execute("grid.sheet.protect", &json!({ "sheet": sheet, "format": true }))
        .expect("the sheet is protected");
    let outcome = session.execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "C3", "value": 1 }));
    assert!(outcome.is_err(), "the cell is locked: {outcome:?}");
}

/// The ranges may only be set while the sheet is **not protected**.
#[test]
fn the_ranges_may_only_be_set_while_the_sheet_is_not_protected() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.protect", &json!({ "sheet": sheet, "format": true }))
        .expect("the sheet is protected");
    let outcome = session.execute("grid.sheet.edit.set", &json!({ "sheet": sheet, "ranges": ["A1:B2"] }));
    assert!(outcome.is_err(), "the sheet is protected: {outcome:?}");
}

/// A **range** that is not one of them is refused.
#[test]
fn a_range_that_is_not_one_of_them_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome =
        session.execute("grid.sheet.edit.set", &json!({ "sheet": sheet, "ranges": ["A1:B2", "nonsense"] }));
    assert!(outcome.is_err(), "the range is not one of them: {outcome:?}");
    assert!(ranges_of(&mut session, sheet).is_empty(), "and no range is set");
}

/// A sheet with **no ranges** has none to read.
#[test]
fn a_sheet_with_no_ranges_has_none_to_read() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    assert!(ranges_of(&mut session, sheet).is_empty(), "the sheet has no edit ranges");
}
