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
    assert_eq!(data["sheets"][0]["paper"].as_str(), Some("A4"), "the answer names the paper");
    assert!(find(&bytes, b"/MediaBox [0 0 595.28 841.89]").is_some(), "the page is an A4 page");
}

/// The whole **document** is written when no sheet is named.
#[test]
fn the_whole_document_is_written() {
    let mut session = session();
    let (workbook, first) = a_sheet(&mut session);
    value(&mut session, first, "A1", json!("ann"));
    let outcome = session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the second sheet is added");
    let second = outcome.data["id"].as_u64().expect("the sheet has an id");
    value(&mut session, second, "A1", json!("bob"));
    let path =
        craftkit_scratch::file("craftkit-v2-grid-export-document.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "path": path.to_string_lossy() }))
        .expect("the document is exported");
    let data = outcome.data.clone();
    assert_eq!(data["count"].as_u64(), Some(2), "both sheets are written");
    assert_eq!(data["pages"].as_u64(), Some(2), "the file has a page for each sheet");
    assert_eq!(data["sheets"][1]["name"].as_str(), Some("Sheet2"), "the sheets are in order");
    let bytes = std::fs::read(&path).expect("the file is there");
    assert!(find(&bytes, b"(ann) Tj").is_some(), "the first sheet is drawn");
    assert!(find(&bytes, b"(bob) Tj").is_some(), "the second sheet is drawn");
}

/// A sheet that holds **nothing** is skipped when the whole document is
/// written, and the answer says so.
#[test]
fn an_empty_sheet_is_skipped_and_counted() {
    let mut session = session();
    let (workbook, first) = a_sheet(&mut session);
    value(&mut session, first, "A1", json!("ann"));
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Empty" }))
        .expect("the empty sheet is added");
    let path =
        craftkit_scratch::file("craftkit-v2-grid-export-skipped.pdf").expect("the scratch gives a path");
    let outcome = session
        .execute("grid.export.pdf", &json!({ "path": path.to_string_lossy() }))
        .expect("the document is exported");
    let data = outcome.data.clone();
    assert_eq!(data["count"].as_u64(), Some(1), "the sheet with something in it is written");
    assert_eq!(data["skipped"].as_u64(), Some(1), "the empty sheet is counted");
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

/// The **place** of a piece of text in the bytes of the file: the x
/// and the y it is drawn at.
fn place_of(bytes: &[u8], needle: &[u8]) -> Option<(f64, f64)> {
    let at = find(bytes, needle)?;
    let before = &bytes[..at];
    // The **matrix** of the text is the last `Tm` before it.
    let window_start = at.saturating_sub(400);
    let window = &before[window_start..];
    let matrix = window.windows(2).rposition(|pair| pair == b"Tm")? + window_start;
    let line = &before[..matrix];
    let start = line.iter().rposition(|byte| *byte == b'\n').map(|index| index + 1).unwrap_or(0);
    let numbers: Vec<f64> = std::str::from_utf8(&line[start..])
        .ok()?
        .split_whitespace()
        .filter_map(|number| number.parse().ok())
        .collect();
    if numbers.len() < 6 {
        return None;
    }
    Some((numbers[4], numbers[5]))
}

/// A **centred** sheet is drawn in the middle of the page, and not in
/// the corner of it.
#[test]
fn a_centred_sheet_is_drawn_in_the_middle() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    let (without, _) = exported(&mut session, sheet);
    let corner = place_of(&without, b"(ann) Tj").expect("the text is drawn");
    session
        .execute("grid.print.centre", &json!({ "sheet": sheet, "horizontal": true }))
        .expect("the sheet is centred");
    let (with, _) = exported(&mut session, sheet);
    let middle = place_of(&with, b"(ann) Tj").expect("the text is drawn");
    assert!(middle.0 > corner.0, "the sheet moved to the right: {corner:?} then {middle:?}");
}

/// A sheet that is centred **down** the page is drawn lower than one
/// that is not.
#[test]
fn a_centred_sheet_is_drawn_lower() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("ann"));
    let (without, _) = exported(&mut session, sheet);
    let corner = place_of(&without, b"(ann) Tj").expect("the text is drawn");
    session
        .execute("grid.print.centre", &json!({ "sheet": sheet, "vertical": true }))
        .expect("the sheet is centred");
    let (with, _) = exported(&mut session, sheet);
    let middle = place_of(&with, b"(ann) Tj").expect("the text is drawn");
    // The page is measured from the top, and the PDF from the bottom.
    assert!(middle.1 < corner.1, "the sheet moved down the page: {corner:?} then {middle:?}");
}

/// A **link** to a web address is drawn over its cell.
#[test]
fn a_link_to_a_web_address_is_drawn() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.link.set",
            &json!({ "sheet": sheet, "cell": "A1", "target": "https://example.org/" }),
        )
        .expect("the link is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, data) = exported(&mut session, sheet);
    assert!(find(&bytes, b"/URI (https://example.org/)").is_some(), "the link is drawn");
    assert_eq!(data["links"].as_u64(), Some(1), "the sheet has one link");
    assert_eq!(data["links_drawn"].as_u64(), Some(1), "the link is drawn");
    assert_eq!(data["links_undrawn"].as_u64(), Some(0), "no link is left out");
}

/// A **link** to a place inside the workbook is not drawn, and the
/// answer says so.
#[test]
fn a_link_inside_the_workbook_is_not_drawn_and_is_counted() {
    let mut session = session();
    let (workbook, sheet) = a_sheet(&mut session);
    session
        .execute("grid.sheet.add", &json!({ "workbook": workbook, "name": "Sheet2" }))
        .expect("the second sheet is added");
    session
        .execute("grid.cell.link.set", &json!({ "sheet": sheet, "cell": "A1", "target": "Sheet2!A1" }))
        .expect("the link is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, data) = exported(&mut session, sheet);
    assert!(find(&bytes, b"/Subtype /Link").is_none(), "the link is not drawn");
    assert_eq!(data["links"].as_u64(), Some(1), "the sheet has one link");
    assert_eq!(data["links_drawn"].as_u64(), Some(0), "the link is not drawn");
    assert_eq!(data["links_undrawn"].as_u64(), Some(1), "the link is counted");
}

/// A **PNG file** of that many pixels across and down.
///
/// It is written with the tree's own writer, so the tree carries no
/// artwork and the fixture is made here and not shipped.
fn a_png_file(name: &str, width: usize, height: usize) -> String {
    let mut image = craftkit_raster::Image::new(width, height, craftkit_sdk::document::Colour::WHITE);
    image.blend(0, 0, craftkit_sdk::document::Colour::BLACK, craftkit_sdk::BlendMode::Normal);
    let path =
        craftkit_scratch::file(&format!("craftkit-v2-grid-export-{name}.png")).expect("the scratch is made");
    std::fs::write(&path, image.to_png()).expect("the picture is written");
    path.to_string_lossy().to_string()
}

/// A **picture** on a cell is drawn on the page.
#[test]
fn a_picture_on_a_cell_is_drawn_on_the_page() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let path = a_png_file("picture", 8, 8);
    session
        .execute("grid.picture.set", &json!({ "sheet": sheet, "cell": "A1", "path": path }))
        .expect("the picture is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, data) = exported(&mut session, sheet);
    assert!(find(&bytes, b"/Subtype /Image").is_some(), "the picture is in the file");
    assert_eq!(data["pictures"].as_u64(), Some(1), "the sheet has one picture");
    assert_eq!(data["pictures_drawn"].as_u64(), Some(1), "the picture is drawn");
    assert_eq!(data["pictures_undrawn"].as_u64(), Some(0), "no picture is left out");
}

/// A **picture** whose bytes are broken is not drawn, and the answer
/// says so.
///
/// The head of the file is whole — so the tree can measure the picture —
/// and the rest of it is cut off, so the pixels cannot be read.
#[test]
fn a_picture_that_cannot_be_decoded_is_counted() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    let path = a_png_file("broken", 8, 8);
    let bytes = std::fs::read(&path).expect("the picture is there");
    std::fs::write(&path, &bytes[..40]).expect("the picture is cut short");
    session
        .execute("grid.picture.set", &json!({ "sheet": sheet, "cell": "A1", "path": path }))
        .expect("the picture is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, data) = exported(&mut session, sheet);
    assert!(find(&bytes, b"/Subtype /Image").is_none(), "the picture is not in the file");
    assert_eq!(data["pictures"].as_u64(), Some(1), "the sheet has one picture");
    assert_eq!(data["pictures_drawn"].as_u64(), Some(0), "the picture is not drawn");
    assert_eq!(data["pictures_undrawn"].as_u64(), Some(1), "the picture is counted");
}

/// The **borders** of a cell are drawn, with the line of their style.
#[test]
fn the_borders_of_a_cell_are_drawn() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "A1", "edge": "top", "style": "dashed" }),
        )
        .expect("the border is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, data) = exported(&mut session, sheet);
    assert!(find(&bytes, b" l S").is_some(), "the border is drawn");
    assert!(find(&bytes, b"[4 2] 0 d").is_some(), "the line is dashed");
    assert_eq!(data["borders"].as_u64(), Some(1), "the sheet has one border");
    assert_eq!(data["borders_drawn"].as_u64(), Some(1), "the border is drawn");
}

/// A **double** border is two lines beside each other.
#[test]
fn a_double_border_is_two_lines() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    session
        .execute(
            "grid.cell.border.set",
            &json!({ "sheet": sheet, "cell": "A1", "edge": "left", "style": "double" }),
        )
        .expect("the border is set");
    value(&mut session, sheet, "A1", json!("ann"));
    let (bytes, _) = exported(&mut session, sheet);
    let lines = bytes.windows(b" l S".len()).filter(|window| *window == b" l S").count();
    assert_eq!(lines, 2, "the border is two lines");
}

/// A **sparkline** on a cell is drawn on the page.
#[test]
fn a_sparkline_on_a_cell_is_drawn_on_the_page() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("the row"));
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "B2", json!(5));
    value(&mut session, sheet, "B3", json!(3));
    session
        .execute(
            "grid.sparkline.set",
            &json!({ "sheet": sheet, "cell": "A1", "range": "B1:B3", "kind": "line" }),
        )
        .expect("the sparkline is set");
    let (bytes, data) = exported(&mut session, sheet);
    assert!(find(&bytes, b"1 j").is_some(), "the line of the sparkline is drawn");
    assert_eq!(data["sparklines"].as_u64(), Some(1), "the sheet has one sparkline");
    assert_eq!(data["sparklines_drawn"].as_u64(), Some(1), "the sparkline is drawn");
}

/// A **column** sparkline is a column of bars.
#[test]
fn a_column_sparkline_is_drawn_as_bars() {
    let mut session = session();
    let (_workbook, sheet) = a_sheet(&mut session);
    value(&mut session, sheet, "A1", json!("the row"));
    value(&mut session, sheet, "B1", json!(1));
    value(&mut session, sheet, "B2", json!(5));
    value(&mut session, sheet, "B3", json!(3));
    session
        .execute(
            "grid.sparkline.set",
            &json!({ "sheet": sheet, "cell": "A1", "range": "B1:B3", "kind": "column" }),
        )
        .expect("the sparkline is set");
    let (bytes, data) = exported(&mut session, sheet);
    assert!(find(&bytes, b" re f").is_some(), "the bars are drawn");
    assert_eq!(data["sparklines_drawn"].as_u64(), Some(1), "the sparkline is drawn");
}
