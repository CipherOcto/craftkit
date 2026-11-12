//! The **quarters** in a text.
//!
//! The tree's rule: the list of the quarters wraps after four, and so
//! does a text that says a quarter — `Qtr3` is filled as `Qtr4` and
//! `Qtr1`.

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

/// A workbook with one sheet, and the sheet's id.
fn a_sheet(session: &mut Session) -> u64 {
    session.execute("grid.workbook.add", &json!({ "name": "the book" })).expect("the workbook is added");
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet1" }))
        .expect("the sheet is added");
    outcome.data["id"].as_u64().expect("the sheet has an id")
}

/// Put a **text** in a cell.
fn put(session: &mut Session, sheet: u64, cell: &str, text: &str) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": text }))
        .expect("the cell is set");
}

/// The **text** of a cell, as the payload holds it.
fn text_of(session: &Session, sheet: u64, cell: &str) -> Option<String> {
    let node = session.document().node(sheet).expect("the sheet is there");
    let entry = node.data["cells"].get(cell)?;
    Some(entry["value"].as_str()?.to_string())
}

/// Fill a range from a cell, and say what the cells hold.
fn filled(session: &mut Session, sheet: u64, from: &str, to: &str, cells: &[&str]) -> Vec<String> {
    session
        .execute("grid.range.fill", &json!({ "sheet": sheet, "from": from, "to": to }))
        .expect("the range is filled");
    cells.iter().filter_map(|cell| text_of(session, sheet, cell)).collect()
}

/// A quarter in a text wraps after four.
#[test]
fn a_quarter_wraps_after_four() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "Qtr3");
    assert_eq!(
        filled(&mut session, sheet, "A1", "A1:A3", &["A1", "A2", "A3"]),
        vec!["Qtr3", "Qtr4", "Qtr1"],
        "the quarter wraps"
    );
}

/// A quarter written out in **words** wraps as well.
#[test]
fn a_quarter_in_words_wraps() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "Quarter 4");
    assert_eq!(
        filled(&mut session, sheet, "A1", "A1:A2", &["A1", "A2"]),
        vec!["Quarter 4", "Quarter 1"],
        "the quarter wraps"
    );
}

/// A text whose number is **not** a quarter counts on: the tree's rule
/// is about the source, and `7` is not a quarter.
#[test]
fn a_number_that_is_not_a_quarter_counts_on() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "Qtr7");
    assert_eq!(
        filled(&mut session, sheet, "A1", "A1:A3", &["A1", "A2", "A3"]),
        vec!["Qtr7", "Qtr8", "Qtr9"],
        "the number counts on"
    );
}

/// A text with a number and a **suffix** is not a quarter: the tree's
/// rule wants nothing after the number.
#[test]
fn a_text_with_a_suffix_is_not_a_quarter() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "Qtr3x");
    assert_eq!(
        filled(&mut session, sheet, "A1", "A1:A3", &["A1", "A2", "A3"]),
        vec!["Qtr3x", "Qtr4x", "Qtr5x"],
        "the number counts on"
    );
}

/// The list of the quarters wraps as well, and it did already (§282).
#[test]
fn the_list_of_the_quarters_wraps() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    put(&mut session, sheet, "A1", "Q3");
    assert_eq!(
        filled(&mut session, sheet, "A1", "A1:A3", &["A1", "A2", "A3"]),
        vec!["Q3", "Q4", "Q1"],
        "the list wraps"
    );
}
