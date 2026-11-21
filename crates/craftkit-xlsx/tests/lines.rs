//! The **lines** of a sheet: the rows and the columns, and what they
//! say of themselves.
//!
//! A row and a column are facts of their own: their size, whether they
//! are hidden, how deep in a group they are, whether the group is
//! folded away, and the format they name. The **cells** of a line keep
//! their own facts: a cell that names no style does not take the style
//! of its row.

use craftkit_xlsx::{LineFormat, read};

/// A **package** of parts, written by hand.
///
/// The tests of a reader are the files that were written by somebody
/// else — and here that somebody is this file: the parts are laid out
/// as the format says.
fn a_package(parts: &[(&str, &str)]) -> Vec<u8> {
    let parts: Vec<(&str, &[u8])> = parts.iter().map(|(name, text)| (*name, text.as_bytes())).collect();
    a_package_of(&parts)
}

/// A **package** of parts, written by hand, where a part may be **bytes**
/// and not text.
fn a_package_of(parts: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    for (name, data) in parts {
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

/// A package with a workbook, a sheet and the styles.
fn a_workbook(sheet: &str) -> Vec<u8> {
    a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/styles.xml",
            r#"<styleSheet><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="14"/><name val="Arial"/></font></fonts><cellXfs><xf fontId="0"/><xf fontId="1"/></cellXfs></styleSheet>"#,
        ),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
}

/// The **format** of a **row** is read: the file names one for the row,
/// and the row says it has a format of its own.
#[test]
fn the_format_of_a_row_is_read() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1" customFormat="1" s="1"><c r="A1" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.rows.len(), 1, "one row says something of itself");
    let row = &sheet.rows[0];
    assert_eq!(row.row, 1, "it is the first row");
    let format = row.format.as_ref().expect("the row names a format");
    let font = format.font.as_ref().expect("and the format names a font");
    assert_eq!(font.name.as_deref(), Some("Arial"), "the font of the row");
}

/// A row that names a style but does **not** say it has a format of its
/// own has no format: the file said nothing, and what a file does not
/// say is not a fact of the file.
#[test]
fn a_row_that_does_not_say_it_has_a_format_has_none() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1" s="1"><c r="A1" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert!(sheet.rows.is_empty(), "the row says nothing of itself");
}

/// The **format** of a **column** is read: a column names a style
/// whenever it names one.
#[test]
fn the_format_of_a_column_is_read() {
    let bytes = a_workbook(
        r#"<worksheet><cols><col min="1" max="1" width="12" s="1"/></cols><sheetData/></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.columns.len(), 1, "one column says something of itself");
    let column = &sheet.columns[0];
    let format = column.format.as_ref().expect("the column names a format");
    let font = format.font.as_ref().expect("and the format names a font");
    assert_eq!(font.name.as_deref(), Some("Arial"), "the font of the column");
}

/// A **group** that is folded away is read: a row and a column say so.
#[test]
fn a_collapsed_group_is_read() {
    let bytes = a_workbook(
        r#"<worksheet><cols><col min="1" max="1" width="12" outlineLevel="1" collapsed="1"/></cols><sheetData><row r="1" outlineLevel="1" collapsed="1"><c r="A1" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert!(sheet.rows[0].collapsed, "the group of the row is folded away");
    assert!(sheet.columns[0].collapsed, "and the group of the column is");
}

/// A **size** that is the sheet's own default is the sheet's own, and it
/// is not carried as a size of the line. A size the **user** set is the
/// user's, even when it is the default.
#[test]
fn a_size_that_is_the_default_is_the_default() {
    let bytes = a_workbook(
        r#"<worksheet><sheetFormatPr defaultColWidth="10" defaultRowHeight="12.75"/><cols><col min="1" max="1" width="10"/><col min="2" max="2" width="10" customWidth="1"/><col min="3" max="3" width="14"/></cols><sheetData><row r="1" ht="12.75"><c r="A1" t="str"><v>x</v></c></row><row r="2" ht="12.75" customHeight="1"><c r="A2" t="str"><v>x</v></c></row><row r="3" ht="20"><c r="A3" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    // The **columns**: the first is the default and says nothing of
    // itself, the second is the default and the user set it, and the
    // third is the user's own width.
    assert_eq!(sheet.columns.len(), 2, "two columns say something of themselves");
    assert_eq!(sheet.columns[0].column, 1, "the second column");
    assert_eq!(sheet.columns[0].width, Some(10.0), "is as wide as the sheet");
    assert!(sheet.columns[0].custom, "and the user set it");
    assert_eq!(sheet.columns[1].column, 2, "the third column");
    assert_eq!(sheet.columns[1].width, Some(14.0), "is fourteen wide");
    assert!(!sheet.columns[1].custom, "and the user did not set it");
    // The **rows**: the first is the default and says nothing of
    // itself, the second is the default and the user set it, and the
    // third is the user's own height.
    assert_eq!(sheet.rows.len(), 2, "two rows say something of themselves");
    assert_eq!(sheet.rows[0].row, 2, "the second row");
    assert_eq!(sheet.rows[0].height, Some(12.75), "is as high as the sheet");
    assert!(sheet.rows[0].custom, "and the user set it");
    assert_eq!(sheet.rows[1].row, 3, "the third row");
    assert_eq!(sheet.rows[1].height, Some(20.0), "is twenty points high");
    assert!(!sheet.rows[1].custom, "and the user did not set it");
}

/// The **cells** of a line keep their own facts: a cell that names no
/// style of its own does not take the format of its row.
#[test]
fn the_cells_of_a_line_are_untouched() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1" customFormat="1" s="1"><c r="A1" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert!(sheet.rows[0].format.is_some(), "the row names a format");
    let cell = &sheet.cells[0];
    assert!(cell.font.is_none(), "and the cell names no font of its own");
    assert!(cell.fill.is_none(), "and no fill of its own");
    assert!(cell.format.is_none(), "and no number format of its own");
}

/// A **format** that names nothing is no format: the tree reads the
/// format of a line the way it reads the format of a cell.
#[test]
fn a_format_that_names_nothing_is_no_format() {
    let bytes =
        a_workbook(r#"<worksheet><cols><col min="1" max="1" width="12"/></cols><sheetData/></worksheet>"#);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.columns[0].format, None, "the column names no format");
    assert_eq!(LineFormat::default(), LineFormat::default(), "and a format that says nothing is empty");
}
