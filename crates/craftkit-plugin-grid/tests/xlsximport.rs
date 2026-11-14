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
