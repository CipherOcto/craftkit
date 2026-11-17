//! A **range** of cells names a style.

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

/// A **sheet** with a workbook above it, and their ids.
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

/// The **cell** as the answer holds it.
fn cell_of(session: &mut Session, sheet: u64, cell: &str) -> Json {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data.clone()
}

/// Every cell of a range **names the style**.
#[test]
fn every_cell_of_a_range_names_the_style() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "Money", "format": "0.00" }))
        .expect("the style is made");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.5 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 2.5 }))
        .expect("the cell is set");
    session
        .execute("grid.range.style", &json!({ "sheet": sheet, "range": "A1:B1", "style": "Money" }))
        .expect("the cells name the style");
    assert_eq!(cell_of(&mut session, sheet, "A1")["text"], json!("1.50"), "the first cell has the format");
    assert_eq!(cell_of(&mut session, sheet, "B1")["text"], json!("2.50"), "and so does the second");
}

/// A style that is **not there** is refused.
#[test]
fn a_style_that_is_not_there_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome =
        session.execute("grid.range.style", &json!({ "sheet": sheet, "range": "A1", "style": "Nothing" }));
    assert!(outcome.is_err(), "there is no style called `Nothing`");
}

/// An **empty name** means the cells name no style.
#[test]
fn an_empty_name_means_the_cells_name_no_style() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "Money", "format": "0.00" }))
        .expect("the style is made");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 1.5 }))
        .expect("the cell is set");
    session
        .execute("grid.range.style", &json!({ "sheet": sheet, "range": "A1", "style": "Money" }))
        .expect("the cell names the style");
    session
        .execute("grid.range.style", &json!({ "sheet": sheet, "range": "A1", "style": "" }))
        .expect("the cell names no style");
    assert_eq!(cell_of(&mut session, sheet, "A1")["text"], json!("1.5"), "the cell has its own format");
}

/// A **locked** cell of a protected sheet is refused.
#[test]
fn a_locked_cell_of_a_protected_sheet_is_refused() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": workbook, "name": "Money", "format": "0.00" }))
        .expect("the style is made");
    session.execute("grid.sheet.protect", &json!({ "sheet": sheet })).expect("the sheet is protected");
    let outcome =
        session.execute("grid.range.style", &json!({ "sheet": sheet, "range": "A1", "style": "Money" }));
    assert!(outcome.is_err(), "a locked cell is not changed");
    let outcome =
        session.execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "Money" }));
    assert!(outcome.is_err(), "a locked cell is not changed, and not one cell at a time either");
}
