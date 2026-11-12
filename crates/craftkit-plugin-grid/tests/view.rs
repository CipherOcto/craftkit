//! The **view** of a sheet.
//!
//! A sheet says whether the gridlines are drawn, whether the headings
//! are, whether the zeros are shown, and whether it runs right to left
//! — and it may have frozen panes.

use craftkit_sdk::serde_json::{Value, json};
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

/// A **sheet**, and its id.
fn a_sheet(session: &mut Session) -> u64 {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "the sheet" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// The **view** of a sheet, as the document holds it.
fn view_of(session: &Session, sheet: u64) -> Value {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data.get("view").cloned().unwrap_or(Value::Null)
}

/// A new sheet has the tree's **own** view: the gridlines and the
/// headings are drawn, the zeros are shown, and the sheet runs left to
/// right.
#[test]
fn a_new_sheet_has_the_trees_own_view() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let view = view_of(&session, sheet);
    assert_eq!(view["gridlines"].as_bool(), Some(true), "the gridlines are drawn");
    assert_eq!(view["headings"].as_bool(), Some(true), "the headings are drawn");
    assert_eq!(view["zeros"].as_bool(), Some(true), "the zeros are shown");
    assert_eq!(view["right_to_left"].as_bool(), Some(false), "the sheet runs left to right");
    assert_eq!(view["freeze"].as_u64(), None, "and no panes are frozen");
}

/// The gridlines may be taken **away**.
#[test]
fn the_gridlines_may_be_taken_away() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "gridlines": false }))
        .expect("the gridlines are off");
    assert_eq!(view_of(&session, sheet)["gridlines"].as_bool(), Some(false), "the gridlines are off");
}

/// A flag that is **not named** keeps its value.
#[test]
fn a_flag_that_is_not_named_keeps_its_value() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "headings": false }))
        .expect("the headings are off");
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "zeros": false }))
        .expect("the zeros are off");
    let view = view_of(&session, sheet);
    assert_eq!(view["headings"].as_bool(), Some(false), "the headings are still off");
    assert_eq!(view["zeros"].as_bool(), Some(false), "the zeros are still off");
    assert_eq!(view["gridlines"].as_bool(), Some(true), "and the gridlines are still drawn");
}

/// A sheet may run **right to left**.
#[test]
fn a_sheet_may_run_right_to_left() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "right_to_left": true }))
        .expect("the sheet runs right to left");
    assert_eq!(
        view_of(&session, sheet)["right_to_left"].as_bool(),
        Some(true),
        "the sheet runs right to left"
    );
}

/// The **panes** of a sheet may be frozen at a cell: the rows above it
/// and the columns to its left.
#[test]
fn the_panes_may_be_frozen_at_a_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.freeze", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the panes are frozen");
    let view = view_of(&session, sheet);
    let freeze = view["freeze"].as_array().expect("the freeze is there");
    assert_eq!(freeze[0].as_u64(), Some(1), "one row is frozen");
    assert_eq!(freeze[1].as_u64(), Some(1), "and one column");
}

/// Freezing at the **first cell** freezes nothing: the tree's command
/// puts no freeze where the cell is `A1`.
#[test]
fn freezing_at_the_first_cell_freezes_nothing() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.freeze", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the panes are frozen");
    assert_eq!(view_of(&session, sheet)["freeze"].as_u64(), None, "and no panes are frozen");
}

/// The panes may be **unfrozen**.
#[test]
fn the_panes_may_be_unfrozen() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.freeze", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the panes are frozen");
    session
        .execute("grid.sheet.freeze.delete", &json!({ "sheet": sheet }))
        .expect("the panes are not frozen");
    assert_eq!(view_of(&session, sheet)["freeze"].as_u64(), None, "and no panes are frozen");
}

/// A cell that is **not a cell** is refused.
#[test]
fn a_cell_that_is_not_a_cell_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.sheet.freeze", &json!({ "sheet": sheet, "cell": "B" }))
        .expect_err("`B` is not a cell");
    assert!(error.to_string().contains("not a cell"), "the error says so: {error}");
}

/// A **zoom** outside the tree's range is refused: the tree's command
/// clamps it, and a zoom asked for as a thousand per cent is not a
/// zoom.
#[test]
fn a_zoom_outside_the_range_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let error = session
        .execute("grid.sheet.zoom", &json!({ "sheet": sheet, "percent": 1000 }))
        .expect_err("a thousand per cent is not a zoom");
    assert!(error.to_string().contains("zoom"), "the error says so: {error}");
    session.execute("grid.sheet.zoom", &json!({ "sheet": sheet, "percent": 150 })).expect("the zoom is set");
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["zoom"].as_f64(), Some(150.0), "the sheet is at a hundred and fifty per cent");
}

/// The view is **saved** with the sheet.
#[test]
fn the_view_is_saved_with_the_sheet() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.sheet.view", &json!({ "sheet": sheet, "gridlines": false }))
        .expect("the gridlines are off");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(sheet).expect("the sheet is there");
    assert_eq!(node.data["view"]["gridlines"].as_bool(), Some(false), "the gridlines are still off");
}
