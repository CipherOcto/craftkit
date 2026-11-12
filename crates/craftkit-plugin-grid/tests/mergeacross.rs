//! The **merge across** of a range: every row of the range is merged
//! on its own.
//!
//! The tree's rules: a range of three rows and two columns becomes
//! three merges of two cells each, and the value of a merged region is
//! the value of its top left cell.

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

/// Every row of the range is **merged on its own**.
#[test]
fn every_row_of_the_range_is_merged_on_its_own() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.merge.across", &json!({ "sheet": sheet, "range": "A1:B3" }))
        .expect("the range is merged across");
    assert_eq!(outcome.data["merges"], json!(["A1:B1", "A2:B2", "A3:B3"]), "every row is a merge of its own");
    assert_eq!(
        merges_of(&session, sheet),
        vec!["A1:B1".to_string(), "A2:B2".to_string(), "A3:B3".to_string()],
        "and the sheet keeps them"
    );
}

/// A range of **one row** is one merge.
#[test]
fn a_range_of_one_row_is_one_merge() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.merge.across", &json!({ "sheet": sheet, "range": "A1:B1" }))
        .expect("the range is merged across");
    assert_eq!(merges_of(&session, sheet), vec!["A1:B1".to_string()], "there is one merge");
}

/// A range of **one column** is refused.
#[test]
fn a_range_of_one_column_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session.execute("grid.merge.across", &json!({ "sheet": sheet, "range": "A1:A3" }));
    assert!(outcome.is_err(), "there is nothing to merge across: {outcome:?}");
}

/// A cell that **holds a value** is refused.
#[test]
fn a_cell_that_holds_a_value_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B2", "value": "the second" }))
        .expect("the cell is set");
    let outcome = session.execute("grid.merge.across", &json!({ "sheet": sheet, "range": "A1:B3" }));
    assert!(outcome.is_err(), "the value would be lost: {outcome:?}");
    assert!(merges_of(&session, sheet).is_empty(), "and nothing is merged");
}

/// A range that **touches** a merge is refused.
#[test]
fn a_range_that_touches_a_merge_is_refused() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "C1:C2" }))
        .expect("the range is merged");
    let outcome = session.execute("grid.merge.across", &json!({ "sheet": sheet, "range": "B1:D3" }));
    assert!(outcome.is_err(), "a cell that is in two regions is in none of them: {outcome:?}");
}
