//! The **export of a sheet as an HTML page**.
//!
//! A page is a table of the sheet, and it carries the look of the
//! sheet.

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

/// The **page** of a sheet.
fn page(session: &mut Session, sheet: u64) -> String {
    let outcome =
        session.execute("grid.export.html", &json!({ "sheet": sheet })).expect("the sheet is exported");
    outcome.data["text"].as_str().expect("the page has text").to_string()
}

/// A sheet is exported as a **table**: a row of the sheet is a row of
/// the table, and a cell is written as it is shown.
#[test]
fn a_sheet_is_exported_as_a_table() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "one" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "B1", "value": 2 }))
        .expect("the cell is set");
    let text = page(&mut session, sheet);
    assert!(text.starts_with("<!doctype html>"), "the page is a page: {text}");
    assert!(text.contains("<title>Sheet1</title>"), "and it is named after the sheet");
    assert!(
        text.contains("<tr><td>one</td><td style=\"text-align:right;\">2</td></tr>"),
        "the row is a row: {text}"
    );
    assert!(text.contains("</table>"), "and the table is closed");
}

/// The **marks** of HTML in a cell's text are escaped: a page is HTML
/// and not text.
#[test]
fn the_marks_of_html_are_escaped() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "<b>&\"</b>" }))
        .expect("the cell is set");
    let text = page(&mut session, sheet);
    assert!(text.contains("&lt;b&gt;&amp;&quot;&lt;/b&gt;"), "the text is escaped: {text}");
    assert!(!text.contains("<b>"), "and it is not a tag of its own");
}

/// The page carries the **look** of the sheet: the bold of a cell, the
/// colour of its text, the colour behind it and the way its text lies.
#[test]
fn the_page_carries_the_look_of_the_sheet() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "one" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.font.bold", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the cell is bold");
    session
        .execute("grid.cell.font.colour", &json!({ "sheet": sheet, "cell": "A1", "colour": "#ff0000" }))
        .expect("the cell is red");
    session
        .execute("grid.cell.align", &json!({ "sheet": sheet, "cell": "A1", "horizontal": "center" }))
        .expect("the cell is centred");
    let text = page(&mut session, sheet);
    assert!(text.contains("font-weight:bold;"), "the cell is bold: {text}");
    assert!(text.contains("color:#ff0000;"), "and it is red");
    assert!(text.contains("text-align:center;"), "and it is centred");
}

/// A **number** is written to the right and the **text** to the left,
/// unless the cell says otherwise.
#[test]
fn a_number_is_written_to_the_right() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 2 }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A2", "value": "two" }))
        .expect("the cell is set");
    let text = page(&mut session, sheet);
    assert!(text.contains("text-align:right;"), "the number is to the right: {text}");
    assert_eq!(text.matches("text-align:right;").count(), 1, "and only the number is");
}

/// A **merged cell** spans its region, and the cells it covers are not
/// written at all.
#[test]
fn a_merged_cell_spans_its_region() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "one" }))
        .expect("the cell is set");
    session
        .execute("grid.merge.set", &json!({ "sheet": sheet, "range": "A1:B1" }))
        .expect("the cells are merged");
    let text = page(&mut session, sheet);
    assert!(text.contains("colspan=\"2\""), "the cell spans the region: {text}");
    assert_eq!(text.matches("<td").count(), 1, "and the cell it covers is not written");
}

/// A **hidden row** is not written at all.
#[test]
fn a_hidden_row_is_not_written() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "one" }))
        .expect("the cell is set");
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A2", "value": "two" }))
        .expect("the cell is set");
    session.execute("grid.row.hide", &json!({ "sheet": sheet, "row": 1 })).expect("the row is hidden");
    let text = page(&mut session, sheet);
    assert!(!text.contains("one"), "the hidden row is not written: {text}");
    assert!(text.contains("two"), "and the row that is not hidden is");
}

/// A sheet that holds **nothing** exports an empty page.
#[test]
fn a_sheet_that_holds_nothing_exports_nothing() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    let outcome =
        session.execute("grid.export.html", &json!({ "sheet": sheet })).expect("the sheet is exported");
    assert_eq!(outcome.data["text"].as_str(), Some(""), "the page is empty");
    assert_eq!(outcome.data["rows"].as_u64(), Some(0), "and it has no rows");
}

/// A node that is **not a sheet** is refused.
#[test]
fn a_node_that_is_not_a_sheet_is_refused() {
    let mut session = session();
    let (workbook, _sheet) = a_workbook(&mut session);
    let error = session
        .execute("grid.export.html", &json!({ "sheet": workbook }))
        .expect_err("the export is refused");
    assert!(error.to_string().contains("not a sheet"), "the error says so: {error}");
}

/// The page of a sheet is **given** and not written to a file: the
/// writing is the caller's business.
#[test]
fn the_page_is_given_and_not_written() {
    let mut session = session();
    let (_workbook, sheet) = a_workbook(&mut session);
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": "one" }))
        .expect("the cell is set");
    let outcome =
        session.execute("grid.export.html", &json!({ "sheet": sheet })).expect("the sheet is exported");
    assert_eq!(outcome.data["rows"].as_u64(), Some(1), "the page has a row");
    assert_eq!(outcome.data["columns"].as_u64(), Some(1), "and a column");
    assert!(outcome.data["text"].as_str().expect("the page has text").contains("one"), "and the cell");
}
