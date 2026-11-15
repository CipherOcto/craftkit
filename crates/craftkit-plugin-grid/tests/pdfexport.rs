//! The **export** of a sheet as a **PDF file**.

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

/// Put a **value** in a cell.
fn value(session: &mut Session, sheet: u64, cell: &str, value: Json) {
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": cell, "value": value }))
        .expect("the value is set");
}

/// The **bytes** of the file the export wrote, and the answer it gave.
fn exported(session: &mut Session, sheet: u64) -> (Vec<u8>, Json) {
    let path = craftkit_scratch::file("craftkit-v2-grid-export.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "sheet": sheet, "path": path.to_string_lossy() }))
        .expect("the sheet is exported");
    let bytes = std::fs::read(&path).expect("the file is there");
    (bytes, outcome.data.clone())
}

/// The **sheet** is written as a PDF file.
#[test]
fn the_sheet_is_written_as_a_pdf_file() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, data) = exported(&mut session, sheet);
    assert!(bytes.starts_with(b"%PDF-1.7"), "the file says what it is");
    assert!(bytes.ends_with(b"%%EOF\n"), "the file ends where it should");
    assert!(find(&bytes, b"/Type /Catalog").is_some(), "the file has a catalogue");
    assert_eq!(data["pages"].as_u64(), Some(1), "the sheet is one page");
}

/// The **text** of a cell is in the file.
#[test]
fn the_text_of_a_cell_is_in_the_file() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    value(&mut session, sheet, "B1", json!(3));
    let (bytes, _) = exported(&mut session, sheet);
    assert!(find(&bytes, b"(ann) Tj").is_some(), "the text of the cell is drawn");
    assert!(find(&bytes, b"(3) Tj").is_some(), "the number of the cell is drawn");
}

/// A sheet with **nothing** in it is refused: there are no pages to write.
#[test]
fn a_sheet_with_nothing_in_it_is_refused() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let path = craftkit_scratch::file("craftkit-v2-grid-export-empty.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "sheet": sheet, "path": path.to_string_lossy() }))
        .expect_err("an empty sheet is refused");
    let message = outcome.to_string();
    assert!(message.contains("nothing to export"), "the refusal says why: {message}");
}

/// The **paper** of the file is the paper the sheet prints with.
#[test]
fn the_paper_of_the_file_is_the_paper_the_sheet_prints_with() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session.execute("grid.print.paper", &json!({ "sheet": sheet, "paper": "A4" })).expect("the paper is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, data) = exported(&mut session, sheet);
    assert_eq!(data["paper"].as_str(), Some("A4"), "the answer names the paper");
    assert!(find(&bytes, b"/MediaBox [0 0 595.28 841.89]").is_some(), "the page is an A4 page");
}

/// The **gridlines** are drawn when the sheet says they are printed.
#[test]
fn the_gridlines_are_drawn_when_the_sheet_says_so() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    let (without, _) = exported(&mut session, sheet);
    assert!(find(&without, b" l S").is_none(), "no gridlines without them");
    session
        .execute("grid.print.gridlines", &json!({ "sheet": sheet, "on": true }))
        .expect("the gridlines are printed");
    let (with, _) = exported(&mut session, sheet);
    assert!(find(&with, b" l S").is_some(), "the gridlines are drawn");
}

/// A **number** that does not fit in its cell is drawn as a row of `#`.
#[test]
fn a_number_that_does_not_fit_is_drawn_as_hashes() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.column.width", &json!({ "sheet": sheet, "column": "A", "width": 1 }))
        .expect("the column is narrow");
    value(&mut session, sheet, "A1", json!(1234567890));
    let (bytes, _) = exported(&mut session, sheet);
    assert!(find(&bytes, b"(#) Tj").is_some(), "the number is a row of hashes");
    assert!(find(&bytes, b"(1234567890) Tj").is_none(), "the number is not drawn whole");
}

/// Where a piece of text is in the bytes of the file.
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|window| window == needle)
}

/// The **header** of the page is drawn.
#[test]
fn the_header_of_the_page_is_drawn() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.print.header", &json!({ "sheet": sheet, "text": "&Lthe book&Rpage &P of &N" }))
        .expect("the header is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, _) = exported(&mut session, sheet);
    assert!(find(&bytes, b"(the book) Tj").is_some(), "the left section is drawn");
    assert!(find(&bytes, b"(page 1 of 1) Tj").is_some(), "the codes are worked out");
}

/// The **footer** of the page is drawn.
#[test]
fn the_footer_of_the_page_is_drawn() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.print.footer", &json!({ "sheet": sheet, "text": "&Cthe end" }))
        .expect("the footer is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, _) = exported(&mut session, sheet);
    assert!(find(&bytes, b"(the end) Tj").is_some(), "the footer is drawn");
}

/// The **headings** of the sheet are drawn when the sheet says they are.
#[test]
fn the_headings_of_the_sheet_are_drawn() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    let (without, _) = exported(&mut session, sheet);
    assert!(find(&without, b"(A) Tj").is_none(), "no headings without them");
    session
        .execute("grid.print.headings", &json!({ "sheet": sheet, "on": true }))
        .expect("the headings are printed");
    let (with, _) = exported(&mut session, sheet);
    assert!(find(&with, b"(A) Tj").is_some(), "the name of the column is drawn");
    assert!(find(&with, b"(1) Tj").is_some(), "the number of the row is drawn");
}
