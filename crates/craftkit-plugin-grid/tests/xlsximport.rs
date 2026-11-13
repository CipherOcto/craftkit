//! The **import** of a workbook file into the document.

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

/// A **package** of parts, written by hand.
fn a_package(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    for (name, text) in parts {
        let name = name.as_bytes();
        let data = text.as_bytes();
        let checksum = craftkit_zip::crc32(data);
        let offset = out.len() as u32;
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

/// A **workbook file**: a package with a workbook, a sheet and strings.
fn a_workbook_file(sheet: &str, strings: &str) -> Vec<u8> {
    a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/sharedStrings.xml", strings),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
}

/// A **workbook file** written into the scratch, and its path.
fn a_file(bytes: Vec<u8>) -> String {
    let path = craftkit_scratch::file("book.xlsx").expect("the scratch gives a path");
    std::fs::write(&path, bytes).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// The **value** of a cell.
fn value_of(session: &mut Session, sheet: u64, cell: &str) -> Option<craftkit_sdk::serde_json::Value> {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    let value = outcome.data["value"].clone();
    if value.is_null() { None } else { Some(value) }
}

/// The **formula** of a cell.
fn formula_of(session: &mut Session, sheet: u64, cell: &str) -> Option<String> {
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read");
    outcome.data["formula"].as_str().map(|text| text.to_string())
}

/// A workbook file becomes a workbook in the document.
#[test]
fn a_workbook_file_becomes_a_workbook() {
    let mut session = session();
    let path = a_file(a_workbook_file(
        r#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1"><v>42</v></c></row></sheetData></worksheet>"#,
        r#"<sst><si><t>the text</t></si></sst>"#,
    ));
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    assert_eq!(outcome.data["sheets"].as_u64(), Some(1), "one sheet");
    assert_eq!(outcome.data["cells"].as_u64(), Some(2), "two cells");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!("the text")), "the text");
    assert_eq!(value_of(&mut session, sheet, "B1"), Some(json!(42.0)), "the number");
}

/// A **formula** in the file becomes a formula in the sheet.
#[test]
fn a_formula_in_the_file_is_imported() {
    let mut session = session();
    let path = a_file(a_workbook_file(
        r#"<worksheet><sheetData><row r="1"><c r="A1"><f>SUM(B1:B2)</f><v>3</v></c></row></sheetData></worksheet>"#,
        r#"<sst/>"#,
    ));
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["formulas"].as_u64(), Some(1), "one formula");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    assert_eq!(formula_of(&mut session, sheet, "A1"), Some("SUM(B1:B2)".to_string()), "the formula");
}

/// A formula the tree **cannot read** is not imported as a formula, and
/// the import says so.
#[test]
fn a_formula_that_cannot_be_read_is_not_imported() {
    let mut session = session();
    let path = a_file(a_workbook_file(
        r#"<worksheet><sheetData><row r="1"><c r="A1"><f>NOTAFUNCTION(1</f><v>3</v></c></row></sheetData></worksheet>"#,
        r#"<sst/>"#,
    ));
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["unread_formulas"].as_u64(), Some(1), "one formula was not read");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    assert_eq!(formula_of(&mut session, sheet, "A1"), None, "no formula");
    assert_eq!(value_of(&mut session, sheet, "A1"), Some(json!(3.0)), "but the value is kept");
}

/// A file that is **not a workbook** is refused.
#[test]
fn a_file_that_is_not_a_workbook_is_refused() {
    let mut session = session();
    let path = a_file(a_package(&[("hello.txt", "not a workbook")]));
    let error =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect_err("that is not a workbook");
    assert!(error.to_string().contains("workbook"), "{error}");
}

/// The **date system** of the file is the date system of the workbook.
#[test]
fn the_date_system_of_the_file_is_the_date_system_of_the_workbook() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><workbookPr date1904="1"/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["system"].as_str(), Some("1904"), "the 1904 system");
}
