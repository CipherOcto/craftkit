//! The **trace** of a cell at **every** level: the cells a cell reads
//! and the cells that read it, all the way down the chain.
//!
//! The tree's own flag is `all`, and it is the tree's own shortcut
//! with the shift key: `Ctrl+Shift+[` and `Ctrl+Shift+]`.

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

/// A **number** in a cell.
fn a_number(session: &mut Session, sheet: u64, cell: &str, value: f64) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the cell is set");
}

/// A **formula** in a cell.
fn a_formula(session: &mut Session, sheet: u64, cell: &str, formula: &str) {
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": cell, "formula": formula }))
        .expect("the formula is set");
}

/// The **precedents** of a cell, at every level.
fn precedents(session: &mut Session, sheet: u64, cell: &str) -> Vec<String> {
    let outcome = session
        .execute("grid.precedents", &json!({ "sheet": sheet, "cell": cell, "all": true }))
        .expect("the precedents are read");
    outcome.data["precedents"]
        .as_array()
        .map(|cells| {
            cells.iter().map(|entry| entry["cell"].as_str().expect("a cell is named").to_string()).collect()
        })
        .unwrap_or_default()
}

/// The **dependents** of a cell, at every level.
fn dependents(session: &mut Session, sheet: u64, cell: &str) -> Vec<String> {
    let outcome = session
        .execute("grid.dependents", &json!({ "sheet": sheet, "cell": cell, "all": true }))
        .expect("the dependents are read");
    outcome.data["dependents"]
        .as_array()
        .map(|cells| {
            cells.iter().map(|entry| entry["cell"].as_str().expect("a cell is named").to_string()).collect()
        })
        .unwrap_or_default()
}

/// The trace at **every** level walks the whole chain.
#[test]
fn the_trace_at_every_level_walks_the_whole_chain() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_formula(&mut session, sheet, "A2", "=A1+1");
    a_formula(&mut session, sheet, "A3", "=A2+1");
    let found = precedents(&mut session, sheet, "A3");
    assert!(found.contains(&"A2".to_string()), "the cell it reads is there: {found:?}");
    assert!(found.contains(&"A1".to_string()), "and the cell that one reads: {found:?}");
    assert!(!found.contains(&"A3".to_string()), "and not the cell itself: {found:?}");
}

/// The **dependents** at every level walk the chain too.
#[test]
fn the_dependents_at_every_level_walk_the_chain() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_formula(&mut session, sheet, "A2", "=A1+1");
    a_formula(&mut session, sheet, "A3", "=A2+1");
    let found = dependents(&mut session, sheet, "A1");
    assert!(found.contains(&"A2".to_string()), "the cell that reads it is there: {found:?}");
    assert!(found.contains(&"A3".to_string()), "and the cell that reads that one: {found:?}");
    assert!(!found.contains(&"A1".to_string()), "and not the cell itself: {found:?}");
}

/// The first level is the **direct** one: the areas a formula reads.
#[test]
fn the_first_level_is_the_direct_one() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_formula(&mut session, sheet, "A2", "=A1+1");
    a_formula(&mut session, sheet, "A3", "=A2+1");
    let outcome = session
        .execute("grid.precedents", &json!({ "sheet": sheet, "cell": "A3" }))
        .expect("the precedents are read");
    let areas = outcome.data["precedents"].as_array().expect("the areas are there");
    assert_eq!(areas.len(), 1, "one area is read at the first level");
    assert_eq!(areas[0]["area"].as_str(), Some("A2"), "and it is the cell itself");
}

/// A formula that names **no sheet** reads the sheet it is in.
#[test]
fn a_formula_that_names_no_sheet_reads_the_sheet_it_is_in() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": 1, "name": "Sheet2" }))
        .expect("the sheet is added");
    let other = outcome.data["id"].as_u64().expect("the sheet has an id");
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, other, "A1", 2.0);
    // A formula in the other sheet that says `=A1` reads **that**
    // sheet's cell and not this one's.
    a_formula(&mut session, other, "B1", "=A1");
    let found = dependents(&mut session, sheet, "A1");
    assert!(found.is_empty(), "the other sheet's formula does not read this cell: {found:?}");
    let found = dependents(&mut session, other, "A1");
    assert_eq!(found, vec!["B1".to_string()], "and it reads its own sheet's cell");
}

/// A **blank** cell inside an area is not one of the cells that are
/// read.
#[test]
fn a_blank_cell_inside_an_area_is_not_read() {
    let mut session = session();
    let sheet = a_sheet(&mut session);
    a_number(&mut session, sheet, "A1", 1.0);
    a_number(&mut session, sheet, "A3", 3.0);
    // `A2` is blank, and the area `A1:A3` holds it.
    a_formula(&mut session, sheet, "B2", "=SUM(A1:A3)");
    a_formula(&mut session, sheet, "B1", "=B2");
    let found = precedents(&mut session, sheet, "B1");
    assert!(found.contains(&"A1".to_string()), "the cell that is there is read: {found:?}");
    assert!(found.contains(&"A3".to_string()), "and the other one: {found:?}");
    assert!(found.contains(&"B2".to_string()), "and the cell that reads the area: {found:?}");
    assert!(!found.contains(&"A2".to_string()), "and the blank one is not read: {found:?}");
    assert!(!found.contains(&"B1".to_string()), "and the cell itself is not: {found:?}");
}
