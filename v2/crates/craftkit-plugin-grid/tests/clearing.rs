//! **Clearing** a cell.

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

/// What a cell **holds** and how it is **shown**, as the document holds it.
fn of(session: &Session, sheet: u64, cell: &str) -> (Option<String>, Option<String>, Option<String>) {
    let node = session.document().node(sheet).expect("the sheet is there");
    let one = node.data["cells"].get(cell).cloned().unwrap_or(json!({}));
    let value = one.get("value").and_then(|value| value.as_str()).map(|value| value.to_string());
    let formula = one.get("formula").and_then(|formula| formula.as_str()).map(|formula| formula.to_string());
    let format = one.get("format").and_then(|format| format.as_str()).map(|format| format.to_string());
    (value.clone(), formula.or(value), format)
}

/// The **contents** of a cell may be cleared and the format kept.
#[test]
fn the_contents_may_be_cleared_and_the_format_kept() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    session
        .execute("grid.cell.clear.contents", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the contents are cleared");
    let (value, formula, format) = of(&session, sheet, "A1");
    assert!(value.is_none(), "the cell holds nothing");
    assert!(formula.is_none(), "and no formula either");
    assert_eq!(format.as_deref(), Some("0.00"), "and the format is kept");
}

/// The **format** of a cell may be cleared and the contents kept.
#[test]
fn the_format_may_be_cleared_and_the_contents_kept() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    session
        .execute("grid.cell.clear.format", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the format is cleared");
    let (value, _, format) = of(&session, sheet, "A1");
    assert_eq!(value.as_deref(), Some("red"), "the cell still holds it");
    assert!(format.is_none(), "and the format is gone");
}

/// A **formula** is a content, and it goes when the contents go.
#[test]
fn a_formula_is_a_content() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.formula.set", &json!({ "sheet": sheet, "cell": "A1", "formula": "=1+1" }))
        .expect("the formula is set");
    session
        .execute("grid.cell.clear.contents", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the contents are cleared");
    let (_, formula, _) = of(&session, sheet, "A1");
    assert!(formula.is_none(), "the formula is gone");
}

/// The **style** of a cell is cleared with the format.
#[test]
fn the_style_is_cleared_with_the_format() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.style.add", &json!({ "workbook": 1, "name": "bold", "format": "0.00" }))
        .expect("the style is added");
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "bold" }))
        .expect("the style is set");
    session
        .execute("grid.cell.clear.format", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the format is cleared");
    let node = session.document().node(sheet).expect("the sheet is there");
    let one = node.data["cells"].get("A1").cloned().unwrap_or(json!({}));
    assert!(one.get("style").is_none(), "the style is gone");
}

/// The clearing of the **whole cell** is still here.
#[test]
fn the_clearing_of_the_whole_cell_is_still_here() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "red" }))
        .expect("the value is set");
    session
        .execute("grid.cell.format", &json!({ "sheet": sheet, "cell": "A1", "format": "0.00" }))
        .expect("the format is set");
    session
        .execute("grid.cell.clear", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is cleared");
    let (value, formula, format) = of(&session, sheet, "A1");
    assert!(value.is_none(), "the cell holds nothing");
    assert!(formula.is_none(), "and no formula either");
    assert!(format.is_none(), "and the format is gone");
}

/// A cell that is **not there** is not refused: there is nothing to
/// clear, and the answer says so.
#[test]
fn a_cell_that_is_not_there_is_not_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let outcome = session
        .execute("grid.cell.clear.contents", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the clearing is allowed");
    assert_eq!(outcome.data["had"].as_bool(), Some(false), "the answer says there was nothing");
}

/// A cell that is **not a cell** is refused.
#[test]
fn a_cell_that_is_not_a_cell_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let error = session
        .execute("grid.cell.clear.contents", &json!({ "sheet": sheet, "cell": "A" }))
        .expect_err("the clearing is refused");
    let message = error.to_string();
    assert!(message.contains("not a cell"), "it says why: {message}");
    let error = session
        .execute("grid.cell.clear.format", &json!({ "sheet": 9999, "cell": "A1" }))
        .expect_err("the clearing is refused");
    let message = error.to_string();
    assert!(message.contains("there is no sheet"), "and there is no sheet: {message}");
}
