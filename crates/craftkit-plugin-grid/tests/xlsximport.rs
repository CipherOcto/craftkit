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
            r##"<styleSheet><dxfs count="1"><dxf><numFmt numFmtId="0" formatCode="#,##0.00"/><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill></dxf></dxfs></styleSheet>"##,
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
