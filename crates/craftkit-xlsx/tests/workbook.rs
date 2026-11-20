//! The **workbook reader**: a workbook file, read as a workbook.

use craftkit_xlsx::{DateSystem, FormulaKind, Value, read};

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

/// The **default size** of a sheet is read: the width of a column and
/// the height of a row that has no size of its own.
///
/// A sheet that says nothing has no default size, and that is not the
/// tree's own size: what a file does not say is not a fact of the
/// file.
#[test]
fn the_default_size_of_a_sheet_is_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.default_width, Some(12.25), "the columns are twelve and a quarter wide");
    assert_eq!(sheet.default_height, Some(24.0), "and the rows are twenty four points high");
}

/// A sheet that says nothing of its **default size** has none.
#[test]
fn a_sheet_with_no_default_size_has_none() {
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
            r#"<worksheet><sheetFormatPr baseColWidth="8"/><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.default_width, None, "the columns have no default width");
    assert_eq!(sheet.default_height, None, "and the rows have no default height");
}

/// The **palette** of a file is read: the colours it names by their
/// number are the file's own.
#[test]
fn the_palette_of_a_file_is_read() {
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
            r#"<styleSheet><fonts count="1"><font><sz val="11"/><color indexed="1"/><name val="Calibri"/></font></fonts><fills count="1"><fill><patternFill patternType="none"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><colors><indexedColors><rgbColor rgb="FF000000"/><rgbColor rgb="FFFF0000"/></indexedColors></colors></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="0"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    let font = sheet.cells[0].font.as_ref().expect("the cell has a font");
    // The file's palette says that the colour number one is red.
    assert_eq!(font.colour.as_deref(), Some("#ff0000"), "the colour is the file's own");
}

/// A file that names **no palette** has the tree's own: the colours
/// are numbered the way the tree numbers them.
#[test]
fn a_file_with_no_palette_has_the_trees_own() {
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
            r#"<styleSheet><fonts count="1"><font><sz val="11"/><color indexed="1"/><name val="Calibri"/></font></fonts><fills count="1"><fill><patternFill patternType="none"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs></styleSheet>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="0"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    let font = sheet.cells[0].font.as_ref().expect("the cell has a font");
    // The tree's own colour number one is white.
    assert_eq!(font.colour.as_deref(), Some("#ffffff"), "the colour is the tree's own");
}

/// The **named styles** of a workbook are read: the styles the file
/// names, each with the look it carries.
#[test]
fn the_named_styles_of_a_workbook_are_read() {
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
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/styles.xml",
            r#"<styleSheet><fonts count="2"><font><sz val="11"/><name val="Calibri"/></font><font><b/><sz val="14"/><name val="Calibri"/></font></fonts><fills count="1"><fill><patternFill patternType="none"/></fill></fills><borders count="1"><border/></borders><cellStyleXfs count="1"><xf numFmtId="4" fontId="1" fillId="0" borderId="0"/></cellStyleXfs><cellXfs count="1"><xf numFmtId="0" fontId="0" fillId="0" borderId="0" xfId="0"/></cellXfs><cellStyles count="1"><cellStyle name="Heading 1" xfId="0"/></cellStyles></styleSheet>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.named_styles.len(), 1, "the workbook names one style");
    let style = &workbook.named_styles[0];
    assert_eq!(style.name, "Heading 1", "it is called Heading 1");
    assert_eq!(style.format.as_deref(), Some("#,##0.00"), "and it is written with two decimals");
    let font = style.font.as_ref().expect("the style has a font");
    assert!(font.bold, "the font is bold");
    assert_eq!(font.size, Some(14.0), "and it is fourteen points high");
}

/// The **edit ranges** of a sheet are read: the ranges that stay
/// editable while the sheet is protected.
#[test]
fn the_edit_ranges_of_a_sheet_are_read() {
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
            r#"<worksheet><sheetProtection sheet="1"/><protectedRanges><protectedRange name="Range1" password="83AF" sqref="A1:B2 D4:E5"/></protectedRanges><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.edit_ranges.len(), 1, "one range says it is editable");
    let range = &sheet.edit_ranges[0];
    assert_eq!(range.ranges, vec!["A1:B2".to_string(), "D4:E5".to_string()], "and it is two ranges");
    assert_eq!(range.name.as_deref(), Some("Range1"), "the file names it");
    assert!(range.password, "and it names a password for it");
}

/// The **protection** of a workbook is read: the structure of it may
/// be locked, and the file may name a password for it.
#[test]
fn the_protection_of_a_workbook_is_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    let protection = workbook.protection.as_ref().expect("the workbook is protected");
    assert!(protection.structure, "the structure is locked");
    assert!(protection.password, "and the file names a password for it");
}

/// A workbook that says nothing about its **protection** has none.
#[test]
fn a_workbook_with_no_protection_has_none() {
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
    assert_eq!(workbook.protection, None, "the workbook is not protected");
}

/// The **options** of the print of a sheet are read: the marks it
/// prints and where the sheet sits on the page.
#[test]
fn the_options_of_a_print_are_read() {
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
            r#"<worksheet><printOptions gridLines="1" headings="1" horizontalCentered="1" verticalCentered="0"/><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    let options = sheet.print_options.as_ref().expect("the sheet has the options of its print");
    assert!(options.gridlines, "the gridlines are printed");
    assert!(options.headings, "the headings are printed");
    assert!(options.centred_horizontally, "the sheet is centred sideways");
    assert!(!options.centred_vertically, "and not down the page");
}

/// The **header** and the **footer** of the pages of a sheet are read:
/// the texts of the odd, the even and the first pages.
#[test]
fn the_header_and_the_footer_of_a_sheet_are_read() {
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
            r#"<worksheet><headerFooter differentOddEven="1" differentFirst="1"><oddHeader>&amp;LOdd&amp;CCentre</oddHeader><oddFooter>&amp;ROdd</oddFooter><evenHeader>&amp;LEven</evenHeader><evenFooter>&amp;REven</evenFooter><firstHeader>&amp;LFirst</firstHeader><firstFooter>&amp;RFirst</firstFooter></headerFooter><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    let header_footer = sheet.header_footer.as_ref().expect("the sheet has a header and a footer");
    assert_eq!(header_footer.odd_header.as_deref(), Some("&LOdd&CCentre"), "the odd header");
    assert_eq!(header_footer.odd_footer.as_deref(), Some("&ROdd"), "the odd footer");
    assert_eq!(header_footer.even_header.as_deref(), Some("&LEven"), "the even header");
    assert_eq!(header_footer.even_footer.as_deref(), Some("&REven"), "the even footer");
    assert_eq!(header_footer.first_header.as_deref(), Some("&LFirst"), "the first header");
    assert_eq!(header_footer.first_footer.as_deref(), Some("&RFirst"), "the first footer");
}

/// A sheet that says **no header** has none.
#[test]
fn a_sheet_with_no_header_has_none() {
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
            r#"<worksheet><headerFooter differentOddEven="1"><oddHeader></oddHeader></headerFooter><sheetData/></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let sheet = &workbook.sheets[0];
    assert_eq!(sheet.header_footer, None, "the sheet has no header and no footer");
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

/// The **sparklines** of a sheet are read: the little drawings in its
/// cells.
#[test]
fn the_sparklines_of_a_sheet_are_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    let sparklines = &workbook.sheets[0].sparklines;
    assert_eq!(sparklines.len(), 1, "one sparkline is read");
    assert_eq!(sparklines[0].cell, "G2", "the cell it is drawn in");
    assert_eq!(sparklines[0].source, "One!B2:F2", "the range it reads");
    assert_eq!(sparklines[0].kind, "column", "and the kind of it");
    assert_eq!(sparklines[0].colour.as_deref(), Some("#376092"), "and the colour of it");
    assert!(sparklines[0].markers, "and that its markers are drawn");
}

/// The **hidden sheets** of a workbook are read: the state of each of
/// them.
#[test]
fn the_hidden_sheets_of_a_workbook_are_read() {
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet2.xml"/><Relationship Id="rId3" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet3.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/><sheet name="Two" sheetId="2" state="hidden" r:id="rId2"/><sheet name="Three" sheetId="3" state="veryHidden" r:id="rId3"/></sheets></workbook>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
        ("xl/worksheets/sheet2.xml", r#"<worksheet><sheetData/></worksheet>"#),
        ("xl/worksheets/sheet3.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.sheets.len(), 3, "three sheets are read");
    assert_eq!(workbook.sheets[0].state, None, "the first is not hidden");
    assert_eq!(workbook.sheets[1].state.as_deref(), Some("hidden"), "the second is hidden");
    assert_eq!(workbook.sheets[2].state.as_deref(), Some("veryHidden"), "and the third very well");
}

/// The **colour** of a sheet's tab is read.
#[test]
fn the_colour_of_a_sheets_tab_is_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.sheets[0].tab_colour.as_deref(), Some("#00b050"), "the colour of the tab");
}

/// The **charts** of a sheet are read: the drawings of its cells.
#[test]
fn the_charts_of_a_sheet_are_read() {
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
            r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:title><c:tx><c:rich><a:p><a:r><a:t>The Sales</a:t></a:r></a:p></c:rich></c:tx></c:title><c:plotArea><c:catAx><c:title><c:tx><c:rich><a:p><a:r><a:t>Months</a:t></a:r></a:p></c:rich></c:tx></c:title></c:catAx><c:valAx><c:title><c:tx><c:rich><a:p><a:r><a:t>Money</a:t></a:r></a:p></c:rich></c:tx></c:title><c:majorGridlines/></c:valAx><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val></c:ser></c:barChart></c:plotArea><c:legend><c:legendPos val="b"/></c:legend></c:chart></c:chartSpace>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.charts.len(), 1, "one sheet has charts");
    let (_, charts) = &workbook.charts[0];
    assert_eq!(charts.len(), 1, "one chart");
    assert_eq!(charts[0].kind, "column", "the kind of it");
    assert_eq!(charts[0].title.as_deref(), Some("The Sales"), "its title");
    assert_eq!(charts[0].legend.as_deref(), Some("bottom"), "where its legend is");
    assert_eq!(charts[0].x_title.as_deref(), Some("Months"), "the title of its axis of the categories");
    assert_eq!(charts[0].y_title.as_deref(), Some("Money"), "and the title of the axis of the values");
    assert!(charts[0].gridlines, "and that its gridlines are shown");
    assert!(!charts[0].data_labels, "and that its data labels are not");
    assert_eq!(charts[0].categories.as_deref(), Some("One!$A$2:$A$10"), "the cells of its categories");
    assert_eq!(charts[0].series, vec!["One!$B$2:$B$10".to_string()], "and the cells of its series");
    assert!(charts[0].colours.is_empty(), "and the series have no colour of their own");
}

/// A chart that draws **two kinds** in one is a chart of two kinds,
/// and the tree says so.
#[test]
fn a_chart_of_two_kinds_is_a_combo_chart() {
    let package = a_package_of(&[
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
            br#"<workbook><workbookPr/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/_rels/sheet1.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            br#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#,
        ),
        (
            "xl/drawings/drawing1.xml",
            br#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
        ),
        (
            "xl/charts/chart1.xml",
            br#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:grouping val="clustered"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val></c:ser></c:barChart><c:lineChart><c:grouping val="standard"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$C$2:$C$10</c:f></c:numRef></c:val></c:ser></c:lineChart></c:plotArea></c:chart></c:chartSpace>"#,
        ),
    ]);
    let read = read(&package).expect("the workbook is read");
    let (_, charts) = &read.charts[0];
    assert_eq!(charts[0].kind, "combo", "the kind of the chart");
    assert!(
        read.warnings.iter().any(|warning| warning.contains("two kinds")),
        "and the tree says so: {:?}",
        read.warnings
    );
}

/// The **markers** of a chart are read: the file says of the group
/// whether it has them, and of each of its series whether it shows
/// them.
#[test]
fn the_markers_of_a_chart_are_read() {
    for (group, series, expected) in [
        (None, "", true),
        (Some(r#"<c:marker val="0"/>"#), "", false),
        (None, r#"<c:marker><c:symbol val="none"/></c:marker>"#, false),
        (Some(r#"<c:marker val="1"/>"#), "", true),
    ] {
        let group = group.unwrap_or("");
        let package = a_package_of(&[
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
                br#"<workbook><workbookPr/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/worksheets/_rels/sheet1.xml.rels",
                br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                br#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#,
            ),
            (
                "xl/drawings/drawing1.xml",
                br#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
            ),
            (
                "xl/drawings/_rels/drawing1.xml.rels",
                br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
            ),
            (
                "xl/charts/chart1.xml",
                format!(
                    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:lineChart><c:grouping val="standard"/>{group}<c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val>{series}</c:ser></c:lineChart></c:plotArea></c:chart></c:chartSpace>"#
                )
                .as_bytes(),
            ),
        ]);
        let read = read(&package).expect("the workbook is read");
        let (_, charts) = &read.charts[0];
        assert_eq!(charts[0].markers, expected, "the markers of the chart");
    }
}

/// The **smooth line** of a chart is read: the file says of each of
/// its series whether the line is smooth.
#[test]
fn the_smooth_line_of_a_chart_is_read() {
    for (smooths, expected) in [
        (vec![true], Some(true)),
        (vec![false], Some(false)),
        (vec![true, true], Some(true)),
        (vec![true, false], None),
    ] {
        let series: String = smooths
            .iter()
            .enumerate()
            .map(|(index, smooth)| {
                format!(
                    r#"<c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!${}$2:${}$10</c:f></c:numRef></c:val><c:smooth val="{}"/></c:ser>"#,
                    (b'B' + index as u8) as char,
                    (b'B' + index as u8) as char,
                    if *smooth { "1" } else { "0" },
                )
            })
            .collect();
        let package = a_package_of(&[
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
                br#"<workbook><workbookPr/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/worksheets/_rels/sheet1.xml.rels",
                br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                br#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#,
            ),
            (
                "xl/drawings/drawing1.xml",
                br#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
            ),
            (
                "xl/drawings/_rels/drawing1.xml.rels",
                br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
            ),
            (
                "xl/charts/chart1.xml",
                format!(
                    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:lineChart><c:grouping val="standard"/>{series}</c:lineChart></c:plotArea></c:chart></c:chartSpace>"#
                )
                .as_bytes(),
            ),
        ]);
        let read = read(&package).expect("the workbook is read");
        let (_, charts) = &read.charts[0];
        assert_eq!(charts[0].smooth, expected, "the smooth line of the chart");
    }
}

/// The **stacking** of a chart is read: a chart of a file may be
/// stacked, and stacked to a hundred percent.
#[test]
fn the_stacking_of_a_chart_is_read() {
    for (grouping, expected) in
        [("stacked", Some("stacked")), ("percentStacked", Some("percentStacked")), ("clustered", None)]
    {
        let package = a_package_of(&[
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
                br#"<workbook><workbookPr/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
            ),
            (
                "xl/worksheets/_rels/sheet1.xml.rels",
                br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" Target="../drawings/drawing1.xml"/></Relationships>"#,
            ),
            (
                "xl/worksheets/sheet1.xml",
                br#"<worksheet><drawing r:id="rId1"/><sheetData/></worksheet>"#,
            ),
            (
                "xl/drawings/drawing1.xml",
                br#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:twoCellAnchor><xdr:graphicFrame><xdr:nvGraphicFramePr><xdr:cNvPr id="2" name="Chart 1"/></xdr:nvGraphicFramePr><xdr:xfrm/><a:graphic><a:graphicData uri="http://schemas.openxmlformats.org/drawingml/2006/chart"><c:chart xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" r:id="rId1"/></a:graphicData></a:graphic></xdr:graphicFrame><xdr:clientData/></xdr:twoCellAnchor></xdr:wsDr>"#,
            ),
            (
                "xl/drawings/_rels/drawing1.xml.rels",
                br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/chart" Target="../charts/chart1.xml"/></Relationships>"#,
            ),
            (
                "xl/charts/chart1.xml",
                format!(
                    r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:grouping val="{grouping}"/><c:ser><c:cat><c:strRef><c:f>One!$A$2:$A$10</c:f></c:strRef></c:cat><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#
                )
                .as_bytes(),
            ),
        ]);
        let read = read(&package).expect("the workbook is read");
        let (_, charts) = &read.charts[0];
        assert_eq!(charts[0].stacking.as_deref(), expected, "the stacking of the chart");
    }
}

/// The **colours** of the series of a chart are read: the colour a
/// series is drawn in.
#[test]
fn the_colours_of_the_series_of_a_chart_are_read() {
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
            r#"<c:chartSpace xmlns:c="http://schemas.openxmlformats.org/drawingml/2006/chart" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main"><c:chart><c:plotArea><c:barChart><c:barDir val="col"/><c:ser><c:val><c:numRef><c:f>One!$B$2:$B$10</c:f></c:numRef></c:val><c:spPr><a:solidFill><a:srgbClr val="FF4472C4"/></a:solidFill></c:spPr></c:ser><c:ser><c:val><c:numRef><c:f>One!$C$2:$C$10</c:f></c:numRef></c:val><c:spPr><a:solidFill><a:srgbClr val="FFFF0000"/></a:solidFill></c:spPr></c:ser></c:barChart></c:plotArea></c:chart></c:chartSpace>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let (_, charts) = &workbook.charts[0];
    assert_eq!(charts.len(), 1, "one chart");
    assert_eq!(
        charts[0].colours,
        vec!["#4472c4".to_string(), "#ff0000".to_string()],
        "the colours of the series"
    );
}

/// A cell that holds a **date** in the file's own words is read: the
/// date is counted in the file's own system.
#[test]
fn the_dates_of_a_sheet_are_read() {
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
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="d"><v>2024-01-01</v></c><c r="B1" t="d"><v>2024-01-01T12:00:00Z</v></c><c r="C1" t="d"><v>12:00:00</v></c><c r="D1" t="d"><v>not a date</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    assert_eq!(cells[0].value, Value::Number(45292.0), "the day is counted from the epoch of the file");
    assert_eq!(cells[1].value, Value::Number(45292.5), "and a day with a time is half a day later");
    assert_eq!(cells[2].value, Value::Number(0.5), "and a time alone is a part of a day");
    assert_eq!(cells[3].value, Value::Empty, "and a text that is not a date is nothing");
}

/// The **pivot tables** of a sheet are read: the cells they were made
/// from, and the fields they gather them by.
#[test]
fn the_pivot_tables_of_a_sheet_are_read() {
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
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
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
    let workbook = read(&bytes).expect("the workbook is read");
    let pivots = &workbook.sheets[0].pivots;
    assert_eq!(pivots.len(), 1, "one pivot table is read");
    assert_eq!(pivots[0].name.as_deref(), Some("The Pivot"), "its name");
    assert_eq!(pivots[0].source, "One!A1:C10", "the cells it was made from");
    assert_eq!(pivots[0].row_fields, vec!["Region".to_string()], "the field it gathers the rows by");
    assert_eq!(pivots[0].column_fields, vec!["Product".to_string()], "the field it spreads the groups over");
    assert_eq!(
        pivots[0].value,
        Some(("Amount".to_string(), "sum".to_string())),
        "and the field it puts together"
    );
}

/// The **conditional formats** of the extension list are not read,
/// and the reader says so: they are not comparisons, and the tree's
/// rule is a comparison.
#[test]
fn the_conditional_formats_of_the_extension_list_are_not_read() {
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
            r#"<worksheet><sheetData/><extLst><ext uri="{78C0D931-6437-407d-A8EE-F0AAD7539E65}"><x14:conditionalFormattings xmlns:x14="http://schemas.microsoft.com/office/spreadsheetml/2009/9/main"><x14:conditionalFormatting xmlns:xm="http://schemas.microsoft.com/office/excel/2006/main"><x14:cfRule type="dataBar" id="{1}"><x14:dataBar><x14:cfvo type="min"/></x14:dataBar></x14:cfRule></x14:conditionalFormatting></x14:conditionalFormattings></ext></extLst></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(
        workbook.warnings.iter().any(|warning| warning.contains("1 conditional format(s)")),
        "the reader says what it left out: {:?}",
        workbook.warnings
    );
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

/// A file that is a **compound file** is not a package, and the
/// refusal says what it is: a workbook that is encrypted, or a
/// workbook of the old format.
#[test]
fn a_compound_file_is_refused_and_says_what_it_is() {
    let mut bytes = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    bytes.extend_from_slice(&[0u8; 64]);
    let error = read(&bytes).expect_err("that is not a package");
    assert!(matches!(error, craftkit_xlsx::Error::Compound(_)), "{error}");
    assert!(error.to_string().contains("old format"), "the refusal says what it is: {error}");
}

/// A compound file that holds an **encrypted** workbook says so.
#[test]
fn an_encrypted_workbook_says_that_it_is_encrypted() {
    let mut bytes = vec![0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
    bytes.extend_from_slice(&[0u8; 64]);
    for unit in "EncryptedPackage".encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    let error = read(&bytes).expect_err("that is not a package");
    assert!(matches!(error, craftkit_xlsx::Error::Compound(_)), "{error}");
    assert!(error.to_string().contains("encrypted"), "the refusal says what it is: {error}");
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

/// The **theme** of a workbook is read: the name of it, the twelve
/// colours and the two fonts.
#[test]
fn the_theme_of_a_workbook_is_read() {
    let bytes = a_package(&[
        (
            "_rels/.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>"#,
        ),
        (
            "xl/_rels/workbook.xml.rels",
            r#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/><Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" Target="theme/theme1.xml"/></Relationships>"#,
        ),
        (
            "xl/workbook.xml",
            r#"<workbook><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/theme/theme1.xml",
            r#"<theme name="Office"><themeElements><clrScheme><dk1><sysClr val="windowText" lastClr="000000"/></dk1><lt1><sysClr val="window" lastClr="FFFFFF"/></lt1><dk2><srgbClr val="44546A"/></dk2><lt2><srgbClr val="E7E6E6"/></lt2><accent1><srgbClr val="4472C4"/></accent1><accent2><srgbClr val="ED7D31"/></accent2><accent3><srgbClr val="A5A5A5"/></accent3><accent4><srgbClr val="FFC000"/></accent4><accent5><srgbClr val="5B9BD5"/></accent5><accent6><srgbClr val="70AD47"/></accent6><hlink><srgbClr val="0563C1"/></hlink><folHlink><srgbClr val="954F72"/></folHlink></clrScheme><fontScheme><majorFont><latin typeface="Calibri Light"/></majorFont><minorFont><latin typeface="Calibri"/></minorFont></fontScheme></themeElements></theme>"#,
        ),
        ("xl/worksheets/sheet1.xml", r#"<worksheet><sheetData/></worksheet>"#),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let theme = &workbook.theme;
    assert_eq!(theme.name, Some("Office".to_string()), "the name of the theme");
    assert_eq!(theme.colour(0), Some("#ffffff"), "the first colour is the light one");
    assert_eq!(theme.colour(1), Some("#000000"), "and the second is the dark one");
    assert_eq!(theme.colour(2), Some("#e7e6e6"), "and the third is the second light one");
    assert_eq!(theme.colour(4), Some("#4472c4"), "and the fifth is the first accent");
    assert_eq!(theme.colour(11), Some("#954f72"), "and the last is the followed link");
    assert_eq!(theme.major_font, Some("Calibri Light".to_string()), "the major font");
    assert_eq!(theme.minor_font, Some("Calibri".to_string()), "and the minor font");
}

/// A workbook with **no theme** has no theme, and that is not an
/// error.
#[test]
fn a_workbook_with_no_theme_has_none() {
    let bytes = a_workbook(r#"<worksheet><sheetData/></worksheet>"#, r#"<sst/>"#);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.theme.is_empty(), "the theme says nothing");
    assert_eq!(workbook.theme.colour(0), None, "and it has no colour");
}

/// A cell that names a colour **by the theme** is coloured with it.
#[test]
fn a_cell_that_names_a_colour_by_the_theme_is_coloured_with_it() {
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
            r#"<styleSheet><fonts><font><scheme val="minor"/></font></fonts><fills><fill><patternFill patternType="none"/></fill><fill><patternFill patternType="solid"><fgColor theme="4"/></patternFill></fill></fills><cellXfs><xf numFmtId="0" fontId="0" fillId="0"/><xf numFmtId="0" fontId="0" fillId="1"/></cellXfs></styleSheet>"#,
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
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    assert_eq!(
        cells[0].fill.as_ref().and_then(|fill| fill.colour.as_deref()),
        Some("#4472c4"),
        "the fill is the colour of the theme it names"
    );
    assert_eq!(
        cells[0].font.as_ref().and_then(|font| font.name.as_deref()),
        Some("Calibri"),
        "and the font is the face of the theme it names"
    );
}

/// A **tint** is the file's way of saying that a colour is lighter or
/// darker than the colour it names.
#[test]
fn a_tint_moves_the_luminance_of_the_colour() {
    let darker = craftkit_xlsx::apply_tint("#4472c4", -0.5).expect("the colour is tinted");
    let lighter = craftkit_xlsx::apply_tint("#4472c4", 0.5).expect("the colour is tinted");
    assert_ne!(darker, "#4472c4", "a negative tint darkens the colour");
    assert_ne!(lighter, "#4472c4", "and a positive one lightens it");
    assert_eq!(darker, "#203864", "the dark one is half as light");
    assert_eq!(lighter, "#a2b8e2", "and the light one is half as dark");
    assert_eq!(
        craftkit_xlsx::apply_tint("#4472c4", 0.0),
        Some("#4472c4".to_string()),
        "and a tint of nothing is the colour it was"
    );
}

/// A **checkbox** is a cell control, and a file holds it beside its
/// cells: the bags of the file say that a cell is one, and the cell's
/// style names the bag.
#[test]
fn the_checkbox_of_a_cell_is_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    let cells = &workbook.sheets[0].cells;
    assert!(cells[0].checkbox, "the cell that names the bag is a checkbox");
    assert_eq!(cells[0].value, Value::Bool(true), "and it holds what the file holds");
    assert!(!cells[1].checkbox, "and a cell that names no style is not a checkbox");
}

/// A **broken chain** is not a checkbox: a bag that points at a bag
/// that is not there, or at a bag of another kind, resolves to nothing.
#[test]
fn a_broken_chain_is_not_a_checkbox() {
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
            r#"<FeaturePropertyBags xmlns="http://schemas.microsoft.com/office/spreadsheetml/2022/featurepropertybag" count="4"><bag type="Checkbox"/><bag type="XFControls"><bagId k="CellControl">99</bagId></bag><bag type="XFComplement"><bagId k="XFControls">1</bagId></bag><bag type="XFComplements"><a k="MappedFeaturePropertyBags"><bagId>2</bagId></a></bag></FeaturePropertyBags>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" s="1" t="b"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(!workbook.sheets[0].cells[0].checkbox, "half a chain is not a checkbox");
}

/// The **calculation settings** of a workbook are read: how it
/// calculates, whether it iterates, and how much.
#[test]
fn the_calculation_settings_of_a_workbook_are_read() {
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
            r#"<workbook><calcPr calcMode="manual" iterate="1" iterateCount="50" iterateDelta="0.01" fullPrecision="0"/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let calculation = &workbook.calculation;
    assert_eq!(calculation.mode, craftkit_xlsx::CalculationMode::Manual, "it calculates by hand");
    assert!(calculation.iterative, "and it iterates the circular references");
    assert_eq!(calculation.max_iterations, 50, "fifty times at most");
    assert_eq!(calculation.max_change, 0.01, "and it stops at a change of a hundredth");
    assert!(calculation.precision_as_displayed, "and it calculates with the precision shown");
    assert!(workbook.warnings.is_empty(), "and nothing was left out");
}

/// A workbook that says nothing calculates by itself.
#[test]
fn a_workbook_without_settings_calculates_by_itself() {
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
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let calculation = &workbook.calculation;
    assert_eq!(calculation.mode, craftkit_xlsx::CalculationMode::Automatic, "it calculates by itself");
    assert!(!calculation.iterative, "and it does not iterate");
    assert_eq!(calculation.max_iterations, craftkit_xlsx::DEFAULT_ITERATIONS, "a hundred times at most");
    assert_eq!(
        calculation.max_change,
        craftkit_xlsx::DEFAULT_CHANGE,
        "and it stops at a change of a thousandth"
    );
}

/// A setting the tree cannot read is **not read**: the tree says so and
/// keeps the default rather than guessing.
#[test]
fn a_setting_that_is_not_read_is_said() {
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
            r#"<workbook><calcPr calcMode="whenever" iterateCount="999999"/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.calculation.mode, craftkit_xlsx::CalculationMode::Automatic, "the mode falls back");
    assert_eq!(
        workbook.calculation.max_iterations,
        craftkit_xlsx::DEFAULT_ITERATIONS,
        "and so do the iterations"
    );
    assert_eq!(workbook.warnings.len(), 2, "and both are said: {}", workbook.warnings.join("; "));
}

/// A **reading** of a text — the phonetic guide a file holds beside it
/// — is read with the text it is over.
#[test]
fn the_reading_of_a_text_is_read() {
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
            r#"<sst count="1" uniqueCount="1"><si><t>東京都</t><rPh sb="0" eb="2"><t>トウキョウ</t></rPh><rPh sb="2" eb="3"><t>ト</t></rPh><phoneticPr fontId="1" type="noConversion" alignment="center"/></si></sst>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let cell = &workbook.sheets[0].cells[0];
    assert_eq!(cell.value, Value::Text("東京都".to_string()), "the text is the text");
    let phonetic = cell.phonetic.as_ref().expect("the reading is read");
    assert_eq!(phonetic.runs.len(), 2, "two runs of the reading");
    assert_eq!(phonetic.runs[0].start, 0, "the first run starts at the beginning");
    assert_eq!(phonetic.runs[0].end, 2, "and ends after the first two units");
    assert_eq!(phonetic.runs[0].text, "トウキョウ", "and holds the reading");
    let props = phonetic.props.as_ref().expect("the properties are read");
    assert_eq!(props.font_id, Some(1), "the font of the reading");
    assert_eq!(props.kind.as_deref(), Some("noConversion"), "and its kind");
    assert_eq!(props.alignment.as_deref(), Some("center"), "and how it sits");
    assert!(workbook.warnings.is_empty(), "and nothing was left out");
}

/// A reading that does not fit the text is **not read**: the tree says
/// how many it left out rather than repairing them.
#[test]
fn a_reading_that_does_not_fit_is_not_read() {
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
            r#"<sst count="1" uniqueCount="1"><si><t>ab</t><rPh sb="0" eb="9"><t>past the end</t></rPh><rPh sb="1" eb="0"><t>the wrong way round</t></rPh><rPh sb="0"><t>no end</t></rPh><rPh sb="0" eb="1"><t>ok</t></rPh></si></sst>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let phonetic = workbook.sheets[0].cells[0].phonetic.as_ref().expect("the reading is read");
    assert_eq!(phonetic.runs.len(), 1, "only the run that fits is read");
    assert_eq!(phonetic.runs[0].text, "ok", "and it is the one that fits");
    assert_eq!(workbook.warnings.len(), 1, "and the three that do not fit are said");
}

/// Whether the **objects** of a workbook are drawn is a fact of the
/// file, and it is read.
#[test]
fn the_objects_of_a_workbook_are_read() {
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
            r#"<workbook><workbookPr showObjects="none"/><sheets><sheet name="One" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.objects, craftkit_xlsx::ShowObjects::None, "the objects are not drawn");
}

/// A workbook that says nothing draws its objects.
#[test]
fn a_workbook_without_a_word_draws_its_objects() {
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
            r#"<worksheet><sheetData><row r="1"><c r="A1"><v>1</v></c></row></sheetData></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.objects, craftkit_xlsx::ShowObjects::All, "the objects are drawn");
}

/// The **kinds of rule** a file holds are read: the words of a cell, the
/// empty cells, the errors and the formula — and the kinds this tree does
/// not have are counted, and the reader says so.
#[test]
fn the_kinds_of_conditional_format_are_read() {
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
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="containsText" text="ann" priority="1" dxfId="0"><formula>NOT(ISERROR(SEARCH("ann",A1)))</formula></cfRule></conditionalFormatting><conditionalFormatting sqref="B1:B5"><cfRule type="beginsWith" text="b" priority="2"><formula>LEFT(B1,1)="b"</formula></cfRule></conditionalFormatting><conditionalFormatting sqref="C1:C5"><cfRule type="containsBlanks" priority="3"/></conditionalFormatting><conditionalFormatting sqref="D1:D5"><cfRule type="notContainsErrors" priority="4"/></conditionalFormatting><conditionalFormatting sqref="E1:E5"><cfRule type="expression" priority="5"><formula>E1&gt;0</formula></cfRule></conditionalFormatting><conditionalFormatting sqref="F1:F5"><cfRule type="timePeriod" priority="6" timePeriod="today"/></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let rules = &workbook.sheets[0].conditionals;
    assert_eq!(rules.len(), 5, "the five rules this tree has are read");
    // The **words** of a cell: the rule is the operation and the words.
    assert_eq!(rules[0].op, "contains", "the first looks for the words");
    assert_eq!(rules[0].value, "ann", "and the words are ann");
    assert_eq!(rules[0].fill.as_deref(), Some("#ffc7ce"), "and it carries the look of the rule");
    assert_eq!(rules[1].op, "beginsWith", "the second is a begins-with");
    assert_eq!(rules[1].value, "b", "and the word is b");
    // The **empty cells** and the **errors**: the rule names a cell by
    // what it holds, and it has no value.
    assert_eq!(rules[2].op, "isEmpty", "the third is the empty cells");
    assert_eq!(rules[2].value, "", "and it has no value");
    assert_eq!(rules[3].op, "isNotError", "the fourth is the cells that hold no error");
    // The **formula**: the cells whose formula is true are the cells
    // that match.
    assert_eq!(rules[4].formula.as_deref(), Some("E1>0"), "the fifth is a formula");
    // The kind this tree does not have is **counted**, and the reader
    // says so.
    let said = workbook.warnings.join(" ");
    assert!(
        said.contains("1 conditional format(s) of a kind this tree does not have"),
        "it says how many: {said}"
    );
}

/// A workbook's **colour scale** is read: the colours of the scale, and
/// the stops of it that the tree has.
#[test]
fn a_colour_scale_of_a_sheet_is_read() {
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
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="colorScale" priority="1"><colorScale><cfvo type="min"/><cfvo type="percentile" val="50"/><cfvo type="max"/><color rgb="FFF8696B"/><color rgb="FFFFEB84"/><color rgb="FF63BE7B"/></colorScale></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let rules = &workbook.sheets[0].conditionals;
    assert_eq!(rules.len(), 1, "the scale is read");
    let scale = rules[0].colour_scale.as_ref().expect("the rule is a scale");
    assert_eq!(
        scale.colours,
        vec!["#f8696b".to_string(), "#ffeb84".to_string(), "#63be7b".to_string()],
        "the colours of the scale are read"
    );
    assert!(workbook.warnings.is_empty(), "and nothing is left out: {:?}", workbook.warnings);
}

/// A scale whose stops are not the ones the tree has is **counted**.
#[test]
fn a_scale_that_the_tree_cannot_read_is_counted() {
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
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="colorScale" priority="1"><colorScale><cfvo type="min"/><cfvo type="percentile" val="30"/><cfvo type="max"/><color rgb="FFF8696B"/><color rgb="FFFFEB84"/><color rgb="FF63BE7B"/></colorScale></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets[0].conditionals.is_empty(), "the scale is not read");
    let said = workbook.warnings.join(" ");
    assert!(said.contains("1 colour scale(s) were not read"), "it says how many: {said}");
}

/// The **kinds of rule** that are about a range as a whole are read: the
/// top ten, the above-the-average, the duplicates and the data bar.
#[test]
fn the_kinds_of_rule_about_a_range_are_read() {
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
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="top10" priority="1" rank="3"><cfvo type="num" val="3"/></cfRule></conditionalFormatting><conditionalFormatting sqref="B1:B5"><cfRule type="top10" priority="2" bottom="1"><cfvo type="percent" val="25"/></cfRule></conditionalFormatting><conditionalFormatting sqref="C1:C5"><cfRule type="aboveAverage" priority="3"/></conditionalFormatting><conditionalFormatting sqref="D1:D5"><cfRule type="duplicateValues" priority="4"/></conditionalFormatting><conditionalFormatting sqref="E1:E5"><cfRule type="uniqueValues" priority="5"/></conditionalFormatting><conditionalFormatting sqref="F1:F5"><cfRule type="dataBar" priority="6"><dataBar><cfvo type="min"/><cfvo type="max"/><color rgb="FF638EC6"/></dataBar></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let rules = &workbook.sheets[0].conditionals;
    assert_eq!(rules.len(), 6, "the six rules are read");
    let top = rules[0].top.as_ref().expect("the first is a top ten");
    assert_eq!(top.count, 3, "the count of the top ten is read");
    assert!(!top.bottom, "and it is the highest");
    assert!(!top.percent, "and it is a count and not a percent");
    let lowest = rules[1].top.as_ref().expect("the second is a top ten");
    assert_eq!(lowest.count, 25, "the count of the percent is read");
    assert!(lowest.bottom, "and it is the lowest");
    assert!(lowest.percent, "and it is a percent");
    let average = rules[2].average.as_ref().expect("the third is the average");
    assert!(average.above, "and it is the numbers above it");
    let duplicate = rules[3].duplicate.as_ref().expect("the fourth is the duplicate");
    assert!(!duplicate.unique, "and it is the repeated values");
    let unique = rules[4].duplicate.as_ref().expect("the fifth is the duplicate");
    assert!(unique.unique, "and it is the values held once");
    let bar = rules[5].data_bar.as_ref().expect("the sixth is a bar");
    assert_eq!(bar.colour, "#638ec6", "and the colour of the bar is read");
    assert!(workbook.warnings.is_empty(), "and nothing is left out: {:?}", workbook.warnings);
}

/// An above-the-average with a **standard deviation** is a kind the tree
/// does not have, and it is counted.
#[test]
fn an_above_the_average_with_a_standard_deviation_is_counted() {
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
            r#"<worksheet><sheetData/><conditionalFormatting sqref="C1:C5"><cfRule type="aboveAverage" priority="3" stdDev="2"/></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets[0].conditionals.is_empty(), "the rule is not read");
    let said = workbook.warnings.join(" ");
    assert!(
        said.contains("1 conditional format(s) of a kind this tree does not have"),
        "it says how many: {said}"
    );
}

/// A workbook's **icon set** is read: the two percents that tell which
/// of the three icons a cell has.
#[test]
fn an_icon_set_of_a_sheet_is_read() {
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
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="iconSet" priority="1"><iconSet iconSet="3TrafficLights1"><cfvo type="percent" val="0"/><cfvo type="percent" val="33"/><cfvo type="percent" val="67"/></iconSet></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let rules = &workbook.sheets[0].conditionals;
    assert_eq!(rules.len(), 1, "the icon set is read");
    let icons = rules[0].icon_set.as_ref().expect("the rule is an icon set");
    assert_eq!(icons.high, 67.0, "the high icon is the 67th percentile");
    assert_eq!(icons.low, 33.0, "and the middle icon is the 33rd");
    assert!(workbook.warnings.is_empty(), "and nothing is left out: {:?}", workbook.warnings);
}

/// An icon set that the tree does not have — one that is **reversed**,
/// and one of **four** icons — is **counted**.
#[test]
fn an_icon_set_that_the_tree_cannot_read_is_counted() {
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
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="iconSet" priority="1"><iconSet iconSet="3TrafficLights1" reverse="1"><cfvo type="percent" val="0"/><cfvo type="percent" val="33"/><cfvo type="percent" val="67"/></iconSet></cfRule></conditionalFormatting><conditionalFormatting sqref="B1:B5"><cfRule type="iconSet" priority="2"><iconSet iconSet="4Rating"><cfvo type="percent" val="0"/><cfvo type="percent" val="25"/><cfvo type="percent" val="50"/><cfvo type="percent" val="75"/></iconSet></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.sheets[0].conditionals.is_empty(), "neither icon set is read");
    let said = workbook.warnings.join(" ");
    assert!(
        said.contains("2 conditional format(s) of a kind this tree does not have"),
        "it says how many: {said}"
    );
}

/// The **pictures** of a sheet are read: the cell they are drawn in, the
/// name the file gives them and their bytes.
#[test]
fn the_pictures_of_a_sheet_are_read() {
    // A **picture**: the bytes of a small PNG file.
    let picture: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x01];
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
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.pictures.len(), 1, "one sheet has pictures");
    let (sheet, pictures) = &workbook.pictures[0];
    assert_eq!(sheet, "One", "the sheet of them");
    assert_eq!(pictures.len(), 1, "one picture");
    assert_eq!(pictures[0].cell, "B3", "the cell it is drawn in");
    assert_eq!(pictures[0].name, "Picture 1", "the name the file gives it");
    assert_eq!(pictures[0].alt, "the logo", "and its alt text");
    assert_eq!(pictures[0].format, "png", "and the format of its bytes");
    assert_eq!(pictures[0].bytes, picture, "and the bytes themselves");
    assert!(workbook.warnings.is_empty(), "and nothing is left out: {:?}", workbook.warnings);
}

/// A picture that is in no cell — one that is in no **anchor** — is
/// counted.
#[test]
fn a_picture_that_is_in_no_cell_is_counted() {
    let picture: &[u8] = &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x01];
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
            br#"<xdr:wsDr xmlns:xdr="http://schemas.openxmlformats.org/drawingml/2006/spreadsheetDrawing" xmlns:a="http://schemas.openxmlformats.org/drawingml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><xdr:pic><xdr:nvPicPr><xdr:cNvPr id="2" name="Picture 1"/></xdr:nvPicPr><xdr:blipFill><a:blip r:embed="rId1"/></xdr:blipFill></xdr:pic></xdr:wsDr>"#,
        ),
        (
            "xl/drawings/_rels/drawing1.xml.rels",
            br#"<Relationships><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/image" Target="../media/image1.png"/></Relationships>"#,
        ),
        ("xl/media/image1.png", picture),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.pictures.is_empty(), "the picture is not read");
    let said = workbook.warnings.join(" ");
    assert!(said.contains("1 picture(s) of the sheet were not read"), "it says how many: {said}");
}

/// A rule's **colour of the text** is read: the font of the
/// differential format names it.
#[test]
fn the_colour_of_the_text_of_a_rule_is_read() {
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
            r##"<styleSheet><dxfs count="1"><dxf><font><color rgb="FF9C0006"/></font><fill><patternFill><bgColor rgb="FFFFC7CE"/></patternFill></fill><border><top style="thin"><color rgb="FF00B050"/></top></border></dxf></dxfs></styleSheet>"##,
        ),
        (
            "xl/worksheets/sheet1.xml",
            r#"<worksheet><sheetData/><conditionalFormatting sqref="A1:A5"><cfRule type="cellIs" operator="greaterThan" priority="1" dxfId="0"><formula>10</formula></cfRule></conditionalFormatting></worksheet>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let rules = &workbook.sheets[0].conditionals;
    assert_eq!(rules.len(), 1, "the rule is read");
    assert_eq!(rules[0].colour.as_deref(), Some("#9c0006"), "the colour of the text is read");
    assert_eq!(rules[0].fill.as_deref(), Some("#ffc7ce"), "and the fill behind the cells");
    assert_eq!(rules[0].borders.edges.len(), 1, "and the border of the cells is read");
    let border = &rules[0].borders.edges[0];
    assert_eq!(border.edge, "top", "the edge the line is on");
    assert_eq!(border.style, "thin", "the style of the line");
    assert_eq!(border.colour.as_deref(), Some("#00b050"), "and the colour of it");
}

/// A table whose **header row is off** is not a table of this tree, and
/// the reader says that it did not read it.
#[test]
fn a_table_with_no_header_row_is_not_read() {
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
    let workbook = read(&bytes).expect("the workbook is read");
    assert!(workbook.tables.is_empty(), "the sheet has no tables");
    assert_eq!(
        workbook.warnings,
        vec![
            "the table `Table1` has no header row, and a table of this tree has one; it was not read"
                .to_string()
        ],
        "and the reader says why"
    );
}

/// A table with a **filter of its own** is read, and the filter is
/// left out — and the reader says so.
#[test]
fn the_filter_of_a_table_is_left_out_and_said() {
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
            r#"<table id="1" name="Table1" displayName="Table1" ref="A1:C5"><autoFilter ref="A1:C5"/><tableColumns count="3"><tableColumn id="1" name="who"/><tableColumn id="2" name="amount"/><tableColumn id="3" name="when"/></tableColumns></table>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    assert_eq!(workbook.tables.len(), 1, "the sheet has a table");
    assert_eq!(workbook.tables[0].1.len(), 1, "and the table is read");
    assert_eq!(
        workbook.warnings,
        vec![
            "the table `Table1` has a filter of its own, and the filters of this tree are the sheet's; it was not read"
                .to_string()
        ],
        "and the reader says that the filter was not read"
    );
}

/// A column totalled in a **way this tree does not have** keeps its
/// table, and the reader says that the way was not read.
#[test]
fn a_way_this_tree_does_not_have_is_said() {
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
            r#"<table id="1" name="Table1" displayName="Table1" ref="A1:C5" totalsRowCount="1"><tableColumns count="3"><tableColumn id="1" name="who" totalsRowFunction="custom"/><tableColumn id="2" name="amount" totalsRowFunction="sum"/><tableColumn id="3" name="when"/></tableColumns></table>"#,
        ),
    ]);
    let workbook = read(&bytes).expect("the workbook is read");
    let (_, tables) = &workbook.tables[0];
    assert_eq!(tables.len(), 1, "the table is read");
    assert_eq!(tables[0].ways, vec![(1, "sum".to_string())], "and only the way this tree has is kept");
    assert_eq!(
        workbook.warnings,
        vec![
            "the column `who` of the table `Table1` is totalled in a way this tree does not have; it was not read"
                .to_string()
        ],
        "and the reader says that the way was not read"
    );
}
