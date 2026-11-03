//! The **merges** of a sheet.
//!
//! A merge says that a range of cells is one cell: the value lives in
//! the cell at the top left and the other cells are covered.

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
fn a_workbook(session: &mut Session) -> (u64, u64) {
    let outcome =
        session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet1" }))
        .expect("the sheet is added");
    let sheet = outcome.data["id"].as_u64().expect("the sheet has an id");
    (workbook, sheet)
}

/// The **merges** of a sheet.
fn merges(session: &Session, sheet: u64) -> Vec<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["merges"]
        .as_array()
        .map(|merges| {
            merges.iter().map(|merge| merge.as_str().expect("a merge is text").to_string()).collect()
        })
        .unwrap_or_default()
}

/// A range may be **merged**, and the sheet says so.
#[test]
fn a_range_may_be_merged() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    assert_eq!(merges(&session, sheet), vec!["A1:B2".to_string()], "the sheet says so");
}

/// The **top left** cell of a merged region is the cell that holds the
/// value, and the other cells are **covered**: a value may not be put
/// in one.
#[test]
fn a_covered_cell_may_not_be_written_to() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let error = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 5 }))
        .expect_err("the cell is refused");
    assert!(error.to_string().contains("is covered by the merge A1:B2"), "the error says so: {error}");
    let outcome = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 5 }))
        .expect("the top left cell is written");
    assert!(outcome.changed, "and the top left cell may be written");
}

/// A **formula** may not be put in a covered cell either.
#[test]
fn a_covered_cell_may_not_hold_a_formula() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let error = session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A2", "formula": "=1+1" }))
        .expect_err("the formula is refused");
    assert!(error.to_string().contains("is covered by the merge"), "the error says so: {error}");
}

/// A merge that would be made over cells that hold a **value** is
/// refused, and the refusal says which cell holds one.
#[test]
fn a_merge_over_a_value_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 5 }))
        .expect("the cell is set");
    let error = session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect_err("the merge is refused");
    assert!(error.to_string().contains("the cell B2 holds a value"), "the error says so: {error}");
    assert!(merges(&session, sheet).is_empty(), "and the sheet has no merge");
}

/// A merge that **overlaps** another merge is refused.
#[test]
fn a_merge_that_overlaps_another_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let error = session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "B1:C3" }))
        .expect_err("the merge is refused");
    assert!(error.to_string().contains("touches the merge A1:B2"), "the error says so: {error}");
    assert_eq!(merges(&session, sheet).len(), 1, "and the sheet has one merge");
}

/// Two merges that do not **touch** may be on the same sheet.
#[test]
fn two_merges_that_do_not_touch_may_be_on_a_sheet() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the first range is merged");
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "D1:E2" }))
        .expect("the second range is merged");
    assert_eq!(merges(&session, sheet).len(), 2, "the sheet has two merges");
}

/// A merge of **one cell** is refused: it is not a merge.
#[test]
fn a_merge_of_one_cell_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1" }))
        .expect_err("the merge is refused");
    assert!(error.to_string().contains("is not a merge"), "the error says so: {error}");
}

/// A range that is **not a range** is refused.
#[test]
fn a_range_that_is_not_a_range_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "the top left" }))
        .expect_err("the merge is refused");
    assert!(error.to_string().contains("is not a range of cells"), "the error says so: {error}");
}

/// A merged region may be **unmerged**, and the cells are ordinary
/// again.
#[test]
fn a_merged_region_may_be_unmerged() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    session
        .execute("grid.merge.clear", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is unmerged");
    assert!(merges(&session, sheet).is_empty(), "the sheet has no merge");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": 5 }))
        .expect("and the cell may be written");
}

/// A region that is **not merged** may not be unmerged, and the refusal
/// says which range was asked for.
#[test]
fn a_region_that_is_not_merged_may_not_be_unmerged() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let error = session
        .execute("grid.merge.clear", &json!({ "sheet": sheet, "range": "A1:B3" }))
        .expect_err("the unmerge is refused");
    assert!(error.to_string().contains("there is no merge of A1:B3"), "the error says so: {error}");
    assert_eq!(merges(&session, sheet).len(), 1, "and the merge is still there");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.merge.set", &json!({ "sheet": workbook, "range": "A1:B2" }))
        .expect_err("the merge is refused");
    assert!(error.to_string().contains("is not a sheet"), "the error says so: {error}");
}

/// A **fill** may not write into a covered cell either.
#[test]
fn a_fill_may_not_write_into_a_covered_cell() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1 }))
        .expect("the cell is set");
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let error = session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": "A1", "to": "A1:B2" }))
        .expect_err("the fill is refused");
    assert!(error.to_string().contains("is covered by the merge"), "the error says so: {error}");
}

/// The merge is **saved**: it is in the document after it is done.
#[test]
fn the_merge_is_saved() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(sheet).expect("the sheet is in the document");
    let merges = node.data["merges"].as_array().expect("the merges are a list");
    assert_eq!(merges.len(), 1, "there is one merge");
    assert_eq!(merges[0], "A1:B2", "and it is the range");
}
