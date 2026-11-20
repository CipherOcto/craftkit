//! The **import** of a workbook file into the document.

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

/// A **package** of parts, written by hand.
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

/// A function that a file writes with the **prefix** — `_xlfn.` — is
/// read: the prefix is how the file says that the function is newer
/// than the format.
#[test]
fn a_function_with_a_prefix_is_read() {
    let mut session = session();
    let path = a_file(a_workbook_file(
        r#"<worksheet><sheetData><row r="1"><c r="A1"><f>_xlfn.XLOOKUP(1,B1:B3,C1:C3)</f><v>3</v></c></row></sheetData></worksheet>"#,
        r#"<sst/>"#,
    ));
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["formulas"].as_u64(), Some(1), "the formula was read");
    assert_eq!(outcome.data["unread_formulas"].as_u64(), Some(0), "and none was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    assert_eq!(
        formula_of(&mut session, sheet, "A1"),
        Some("_xlfn.XLOOKUP(1,B1:B3,C1:C3)".to_string()),
        "the text is the file's own"
    );
}

/// A cell with a **date format** is a date: the number is written as
/// the date it is.
#[test]
fn a_date_in_the_file_is_a_date() {
    let mut session = session();
    let bytes = a_package(&[
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
            r#"<styleSheet><cellXfs><xf numFmtId="0"/><xf numFmtId="14"/></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1"><v>45000</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["formats"].as_u64(), Some(1), "one format was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let read =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    let text = read.data["text"].as_str().expect("the cell shows a text");
    assert!(text.contains("2023"), "the cell shows the date: {text}");
}

/// The **names** of a workbook are imported: the words its formulas
/// use.
#[test]
fn the_names_of_a_workbook_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets><definedNames><definedName name="Sales">One!$A$1:$B$4</definedName></definedNames></workbook>"#,
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
    assert_eq!(outcome.data["names"].as_u64(), Some(1), "one name was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let read = session
        .execute("grid.name.get", &json!({ "workbook": workbook, "name": "Sales" }))
        .expect("the name is read");
    assert_eq!(read.data["refers_to"].as_str(), Some("One!A1:B4"), "and what it stands for");
}

/// The **filter** of a sheet is imported: the rows it hides.
#[test]
fn the_filter_of_a_sheet_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><autoFilter ref="B1:D10"><filterColumn colId="1"><customFilters and="1"><customFilter operator="greaterThan" val="3"/></customFilters></filterColumn></autoFilter><sheetData><row r="1"><c r="B1"><v>1</v></c><c r="C1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["filters"].as_u64(), Some(1), "one filter was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    // The filter is over the column **C**: the range begins at B, and
    // the criterion is one column into it.
    let read = session.execute("grid.filter.rows", &json!({ "sheet": sheet })).expect("the filter is read");
    let rows = read.data["rows"].as_array().expect("the rows are a list");
    assert_eq!(rows.len(), 1, "one row is hidden: {rows:?}");
    assert_eq!(rows[0].as_u64(), Some(1), "and it is the first row");
    // The **column** the filter judges is **C**: the range begins at
    // B, and the criterion is one column into it.
    let read =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "C1" })).expect("the cell is read");
    assert_eq!(read.data["value"].as_f64(), Some(1.0), "and the cell is the one that is judged");
}

/// The **links** of a sheet are imported: the places its cells go to.
#[test]
fn the_links_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink" Target="https://example.invalid/page" TargetMode="External"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><hyperlinks><hyperlink ref="A1:B1" r:id="rId1"/></hyperlinks><sheetData/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["links"].as_u64(), Some(2), "the link is on both cells of its range");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    for address in ["A1", "B1"] {
        let read = session
            .execute("grid.cell.link.get", &json!({ "sheet": sheet, "cell": address }))
            .expect("the link is read");
        assert_eq!(
            read.data["link"]["target"].as_str(),
            Some("https://example.invalid/page"),
            "the link of {address}"
        );
    }
}

/// The **comments** of a sheet are imported: the notes its cells
/// carry.
#[test]
fn the_comments_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
        (
            "xl/comments1.xml",
            r#"<comments><authors><author>ann</author></authors><commentList><comment ref="A1" authorId="0"><text><r><t>ann:</t></r><r><t>&#10;the text</t></r></text></comment></commentList></comments>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["comments"].as_u64(), Some(1), "one comment was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let read = session
        .execute("grid.cell.comment.get", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the comment is read");
    assert_eq!(read.data["comment"]["text"].as_str(), Some("the text"), "its text");
    assert_eq!(read.data["comment"]["author"].as_str(), Some("ann"), "and its author");
}

/// A note may be **shown**: the drawing of the notes says so, and the
/// import keeps it.
#[test]
fn a_note_may_be_shown() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
        (
            "xl/comments1.xml",
            r#"<comments><authors><author>ann</author></authors><commentList><comment ref="A1" authorId="0"><text><r><t>the shown one</t></r></text></comment><comment ref="B2" authorId="0"><text><r><t>the hidden one</t></r></text></comment></commentList></comments>"#,
        ),
        (
            "xl/drawings/vmlDrawing1.vml",
            r##"<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:x="urn:schemas-microsoft-com:office:excel"><v:shape id="_x0000_s1025" type="#_x0000_t202"><x:ClientData ObjectType="Note"><x:Row>0</x:Row><x:Column>0</x:Column><x:Visible/></x:ClientData></v:shape></xml>"##,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["comments"].as_u64(), Some(2), "two comments were imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let read = session
        .execute("grid.cell.comment.get", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the comment is read");
    assert_eq!(read.data["comment"]["visible"].as_bool(), Some(true), "the note of A1 is shown");
    let read = session
        .execute("grid.cell.comment.get", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the comment is read");
    assert_eq!(read.data["comment"]["visible"].as_bool(), Some(false), "and the note of B2 is not");
}

/// A comment that is **edited** keeps the note as it was shown: what is
/// not said stays as it was.
#[test]
fn an_edited_comment_keeps_the_note_as_it_was_shown() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
        (
            "xl/comments1.xml",
            r#"<comments><authors><author>ann</author></authors><commentList><comment ref="A1" authorId="0"><text><r><t>the old text</t></r></text></comment></commentList></comments>"#,
        ),
        (
            "xl/drawings/vmlDrawing1.vml",
            r##"<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:x="urn:schemas-microsoft-com:office:excel"><v:shape id="_x0000_s1025" type="#_x0000_t202"><x:ClientData ObjectType="Note"><x:Row>0</x:Row><x:Column>0</x:Column><x:Visible/></x:ClientData></v:shape></xml>"##,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    session
        .execute("grid.cell.comment.set", &json!({ "sheet": sheet, "cell": "A1", "text": "the new text" }))
        .expect("the comment is written");
    let read = session
        .execute("grid.cell.comment.get", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the comment is read");
    assert_eq!(read.data["comment"]["text"].as_str(), Some("the new text"), "the new text");
    assert_eq!(read.data["comment"]["visible"].as_bool(), Some(true), "and the note is still shown");
}

/// The **box** of a note is imported: where it sits and how big it is.
#[test]
fn the_box_of_a_note_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" Target="../comments1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" Target="../drawings/vmlDrawing1.vml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
        (
            "xl/comments1.xml",
            r#"<comments><authors><author>ann</author></authors><commentList><comment ref="B2" authorId="0"><text><r><t>the note</t></r></text></comment></commentList></comments>"#,
        ),
        (
            "xl/drawings/vmlDrawing1.vml",
            r##"<xml xmlns:v="urn:schemas-microsoft-com:vml" xmlns:x="urn:schemas-microsoft-com:office:excel"><v:shape id="_x0000_s1025" type="#_x0000_t202" style="position:absolute;width:108pt;height:59.25pt"><x:ClientData ObjectType="Note"><x:Anchor>2, 15, 1, 2, 4, 15, 5, 16</x:Anchor><x:Row>1</x:Row><x:Column>1</x:Column></x:ClientData></v:shape></xml>"##,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let read = session
        .execute("grid.cell.comment.get", &json!({ "sheet": sheet, "cell": "B2" }))
        .expect("the comment is read");
    let box_of = &read.data["comment"]["note_box"];
    assert_eq!(box_of["from"]["column"].as_i64(), Some(1), "the top-left corner is a column right");
    assert_eq!(box_of["from"]["column_px"].as_u64(), Some(15), "and fifteen pixels into it");
    assert_eq!(box_of["to"]["row"].as_i64(), Some(4), "the bottom-right corner is four rows down");
    assert_eq!(box_of["size_pt"]["width"].as_f64(), Some(108.0), "and the box is a hundred and eight wide");
    assert_eq!(box_of["size_pt"]["height"].as_f64(), Some(59.25), "and fifty-nine and a quarter tall");
}

/// The **tables** of a sheet are imported: the ranges with a name and
/// a way of being totalled.
#[test]
fn the_tables_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/table" Target="../tables/table1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><tableParts count="1"><tablePart r:id="rId1"/></tableParts></worksheet>"#,
        ),
        (
            "xl/tables/table1.xml",
            r#"<table id="1" name="Table1" displayName="Table1" ref="$A$1:$C$5" totalsRowCount="1"><tableColumns count="3"><tableColumn id="1" name="who"/><tableColumn id="2" name="amount" totalsRowFunction="sum" totalsRowLabel="Total"/><tableColumn id="3" name="when" totalsRowFunction="average"/></tableColumns></table>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["tables"].as_u64(), Some(1), "one table was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let read = session.execute("grid.table.read", &json!({ "sheet": sheet })).expect("the tables are read");
    let tables = read.data["tables"].as_array().expect("there are tables");
    assert_eq!(tables.len(), 1, "one table");
    assert_eq!(tables[0]["name"].as_str(), Some("Table1"), "its name");
    assert_eq!(tables[0]["range"].as_str(), Some("A1:C5"), "the range it reaches");
    assert_eq!(tables[0]["totals"].as_bool(), Some(true), "and it has a total row");
    assert_eq!(tables[0]["way"].as_str(), Some("sum"), "the way the columns are totalled in");
    assert_eq!(tables[0]["label"].as_str(), Some("Total"), "and the label of the total row");
    let ways = tables[0]["ways"].as_array().expect("there are ways");
    assert_eq!(ways.len(), 2, "two columns have a way of their own");
    assert_eq!(ways[0]["column"].as_u64(), Some(1), "the second column");
    assert_eq!(ways[0]["way"].as_str(), Some("sum"), "is totalled in the sum");
    assert_eq!(ways[1]["column"].as_u64(), Some(2), "and the third column");
    assert_eq!(ways[1]["way"].as_str(), Some("average"), "is totalled in the average");
}

/// The **styles** a cell carries are imported: the font of its text,
/// the fill behind it, how its text lies and the lines on its edges.
#[test]
fn the_styles_of_a_cell_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
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
            r#"<styleSheet><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="14"/><color rgb="FFFF0000"/><name val="Arial"/></font></fonts><fills count="2"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FF00FF00"/></patternFill></fill></fills><borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left style="thin"><color rgb="FF0000FF"/></left></border></borders><cellXfs><xf fontId="0" fillId="0" borderId="0"/><xf fontId="1" fillId="1" borderId="1" applyFont="1" applyFill="1" applyBorder="1"><alignment horizontal="center" vertical="center"/></xf></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["fonts"].as_u64(), Some(1), "one font was imported");
    assert_eq!(outcome.data["fills"].as_u64(), Some(1), "one fill was imported");
    assert_eq!(outcome.data["aligns"].as_u64(), Some(1), "one alignment was imported");
    assert_eq!(outcome.data["borders"].as_u64(), Some(1), "one border was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let read =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(read.data["font"]["name"].as_str(), Some("Arial"), "the face of the text");
    assert_eq!(read.data["font"]["size"].as_f64(), Some(14.0), "the size of it");
    assert_eq!(read.data["font"]["bold"].as_bool(), Some(true), "and it is bold");
    assert_eq!(read.data["font"]["colour"].as_str(), Some("#ff0000"), "and red");
    assert_eq!(read.data["fill"].as_str(), Some("#00ff00"), "the colour behind the text");
    assert_eq!(
        read.data["align"]["horizontal"].as_str(),
        Some("center"),
        "how the text lies across the cell"
    );
    assert_eq!(read.data["align"]["vertical"].as_str(), Some("middle"), "and how it lies down it");
    let borders = session
        .execute("grid.cell.border.get", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the borders are read");
    assert_eq!(borders.data["borders"]["left"]["style"].as_str(), Some("thin"), "the line on the left");
    assert_eq!(borders.data["borders"]["left"]["colour"].as_str(), Some("#0000ff"), "and it is blue");
}

/// The **protection** of a sheet and the **lock** of a cell are
/// imported.
#[test]
fn the_protection_of_a_sheet_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
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
        ("xl/styles.xml", r#"<styleSheet><cellXfs><xf><protection locked="0"/></xf></cellXfs></styleSheet>"#),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetProtection sheet="1" sort="0" autoFilter="0" formatCells="0"/><sheetData><row r="1"><c r="A1" s="0" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["protections"].as_u64(), Some(1), "one sheet was protected");
    assert_eq!(outcome.data["locks"].as_u64(), Some(1), "one cell was locked");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    // The **sheet** is protected: the sorting, the filtering and the
    // formatting are still allowed, and the cells are locked.
    let read = session.execute("grid.sheet.get", &json!({ "sheet": sheet })).expect("the sheet is read");
    assert_eq!(read.data["protection"]["sort"].as_bool(), Some(true), "the sorting is allowed");
    assert_eq!(read.data["protection"]["filter"].as_bool(), Some(true), "and the filtering");
    assert_eq!(read.data["protection"]["format"].as_bool(), Some(true), "and the formatting");
    // The **cell** says of itself that it is unlocked, and its own word
    // comes before the sheet's.
    let read =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(read.data["locked"].as_bool(), Some(false), "the cell is unlocked");
}

/// The **rows** and the **columns** of a sheet are imported: their
/// heights, their widths and which of them are hidden.
#[test]
fn the_rows_and_the_columns_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><cols><col min="1" max="1" width="12.5" customWidth="1"/><col min="3" max="3" width="8" hidden="1"/></cols><sheetData><row r="1" ht="30" customHeight="1"><c r="A1" t="str"><v>x</v></c></row><row r="2" hidden="1"><c r="A2" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    // The **sheet** holds the height of the row, the width of the
    // column, and the rows and the columns that are hidden.
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["rows"]["1"].as_f64(), Some(30.0), "the row is thirty points high");
    assert_eq!(node.data["columns"]["A"].as_f64(), Some(12.5), "the column is twelve and a half wide");
    assert_eq!(node.data["hidden_rows"].as_array().map(Vec::len), Some(1), "one row is hidden");
    assert_eq!(node.data["hidden_rows"][0].as_u64(), Some(2), "and it is the second row");
    assert_eq!(node.data["hidden_columns"].as_array().map(Vec::len), Some(1), "one column is hidden");
    assert_eq!(node.data["hidden_columns"][0].as_str(), Some("C"), "and it is the third column");
}

/// The **default size** of a sheet is imported: the width of a column
/// and the height of a row that has no size of its own.
#[test]
fn the_default_size_of_a_sheet_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetFormatPr defaultColWidth="12.25" defaultRowHeight="24"/><sheetData/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["default_width"].as_f64(), Some(12.25), "the columns are twelve and a quarter wide");
    assert_eq!(node.data["default_height"].as_f64(), Some(24.0), "and the rows are twenty four points high");
}

/// The **merges** of a sheet are imported: the ranges that are one
/// cell.
#[test]
fn the_merges_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="str"><v>the value</v></c></row></sheetData><mergeCells count="1"><mergeCell ref="A1:B2"/></mergeCells></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["merges"].as_u64(), Some(1), "one merge was imported");
    assert_eq!(outcome.data["unread_merges"].as_u64(), Some(0), "and none was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["merges"][0].as_str(), Some("A1:B2"), "the range is merged");
}

/// A merge with a **value under it** is not imported: the value of a
/// merged region is the value of its top left cell, and the values of
/// the others would be lost.
#[test]
fn a_merge_with_a_value_under_it_is_not_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="str"><v>the value</v></c><c r="B1" t="str"><v>another</v></c></row></sheetData><mergeCells count="1"><mergeCell ref="A1:B1"/></mergeCells></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["merges"].as_u64(), Some(0), "no merge was imported");
    assert_eq!(outcome.data["unread_merges"].as_u64(), Some(1), "and one was left out");
}

/// Two merges that **touch** are not both imported: a cell that is in
/// two regions is in none of them.
#[test]
fn two_merges_that_touch_are_not_both_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><mergeCells count="2"><mergeCell ref="A1:B2"/><mergeCell ref="B2:C3"/></mergeCells></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["merges"].as_u64(), Some(1), "one merge was imported");
    assert_eq!(outcome.data["unread_merges"].as_u64(), Some(1), "and one was left out");
}

/// The **view** of a sheet is imported: what is drawn of it, and what
/// is not.
#[test]
fn the_view_of_a_sheet_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetViews><sheetView showGridLines="0" rightToLeft="1" showFormulas="1" zoomScale="150"><pane xSplit="2" ySplit="3" topLeftCell="C4" state="frozen"/></sheetView></sheetViews><sheetData/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["views"].as_u64(), Some(1), "one view was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["view"]["gridlines"].as_bool(), Some(false), "the gridlines are not drawn");
    assert_eq!(node.data["view"]["right_to_left"].as_bool(), Some(true), "and the sheet runs right to left");
    assert_eq!(node.data["view"]["show_formulas"].as_bool(), Some(true), "and the formulas are shown");
    assert_eq!(node.data["view"]["freeze"][0].as_u64(), Some(3), "and three rows are frozen");
    assert_eq!(node.data["view"]["freeze"][1].as_u64(), Some(2), "and two columns");
    assert_eq!(node.data["zoom"].as_u64(), Some(150), "and the sheet is at a hundred and fifty percent");
}

/// The **conditional formats** of a sheet are imported: the rules that
/// are about a range of cells.
#[test]
fn the_conditional_formats_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
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
            r##"<styleSheet><dxfs count="1"><dxf><numFmt numFmtId="0" formatCode="#,##0.00"/><font><color rgb="FF9C0006"/></font><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill><border><top style="thin"><color rgb="FF00B050"/></top></border></dxf></dxfs></styleSheet>"##,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="cellIs" operator="greaterThan" priority="1" dxfId="0" stopIfTrue="1"><formula>10</formula></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["conditionals"].as_u64(), Some(1), "one rule was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let rule = &node.data["conditionals"][0];
    assert_eq!(rule["range"].as_str(), Some("A1:A5"), "the range it is about");
    assert_eq!(rule["op"].as_str(), Some(">"), "the operation");
    assert_eq!(rule["value"].as_str(), Some("10"), "the value");
    assert_eq!(rule["fill"].as_str(), Some("#ffc7ce"), "the colour behind the cells");
    assert_eq!(rule["colour"].as_str(), Some("#9c0006"), "and the colour of their text");
    assert_eq!(rule["border"]["top"]["style"].as_str(), Some("thin"), "and the border they draw");
    assert_eq!(rule["border"]["top"]["colour"].as_str(), Some("#00b050"), "and the colour of it");
    assert_eq!(rule["format"].as_str(), Some("#,##0.00"), "and the format they are written with");
    assert_eq!(rule["stop"].as_bool(), Some(true), "and it stops the walk");
    assert_eq!(rule["priority"].as_u64(), Some(1), "and it is tried first");
}

/// The **validations** of a sheet are imported: the rules on what a
/// cell may hold.
#[test]
fn the_validations_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><dataValidations count="3"><dataValidation type="list" allowBlank="1" sqref="B1:B5"><formula1>"a,b,c"</formula1></dataValidation><dataValidation type="whole" operator="between" errorStyle="warning" sqref="C1" errorTitle="too big" error="not between one and ten"><formula1>1</formula1><formula2>10</formula2></dataValidation><dataValidation type="custom" sqref="D1"><formula1>A1&gt;0</formula1></dataValidation></dataValidations></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["validations"].as_u64(), Some(2), "two rules were imported");
    assert_eq!(outcome.data["unread_validations"].as_u64(), Some(1), "one rule was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let rule = &node.data["validations"][0];
    assert_eq!(rule["range"].as_str(), Some("B1:B5"), "the range it is about");
    assert_eq!(rule["kind"].as_str(), Some("list"), "the kind of the rule");
    assert_eq!(rule["formula1"].as_str(), Some("\"a,b,c\""), "the list of words");
    assert_eq!(rule["allow_blank"].as_bool(), Some(true), "a blank is allowed");
    assert_eq!(rule["dropdown"].as_bool(), Some(true), "and the list is shown in the cell");
    let rule = &node.data["validations"][1];
    assert_eq!(rule["range"].as_str(), Some("C1"), "the range of the second rule");
    assert_eq!(rule["kind"].as_str(), Some("whole"), "a whole number");
    assert_eq!(rule["operator"].as_str(), Some("between"), "between two");
    assert_eq!(rule["formula1"].as_str(), Some("1"), "one");
    assert_eq!(rule["formula2"].as_str(), Some("10"), "and ten");
    assert_eq!(rule["error_style"].as_str(), Some("warning"), "and it warns rather than refuses");
    assert_eq!(rule["error_title"].as_str(), Some("too big"), "with a title for the error");
    // The imported rules are the tree's own rules: they check a value.
    let check =
        session.execute("grid.validation.check", &json!({ "sheet": sheet, "cell": "B1", "input": "b" }));
    let check = check.expect("the value is checked");
    assert_eq!(check.data["ok"].as_bool(), Some(true), "a word of the list passes");
    let check =
        session.execute("grid.validation.check", &json!({ "sheet": sheet, "cell": "B1", "input": "z" }));
    let check = check.expect("the value is checked");
    assert_eq!(check.data["ok"].as_bool(), Some(false), "a word that is not of the list does not");
    let check =
        session.execute("grid.validation.check", &json!({ "sheet": sheet, "cell": "C1", "input": "50" }));
    let check = check.expect("the value is checked");
    assert_eq!(check.data["ok"].as_bool(), Some(false), "a number out of the range does not");
    assert_eq!(check.data["style"].as_str(), Some("warning"), "and the rule warns");
}

/// The **printing** of a sheet is imported: the area it prints, and the
/// titles it repeats on every page.
#[test]
fn the_printing_of_a_sheet_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets><definedNames><definedName name="_xlnm.Print_Area" localSheetId="0">One!$A$1:$C$10</definedName><definedName name="_xlnm.Print_Titles" localSheetId="0">One!$A:$B,One!$1:$2</definedName></definedNames></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["prints"].as_u64(), Some(2), "the area and the titles were imported");
    assert_eq!(outcome.data["unread_prints"].as_u64(), Some(0), "and none was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["print_area"].as_str(), Some("A1:C10"), "the area the sheet prints");
    assert_eq!(node.data["title_rows"][0].as_u64(), Some(0), "the first row that repeats");
    assert_eq!(node.data["title_rows"][1].as_u64(), Some(1), "and the second");
    assert_eq!(node.data["title_columns"][0].as_u64(), Some(0), "the first column that repeats");
    assert_eq!(node.data["title_columns"][1].as_u64(), Some(1), "and the second");
}

/// The **marks** of a print are imported: a file that prints its
/// gridlines and its headings has a sheet that prints them.
#[test]
fn the_marks_of_a_print_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><printOptions gridLines="1" headings="1"/><sheetData/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["marks"].as_u64(), Some(2), "both marks were imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let outcome = session.execute("grid.print.read", &json!({ "sheet": sheet })).expect("the setup is read");
    assert_eq!(outcome.data["gridlines"].as_bool(), Some(true), "the gridlines are printed");
    assert_eq!(outcome.data["headings"].as_bool(), Some(true), "and the headings are printed");
}

/// A file that says nothing about the **marks** of its print has none:
/// the gridlines and the headings are not printed.
#[test]
fn a_file_with_no_marks_has_none() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["marks"].as_u64(), Some(0), "no mark was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let outcome = session.execute("grid.print.read", &json!({ "sheet": sheet })).expect("the setup is read");
    assert_eq!(outcome.data["gridlines"].as_bool(), Some(false), "the gridlines are not printed");
    assert_eq!(outcome.data["headings"].as_bool(), Some(false), "and the headings are not printed");
}

/// The **named styles** of a file are imported: the workbook knows the
/// styles the file names, and a cell may be given one of them.
#[test]
fn the_named_styles_of_a_file_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId2"/></sheets></workbook>"#,
        ),
        (
            "xl/styles.xml",
            r#"<styleSheet><fonts count="1"><font><sz val="11"/><name val="Calibri"/></font></fonts><fills count="1"><fill><patternFill patternType="none"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="4" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Money" xfId="0"/></cellStyles></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1234.5</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    // The **workbook** knows the style the file names.
    let outcome =
        session.execute("grid.workbook.get", &json!({ "workbook": workbook })).expect("the workbook is read");
    let styles = outcome.data["styles"].as_array().expect("the workbook has styles");
    assert_eq!(styles[0]["name"].as_str(), Some("Money"), "the style is called Money");
    assert_eq!(styles[0]["format"].as_str(), Some("#,##0.00"), "and it is written with two decimals");
    // A cell may be **given** it, and the cell is then written with the
    // style's format.
    session
        .execute("grid.cell.style", &json!({ "sheet": sheet, "cell": "A1", "style": "Money" }))
        .expect("the cell is given the style");
    let outcome =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(outcome.data["text"].as_str(), Some("1,234.50"), "the cell is written with the style");
}

/// The **edit ranges** of a file are imported: a cell in one of them
/// may be changed while the sheet is protected, and a cell outside
/// them may not.
#[test]
fn the_edit_ranges_of_a_file_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetProtection sheet="1"/><protectedRanges><protectedRange name="Range1" sqref="A1:B2"/></protectedRanges><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["unread_range_names"].as_u64(), Some(1), "the name was not kept");
    assert_eq!(outcome.data["unread_passwords"].as_u64(), Some(0), "and there was no password");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let outcome = session
        .execute("grid.sheet.edit.read", &json!({ "sheet": sheet }))
        .expect("the edit ranges are read");
    assert_eq!(outcome.data["ranges"][0].as_str(), Some("A1:B2"), "the range that stays editable");
    // A cell **in** the range may be changed.
    session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "A1", "value": 2 }))
        .expect("the cell is in the range");
    // A cell **outside** it may not.
    let error = session
        .execute("grid.cell.set", &json!({ "sheet": sheet, "cell": "C1", "value": 2 }))
        .expect_err("the cell is locked");
    assert!(error.to_string().contains("locked"), "the error says so: {error}");
}

/// The **protection** of a workbook is imported: a file that locks its
/// structure has a workbook whose structure is locked, and its sheets
/// may not be renamed.
#[test]
fn the_protection_of_a_workbook_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><workbookProtection workbookPassword="83AF" lockStructure="1"/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["unread_passwords"].as_u64(), Some(1), "the password was not carried");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let error = session
        .execute("grid.sheet.rename", &json!({ "sheet": sheet, "name": "Two" }))
        .expect_err("the sheet may not be renamed");
    assert!(error.to_string().contains("protected"), "the error says so: {error}");
}

/// The **centre** of a sheet on the page is imported: a file that
/// centres its sheet has a sheet that is centred.
#[test]
fn the_centre_of_a_sheet_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><printOptions horizontalCentered="1" verticalCentered="1"/><sheetData/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["centred"].as_u64(), Some(1), "the sheet was centred");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["print_centred"]["horizontal"].as_bool(), Some(true), "sideways");
    assert_eq!(node.data["print_centred"]["vertical"].as_bool(), Some(true), "and down the page");
}

/// The **header** and the **footer** of a file are imported: the texts
/// of the odd, the even and the first pages.
#[test]
fn the_header_and_the_footer_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><headerFooter differentOddEven="1" differentFirst="1"><oddHeader>&amp;LPage &amp;P of &amp;N</oddHeader><oddFooter>&amp;A</oddFooter><evenHeader>&amp;LEven</evenHeader><firstHeader>&amp;LFirst</firstHeader></headerFooter><sheetData/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["unread_headers"].as_u64(), Some(0), "every header was read");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(
        node.data["print_header"]["odd"].as_str(),
        Some("&LPage &P of &N"),
        "the header of the odd pages"
    );
    assert_eq!(node.data["print_header"]["even"].as_str(), Some("&LEven"), "the header of the even pages");
    assert_eq!(node.data["print_header"]["first"].as_str(), Some("&LFirst"), "the header of the first page");
    assert_eq!(node.data["print_footer"]["odd"].as_str(), Some("&A"), "the footer of the odd pages");
}

/// The header of a file is **printed**: the codes in it are worked out
/// for the page they are on, exactly as they are for a header that was
/// set by hand.
#[test]
fn the_header_of_a_file_is_printed() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><headerFooter><oddHeader>&amp;LPage &amp;P of &amp;N</oddHeader></headerFooter><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let outcome =
        session.execute("grid.print.pages", &json!({ "sheet": sheet })).expect("the pages are read");
    let pages = outcome.data["pages"].as_array().expect("the pages are a list");
    assert_eq!(pages.len(), 1, "the sheet is one page");
    assert_eq!(pages[0]["header"]["left"]["text"], json!("Page 1 of 1"), "the header is printed");
}

/// A header that holds a code the tree does not print is **not**
/// imported: the tree prints no date, and a header with a date in it
/// is not a header the tree can print. It is counted, and said.
#[test]
fn a_header_the_tree_cannot_print_is_not_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><headerFooter><oddHeader>&amp;D</oddHeader><oddFooter>&amp;T</oddFooter></headerFooter><sheetData/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["unread_headers"].as_u64(), Some(2), "both were left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data.get("print_header"), None, "the sheet has no header");
    assert_eq!(node.data.get("print_footer"), None, "and no footer");
}

/// A **print area of one cell** is not imported: an area of one cell
/// is a cell, and the tree's own command refuses it.
#[test]
fn a_print_area_of_one_cell_is_not_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets><definedNames><definedName name="_xlnm.Print_Area" localSheetId="0">One!$A$1</definedName></definedNames></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["prints"].as_u64(), Some(0), "nothing was imported");
    assert_eq!(outcome.data["unread_prints"].as_u64(), Some(1), "and the area was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert!(node.data.get("print_area").is_none(), "and the sheet prints no area of its own");
}

/// The **outline** of a sheet is imported: the groups of rows and of
/// columns, and where the summary of a group sits.
#[test]
fn the_outline_of_a_sheet_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetPr><outlinePr summaryBelow="0"/></sheetPr><cols><col min="3" max="3" outlineLevel="1"/></cols><sheetData><row r="2" hidden="1" outlineLevel="2"/><row r="3" outlineLevel="1"/></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["outlines"].as_u64(), Some(2), "the rows and the columns were imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["row_outline"]["2"].as_u64(), Some(2), "the second row is two deep");
    assert_eq!(node.data["row_outline"]["3"].as_u64(), Some(1), "and the third is one deep");
    assert_eq!(node.data["column_outline"]["C"].as_u64(), Some(1), "and the third column is one deep");
    assert_eq!(node.data["summary_below"].as_bool(), Some(false), "the summary is above the group");
    assert!(node.data.get("summary_right").is_none(), "and the file said nothing of the columns");
}

/// The **threads** of a sheet are imported: the notes of a newer file,
/// with the replies under them.
#[test]
fn the_threads_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.microsoft.com/office/2017/10/relationships/person" Target="persons/person.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.microsoft.com/office/2017/10/relationships/threadedComment" Target="../threadedComments/threadedComment1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
        (
            "xl/persons/person.xml",
            r#"<personList><person id="{p1}" displayName="ann"/><person id="{p2}" displayName="bob"/></personList>"#,
        ),
        (
            "xl/threadedComments/threadedComment1.xml",
            r#"<ThreadedComments><threadedComment ref="B2" dT="2024-01-01T00:00:00.00" personId="{p1}" id="{t1}" done="1"><text>the first</text></threadedComment><threadedComment ref="B2" dT="2024-01-02T00:00:00.00" personId="{p2}" id="{t2}" parentId="{t1}"><text>the answer</text></threadedComment></ThreadedComments>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["comments"].as_u64(), Some(1), "one note was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let note = &node.data["comments"]["B2"];
    assert_eq!(note["text"].as_str(), Some("the first"), "the text of the note");
    assert_eq!(note["author"].as_str(), Some("ann"), "and who wrote it");
    assert_eq!(note["resolved"].as_bool(), Some(true), "and that the thread is resolved");
    assert_eq!(note["replies"][0]["text"].as_str(), Some("the answer"), "the reply under it");
    assert_eq!(note["replies"][0]["author"].as_str(), Some("bob"), "and who wrote the reply");
    assert_eq!(note["date"].as_str(), Some("2024-01-01T00:00:00.00"), "and when the note was written");
    assert_eq!(
        note["replies"][0]["date"].as_str(),
        Some("2024-01-02T00:00:00.00"),
        "and when the reply was written"
    );
}

/// The **page setup** of a sheet is imported: the paper, the
/// orientation, the margins, the scale and the fit.
#[test]
fn the_page_setup_of_a_sheet_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetPr><pageSetUpPr fitToPage="1"/></sheetPr><sheetData/><pageMargins left="0.7" right="0.7" top="0.75" bottom="0.75" header="0.3" footer="0.3"/><pageSetup paperSize="9" orientation="landscape" scale="80" fitToWidth="1" fitToHeight="2"/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["setups"].as_u64(), Some(1), "the page setup was imported");
    assert_eq!(outcome.data["unread_setups"].as_u64(), Some(0), "and all of it was read");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let setup = &node.data["print"];
    assert_eq!(setup["paper"].as_str(), Some("A4"), "the paper it prints on");
    assert_eq!(setup["orientation"].as_str(), Some("landscape"), "and the way it lies");
    // The tree's margins are in points, and the file's are in inches.
    let left = setup["margins"]["left"].as_f64().expect("the left margin is a number");
    assert!((left - 50.4).abs() < 0.001, "the left margin is half an inch and a bit: {left}");
    assert_eq!(setup["scale"].as_f64(), Some(80.0), "and the scale");
    assert_eq!(setup["fit"]["wide"].as_f64(), Some(1.0), "and the fit, a page wide");
    assert_eq!(setup["fit"]["tall"].as_f64(), Some(2.0), "and two pages tall");
}

/// A formula this tree does not have — an **array** or a what-if **data
/// table** — is not imported: the cell keeps the value the file cached
/// for it.
#[test]
fn an_array_formula_is_not_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><f t="array" ref="$A$1:$A$2">SUM(B1:B2)</f><v>3</v></c><c r="A2"><v>3</v></c></row><row r="2"><c r="B2"><f t="dataTable" ref="B2:C3" dt2D="1" r1="B1"/><v>5</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["arrays"].as_u64(), Some(1), "one array formula was left out");
    assert_eq!(outcome.data["data_tables"].as_u64(), Some(1), "and one data table");
    assert_eq!(outcome.data["formulas"].as_u64(), Some(0), "and no formula was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let cell = &node.data["cells"]["A1"];
    assert!(cell.get("formula").is_none(), "the array is not a formula of the cell");
    assert_eq!(cell["value"].as_f64(), Some(3.0), "and the cell keeps the value the file cached");
    let cell = &node.data["cells"]["B2"];
    assert!(cell.get("formula").is_none(), "the table is not a formula of the cell either");
    assert_eq!(cell["value"].as_f64(), Some(5.0), "and its value is kept");
}

/// The **breaks** of the printing of a sheet are imported: the rows and
/// the columns a page begins at.
#[test]
fn the_breaks_of_the_printing_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><rowBreaks count="1" manualBreakCount="1"><brk id="10" max="16383" man="1"/></rowBreaks><colBreaks count="1" manualBreakCount="1"><brk id="2" max="1048575" man="1"/></colBreaks></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["breaks"].as_u64(), Some(2), "the row and the column were imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["row_breaks"][0].as_u64(), Some(10), "the row a page begins at");
    assert_eq!(node.data["column_breaks"][0].as_u64(), Some(2), "and the column a page begins at");
}

/// The **sparklines** of a sheet are imported: the little drawings in
/// its cells.
#[test]
fn the_sparklines_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><extLst><ext uri="{05C60535-1F16-4fd2-B633-F4F36F0B64E0}"><x14:sparklineGroups xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"><x14:sparklineGroup type="column" markers="1"><x14:colorSeries rgb="FF376092"/><x14:sparklines><x14:sparkline><xm:f>One!B2:F2</xm:f><xm:sqref>G2</xm:sqref></x14:sparkline></x14:sparklines></x14:sparklineGroup></x14:sparklineGroups></ext></extLst></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["sparklines"].as_u64(), Some(1), "one sparkline was imported");
    assert_eq!(outcome.data["unread_sparklines"].as_u64(), Some(0), "and none was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let sparkline = &node.data["sparklines"]["G2"];
    assert_eq!(sparkline["range"].as_str(), Some("B2:F2"), "the range it reads");
    assert_eq!(sparkline["kind"].as_str(), Some("column"), "the kind of it");
    assert_eq!(sparkline["markers"].as_bool(), Some(true), "and that its markers are drawn");
    let colour = sparkline["colour"].as_array().expect("the colour is an array of numbers");
    assert_eq!(colour.len(), 4, "with four parts");
    // The tree's colours are in parts of a whole, and not in bytes.
    let red = colour[0].as_f64().expect("the red is a number");
    assert!((red - 0x37 as f64 / 255.0).abs() < 0.001, "the red of it: {red}");
    let green = colour[1].as_f64().expect("the green is a number");
    assert!((green - 0x60 as f64 / 255.0).abs() < 0.001, "the green: {green}");
    let blue = colour[2].as_f64().expect("the blue is a number");
    assert!((blue - 0x92 as f64 / 255.0).abs() < 0.001, "and the blue: {blue}");
}

/// The **hidden sheets** of a workbook are imported: a sheet the file
/// hides is hidden here too.
#[test]
fn the_hidden_sheets_of_a_workbook_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/><sheet name="Two" sheetId="2" state="hidden" r:id="rId2"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
        ("xl/worksheets/sheet2.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["hidden_sheets"].as_u64(), Some(1), "one sheet was hidden");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let node = session.document().node(workbook).expect("the workbook is there");
    let first = node.children[0].id;
    let second = node.children[1].id;
    let first = session.document().node(first).expect("the first sheet is there");
    assert!(first.data.get("hidden").is_none(), "the first sheet is in sight");
    let second = session.document().node(second).expect("the second sheet is there");
    assert_eq!(second.data["hidden"].as_bool(), Some(true), "and the second is hidden");
}

/// The **colour** of a sheet's tab is imported.
#[test]
fn the_colour_of_a_sheets_tab_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetPr><tabColor rgb="FF00B050"/></sheetPr><sheetData/></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["tab_colours"].as_u64(), Some(1), "the tab was coloured");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["tab_color"].as_str(), Some("#00b050"), "the colour of the tab");
}

/// The **charts** of a sheet are imported: the drawings of its cells.
#[test]
fn the_charts_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/><drawing r:id="rId2"/></worksheet>"#),
        (
            "xl/drawings/drawing1.xml",
            r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
        ),
        (
            "xl/charts/chart1.xml",
            r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:title><c:tx><c:rich><a:p><a:r><a:t>The Sales</a:t></a:r></a:p></c:rich></c:tx></c:title><c:plotArea><c:catAx><c:title><c:tx><c:rich><a:p><a:r><a:t>Months</a:t></a:r></a:p></c:rich></c:tx></c:title></c:catAx><c:valAx><c:title><c:tx><c:rich><a:p><a:r><a:t>Money</a:t></a:r></a:p></c:rich></c:tx></c:title><c:majorGridlines/></c:valAx><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val><c:spPr><a:solidFill><a:srgbClr val="FF4472C4"/></a:solidFill></c:spPr></c:ser></c:barChart></c:plotArea><c:legend><c:legendPos val="b"/></c:legend></c:chart></c:chartSpace>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["charts"].as_u64(), Some(1), "one chart was imported");
    assert_eq!(outcome.data["unread_charts"].as_u64(), Some(0), "and none was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let chart = node.children.first().expect("the sheet holds a chart");
    assert_eq!(chart.kind, "grid.chart", "and it is a chart");
    assert_eq!(chart.name, "The Sales", "named by its title");
    assert_eq!(chart.data["range"].as_str(), Some("A2:B10"), "reading the range of its cells");
    assert_eq!(chart.data["kind"].as_str(), Some("column"), "the kind of it");
    assert_eq!(chart.data["legend"].as_str(), Some("bottom"), "where its legend is");
    assert_eq!(chart.data["xTitle"].as_str(), Some("Months"), "the title of its axis of the categories");
    assert_eq!(chart.data["yTitle"].as_str(), Some("Money"), "and the title of the axis of the values");
    assert_eq!(chart.data["gridlines"].as_bool(), Some(true), "and that its gridlines are shown");
    assert_eq!(chart.data["colours"][0].as_str(), Some("#4472c4"), "and the colour of its series");
}

/// An **extended chart** — a histogram, a waterfall, a treemap — is not
/// a chart the tree has, and it is left out and counted.
#[test]
fn an_extended_chart_is_left_out() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        (
            "xl/drawings/drawing1.xml",
            r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships" xmlns:cx="http://schemas.microsoft.com/office/drawing/2015/9/8/chartex"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.microsoft.com/office/drawing/2014/chartex"><cx:chart r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.microsoft.com/office/2014/relationships/chartEx" Target="../charts/chartEx1.xml"/></Relationships>"#,
        ),
        (
            "xl/charts/chartEx1.xml",
            r#"<cx:chartSpace xmlns:cx="http://schemas.microsoft.com/office/drawing/2015/9/8/chartex" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><cx:chartData><cx:data id="0"><cx:strDim type="cat"><cx:f>_xlchart.v1.0</cx:f></cx:strDim><cx:numDim type="val"><cx:f>_xlchart.v1.1</cx:f></cx:numDim></cx:data></cx:chartData><cx:chart><cx:plotArea><cx:plotAreaRegion><cx:series layoutId="waterfall" uniqueId="{1B0D9F02-A0E7-4C1C-9F45-2E1B5A3A0B4C}"><cx:dataId val="0"/></cx:series></cx:plotAreaRegion></cx:plotArea></cx:chart></cx:chartSpace>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["charts"].as_u64(), Some(0), "no chart was imported");
    assert_eq!(outcome.data["unread_charts"].as_u64(), Some(1), "and the chart was left out");
}

/// A chart that draws **two kinds** in one is not a chart the tree has,
/// and it is left out and counted.
#[test]
fn a_chart_of_two_kinds_is_left_out() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        (
            "xl/drawings/drawing1.xml",
            r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
        ),
        (
            "xl/charts/chart1.xml",
            r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val></c:ser></c:barChart><c:lineChart><c:grouping val="standard"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$C$2:$C$10</c:f></c:numRef></c:val></c:ser></c:lineChart></c:plotArea></c:chart></c:chartSpace>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["charts"].as_u64(), Some(0), "no chart was imported");
    assert_eq!(outcome.data["unread_charts"].as_u64(), Some(1), "and the chart was left out");
}

/// The **markers** of a chart are imported.
#[test]
fn the_markers_of_a_chart_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        (
            "xl/drawings/drawing1.xml",
            r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
        ),
        (
            "xl/charts/chart1.xml",
            r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:lineChart><c:grouping val="standard"/><c:marker val="1"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val></c:ser></c:lineChart></c:plotArea></c:chart></c:chartSpace>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["charts"].as_u64(), Some(1), "one chart was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let chart = node.children.first().expect("the sheet holds a chart");
    assert_eq!(chart.data["markers"].as_bool(), Some(true), "and that its points are shown");
}

/// The **trendlines** of a chart are imported: the fits of the lines
/// of its series.
#[test]
fn the_trendlines_of_a_chart_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        (
            "xl/drawings/drawing1.xml",
            r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
        ),
        (
            "xl/charts/chart1.xml",
            r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:lineChart><c:grouping val="standard"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val><c:trendline><c:trendlineType val="linear"/><c:name>the fit</c:name><c:forward val="3"/><c:dispEq val="1"/></c:trendline></c:ser></c:lineChart></c:plotArea></c:chart></c:chartSpace>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["charts"].as_u64(), Some(1), "one chart was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let chart = node.children.first().expect("the sheet holds a chart");
    let trendlines = chart.data["trendlines"].as_array().expect("the chart holds its trendlines");
    assert_eq!(trendlines.len(), 1, "one trendline");
    assert_eq!(trendlines[0]["kind"].as_str(), Some("linear"), "and it is a straight fit");
    assert_eq!(trendlines[0]["series"].as_u64(), Some(0), "of the first series");
    assert_eq!(trendlines[0]["name"].as_str(), Some("the fit"), "with a name of its own");
    assert_eq!(trendlines[0]["forward"].as_f64(), Some(3.0), "carried three forward");
    assert_eq!(trendlines[0]["equation"].as_bool(), Some(true), "and its equation is shown");
    // The chart is read back with its trendlines.
    let read = session.execute("grid.chart.read", &json!({ "chart": chart.id })).expect("the chart is read");
    assert_eq!(read.data["trendlines"].as_array().map(|list| list.len()), Some(1), "and the chart says so");
}

/// The **smooth line** of a chart is imported, and a chart whose
/// series do not all say the same is left out.
#[test]
fn the_smooth_line_of_a_chart_is_imported() {
    for (smooths, imported) in [(vec!["1"], 1u64), (vec!["1", "0"], 0)] {
        let mut session = session();
        let series: String = smooths
            .iter()
            .enumerate()
            .map(|(index, smooth)| {
                let column = (b'B' + index as u8) as char;
                format!(
                    r#"<c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!${column}$2:${column}$10</c:f></c:numRef></c:val><c:smooth val="{smooth}"/></c:ser>"#
                )
            })
            .collect();
        let bytes = a_package(&[
            (
                "_rels/.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
            ),
            (
                "xl/_rels/workbook.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
            ),
            (
                "xl/workbook.xml",
                r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            ("xl/worksheets/sheet1.xml", r#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#),
            (
                "xl/worksheets/_rels/sheet1.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
            ),
            (
                "xl/drawings/drawing1.xml",
                r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
            ),
            (
                "xl/drawings/_rels/drawing1.xml.rels",
                r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
            ),
            (
                "xl/charts/chart1.xml",
                &format!(
                    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:lineChart><c:grouping val="standard"/>{series}</c:lineChart></c:plotArea></c:chart></c:chartSpace>"#
                ),
            ),
        ]);
        let path = a_file(bytes);
        let outcome =
            session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
        assert_eq!(outcome.data["charts"].as_u64(), Some(imported), "the charts that were imported");
        assert_eq!(outcome.data["unread_charts"].as_u64(), Some(1 - imported), "and the ones left out");
        if imported == 1 {
            let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
            let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
            let node = session.document().node(sheet).expect("the sheet is there");
            let chart = node.children.first().expect("the sheet holds a chart");
            assert_eq!(chart.data["smooth"].as_bool(), Some(true), "and that its line is smooth");
        }
    }
}

/// The **stacking** of a chart is imported: a chart of a file may be
/// stacked, and the tree's chart keeps it.
#[test]
fn the_stacking_of_a_chart_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        (
            "xl/drawings/drawing1.xml",
            r#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
        ),
        (
            "xl/charts/chart1.xml",
            r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:grouping val="stacked"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["charts"].as_u64(), Some(1), "one chart was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let chart = node.children.first().expect("the sheet holds a chart");
    assert_eq!(chart.data["kind"].as_str(), Some("column"), "the kind of the chart");
    assert_eq!(chart.data["stacking"].as_str(), Some("stacked"), "and that it is stacked");
}

/// A cell that holds a **date** in the file's own words is imported:
/// the date becomes the number the tree counts the days in.
#[test]
fn the_dates_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="d"><v>2024-01-01</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    assert_eq!(node.data["cells"]["A1"]["value"].as_f64(), Some(45292.0), "the day is counted");
}

/// The **pivot tables** of a sheet are imported: they become the
/// tree's own pivots.
#[test]
fn the_pivot_tables_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotTable" Target="../pivotTables/pivotTable1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Region</t></is></c><c r="B1" t="inlineStr"><is><t>Product</t></is></c><c r="C1" t="inlineStr"><is><t>Amount</t></is></c></row></sheetData></worksheet>"#,
        ),
        (
            "xl/pivotTables/pivotTable1.xml",
            r#"<pivotTableDefinition name="The Pivot" cacheId="1"><location ref="A1:B5"/><rowFields count="1"><field x="0"/></rowFields><colFields count="1"><field x="1"/></colFields><dataFields count="1"><dataField name="Sum of Amount" fld="2" baseField="0" baseItem="0"/></dataFields></pivotTableDefinition>"#,
        ),
        (
            "xl/pivotTables/_rels/pivotTable1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheDefinition" Target="../pivotCacheDefinition1.xml"/></Relationships>"#,
        ),
        (
            "xl/pivotCacheDefinition1.xml",
            r#"<pivotCacheDefinition><cacheSource type="worksheet"><worksheetSource ref="$A$1:$C$10" sheet="One"/></cacheSource><cacheFields count="3"><cacheField name="Region"><sharedItems/></cacheField><cacheField name="Product"><sharedItems/></cacheField><cacheField name="Amount"><sharedItems/></cacheField></cacheFields></pivotCacheDefinition>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["pivots"].as_u64(), Some(1), "one pivot was imported");
    assert_eq!(outcome.data["unread_pivots"].as_u64(), Some(0), "and none was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let pivot = &node.data["pivots"][0];
    assert_eq!(pivot["name"].as_str(), Some("The Pivot"), "the name of the pivot");
    assert_eq!(pivot["range"].as_str(), Some("A1:C10"), "the cells it reads");
    assert_eq!(pivot["row"].as_str(), Some("A"), "the field it gathers the rows by");
    assert_eq!(pivot["column"].as_str(), Some("B"), "the field it spreads the groups over");
    assert_eq!(pivot["value"].as_str(), Some("C"), "and the field it puts together");
    assert_eq!(pivot["aggregate"].as_str(), Some("sum"), "and the way of putting it together");
}

/// The **grand totals** of a pivot are imported: the file says whether
/// the pivot has them, and the tree keeps what it says.
#[test]
fn the_grand_totals_of_a_pivot_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotTable" Target="../pivotTables/pivotTable1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Region</t></is></c><c r="B1" t="inlineStr"><is><t>Product</t></is></c><c r="C1" t="inlineStr"><is><t>Amount</t></is></c></row></sheetData></worksheet>"#,
        ),
        (
            "xl/pivotTables/pivotTable1.xml",
            r#"<pivotTableDefinition name="The Pivot" cacheId="1" colGrandTotals="0"><location ref="A1:B5"/><rowFields count="1"><field x="0"/></rowFields><dataFields count="1"><dataField name="Sum of Amount" fld="2"/></dataFields></pivotTableDefinition>"#,
        ),
        (
            "xl/pivotTables/_rels/pivotTable1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheDefinition" Target="../pivotCacheDefinition1.xml"/></Relationships>"#,
        ),
        (
            "xl/pivotCacheDefinition1.xml",
            r#"<pivotCacheDefinition><cacheSource type="worksheet"><worksheetSource ref="$A$1:$C$10" sheet="One"/></cacheSource><cacheFields count="3"><cacheField name="Region"/><cacheField name="Product"/><cacheField name="Amount"/></cacheFields></pivotCacheDefinition>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["pivots"].as_u64(), Some(1), "one pivot was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let pivot = &node.data["pivots"][0];
    assert_eq!(pivot["grand_totals_rows"].as_bool(), Some(false), "the file has no grand total of the rows");
    assert_eq!(pivot["grand_totals_cols"].as_bool(), Some(true), "and it has the grand total of the columns");
}

/// The way a pivot **shows its value** is imported: the file says, and
/// the tree keeps it.
#[test]
fn the_way_a_pivot_shows_its_value_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotTable" Target="../pivotTables/pivotTable1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Region</t></is></c><c r="B1" t="inlineStr"><is><t>Product</t></is></c><c r="C1" t="inlineStr"><is><t>Amount</t></is></c></row></sheetData></worksheet>"#,
        ),
        (
            "xl/pivotTables/pivotTable1.xml",
            r#"<pivotTableDefinition name="The Pivot" cacheId="1"><location ref="A1:B5"/><rowFields count="1"><field x="0"/></rowFields><dataFields count="1"><dataField name="Share" fld="2" showDataAs="percentOfTotal"/></dataFields></pivotTableDefinition>"#,
        ),
        (
            "xl/pivotTables/_rels/pivotTable1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheDefinition" Target="../pivotCacheDefinition1.xml"/></Relationships>"#,
        ),
        (
            "xl/pivotCacheDefinition1.xml",
            r#"<pivotCacheDefinition><cacheSource type="worksheet"><worksheetSource ref="$A$1:$C$10" sheet="One"/></cacheSource><cacheFields count="3"><cacheField name="Region"/><cacheField name="Product"/><cacheField name="Amount"/></cacheFields></pivotCacheDefinition>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["pivots"].as_u64(), Some(1), "one pivot was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let pivot = &node.data["pivots"][0];
    assert_eq!(pivot["show_as"].as_str(), Some("percentOfGrandTotal"), "the value is shown as a share");
}

/// The **subtotals** of the groups of a pivot are imported: the file
/// says where they are, and the tree keeps it.
#[test]
fn the_subtotals_of_a_pivot_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotTable" Target="../pivotTables/pivotTable1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Region</t></is></c><c r="B1" t="inlineStr"><is><t>Product</t></is></c><c r="C1" t="inlineStr"><is><t>Amount</t></is></c></row></sheetData></worksheet>"#,
        ),
        (
            "xl/pivotTables/pivotTable1.xml",
            r#"<pivotTableDefinition name="The Pivot" cacheId="1"><location ref="A1:B5"/><pivotFields count="3"><pivotField axis="axisRow" defaultSubtotal="1" subtotalTop="0"><items count="2"><item x="0"/><item t="default"/></items></pivotField><pivotField dataField="1"/><pivotField/></pivotFields><rowFields count="1"><field x="0"/></rowFields><dataFields count="1"><dataField name="Sum of Amount" fld="2"/></dataFields></pivotTableDefinition>"#,
        ),
        (
            "xl/pivotTables/_rels/pivotTable1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheDefinition" Target="../pivotCacheDefinition1.xml"/></Relationships>"#,
        ),
        (
            "xl/pivotCacheDefinition1.xml",
            r#"<pivotCacheDefinition><cacheSource type="worksheet"><worksheetSource ref="$A$1:$C$10" sheet="One"/></cacheSource><cacheFields count="3"><cacheField name="Region"/><cacheField name="Product"/><cacheField name="Amount"/></cacheFields></pivotCacheDefinition>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["pivots"].as_u64(), Some(1), "one pivot was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let pivot = &node.data["pivots"][0];
    assert_eq!(pivot["subtotals"].as_str(), Some("bottom"), "the subtotals are at the bottom");
}

/// The **order** of the items of a pivot is imported: the file says of
/// the field and of the order, and the tree keeps it.
#[test]
fn the_order_of_the_items_of_a_pivot_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotTable" Target="../pivotTables/pivotTable1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Region</t></is></c><c r="B1" t="inlineStr"><is><t>Product</t></is></c><c r="C1" t="inlineStr"><is><t>Amount</t></is></c></row></sheetData></worksheet>"#,
        ),
        (
            "xl/pivotTables/pivotTable1.xml",
            r#"<pivotTableDefinition name="The Pivot" cacheId="1"><location ref="A1:B5"/><pivotFields count="3"><pivotField axis="axisRow" sortType="descending"><items count="2"><item x="0"/><item t="default"/></items></pivotField><pivotField dataField="1"/><pivotField/></pivotFields><rowFields count="1"><field x="0"/></rowFields><dataFields count="1"><dataField name="Sum of Amount" fld="2"/></dataFields></pivotTableDefinition>"#,
        ),
        (
            "xl/pivotTables/_rels/pivotTable1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheDefinition" Target="../pivotCacheDefinition1.xml"/></Relationships>"#,
        ),
        (
            "xl/pivotCacheDefinition1.xml",
            r#"<pivotCacheDefinition><cacheSource type="worksheet"><worksheetSource ref="$A$1:$C$10" sheet="One"/></cacheSource><cacheFields count="3"><cacheField name="Region"/><cacheField name="Product"/><cacheField name="Amount"/></cacheFields></pivotCacheDefinition>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["pivots"].as_u64(), Some(1), "one pivot was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let pivot = &node.data["pivots"][0];
    assert_eq!(pivot["sort"]["field"].as_str(), Some("A"), "the items are ordered by the first column");
    assert_eq!(pivot["sort"]["order"].as_str(), Some("desc"), "and from the last to the first");
}

/// The **grouping** of the dates of a field is imported: the file says
/// of the field and of the way its dates are gathered.
#[test]
fn the_grouping_of_the_dates_of_a_field_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotTable" Target="../pivotTables/pivotTable1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>Date</t></is></c><c r="B1" t="inlineStr"><is><t>Product</t></is></c><c r="C1" t="inlineStr"><is><t>Amount</t></is></c></row></sheetData></worksheet>"#,
        ),
        (
            "xl/pivotTables/pivotTable1.xml",
            r#"<pivotTableDefinition name="The Pivot" cacheId="1"><location ref="A1:B5"/><rowFields count="1"><field x="0"/></rowFields><dataFields count="1"><dataField name="Sum of Amount" fld="2"/></dataFields></pivotTableDefinition>"#,
        ),
        (
            "xl/pivotTables/_rels/pivotTable1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/pivotCacheDefinition" Target="../pivotCacheDefinition1.xml"/></Relationships>"#,
        ),
        (
            "xl/pivotCacheDefinition1.xml",
            r#"<pivotCacheDefinition><cacheSource type="worksheet"><worksheetSource ref="$A$1:$C$10" sheet="One"/></cacheSource><cacheFields count="3"><cacheField name="Date"><fieldGroup base="0"><rangePr groupBy="months" startDate="2024-01-01T00:00:00" endDate="2024-12-31T00:00:00"/><groupItems count="1"><s v="Jan"/></groupItems></fieldGroup></cacheField><cacheField name="Product"/><cacheField name="Amount"/></cacheFields></pivotCacheDefinition>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["pivots"].as_u64(), Some(1), "one pivot was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let pivot = &node.data["pivots"][0];
    assert_eq!(pivot["group"]["field"].as_str(), Some("A"), "the first column is grouped");
    assert_eq!(pivot["group"]["by"].as_str(), Some("months"), "and its dates by the month");
}

/// The **conditional formats** of the extension list are not imported,
/// and the import says so.
#[test]
fn the_conditional_formats_of_the_extension_list_are_not_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><extLst><ext uri="{78C0D931-6437-407d-A8EE-F0AAD7539E65}"><x14:conditionalFormattings xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"><x14:conditionalFormatting><x14:cfRule type="dataBar" id="{1}"/></x14:conditionalFormatting></x14:conditionalFormattings></ext></extLst></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let warnings = outcome.data["warnings"].as_array().expect("the import says what it left out");
    assert!(
        warnings.iter().any(|warning| {
            warning.as_str().is_some_and(|warning| warning.contains("1 conditional format(s)"))
        }),
        "the import says what it left out: {warnings:?}"
    );
}

/// The **shared formulas** of a sheet are imported: a cell that shares
/// a formula with another cell gets the formula, moved to where it is.
#[test]
fn the_shared_formulas_of_a_sheet_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="B1"><f t="shared" ref="B1:B3" si="0">A1*2</f><v>2</v></c></row><row r="2"><c r="B2"><f t="shared" si="0"/><v>4</v></c></row><row r="3"><c r="B3"><f t="shared" si="0"/><v>6</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["formulas"].as_u64(), Some(3), "three formulas were imported");
    assert_eq!(outcome.data["unread_formulas"].as_u64(), Some(0), "and none was left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let read =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "B3" })).expect("the cell is read");
    assert_eq!(
        read.data["formula"].as_str(),
        Some("A3 * 2"),
        "the master's formula, moved to where the cell is"
    );
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

/// A file that is a **compound file** — a workbook that is encrypted, or
/// a workbook of the old format — is refused, and the refusal says
/// what the file is.
#[test]
fn a_compound_file_is_refused_and_says_what_it_is() {
    let mut session = session();
    let mut bytes = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    bytes.extend_from_slice(&[0u8; 64]);
    for unit in "EncryptedPackage".encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let path = a_file(bytes);
    let error =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect_err("that is not a package");
    assert!(error.to_string().contains("encrypted"), "the refusal says what it is: {error}");
}

/// A file that is **not encrypted**, with a password, is refused, and
/// the refusal says why.
#[test]
fn a_file_that_is_not_encrypted_is_refused_when_a_password_is_given() {
    let mut session = session();
    let path = a_file(a_workbook_file(r#"<worksheet><sheetData/></worksheet>"#, r#"<sst/>"#));
    let error = session
        .execute("grid.import.xlsx", &json!({ "path": path, "password": "the password" }))
        .expect_err("that file is not encrypted");
    assert!(error.to_string().contains("not encrypted"), "the refusal says why: {error}");
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

/// The **properties** of the file are the properties of the workbook:
/// what the file says about itself.
#[test]
fn the_properties_of_the_file_are_read() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "docProps/core.xml",
            r#"<cp:coreProperties xmlns:cp="http://schemas.openxmlformats.org/package/2006/metadata/core-properties" xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>The sales of the year</dc:title><dc:subject>Sales</dc:subject><dc:creator>A. Writer</dc:creator><cp:keywords>sales year</cp:keywords><dc:description>The numbers</dc:description><cp:lastModifiedBy>B. Editor</cp:lastModifiedBy></cp:coreProperties>"#,
        ),
        (
            "docProps/app.xml",
            r#"<Properties xmlns="http://schemas.openxmlformats.org/officeDocument/2006/extended-properties"><Company>The Company Ltd</Company></Properties>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let answer =
        session.execute("grid.workbook.get", &json!({ "workbook": workbook })).expect("the workbook is read");
    let properties = &answer.data["properties"];
    assert_eq!(properties["title"], json!("The sales of the year"), "the title");
    assert_eq!(properties["subject"], json!("Sales"), "the subject");
    assert_eq!(properties["author"], json!("A. Writer"), "the author");
    assert_eq!(properties["keywords"], json!("sales year"), "the keywords");
    assert_eq!(properties["description"], json!("The numbers"), "the description");
    assert_eq!(properties["last_modified_by"], json!("B. Editor"), "the last one to change it");
    assert_eq!(properties["company"], json!("The Company Ltd"), "the company");
}

/// A file that says **nothing about itself** has no properties, and
/// the answer says so: a property that is not said is nothing.
#[test]
fn a_file_with_no_properties_has_none() {
    let mut session = session();
    let path = a_file(a_workbook_file(r#"<worksheet><sheetData/></worksheet>"#, r#"<sst/>"#));
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let answer =
        session.execute("grid.workbook.get", &json!({ "workbook": workbook })).expect("the workbook is read");
    let properties = &answer.data["properties"];
    assert_eq!(properties["title"], Json::Null, "the title is nothing");
    assert_eq!(properties["company"], Json::Null, "and so is the company");
}

/// The **theme** of the file is the theme of the workbook, and the
/// colours of the cells that name it are the colours of the theme.
#[test]
fn the_theme_of_the_file_is_read() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/styles.xml",
            r#"<styleSheet><fills><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="solid"><fgColor theme="4"/></patternFill></fill></fills><cellXfs><xf numFmtId="0" fillId="0"/><xf numFmtId="0" fillId="1"/></cellXfs></styleSheet>"#,
        ),
        (
            "xl/theme/theme1.xml",
            r#"<theme name="Office"><themeElements><clrScheme><dk1><srgbClr val="000000"/></dk1><lt1><srgbClr val="FFFFFF"/></lt1><dk2><srgbClr val="44546A"/></dk2><lt2><srgbClr val="E7E6E6"/></lt2><accent1><srgbClr val="4472C4"/></accent1><accent2><srgbClr val="ED7D31"/></accent2><accent3><srgbClr val="A5A5A5"/></accent3><accent4><srgbClr val="FFC000"/></accent4><accent5><srgbClr val="5B9BD5"/></accent5><accent6><srgbClr val="70AD47"/></accent6><hlink><srgbClr val="0563C1"/></hlink><folHlink><srgbClr val="954F72"/></folHlink></clrScheme><fontScheme><majorFont><latin typeface="Calibri Light"/></majorFont><minorFont><latin typeface="Calibri"/></minorFont></fontScheme></themeElements></theme>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let answer =
        session.execute("grid.workbook.theme", &json!({ "workbook": workbook })).expect("the theme is read");
    let theme = &answer.data["theme"];
    assert_eq!(theme["name"], json!("Office"), "the name of the theme");
    assert_eq!(theme["colours"]["accent1"], json!("#4472c4"), "and the colour of the first accent");
    assert_eq!(theme["minor_font"], json!("Calibri"), "and the minor font");
    // The **cell** that named the colour of the theme is coloured with
    // it: the colour of a cell is not the name of a colour.
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let cell =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(cell.data["fill"], json!("#4472c4"), "the cell is filled with the theme's colour");
}

/// A **checkbox** of a file is imported: the cell is a checkbox in the
/// tree, and it is ticked as one.
#[test]
fn the_checkbox_of_a_cell_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles" Target="styles.xml"/><Relationship Id="rId3" Type="http://schemas.microsoft.com/office/2022/11/relationships/FeaturePropertyBag" Target="featurePropertyBag/featurePropertyBag.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/styles.xml",
            r#"<styleSheet><cellXfs><xf numFmtId="0"/><xf numFmtId="0"><extLst><ext uri="{C7286773-470A-42A8-94C5-96B5CB345126}" xmlns:xfpb="http://schemas.microsoft.com/office/spreadsheetml/2022/featurepropertybag"><xfpb:xfComplement i="0"/></ext></extLst></xf></cellXfs></styleSheet>"#,
        ),
        (
            "xl/featurePropertyBag/featurePropertyBag.xml",
            r#"<FeaturePropertyBags xmlns="http://schemas.microsoft.com/office/spreadsheetml/2022/featurepropertybag" count="4"><bag type="Checkbox"/><bag type="XFControls"><bagId k="CellControl">0</bagId></bag><bag type="XFComplement"><bagId k="XFControls">1</bagId></bag><bag type="XFComplements"><a k="MappedFeaturePropertyBags"><bagId>2</bagId></a></bag></FeaturePropertyBags>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1" t="b"><v>1</v></c><c r="B1" t="b"><v>0</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["checkboxes"].as_u64(), Some(1), "one checkbox was imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let cell =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(cell.data["format"], json!("checkbox"), "the cell is a checkbox");
    assert_eq!(cell.data["value"], json!(true), "and it holds what the file holds");
    // The **box** is ticked: a checkbox of a file is a checkbox here,
    // and it ticks as one.
    let ticked = session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "A1" }))
        .expect("the checkbox is toggled");
    assert_eq!(ticked.data["value"], json!(false), "the box is unticked");
    // A cell that is **not** a checkbox is left as it is.
    let other = session
        .execute("grid.cell.checkbox.toggle", &json!({ "sheet": sheet, "cell": "B1" }))
        .expect("the cell is asked");
    assert_eq!(other.data["toggled"], json!(false), "the cell is not a checkbox");
}

/// The **calculation settings** of a file are imported: a workbook that
/// calculates by hand says so here too.
#[test]
fn the_calculation_settings_of_a_file_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><calcPr calcMode="manual" iterate="1" iterateCount="50" iterateDelta="0.01"/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let answer = session
        .execute("grid.workbook.calculation", &json!({ "workbook": workbook }))
        .expect("the settings are read");
    let calculation = &answer.data["calculation"];
    assert_eq!(calculation["mode"], json!("manual"), "it calculates by hand");
    assert_eq!(calculation["iterative"], json!(true), "and it iterates the circular references");
    assert_eq!(calculation["max_iterations"], json!(50), "fifty times at most");
    assert_eq!(calculation["max_change"], json!(0.01), "and it stops at a change of a hundredth");
    assert_eq!(calculation["precision_as_displayed"], json!(false), "and it calculates with the number");
}

/// A workbook of the tree's own calculates by itself.
#[test]
fn a_workbook_of_the_tree_calculates_by_itself() {
    let mut session = session();
    let workbook =
        session.execute("grid.workbook.add", &json!({ "name": "One" })).expect("the workbook is added");
    let workbook = workbook.data["id"].as_u64().expect("the workbook has an id");
    let answer = session
        .execute("grid.workbook.calculation", &json!({ "workbook": workbook }))
        .expect("the settings are read");
    let calculation = &answer.data["calculation"];
    assert_eq!(calculation["mode"], json!("automatic"), "it calculates by itself");
    assert_eq!(calculation["iterative"], json!(false), "and it does not iterate");
    assert_eq!(calculation["max_iterations"], json!(100), "a hundred times at most");
}

/// The **reading** of a text — the phonetic guide a file holds beside
/// it — is imported with the text.
#[test]
fn the_reading_of_a_text_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
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
        (
            "xl/sharedStrings.xml",
            r#"<sst count="1" uniqueCount="1"><si><t>東京都</t><rPh sb="0" eb="2"><t>トウキョウ</t></rPh><phoneticPr fontId="1" type="noConversion"/></si></sst>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let cell =
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": "A1" })).expect("the cell is read");
    assert_eq!(cell.data["value"], json!("東京都"), "the text is the text");
    let phonetic = &cell.data["phonetic"];
    assert_eq!(phonetic["runs"].as_array().map(|runs| runs.len()), Some(1), "one run of the reading");
    assert_eq!(phonetic["runs"][0]["text"], json!("トウキョウ"), "and it holds the reading");
    assert_eq!(phonetic["props"]["kind"], json!("noConversion"), "and the reading has a kind");
}

/// Whether the **objects** of a file are drawn is imported with the
/// workbook.
#[test]
fn the_objects_of_a_file_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><workbookPr showObjects="placeholders"/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let answer = session
        .execute("grid.workbook.objects", &json!({ "workbook": workbook }))
        .expect("the objects are read");
    assert_eq!(answer.data["objects"], json!("placeholders"), "the objects are placeholders");
}

/// The **kinds of rule** a file holds are imported: the words of a cell,
/// the empty cells, the errors and the formula — and the kinds this tree
/// does not have are counted, and the reader says so.
#[test]
fn the_kinds_of_conditional_format_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
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
            r##"<styleSheet><dxfs count="1"><dxf><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill></dxf></dxfs></styleSheet>"##,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>ann</t></is></c></row></sheetData><conditionalFormatting sqref="A1:A5"><cfRule type="containsText" text="ann" priority="1" dxfId="0"><formula>NOT(ISERROR(SEARCH("ann",A1)))</formula></cfRule></conditionalFormatting><conditionalFormatting sqref="B1:B5"><cfRule type="containsBlanks" priority="2"/></conditionalFormatting><conditionalFormatting sqref="C1:C5"><cfRule type="expression" priority="3"><formula>C1&gt;0</formula></cfRule></conditionalFormatting><conditionalFormatting sqref="D1:D5"><cfRule type="timePeriod" priority="4" timePeriod="today"/></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["conditionals"].as_u64(), Some(3), "the three rules this tree has are imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let rules = &node.data["conditionals"];
    assert_eq!(rules[0]["op"].as_str(), Some("contains"), "the first looks for the words");
    assert_eq!(rules[0]["value"].as_str(), Some("ann"), "and the words are ann");
    assert_eq!(rules[1]["op"].as_str(), Some("isEmpty"), "the second is the empty cells");
    assert_eq!(rules[1]["value"].as_str(), Some(""), "and it has no value");
    assert_eq!(rules[2]["formula"].as_str(), Some("C1>0"), "and the third is a formula");
    assert!(rules[2]["op"].is_null(), "and it is not a criterion");
    // The rule **judges**: the cell that holds the words is a cell the
    // rule holds for.
    let checked = session
        .execute("grid.conditional.check", &json!({ "sheet": sheet, "range": "A1:A5" }))
        .expect("the cells are read");
    let cells = checked.data["cells"].as_array().map(|cells| cells.len()).unwrap_or(0);
    assert_eq!(cells, 1, "the cell that holds the words matches");
    // The kind this tree does not have is **counted**, and the reader
    // says so.
    let said = outcome.data["warnings"].as_array().map(|warnings| {
        warnings.iter().filter_map(|warning| warning.as_str()).collect::<Vec<&str>>().join(" ")
    });
    assert!(
        said.as_deref()
            .is_some_and(|said| said.contains("1 conditional format(s) of a kind this tree does not have")),
        "it says how many: {said:?}"
    );
}

/// A workbook's **colour scale** is imported: the colours of the scale
/// are in the document, and the cells of the range are filled by it.
#[test]
fn a_colour_scale_of_a_workbook_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/styles.xml", r##"<styleSheet/>"##),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>0</v></c><c r="A2"><v>10</v></c></row></sheetData><conditionalFormatting sqref="A1:A2"><cfRule type="colorScale" priority="1"><colorScale><cfvo type="min"/><cfvo type="percentile" val="50"/><cfvo type="max"/><color rgb="FFF8696B"/><color rgb="FFFFEB84"/><color rgb="FF63BE7B"/></colorScale></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["conditionals"].as_u64(), Some(1), "the scale is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let colours = node.data["conditionals"][0]["colour_scale"]["colours"]
        .as_array()
        .map(|colours| {
            colours
                .iter()
                .map(|colour| colour.as_str().unwrap_or_default().to_string())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    assert_eq!(
        colours,
        vec!["#f8696b".to_string(), "#ffeb84".to_string(), "#63be7b".to_string()],
        "the colours of the scale are in the document"
    );
}

/// The **kinds of rule** that are about a range as a whole are
/// imported: the top ten, the above-the-average, the duplicates and the
/// data bar.
#[test]
fn the_kinds_of_rule_about_a_range_are_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/styles.xml", r##"<styleSheet/>"##),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="top10" priority="1" rank="3"><cfvo type="num" val="3"/></cfRule></conditionalFormatting><conditionalFormatting sqref="B1:B5"><cfRule type="aboveAverage" priority="2"/></conditionalFormatting><conditionalFormatting sqref="C1:C5"><cfRule type="duplicateValues" priority="3"/></conditionalFormatting><conditionalFormatting sqref="D1:D5"><cfRule type="dataBar" priority="4"><dataBar><cfvo type="min"/><cfvo type="max"/><color rgb="FF638EC6"/></dataBar></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["conditionals"].as_u64(), Some(4), "the four rules are imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let rules = &node.data["conditionals"];
    assert_eq!(rules[0]["top"]["count"].as_u64(), Some(3), "the top ten is in the document");
    assert_eq!(rules[1]["average"]["above"].as_bool(), Some(true), "and the average");
    assert_eq!(rules[2]["duplicate"]["unique"].as_bool(), Some(false), "and the duplicates");
    assert_eq!(rules[3]["data_bar"]["colour"].as_str(), Some("#638ec6"), "and the bar");
}

/// A workbook's **icon set** is imported: the two percents are in the
/// document, and the cells of the range carry an icon.
#[test]
fn an_icon_set_of_a_workbook_is_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/styles.xml", r##"<styleSheet/>"##),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c><c r="A2"><v>5</v></c><c r="A3"><v>9</v></c></row></sheetData><conditionalFormatting sqref="A1:A3"><cfRule type="iconSet" priority="1"><iconSet iconSet="3TrafficLights1"><cfvo type="percent" val="0"/><cfvo type="percent" val="33"/><cfvo type="percent" val="67"/></iconSet></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["conditionals"].as_u64(), Some(1), "the icon set is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let rules = &node.data["conditionals"];
    assert_eq!(rules[0]["icon_set"]["high"].as_f64(), Some(67.0), "the high percent is in the document");
    assert_eq!(rules[0]["icon_set"]["low"].as_f64(), Some(33.0), "and the low one");
    // The **icons**: the cell that holds the highest number carries the
    // high icon, and the lowest carries the low one.
    let read = |session: &mut Session, cell: &str| {
        session.execute("grid.cell.get", &json!({ "sheet": sheet, "cell": cell })).expect("the cell is read")
    };
    assert_eq!(read(&mut session, "A3").data["icon"].as_u64(), Some(2), "the highest has the high icon");
    assert_eq!(read(&mut session, "A1").data["icon"].as_u64(), Some(0), "and the lowest the low one");
}

/// A workbook's **pictures** are imported: the cell they are drawn in,
/// their size, and their bytes in the document.
#[test]
fn the_pictures_of_a_workbook_are_imported() {
    let mut session = session();
    // A **picture**: the head of a PNG file that is two pixels wide and
    // three high.
    let picture: &[u8] =
        &[137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R', 0, 0, 0, 2, 0, 0, 0, 3];
    let bytes = a_package_of(&[
        (
            "_rels/.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            br#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        ("xl/worksheets/sheet1.xml", br#"<worksheet><sheetData/><drawing r:id="rId2"/></worksheet>"#),
        (
            "xl/drawings/drawing1.xml",
            br#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:from><xdr:col>1</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>2</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:pic><xdr:nvPicPr><xdr:cNvPr id="2" name="Picture 1" descr="the logo"/></xdr:nvPicPr><xdr:blipFill><a:blip r:embed="rId1"/></xdr:blipFill></xdr:pic><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/></Relationships>"#,
        ),
        ("xl/media/image1.png", picture),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["pictures"].as_u64(), Some(1), "the picture is imported");
    assert_eq!(outcome.data["unread_pictures"].as_u64(), Some(0), "and nothing is left out");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    let node = session.document().node(sheet).expect("the sheet is there");
    let pictures = &node.data["pictures"];
    let entry = pictures.get("B3").expect("the picture is in the cell it is drawn in");
    assert_eq!(entry["width"].as_u64(), Some(2), "the width of the picture is read");
    assert_eq!(entry["height"].as_u64(), Some(3), "and the height of it");
    assert_eq!(entry["format"].as_str(), Some("png"), "and the format of it");
    assert_eq!(entry["alt"].as_str(), Some("the logo"), "and its alt text");
    // The **picture read**: the bytes are in the document, and the
    // answer says so.
    let read = session
        .execute("grid.picture.read", &json!({ "sheet": sheet, "cell": "B3" }))
        .expect("the picture is read");
    assert_eq!(read.data["source"].as_str(), Some("the document"), "the bytes are in the document");
    assert_eq!(read.data["bytes"].as_u64(), Some(24), "and how many they are");
    assert_eq!(read.data["pixels"].as_u64(), Some(6), "and the pixels of the picture");
}

/// A picture that came from a workbook is **drawn**: the bytes are in
/// the document, and the page of the sheet carries the image.
#[test]
fn an_imported_picture_is_drawn_on_the_page() {
    let mut session = session();
    // A **picture**: a PNG of two pixels across and three down, written
    // by the tree's own writer.
    let mut image = craftkit_raster::Image::new(2, 3, craftkit_sdk::document::Colour::WHITE);
    image.blend(0, 0, craftkit_sdk::document::Colour::BLACK, craftkit_sdk::BlendMode::Normal);
    let picture = image.to_png();
    let bytes = a_package_of(&[
        (
            "_rels/.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            br#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            br#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData><drawing r:id="rId2"/></worksheet>"#,
        ),
        (
            "xl/drawings/drawing1.xml",
            br#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:from><xdr:col>0</xdr:col><xdr:colOff>0</xdr:colOff><xdr:row>0</xdr:row><xdr:rowOff>0</xdr:rowOff></xdr:from><xdr:pic><xdr:nvPicPr><xdr:cNvPr id="2" name="Picture 1"/></xdr:nvPicPr><xdr:blipFill><a:blip r:embed="rId1"/></xdr:blipFill></xdr:pic><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/></Relationships>"#,
        ),
        ("xl/media/image1.png", &picture),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    // The **page**: the sheet is written as a PDF file, and the picture
    // is on it.
    let out = craftkit_scratch::file("craftkit-v2-grid-import.pdf").expect("the scratch gives a path");
    session
        .execute("grid.export.pdf", &json!({ "sheet": sheet, "path": out.to_string_lossy() }))
        .expect("the sheet is exported");
    let page = std::fs::read(&out).expect("the file is there");
    assert!(
        page.windows(b"/Subtype /Image".len()).any(|window| window == b"/Subtype /Image"),
        "the picture is in the file"
    );
}

/// A workbook's **formula** may name a **column of a table** of the
/// workbook, and the import keeps the two together.
#[test]
fn a_formula_of_a_workbook_may_name_a_column_of_a_table() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/table" Target="../tables/table1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="str"><v>who</v></c><c r="B1" t="str"><v>amount</v></c></row><row r="2"><c r="A2" t="str"><v>ann</v></c><c r="B2"><v>3</v></c></row><row r="3"><c r="A3" t="str"><v>bob</v></c><c r="B3"><v>4</v></c></row><row r="1"><c r="D1"><f>SUM(Orders[amount])</f></c></row></sheetData><tableParts count="1"><tablePart r:id="rId1"/></tableParts></worksheet>"#,
        ),
        (
            "xl/tables/table1.xml",
            r#"<table id="1" name="Orders" displayName="Orders" ref="A1:B3"><tableColumns count="2"><tableColumn id="1" name="who"/><tableColumn id="2" name="amount"/></tableColumns></table>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["formulas"].as_u64(), Some(1), "one formula");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    assert_eq!(
        formula_of(&mut session, sheet, "D1"),
        Some("SUM(Orders[amount])".to_string()),
        "the formula is imported"
    );
    assert_eq!(value_of(&mut session, sheet, "D1"), Some(json!(7.0)), "and it reads the column");
}

/// A workbook's formula may spell the **row of the cell** the way a
/// file spells it, and the import reads it.
#[test]
fn a_formula_of_a_workbook_may_name_the_row_of_the_cell() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/table" Target="../tables/table1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="str"><v>who</v></c><c r="B1" t="str"><v>amount</v></c></row><row r="2"><c r="A2" t="str"><v>ann</v></c><c r="B2"><v>3</v></c></row><row r="3"><c r="A3" t="str"><v>bob</v></c><c r="B3"><v>4</v></c></row><row r="2"><c r="C2"><f>Orders[[#This Row],[amount]]</f></c></row></sheetData><tableParts count="1"><tablePart r:id="rId1"/></tableParts></worksheet>"#,
        ),
        (
            "xl/tables/table1.xml",
            r#"<table id="1" name="Orders" displayName="Orders" ref="A1:B3"><tableColumns count="2"><tableColumn id="1" name="who"/><tableColumn id="2" name="amount"/></tableColumns></table>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["formulas"].as_u64(), Some(1), "one formula");
    assert_eq!(outcome.data["unread_formulas"].as_u64(), Some(0), "and it was read");
    let workbook = outcome.data["id"].as_u64().expect("the workbook has an id");
    let sheet = session.document().node(workbook).expect("the workbook is there").children[0].id;
    assert_eq!(
        formula_of(&mut session, sheet, "C2"),
        Some("Orders[[#This Row],[amount]]".to_string()),
        "the formula is imported"
    );
    assert_eq!(value_of(&mut session, sheet, "C2"), Some(json!(3.0)), "and it reads the row of the cell");
}

/// A workbook's table that has **no header row** is not a table of the
/// tree, and the import says that it did not read it.
#[test]
fn a_table_with_no_header_row_is_not_imported() {
    let mut session = session();
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/table" Target="../tables/table1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><tableParts count="1"><tablePart r:id="rId1"/></tableParts></worksheet>"#,
        ),
        (
            "xl/tables/table1.xml",
            r#"<table id="1" name="Table1" displayName="Table1" ref="A1:C5" headerRowCount="0"><tableColumns count="3"><tableColumn id="1" name="who"/><tableColumn id="2" name="amount"/><tableColumn id="3" name="when"/></tableColumns></table>"#,
        ),
    ]);
    let path = a_file(bytes);
    let outcome =
        session.execute("grid.import.xlsx", &json!({ "path": path })).expect("the workbook is imported");
    assert_eq!(outcome.data["tables"].as_u64(), Some(0), "no table was imported");
    let warnings = outcome.data["warnings"].as_array().expect("the warnings are a list");
    assert!(
        warnings
            .iter()
            .any(|warning| { warning.as_str().is_some_and(|warning| warning.contains("has no header row")) }),
        "and the import says why: {warnings:?}"
    );
}
