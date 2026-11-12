//! **Reading** a cell.

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

/// A **workbook** with a sheet in it, and their ids.
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

/// The **cell**, as the answer holds it.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// A cell that holds a **value** is read.
#[test]
fn a_cell_that_holds_a_value_is_read() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["value"], Json::String("red".to_string()), "the value is said");
    assert_eq!(cell["type"], Json::String("text".to_string()), "and the type");
    assert_eq!(cell["text"], Json::String("red".to_string()), "and the text the cell shows");
    assert_eq!(cell["formula"], Json::Null, "and there is no formula");
}

/// A cell that holds a **formula** has the value the formula computes.
#[test]
fn a_cell_that_holds_a_formula_has_the_value_the_formula_computes() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 2 }))
        .expect("the value is set");
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "B1", "formula": "=A1*3" }))
        .expect("the formula is set");
    let cell = cell_of(&mut session, sheet, "B1");
    assert_eq!(cell["value"], json!(6.0), "the value is computed");
    assert_eq!(cell["type"], Json::String("number".to_string()), "and the type");
    assert_eq!(cell["formula"], Json::String("=A1*3".to_string()), "and the formula is said");
}

/// The **format**, the **style**, the **note** and the **link** of a
/// cell are said.
#[test]
fn the_format_the_style_the_note_and_the_link_are_said() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the cell is formatted");
    session
        .execute("grid.cell.note.set", &json!({ "sheet": sheet, "cell": "A1", "text": "about it" }))
        .expect("the note is written");
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "somewhere" }))
        .expect("the link is written");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["format"], Json::String("0.00".to_string()), "the format is said");
    assert_eq!(cell["note"]["text"], Json::String("about it".to_string()), "the note is said");
    assert_eq!(cell["link"]["target"], Json::String("somewhere".to_string()), "and the link");
}

/// A cell that is **covered by a merge** says so.
#[test]
fn a_cell_that_is_covered_by_a_merge_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B1" }))
        .expect("the merge is set");
    let cell = cell_of(&mut session, sheet, "B1");
    assert_eq!(cell["merge"], Json::String("A1:B1".to_string()), "the merge is said");
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["merge"], Json::Null, "and the top left cell is not covered by it");
}

/// A cell that is **not there** is empty, and not an error.
#[test]
fn a_cell_that_is_not_there_is_empty() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let cell = cell_of(&mut session, sheet, "A1");
    assert_eq!(cell["type"], Json::String("empty".to_string()), "the cell is empty");
    assert_eq!(cell["value"], Json::Null, "and holds nothing");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.get", &json!({ "sheet": workbook, "cell": "A1" }))
        .expect_err("it is refused");
    assert!(error.to_string().contains("not a sheet"), "it says why: {error}");
    let error =
        session.execute("grid.cell.get", &json!({ "sheet": 9999, "cell": "A1" })).expect_err("it is refused");
    assert!(error.to_string().contains("there is no sheet"), "and there is none: {error}");
}
