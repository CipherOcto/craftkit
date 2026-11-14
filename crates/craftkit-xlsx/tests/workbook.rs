//! The **workbook reader**: a workbook file, read as a workbook.

use craftkit_xlsx::{DateSystem, FormulaKind, Value, read};

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

/// The **font** of a cell is read: the face of its text, the size of
/// it and the marks on it.
#[test]
fn the_font_of_a_cell_is_read() {
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
            r#"<styleSheet><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><i/><strike/><u val="singleAccounting"/><vertAlign val="superscript"/><sz val="14"/><color rgb="FFFF0000"/><name val="Arial"/></font></fonts><cellXfs><xf fontId="0"/><xf fontId="1"/></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1" t="str"><v>x</v></c><c r="B1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    let font = cells[0].font.as_ref().expect("the cell has a font");
    assert_eq!(font.name, Some("Arial".to_string()), "the face of the text");
    assert_eq!(font.size, Some(14.0), "the size of it");
    assert!(font.bold, "and it is bold");
    assert!(font.italic, "and italic");
    assert!(font.strike, "and struck through");
    assert_eq!(font.underline, craftkit_xlsx::Underline::SingleAccounting, "and underlined");
    assert_eq!(font.vert, craftkit_xlsx::Vert::Superscript, "and above its line");
    assert_eq!(font.colour, Some("#ff0000".to_string()), "and red");
    assert!(cells[1].font.is_none(), "and a cell with no style has no font");
}

/// The **fill** of a cell is read: the colour behind its text. Only a
/// **solid** fill is a colour.
#[test]
fn the_fill_of_a_cell_is_read() {
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
            r#"<styleSheet><fills count="3"><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="solid"><fgColor rgb="FF00FF00"/></patternFill></fill><fill><patternFill patternType="gray125"/></fill></fills><cellXfs><xf fillId="0"/><xf fillId="1"/><xf fillId="2"/></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1" t="str"><v>x</v></c><c r="B1" s="2" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    assert_eq!(
        cells[0].fill.as_ref().expect("the cell has a fill").colour,
        Some("#00ff00".to_string()),
        "the colour behind the text"
    );
    assert!(
        cells[1].fill.as_ref().expect("the cell has a fill").colour.is_none(),
        "and a pattern is not a colour"
    );
}

/// The **alignment** of a cell is read: how its text lies across it
/// and how it lies down it.
#[test]
fn the_alignment_of_a_cell_is_read() {
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
            r#"<styleSheet><cellXfs><xf><alignment horizontal="centerContinuous" vertical="center" wrapText="1" indent="2"/></xf></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="0" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let align = workbook.sheets[0].cells[0].align.as_ref().expect("the cell has an alignment");
    assert_eq!(align.horizontal, Some("center-across".to_string()), "how the text lies across the cell");
    assert_eq!(align.vertical, Some("middle".to_string()), "and how it lies down it");
    assert!(align.wrap, "and it wraps");
    assert_eq!(align.indent, 2, "and it is indented");
}

/// The **borders** of a cell are read: the lines on its edges.
#[test]
fn the_borders_of_a_cell_are_read() {
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
            r#"<styleSheet><borders count="2"><border><left/><right/><top/><bottom/><diagonal/></border><border><left style="thin"><color rgb="FF0000FF"/></left><bottom style="mediumDashDot"/></border></borders><cellXfs><xf borderId="0"/><xf borderId="1"/></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let borders = workbook.sheets[0].cells[0].borders.as_ref().expect("the cell has borders");
    assert_eq!(borders.edges.len(), 2, "two edges have a line");
    assert_eq!(borders.edges[0].edge, "bottom", "the first is at the bottom");
    assert_eq!(borders.edges[0].style, "mediumDashDot", "and it is a dashed dot");
    assert!(borders.edges[0].colour.is_none(), "and it has no colour of its own");
    assert_eq!(borders.edges[1].edge, "left", "the second is on the left");
    assert_eq!(borders.edges[1].style, "thin", "and it is a thin line");
    assert_eq!(borders.edges[1].colour, Some("#0000ff".to_string()), "and it is blue");
}

/// The **protection** of a sheet is read: what may still be done to
/// it, and the cells of it are locked.
#[test]
fn the_protection_of_a_sheet_is_read() {
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
            r#"<worksheet><sheetProtection sheet="1" sort="0" autoFilter="0" formatCells="0"/><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let protection = workbook.sheets[0].protection.expect("the sheet is protected");
    assert!(protection.sort, "the sorting is still allowed");
    assert!(protection.filter, "and the filtering");
    assert!(protection.format, "and the formatting");
}

/// A sheet whose protection does not say it is protected is **not**
/// protected.
#[test]
fn a_sheet_that_is_not_protected_is_not_protected() {
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
            r#"<worksheet><sheetProtection sheet="0" sort="0"/><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets[0].protection.is_none(), "the sheet is not protected");
}

/// The **lock** of a cell is read: whether it is locked and whether
/// its text is hidden.
#[test]
fn the_lock_of_a_cell_is_read() {
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
            r#"<styleSheet><cellXfs><xf><protection locked="0"/></xf><xf><protection locked="1" hidden="1"/></xf></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="0" t="str"><v>x</v></c><c r="B1" s="1" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    let first = cells[0].protection.expect("the cell has a protection");
    assert!(!first.locked, "the first cell is unlocked");
    assert!(!first.hidden, "and its text is not hidden");
    let second = cells[1].protection.expect("the cell has a protection");
    assert!(second.locked, "the second cell is locked");
    assert!(second.hidden, "and its text is hidden");
}

/// The **rows** and the **columns** of a sheet are read: their heights,
/// their widths and which of them are hidden.
#[test]
fn the_rows_and_the_columns_of_a_sheet_are_read() {
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
            r#"<worksheet><cols><col min="1" max="2" width="12.5" customWidth="1"/><col min="3" max="3" width="8" hidden="1"/></cols><sheetData><row r="1" ht="30" customHeight="1"><c r="A1" t="str"><v>x</v></c></row><row r="2" hidden="1"><c r="A2" t="str"><v>x</v></c></row><row r="3"><c r="A3" t="str"><v>x</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    // The **rows**: the first has a height, the second is hidden, and
    // the third says nothing of itself.
    assert_eq!(sheet.rows.len(), 2, "two rows say something of themselves");
    assert_eq!(sheet.rows[0].row, 1, "the first is the first row");
    assert_eq!(sheet.rows[0].height, Some(30.0), "and it is thirty points high");
    assert!(!sheet.rows[0].hidden, "and it is not hidden");
    assert_eq!(sheet.rows[1].row, 2, "the second is the second row");
    assert_eq!(sheet.rows[1].height, None, "and it has no height of its own");
    assert!(sheet.rows[1].hidden, "and it is hidden");
    // The **columns**: a span of two columns is two columns, and a
    // hidden column is a column of its own.
    assert_eq!(sheet.columns.len(), 3, "three columns say something of themselves");
    assert_eq!(sheet.columns[0].column, 0, "the first is the first column");
    assert_eq!(sheet.columns[0].width, Some(12.5), "and it is twelve and a half wide");
    assert_eq!(sheet.columns[1].column, 1, "the second is the second column");
    assert_eq!(sheet.columns[1].width, Some(12.5), "and it is as wide as the first");
    assert_eq!(sheet.columns[2].column, 2, "the third is the third column");
    assert!(sheet.columns[2].hidden, "and it is hidden");
}

/// The **merges** of a sheet are read: the ranges that are one cell.
#[test]
fn the_merges_of_a_sheet_are_read() {
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
            r#"<worksheet><sheetData/><mergeCells count="3"><mergeCell ref="A1:B2"/><mergeCell ref="$C$3:$D$4"/><mergeCell ref="E5:E5"/></mergeCells></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let merges = &workbook.sheets[0].merges;
    assert_eq!(merges.len(), 2, "two ranges are merges");
    assert_eq!(merges[0], "A1:B2", "the first is the first range");
    assert_eq!(merges[1], "C3:D4", "and the second is the second, without its dollars");
}

/// The **view** of a sheet is read: what is drawn of it, and what is
/// not.
#[test]
fn the_view_of_a_sheet_is_read() {
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
            r#"<worksheet><sheetViews><sheetView showGridLines="0" showRowColHeaders="0" showZeros="0" rightToLeft="1" showOutlineSymbols="0" showFormulas="1" zoomScale="120"><pane xSplit="2" ySplit="3" topLeftCell="C4" state="frozen"/></sheetView></sheetViews><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let view = workbook.sheets[0].view.as_ref().expect("the sheet has a view");
    assert!(!view.gridlines, "the gridlines are not drawn");
    assert!(!view.headings, "nor the headings");
    assert!(!view.zeros, "nor the zeros");
    assert!(view.right_to_left, "and the sheet runs right to left");
    assert!(!view.outline_symbols, "and the outline symbols are not drawn");
    assert!(view.show_formulas, "and the formulas are shown");
    assert_eq!(view.zoom, Some(120), "and the sheet is at a hundred and twenty percent");
    assert_eq!(view.freeze, Some((3, 2)), "and three rows and two columns are frozen");
}

/// A sheet with **no view** of its own has the default one, and the
/// default one is not carried.
#[test]
fn a_sheet_with_no_view_has_the_default_one() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets[0].view.is_none(), "the sheet has no view of its own");
}

/// The **conditional formats** of a sheet are read: the rules that are
/// about a range of cells.
#[test]
fn the_conditional_formats_of_a_sheet_are_read() {
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
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="cellIs" operator="greaterThan" priority="1" dxfId="0" stopIfTrue="1"><formula>10</formula></cfRule></conditionalFormatting><conditionalFormatting sqref="B1:B2 C1:C2"><cfRule type="cellIs" operator="equal" priority="2"><formula>3</formula></cfRule></conditionalFormatting><conditionalFormatting sqref="D1"><cfRule type="colorScale"><colorScale><cfvo type="min"/><cfvo type="max"/><color rgb="FF00FF00"/></colorScale></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let rules = &workbook.sheets[0].conditionals;
    // The first rule: a comparison, with the look of the differential
    // format it names.
    assert_eq!(rules.len(), 3, "three rules are read");
    assert_eq!(rules[0].range, "A1:A5", "the first is about the first range");
    assert_eq!(rules[0].op, ">", "and it is a comparison");
    assert_eq!(rules[0].value, "10", "against ten");
    assert_eq!(rules[0].format.as_deref(), Some("#,##0.00"), "and the format of the look");
    assert_eq!(rules[0].fill.as_deref(), Some("#ffc7ce"), "and the colour of it");
    assert_eq!(rules[0].stop, Some(true), "and it stops the walk");
    assert_eq!(rules[0].priority, Some(1), "and it is tried first");
    // A rule over two ranges is two rules here.
    assert_eq!(rules[1].range, "B1:B2", "the second is about the first of its ranges");
    assert_eq!(rules[1].op, "=", "and it is an equality");
    assert_eq!(rules[2].range, "C1:C2", "and the third is about the second");
    assert_eq!(rules[2].op, "=", "with the same criterion");
}

/// The **shared formulas** of a sheet are read: a group of cells that
/// share one formula, and the distance of each of them from the master.
#[test]
fn the_shared_formulas_of_a_sheet_are_read() {
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
            r#"<worksheet><sheetData><row r="1"><c r="B1"><f t="shared" ref="B1:B3" si="0">A1*2</f><v>2</v></c><c r="C1"><f t="shared" si="1">A1*3</f><v>3</v></c></row><row r="2"><c r="B2"><f t="shared" si="0"/><v>4</v></c></row><row r="3"><c r="B3"><f t="shared" si="0"/><v>6</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    // The master writes the formula.
    assert_eq!(cells[0].formula.as_deref(), Some("A1*2"), "the master writes the formula");
    assert!(cells[0].shared.is_none(), "and it shares nothing");
    // The followers name it.
    let follower = cells[2].shared.as_ref().expect("the cell shares a formula");
    assert_eq!(follower.text, "A1*2", "the master's text");
    assert_eq!(follower.row_delta, 1, "and it is one row below the master");
    assert_eq!(follower.column_delta, 0, "and in the same column");
    let last = cells[3].shared.as_ref().expect("the cell shares a formula");
    assert_eq!(last.row_delta, 2, "and it is two rows below the master");
    // A group of its own is a group of its own.
    assert_eq!(cells[1].formula.as_deref(), Some("A1*3"), "the other master writes its formula");
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

/// The **validations** of a sheet are read: the rules on what a cell
/// may hold.
#[test]
fn the_validations_of_a_sheet_are_read() {
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
            r#"<worksheet><sheetData/><dataValidations count="3"><dataValidation type="list" allowBlank="1" showInputMessage="1" showErrorMessage="1" sqref="B1:B5 D1"><formula1>"a,b,c"</formula1></dataValidation><dataValidation type="whole" operator="between" errorStyle="warning" showDropDown="1" sqref="C1" prompt="p" promptTitle="pt" error="e" errorTitle="et"><formula1>1</formula1><formula2>10</formula2></dataValidation></dataValidations><extLst><ext uri="{CCE6A557-97BC-4b89-ADB6-D9C93CAAB3DF}"><x14:dataValidations xmlns:x14="x" xmlns:xm="m" count="1"><x14:dataValidation type="list"><x14:formula1><xm:f>Other!$A$1:$A$3</xm:f></x14:formula1><xm:sqref>G1:G4</xm:sqref></x14:dataValidation></x14:dataValidations></ext></extLst></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let rules = &workbook.sheets[0].validations;
    // A rule over two ranges is two rules here.
    assert_eq!(rules.len(), 4, "four rules are read");
    assert_eq!(rules[0].ranges, vec!["B1:B5".to_string()], "the first is about the first range");
    assert_eq!(rules[0].kind, "list", "and it is a list");
    assert_eq!(rules[0].formula1.as_deref(), Some("\"a,b,c\""), "of three words");
    assert!(rules[0].allow_blank, "and a blank is allowed");
    assert!(rules[0].show_input, "and the input message is shown");
    assert!(rules[0].show_error, "and the error message is shown");
    assert_eq!(rules[1].ranges, vec!["D1".to_string()], "the second is about the second range");
    assert_eq!(rules[1].kind, "list", "and it is the same list");
    // The second rule of the file: a whole number between two.
    assert_eq!(rules[2].kind, "whole", "the third is a whole number");
    assert_eq!(rules[2].operator.as_deref(), Some("between"), "between two");
    assert_eq!(rules[2].formula1.as_deref(), Some("1"), "one");
    assert_eq!(rules[2].formula2.as_deref(), Some("10"), "and ten");
    assert_eq!(rules[2].error_style, "warning", "and it warns rather than refuses");
    assert_eq!(rules[2].input_title, "pt", "with a title for the input");
    assert_eq!(rules[2].input_message, "p", "and the input message");
    assert_eq!(rules[2].error_title, "et", "and a title for the error");
    assert_eq!(rules[2].error_message, "e", "and the error message");
    // A file's `showDropDown` hides the arrow of the list.
    assert!(!rules[2].in_cell_dropdown, "and the arrow is not shown");
    // The rule of the extension list: the range is a child, and the
    // formula is in a child of a child.
    assert_eq!(rules[3].ranges, vec!["G1:G4".to_string()], "the fourth is about its range");
    assert_eq!(rules[3].kind, "list", "and it is a list");
    assert_eq!(rules[3].formula1.as_deref(), Some("Other!$A$1:$A$3"), "of another sheet's cells");
}

/// The **printing** of a sheet is read: the area it prints, and the
/// titles it repeats on every page.
///
/// A file says these with the **built-in names**, and they are not
/// names a formula uses.
#[test]
fn the_printing_of_a_sheet_is_read() {
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
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets><definedNames><definedName name="_xlnm.Print_Area" localSheetId="0">One!$A$1:$C$10</definedName><definedName name="_xlnm.Print_Titles" localSheetId="0">One!$A:$B,One!$1:$2</definedName><definedName name="_xlnm._FilterDatabase" localSheetId="0" hidden="1">One!$A$1:$C$10</definedName></definedNames></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.print_area.as_deref(), Some("A1:C10"), "the area the sheet prints");
    assert_eq!(sheet.print_titles.as_deref(), Some("One!$A:$B,One!$1:$2"), "and the titles it repeats");
    // The built-in names are not names a formula uses.
    assert!(workbook.names.is_empty(), "and no name is read");
}

/// The **outline** of a sheet is read: the groups of rows and of
/// columns, and where the summary of a group sits.
#[test]
fn the_outline_of_a_sheet_is_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    // The rows of the groups.
    assert_eq!(sheet.rows.len(), 2, "two rows say something of themselves");
    assert_eq!(sheet.rows[0].row, 2, "the first is the second row");
    assert_eq!(sheet.rows[0].outline, 2, "and it is two deep in a group");
    assert!(sheet.rows[0].hidden, "and it is hidden");
    assert_eq!(sheet.rows[1].row, 3, "the second is the third row");
    assert_eq!(sheet.rows[1].outline, 1, "and it is one deep in a group");
    // The columns of the groups.
    assert_eq!(sheet.columns.len(), 1, "one column says something of itself");
    assert_eq!(sheet.columns[0].column, 2, "it is the third column");
    assert_eq!(sheet.columns[0].outline, 1, "and it is one deep in a group");
    // Where the summary sits.
    assert_eq!(sheet.summary_below, Some(false), "the summary of a group is above it");
    assert_eq!(sheet.summary_right, None, "and the file says nothing of the columns");
}

/// The **page setup** of a sheet is read: the paper, the orientation,
/// the margins, the scale and the fit.
#[test]
fn the_page_setup_of_a_sheet_is_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    let print = workbook.sheets[0].print.as_ref().expect("the sheet has a page setup");
    assert_eq!(print.paper.as_deref(), Some("A4"), "the paper it prints on");
    assert_eq!(print.orientation.as_deref(), Some("landscape"), "and the way it lies");
    assert_eq!(
        print.margins,
        Some([0.7, 0.7, 0.75, 0.75, 0.3, 0.3]),
        "and the margins, in the file's own inches"
    );
    assert_eq!(print.scale, Some(80), "and the scale");
    assert_eq!(print.fit, Some((1, 2)), "and the fit");
    assert!(print.fit_to_page, "and that it prints to fit");
}

/// The **kind** of a formula is read: an **array** fills a range of
/// cells, and a **data table** is a what-if table of values.
#[test]
fn the_kind_of_a_formula_is_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    // The array: the text is the file's own, and the range it fills.
    assert_eq!(cells[0].formula.as_deref(), Some("SUM(B1:B2)"), "the text of the array");
    assert_eq!(cells[0].formula_kind, Some(FormulaKind::Array), "and that it is an array");
    assert_eq!(cells[0].array_range.as_deref(), Some("A1:A2"), "and the range it fills");
    // The cell of the array that holds only a value holds only a value.
    assert!(cells[1].formula.is_none(), "the other cell of the array holds no formula");
    assert_eq!(cells[1].formula_kind, None, "and is not of any kind");
    // The data table: it has no text at all, and it says what it is.
    assert_eq!(cells[2].formula_kind, Some(FormulaKind::DataTable), "the table says what it is");
    assert!(cells[2].formula.is_none(), "and it holds no text");
}

/// The **breaks** of the printing of a sheet are read: the rows and the
/// columns a page begins at.
#[test]
fn the_breaks_of_the_printing_are_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    let breaks = &workbook.sheets[0].breaks;
    assert_eq!(breaks.rows, vec![10], "the row a page begins at");
    assert_eq!(breaks.columns, vec![2], "and the column a page begins at");
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

/// The **comments** of a sheet are read: they are in a part of their
/// own that the sheet is related to.
#[test]
fn the_comments_of_a_sheet_are_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.comments.len(), 1, "one sheet has comments");
    let (_, comments) = &workbook.comments[0];
    assert_eq!(comments.len(), 1, "one comment");
    assert_eq!(comments[0].author, "ann", "its author");
    assert_eq!(comments[0].text, "the text", "and its text, without the author line");
    assert_eq!(comments[0].column, 0, "and the cell it is on");
}

/// The **threads** of a sheet are read: the notes of a newer file, with
/// the replies under them.
#[test]
fn the_threads_of_a_sheet_are_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.comments.len(), 1, "one sheet has comments");
    let (_, comments) = &workbook.comments[0];
    assert_eq!(comments.len(), 1, "one comment");
    assert_eq!(comments[0].author, "ann", "its author");
    assert_eq!(comments[0].text, "the first", "and its text");
    assert_eq!(comments[0].column, 1, "and the cell it is on");
    assert_eq!(comments[0].row, 1, "and the row of it");
    assert!(comments[0].resolved, "and the thread is resolved");
    assert_eq!(comments[0].replies.len(), 1, "and it has a reply");
    assert_eq!(comments[0].replies[0].author, "bob", "written by");
    assert_eq!(comments[0].replies[0].text, "the answer", "and saying");
}

/// The **tables** of a sheet are read: they are in parts of their own
/// that the sheet names by a relationship.
#[test]
fn the_tables_of_a_sheet_are_read() {
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
            r#"<table id="1" name="Table1" displayName="Table1" ref="A1:C5" totalsRowCount="1"><tableColumns count="3"><tableColumn id="1" name="who"/><tableColumn id="2" name="amount" totalsRowFunction="sum" totalsRowLabel="Total"/><tableColumn id="3" name="when" totalsRowFunction="average"/></tableColumns></table>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.tables.len(), 1, "one sheet has tables");
    let (sheet, tables) = &workbook.tables[0];
    assert_eq!(sheet, "One", "the sheet they are on");
    assert_eq!(tables.len(), 1, "one table");
    let table = &tables[0];
    assert_eq!(table.name, "Table1", "its name");
    assert_eq!(table.range, "A1:C5", "the range it reaches");
    assert!(table.totals, "and it has a total row");
    assert_eq!(table.way, "sum", "the way the columns are totalled in");
    assert_eq!(
        table.ways,
        vec![(1, "sum".to_string()), (2, "average".to_string())],
        "and the ways of the columns that have one of their own"
    );
    assert_eq!(table.label, Some("Total".to_string()), "and the label of the total row");
}

/// A table whose way this tree does not have is **left out**, rather
/// than given a name that means nothing here.
#[test]
fn a_way_this_tree_does_not_have_is_left_out() {
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
            r#"<table id="1" name="Table1" displayName="Table1" ref="A1:C5" totalsRowCount="1"><tableColumns count="3"><tableColumn id="1" name="who" totalsRowFunction="custom"/><tableColumn id="2" name="amount" totalsRowFunction="countNums"/><tableColumn id="3" name="when"/></tableColumns></table>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let (_, tables) = &workbook.tables[0];
    let table = &tables[0];
    assert_eq!(table.ways, vec![(1, "count-numbers".to_string())], "only the way this tree has is kept");
    assert_eq!(table.way, "count-numbers", "and it is the way of the table");
    assert_eq!(table.label, None, "and there is no label");
}

/// A sheet with **no table parts** has no tables, and that is not an
/// error.
#[test]
fn a_sheet_with_no_tables_is_empty() {
    let bytes = a_workbook(r#"<worksheet><sheetData/></worksheet>"#, r#"<sst/>"#);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.tables.is_empty(), "no sheet has tables");
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
