//! The **workbook reader**: a workbook file, read as a workbook.

use craftkit_xlsx::{DateSystem, Value, read};

/// A **package** of parts, written by hand.
///
/// The tests of a reader are the files that were written by somebody
/// else — and here that somebody is this file: the parts are laid out
/// as the format says.
fn a_package(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    let mut directory: Vec<u8> = Vec::new();
    for (name, text) in parts {
        let name = name.as_bytes();
        let data = text.as_bytes();
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

/// A package with a workbook, a sheet and the strings.
fn a_workbook(sheet: &str, strings: &str) -> Vec<u8> {
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
            r#"<workbook><workbookPr/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        ("xl/sharedStrings.xml", strings),
        ("xl/worksheets/sheet1.xml", sheet),
    ])
}

/// The workbook of a package is read.
#[test]
fn a_workbook_is_read() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row></sheetData></worksheet>"#,
        r#"<sst><si><t>the text</t></si></sst>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.sheets.len(), 1, "one sheet");
    assert_eq!(workbook.sheets[0].name, "One", "its name");
    assert_eq!(workbook.sheets[0].cells.len(), 1, "one cell");
    assert_eq!(workbook.sheets[0].cells[0].value, Value::Text("the text".to_string()), "its value");
    assert_eq!(workbook.sheets[0].cells[0].column, 0, "the column");
    assert_eq!(workbook.sheets[0].cells[0].row, 0, "the row");
}

/// A **number** is read, and so is a **boolean** and an **error**.
#[test]
fn the_kinds_of_a_value_are_read() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1"><c r="A1"><v>42</v></c><c r="B1" t="b"><v>1</v></c><c r="C1" t="e"><v>#DIV/0!</v></c><c r="D1" t="str"><v>the text</v></c></row></sheetData></worksheet>"#,
        r#"<sst/>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    assert_eq!(cells[0].value, Value::Number(42.0), "a number");
    assert_eq!(cells[1].value, Value::Bool(true), "a boolean");
    assert_eq!(cells[2].value, Value::Error("#DIV/0!".to_string()), "an error");
    assert_eq!(cells[3].value, Value::Text("the text".to_string()), "a text");
}

/// An **inline** text is read.
#[test]
fn an_inline_text_is_read() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1"><c r="A1" t="inlineStr"><is><t>the text</t></is></c></row></sheetData></worksheet>"#,
        r#"<sst/>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(
        workbook.sheets[0].cells[0].value,
        Value::Text("the text".to_string()),
        "the text of the cell"
    );
}

/// A **rich** text is read: its own text and its runs, in order.
#[test]
fn a_rich_text_is_read() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row></sheetData></worksheet>"#,
        r#"<sst><si><t>one </t><r><t>two</t></r><r><t> three</t></r></si></sst>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(
        workbook.sheets[0].cells[0].value,
        Value::Text("one two three".to_string()),
        "the text of the string"
    );
}

/// A **formula** is read beside the value the file cached for it.
#[test]
fn a_formula_is_read() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1"><c r="A1"><f>SUM(B1:B2)</f><v>3</v></c></row></sheetData></worksheet>"#,
        r#"<sst/>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    let cell = &workbook.sheets[0].cells[0];
    assert_eq!(cell.formula, Some("SUM(B1:B2)".to_string()), "the formula");
    assert_eq!(cell.value, Value::Number(3.0), "and the value");
}

/// The **date system** of the workbook is read.
#[test]
fn the_date_system_is_read() {
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        ("xl/workbook.xml", r#"<workbook><workbookPr date1904="1"/><sheets/></workbook>"#),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.system, DateSystem::Y1904, "the 1904 system");
}

/// A cell's **format** is read: the style of the cell says which
/// number format it is written with.
#[test]
fn the_format_of_a_cell_is_read() {
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
            r#"<styleSheet><numFmts><numFmt numFmtId="164" formatCode="yyyy-mm-dd"/></numFmts><cellXfs><xf numFmtId="0"/><xf numFmtId="14"/><xf numFmtId="164"/></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1"><v>45000</v></c><c r="B1" s="2"><v>45000</v></c><c r="C1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    assert_eq!(cells[0].format, Some("m/d/yyyy".to_string()), "a built-in format");
    assert_eq!(cells[1].format, Some("yyyy-mm-dd".to_string()), "a custom format");
    assert_eq!(cells[2].format, None, "and a cell with no style has no format");
}

/// The **names** of a workbook are read: the words its formulas use.
#[test]
fn the_names_of_a_workbook_are_read() {
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/><sheet name="Two" sheetId="2" r:id="rId2"/></sheets><definedNames><definedName name="Sales">One!$A$1:$B$4</definedName><definedName name="Local" localSheetId="1">$A$1</definedName><definedName name="_xlnm.Print_Area" localSheetId="0">One!$A$1:$B$2</definedName></definedNames></workbook>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.names.len(), 2, "two names: the built-in one is not read");
    assert_eq!(workbook.names[0].name, "Sales", "the first name");
    assert_eq!(workbook.names[0].refers_to, "One!$A$1:$B$4", "and what it stands for");
    assert_eq!(workbook.names[0].scope, None, "and it belongs to the workbook");
    assert_eq!(workbook.names[1].name, "Local", "the second name");
    assert_eq!(workbook.names[1].scope, Some("Two".to_string()), "and it belongs to a sheet");
}

/// The **filter** of a sheet is read: the range it is over, and the
/// criterion of each of its columns.
#[test]
fn the_filter_of_a_sheet_is_read() {
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
            r#"<worksheet><autoFilter ref="B1:D10"><filterColumn colId="0"><filters blank="1"><filter val="ann"/></filters></filterColumn><filterColumn colId="1"><customFilters and="1"><customFilter operator="greaterThan" val="3"/></customFilters></filterColumn></autoFilter><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.filters.len(), 1, "one sheet has a filter");
    let (_, filter) = &workbook.filters[0];
    assert_eq!(filter.range, Some("B1:D10".to_string()), "the range of the filter");
    assert_eq!(filter.criteria.len(), 2, "two columns have criteria");
    assert_eq!(filter.criteria[0].0, 0, "the first column is the first of the range");
    assert_eq!(
        filter.criteria[0].1,
        craftkit_xlsx::FilterCriterion::Values { values: vec!["ann".to_string()], blanks: true },
        "and it keeps a list of values"
    );
    assert_eq!(
        filter.criteria[1].1,
        craftkit_xlsx::FilterCriterion::Custom {
            first: (">".to_string(), "3".to_string()),
            second: None,
            and: true
        },
        "and the second compares"
    );
}

/// The **links** of a sheet are read: the places its cells go to.
#[test]
fn the_links_of_a_sheet_are_read() {
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
            r#"<worksheet><hyperlinks><hyperlink ref="A1:B2" r:id="rId1" tooltip="the page"/><hyperlink ref="C1" location="One!A1"/></hyperlinks><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.links.len(), 1, "one sheet has links");
    let (_, links) = &workbook.links[0];
    assert_eq!(links.len(), 2, "two links");
    assert_eq!(links[0].target, "https://example.invalid/page", "the first goes outside");
    assert_eq!(links[0].tooltip, Some("the page".to_string()), "and it has a tooltip");
    assert_eq!(links[0].column, 0, "it is on the first cell of its range");
    assert_eq!(links[0].last_column, 1, "and it covers the second column");
    assert_eq!(links[1].target, "One!A1", "the second goes inside the workbook");
    assert!(!links[1].external, "and it is not outside");
}

/// A package with **no workbook** is refused.
#[test]
fn a_package_with_no_workbook_is_refused() {
    let bytes = a_package(&[("hello.txt", "not a workbook")]);
    let error = read(&bytes).expect_err("that is not a workbook");
    assert!(matches!(error, craftkit_xlsx::Error::Malformed(_)), "{error}");
}

/// A sheet whose part is **not there** is empty, and says so.
#[test]
fn a_sheet_that_is_not_there_says_so() {
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets.is_empty(), "the sheet has no part, and so it is not read");
    assert!(!workbook.warnings.is_empty(), "and the reader says so");
}

/// A cell that names a **shared string that is not there** is empty.
#[test]
fn a_cell_with_a_missing_string_is_empty() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>9</v></c></row></sheetData></worksheet>"#,
        r#"<sst><si><t>the text</t></si></sst>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.sheets[0].cells[0].value, Value::Empty, "the cell is empty");
}

/// A cell that is **outside the grid** is dropped, and the reader says
/// so.
#[test]
fn a_cell_outside_the_grid_is_dropped() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c><c r="XFE1"><v>2</v></c></row></sheetData></worksheet>"#,
        r#"<sst/>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.sheets[0].cells.len(), 1, "one cell is kept");
    assert!(!workbook.warnings.is_empty(), "and the reader says what it dropped");
}

/// A cell whose address is **not an address** is dropped.
#[test]
fn a_cell_with_a_bad_address_is_dropped() {
    let bytes = a_workbook(
        r#"<worksheet><sheetData><row r="1"><c r="nonesuch"><v>1</v></c></row></sheetData></worksheet>"#,
        r#"<sst/>"#,
    );
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets[0].cells.is_empty(), "the cell is dropped");
}
