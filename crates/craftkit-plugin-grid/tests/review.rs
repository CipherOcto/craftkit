//! The **review** of a workbook: what is in it, and what a reader may
//! not be able to read.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A **workbook** with a **sheet** in it, and their ids.
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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **statistics** of the document.
fn stats(session: &mut Session) -> Json {
    let outcome = session.execute("grid.workbook.stats", &json!({})).expect("the statistics are read");
    outcome.data.clone()
}

/// The **issues** of the document.
fn issues(session: &mut Session) -> Vec<Json> {
    let outcome = session.execute("grid.workbook.accessibility", &json!({})).expect("the issues are read");
    outcome.data["issues"].as_array().cloned().unwrap_or_default()
}

#[test]
fn the_statistics_count_what_is_in_the_workbook() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("one"));
    value(&mut session, sheet, "B1", json!("two"));
    value(&mut session, sheet, "A2", json!(1.0));
    value(&mut session, sheet, "B2", json!(2.0));
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "C1", "formula": "=A2+1" }))
        .expect("the formula is set");
    session
        .execute("grid.table.create", &json!({ "sheet": sheet, "name": "Book", "range": "A1:B2" }))
        .expect("the table is made");
    session
        .execute("grid.chart.add", &json!({ "sheet": sheet, "range": "A1:B2", "kind": "column" }))
        .expect("the chart is made");
    let data = stats(&mut session);
    // The **sheet**: its cells, its formulas, its table and its chart.
    let sheets = data["sheets"].as_array().expect("the sheets are a list");
    assert_eq!(sheets.len(), 1);
    assert_eq!(sheets[0]["name"], "Sheet1");
    assert_eq!(sheets[0]["cells"], 5);
    assert_eq!(sheets[0]["formulas"], 1);
    assert_eq!(sheets[0]["tables"], 1);
    assert_eq!(sheets[0]["charts"], 1);
    // And the **workbook** as a whole.
    assert_eq!(data["workbook"]["sheets"], 1);
    assert_eq!(data["workbook"]["cells"], 5);
    assert_eq!(data["workbook"]["tables"], 1);
    assert_eq!(data["workbook"]["charts"], 1);
}

#[test]
fn the_accessibility_of_a_workbook_is_a_list_of_issues() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    // A sheet with no issues at all.
    let mut found = issues(&mut session);
    // The name of the sheet is the default one.
    assert_eq!(found.len(), 1);
    assert_eq!(found[0]["issue"], "default sheet name");
    assert_eq!(found[0]["sheet"], "Sheet1");
    // A merge is an issue of its own.
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B2" }))
        .expect("the cells are merged");
    found = issues(&mut session);
    assert_eq!(found.len(), 2);
    let merge = found.iter().find(|issue| issue["issue"] == "merged cells").expect("the merge is an issue");
    assert_eq!(merge["count"], 1);
}
