//! The **limits of the xlsx reader**: the numbers it will not go past.
//!
//! The xlsx crate is a **reader**: its numbers are about how much of a
//! file is read, and a reader that stops must **say** that it stopped.
//! A limit that no test names is a limit nothing proves holds.

use craftkit_xlsx::read;

/// A **package** of parts, written by hand.
///
/// The tests of a reader are the files that were written by somebody
/// else — and here that somebody is this file: the parts are laid out
/// as the format says.
fn a_package(parts: &[(&str, &str)]) -> Vec<u8> {
    let parts: Vec<(&str, &[u8])> = parts.iter().map(|(name, text)| (*name, text.as_bytes())).collect();
    let mut out: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    for (name, data) in &parts {
        let name = name.as_bytes();
        let checksum = craftkit_zip::crc32(data);
        let offset = out.len() as u32;
        // The **local header**.
        out.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        out.extend_from_slice(&20u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&checksum.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&(name.len() as u16).to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(name);
        out.extend_from_slice(data);
        // The **directory entry**.
        directory.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&20u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&checksum.to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(data.len() as u32).to_le_bytes());
        directory.extend_from_slice(&(name.len() as u16).to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u16.to_le_bytes());
        directory.extend_from_slice(&0u32.to_le_bytes());
        directory.extend_from_slice(&offset.to_le_bytes());
        directory.extend_from_slice(name);
    }
    let directory_at = out.len() as u32;
    out.extend_from_slice(&directory);
    // The **end of the directory**.
    out.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    out.extend_from_slice(&(parts.len() as u16).to_le_bytes());
    out.extend_from_slice(&(directory.len() as u32).to_le_bytes());
    out.extend_from_slice(&directory_at.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

/// A package with a workbook and a sheet.
fn a_workbook(workbook: &str, rels: &str, sheet: &str) -> Vec<u8> {
    a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        ("xl/_rels/workbook.xml.rels", rels),
        ("xl/workbook.xml", workbook),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
}

/// The **relations** of a workbook with one sheet.
const ONE_SHEET: &str = r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#;

/// The **workbook** of a package with one sheet.
const ONE_SHEET_BOOK: &str =
    r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#;

/// A cell in a row **beyond** `MAX_ROWS` is dropped, and the file is
/// told.
///
/// A sheet is a million rows tall, and the cell past the last row is
/// not a cell of the sheet.
#[test]
fn a_cell_beyond_the_rows_is_dropped_and_the_file_is_told() {
    let bytes = a_workbook(
        ONE_SHEET_BOOK,
        ONE_SHEET,
        r#"<worksheet><sheetData><row r="1048577"><c r="A1048577"><v>1</v></c></row></sheetData></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets[0].cells.is_empty(), "the cell is not there");
    assert!(
        workbook.warnings.iter().any(|warning| warning.contains("outside")),
        "the file is told: {:?}",
        workbook.warnings
    );
}

/// A cell in a column **beyond** `MAX_COLS` is dropped, and the file is
/// told.
#[test]
fn a_cell_beyond_the_columns_is_dropped_and_the_file_is_told() {
    let bytes = a_workbook(
        ONE_SHEET_BOOK,
        ONE_SHEET,
        r#"<worksheet><sheetData><row r="1"><c r="XFE1"><v>1</v></c></row></sheetData></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets[0].cells.is_empty(), "the cell is not there");
    assert!(
        workbook.warnings.iter().any(|warning| warning.contains("outside")),
        "the file is told: {:?}",
        workbook.warnings
    );
}

/// A workbook with **more sheets** than `MAX_SHEETS` says so.
///
/// The sheets past the limit are not read, and the file is told that
/// they were not.
#[test]
fn a_workbook_with_too_many_sheets_says_so() {
    let mut rels = String::from("<Relationships>");
    let mut sheets = String::new();
    let mut parts: Vec<(String, String)> = vec![
        (
            "_rels/.rels".to_string(),
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#.to_string(),
        ),
    ];
    for index in 0..1025 {
        let id = format!("rId{index}");
        rels.push_str(&format!(
            "<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{index}.xml\"/>"
        ));
        sheets.push_str(&format!("<sheet name=\"S{index}\" sheetId=\"{index}\" r:id=\"{id}\"/>"));
        parts.push((
            format!("xl/worksheets/sheet{index}.xml"),
            r#"<worksheet><sheetData/></worksheet>"#.to_string(),
        ));
    }
    rels.push_str("</Relationships>");
    parts.push(("xl/_rels/workbook.xml.rels".to_string(), rels));
    parts.push(("xl/workbook.xml".to_string(), format!("<workbook><sheets>{sheets}</sheets></workbook>")));
    let parts: Vec<(&str, &str)> = parts.iter().map(|(name, text)| (name.as_str(), text.as_str())).collect();
    let bytes = a_package(&parts);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.sheets.len(), 1024, "the sheets that were read: {:?}", workbook.warnings);
    assert!(
        workbook.warnings.iter().any(|warning| warning.contains("more than 1024 sheets")),
        "the file is told: {:?}",
        workbook.warnings
    );
}

/// A row with an **outline deeper** than `MAX_OUTLINE` is cut, and the
/// file is told.
#[test]
fn an_outline_deeper_than_the_tree_is_cut_and_the_file_is_told() {
    let bytes = a_workbook(
        ONE_SHEET_BOOK,
        ONE_SHEET,
        r#"<worksheet><sheetData><row r="1" outlineLevel="9"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.sheets[0].rows[0].outline, 7, "the outline is cut at the limit");
    assert!(
        workbook.warnings.iter().any(|warning| warning.contains("outline level")),
        "the file is told: {:?}",
        workbook.warnings
    );
}

/// A file that names **more iterations** than `MAX_ITERATIONS` says so.
///
/// The default is used, and the file is told that its own number was
/// not the one that was read.
#[test]
fn a_file_that_names_too_many_iterations_says_so() {
    let bytes = a_workbook(
        r#"<workbook><calcPr iterate="1" iterateCount="40000"/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ONE_SHEET,
        r#"<worksheet><sheetData/></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.calculation.max_iterations, 100, "the default is used");
    assert!(
        workbook.warnings.iter().any(|warning| warning.contains("iterations")),
        "the file is told: {:?}",
        workbook.warnings
    );
}

/// A file that names **more bags** than `MAX_BAGS` says so.
///
/// The bags past the limit are not read, and the file is told that they
/// were not.
#[test]
fn a_file_with_too_many_bags_says_so() {
    let mut bags = String::from("<FeaturePropertyBags>");
    for index in 0..100_001 {
        bags.push_str(&format!("<bag type=\"B{index}\"/>"));
    }
    bags.push_str("</FeaturePropertyBags>");
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.microsoft.com/office/2017/06/relationships/featurePropertyBag" Target="featurePropertyBags/bags.xml"/></Relationships>"#,
        ),
        ("xl/workbook.xml", r#"<workbook><sheets/></workbook>"#),
        ("xl/featurePropertyBags/bags.xml", &bags),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(
        workbook.warnings.iter().any(|warning| warning.contains("feature property bags")),
        "the file is told: {:?}",
        workbook.warnings
    );
}
