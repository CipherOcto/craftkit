//! The **merge and centre** of a range: the range becomes one cell and
//! its text is centred.
//!
//! The tree's rules: the range is merged and its text is centred, and
//! a range that is already merged is unmerged again — the button
//! pressed twice does the opposite.

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

/// The **merges** of a sheet, as the sheet keeps them.
fn merges_of(session: &Session, sheet: u64) -> Vec<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["merges"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|merge| merge.as_str().map(|merge| merge.to_string()))
        .collect()
}

/// The **horizontal** alignment of a cell.
fn horizontal_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    node.data["cells"][cell]["align"]["horizontal"].as_str().map(|text| text.to_string())
}

/// The range is merged and its text is **centred**.
#[test]
fn the_range_is_merged_and_its_text_is_centred() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "the title" }))
        .expect("the cell is set");
    let outcome = session
        .execute("grid.merge.center", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    assert_eq!(outcome.data["merged"], json!(true), "the range is merged");
    assert_eq!(merges_of(&session, sheet), vec!["A1:B2".to_string()], "and the sheet keeps the merge");
    assert_eq!(horizontal_of(&session, sheet, "A1"), Some("center".to_string()), "the text is centred");
}

/// A range that is **already merged** is unmerged again.
#[test]
fn a_range_that_is_already_merged_is_unmerged_again() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.merge.center", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    assert_eq!(merges_of(&session, sheet), vec!["A1:B2".to_string()], "the range is merged");
    let outcome = session
        .execute("grid.merge.center", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is unmerged");
    assert_eq!(outcome.data["merged"], json!(false), "the range is not merged any more");
    assert!(merges_of(&session, sheet).is_empty(), "and the sheet has no merge");
}

/// The text of the region is the text of its **top left cell**.
#[test]
fn the_text_of_the_region_is_the_text_of_its_top_left_cell() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "the title" }))
        .expect("the cell is set");
    session
        .execute("grid.merge.center", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the range is merged");
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["cells"]["A1"]["value"], json!("the title"), "the cell keeps its value");
}

/// A cell that **holds a value** is refused.
#[test]
fn a_cell_that_holds_a_value_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": "the second" }))
        .expect("the cell is set");
    let outcome = session.execute("grid.merge.center", &json!({ "sheet": sheet, "range": "A1:B2" }));
    assert!(outcome.is_err(), "the value would be lost: {outcome:?}");
    assert!(merges_of(&session, sheet).is_empty(), "and nothing is merged");
}

/// A range of **one cell** is not a merge.
#[test]
fn a_range_of_one_cell_is_not_a_merge() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.merge.center", &json!({ "sheet": sheet, "range": "A1" }));
    assert!(outcome.is_err(), "a merge of one cell is not a merge: {outcome:?}");
}
