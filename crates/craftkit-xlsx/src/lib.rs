//! `craftkit-xlsx` — the **workbook reader**: a workbook file, read as a
//! workbook.
//!
//! # What this is
//!
//! An XLSX file is a **package** (§902) of **XML** parts (§903). This
//! crate reads that package and gives back the workbook it holds: its
//! sheets, their names, and the cells of each of them — their values
//! and their formulas.
//!
//! The result is a **plain** model: a workbook, sheets, cells. It knows
//! nothing about a document, a canvas or the kit. Turning a workbook
//! into a document is the work of the battery that owns the
//! spreadsheet model, not of this reader.
//!
//! # What a workbook is
//!
//! - The **workbook part** says which **sheets** there are, in what
//!   order, and which **date system** the numbers of its cells are
//!   counted in — the 1900 system or the 1904 one.
//! - The **relationships** of the workbook part say which part each
//!   sheet is; the sheet's name is not its part.
//! - The **shared strings** part holds the texts the cells share, and a
//!   cell that is a text names one of them by its number.
//! - A **cell** has an address, a kind and a value: a number, a text, a
//!   boolean or an error — and it may hold a **formula** beside the
//!   value the file cached for it.
//!
//! # The file is hostile
//!
//! A workbook came from somewhere else, so it may be a bomb: a sheet
//! of endless empty cells fits in a small file and takes gigabytes of
//! memory. The reader refuses what it cannot make sense of, and it
//! refuses **within limits**: a sheet may be **1 048 576** rows and
//! **16 384** columns, and a cell outside that is dropped with a
//! warning rather than kept.
//!
//! # What this does not do
//!
//! - It does not **write** a workbook. A writer is a different
//!   program.
//! - It does not read the **styles**: a cell's number format is not
//!   read here, and so a date is a number and not a date. The battery
//!   that owns the spreadsheet model reads the formats itself.
//! - It does not fill in **shared formulas**: a cell that shares a
//!   formula with another cell keeps its value and has no formula.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod rels;
mod styles;

pub use rels::{Rel, rels_of, resolve, split_dir};
pub use styles::{Styles, read_styles};

/// The most **rows** a sheet may have: the tree's own bound.
pub const MAX_ROWS: u64 = 1_048_576;

/// The most **columns** a sheet may have: the tree's own bound.
pub const MAX_COLS: u64 = 16_384;

/// The most **sheets** a workbook may have.
///
/// A workbook of many sheets is a workbook of many parts, and the
/// package already bounds the parts; this bounds the sheets that are
/// read out of them.
pub const MAX_SHEETS: usize = 1_024;

/// Why a workbook could not be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file is not a workbook, or is broken in a way that cannot be
    /// read.
    Malformed(String),
    /// The file asks for more than the reader allows.
    TooLarge(String),
    /// The file is a package that could not be opened.
    Package(String),
    /// A part of the file is not XML, or is broken XML.
    Xml(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Malformed(message) => write!(f, "malformed workbook: {message}"),
            Error::TooLarge(message) => write!(f, "limit exceeded: {message}"),
            Error::Package(message) => write!(f, "the package could not be read: {message}"),
            Error::Xml(message) => write!(f, "a part of the workbook is not readable xml: {message}"),
        }
    }
}

impl std::error::Error for Error {}

/// The **value** of a cell.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// Nothing: the cell is empty.
    Empty,
    /// A number.
    Number(f64),
    /// A text.
    Text(String),
    /// A boolean.
    Bool(bool),
    /// An error, and what it says.
    Error(String),
}

impl Value {
    /// Whether the value is nothing.
    pub fn is_empty(&self) -> bool {
        matches!(self, Value::Empty)
    }
}

/// A **cell**: a place in a sheet, a value, and maybe a formula.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    /// The **column**, counted from zero.
    pub column: u32,
    /// The **row**, counted from zero.
    pub row: u32,
    /// The value the file holds.
    pub value: Value,
    /// The formula's text, when the cell holds one.
    ///
    /// The text is the file's own: a file spells some functions
    /// differently, and the battery that owns the spreadsheet model
    /// knows how to read those.
    pub formula: Option<String>,
    /// The **number format** of the cell, when it has one.
    ///
    /// The code is the file's own, and a cell with no format of its own
    /// has none: the general format is the format of a cell that names
    /// nothing, and it is not carried.
    pub format: Option<String>,
}

/// A **sheet** of a workbook: a name and its cells.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sheet {
    /// The name of the sheet.
    pub name: String,
    /// The cells, in the order the file holds them.
    pub cells: Vec<Cell>,
}

/// The **date system** a workbook's numbers are counted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DateSystem {
    /// The 1900 system, with the day in it that never happened.
    #[default]
    Y1900,
    /// The 1904 system, without one.
    Y1904,
}

/// A **workbook**: its sheets and its date system.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Workbook {
    /// The sheets, in the order the file holds them.
    pub sheets: Vec<Sheet>,
    /// The date system the numbers of the cells are counted in.
    pub system: DateSystem,
    /// The **warnings**: what could not be read, and what was kept
    /// instead.
    ///
    /// A workbook that is damaged is still a workbook, and the reader
    /// says what it had to leave out rather than refusing the whole
    /// file.
    pub warnings: Vec<String>,
}

/// Read a workbook file.
///
/// The file is a package; the workbook part is the one the package's
/// own relationships name, and `xl/workbook.xml` is the fallback for a
/// package that does not say.
pub fn read(bytes: &[u8]) -> Result<Workbook, Error> {
    let mut package =
        craftkit_zip::Package::open(bytes).map_err(|error| Error::Package(error.to_string()))?;
    let mut workbook = Workbook::default();

    let root = rels_of(&mut package, "")?;
    let part = root
        .iter()
        .find(|rel| rel.kind == "officeDocument")
        .map(|rel| rel.target.clone())
        .unwrap_or_else(|| "xl/workbook.xml".to_string());

    let workbook_xml = read_xml(&mut package, &part)?;
    let Some(workbook_xml) = workbook_xml else {
        return Err(Error::Malformed("the package holds no workbook part".to_string()));
    };
    if workbook_xml.name != "workbook" {
        return Err(Error::Malformed(format!(
            "the workbook part holds <{}> and not <workbook>",
            workbook_xml.name
        )));
    }

    // The **date system** is a fact of the workbook, and every number
    // of every sheet is counted in it.
    if let Some(properties) = workbook_xml.child("workbookPr")
        && properties.flag("date1904", false)
    {
        workbook.system = DateSystem::Y1904;
    }

    let workbook_rels = rels_of(&mut package, &part)?;
    let strings = shared_strings(&mut package, &workbook_rels, &mut workbook.warnings)?;
    let styles = read_styles_of(&mut package, &workbook_rels, &mut workbook.warnings);

    let sheet_elements = workbook_xml
        .child("sheets")
        .map(|sheets| sheets.kids("sheet").cloned().collect::<Vec<_>>())
        .unwrap_or_default();

    for (index, element) in sheet_elements.iter().enumerate() {
        if workbook.sheets.len() >= MAX_SHEETS {
            workbook
                .warnings
                .push(format!("the workbook holds more than {MAX_SHEETS} sheets; the rest were not read"));
            break;
        }
        let name = element.attr("name").unwrap_or("").to_string();
        let id = element.attr("id");
        let Some(relation) = id.and_then(|id| workbook_rels.iter().find(|rel| rel.id == id)) else {
            workbook.warnings.push(format!("the sheet \"{name}\" has no part; it was not read"));
            continue;
        };
        if relation.kind != "worksheet" {
            workbook.warnings.push(format!("the sheet \"{name}\" is a {} and was not read", relation.kind));
            continue;
        }
        let mut sheet = read_sheet(&mut package, &relation.target, &strings, &styles, &mut workbook.warnings);
        sheet.name = name;
        workbook.sheets.push(sheet);
        let _ = index;
    }

    Ok(workbook)
}

/// The **shared strings** of a workbook: the texts its cells share.
fn shared_strings(
    package: &mut craftkit_zip::Package<'_>,
    workbook_rels: &[Rel],
    warnings: &mut Vec<String>,
) -> Result<Vec<String>, Error> {
    let Some(relation) = workbook_rels.iter().find(|rel| rel.kind == "sharedStrings") else {
        return Ok(Vec::new());
    };
    let Some(root) = read_xml(package, &relation.target)? else {
        warnings.push(format!("the part {} is not there", relation.target));
        return Ok(Vec::new());
    };
    let mut strings = Vec::new();
    for item in root.kids("si") {
        strings.push(rich_text(item));
    }
    Ok(strings)
}

/// The text of a shared string: its own text and the text of its rich
/// runs, in the order they come.
fn rich_text(element: &craftkit_xml::El) -> String {
    let mut text = String::new();
    if let Some(t) = element.child("t") {
        text.push_str(&t.text);
    }
    for run in element.kids("r") {
        if let Some(t) = run.child("t") {
            text.push_str(&t.text);
        }
    }
    text
}

/// Read a **sheet**: its cells.
fn read_sheet(
    package: &mut craftkit_zip::Package<'_>,
    part: &str,
    strings: &[String],
    styles: &Styles,
    warnings: &mut Vec<String>,
) -> Sheet {
    let mut sheet = Sheet::default();
    let Some(root) = read_xml(package, part).ok().flatten() else {
        warnings.push(format!("the part {part} could not be read; the sheet is empty"));
        return sheet;
    };
    let Some(data) = root.child("sheetData") else {
        return sheet;
    };

    // The row the reader is on: a row that does not say which row it is
    // is the row after the last one, as the tree's reader does.
    let mut next_row: u64 = 0;
    let mut dropped = 0u64;
    let mut next_column: u32;

    for row in data.kids("row") {
        let row_index = match row.attr_u32("r") {
            Some(number) if number >= 1 => u64::from(number) - 1,
            Some(_) => {
                dropped += 1;
                continue;
            }
            None => next_row,
        };
        if row_index >= MAX_ROWS {
            dropped += 1;
            continue;
        }
        next_row = row_index + 1;
        next_column = 0;

        for cell in row.kids("c") {
            let (column, row_index) = match cell.attr("r") {
                Some(address) => match address_of(address.trim()) {
                    Some(place) => place,
                    None => {
                        dropped += 1;
                        continue;
                    }
                },
                None => (next_column, row_index),
            };
            if u64::from(column) >= MAX_COLS || row_index >= MAX_ROWS {
                dropped += 1;
                continue;
            }
            next_column = column + 1;
            let value = value_of(cell, strings);
            let formula = cell.child("f").map(|f| f.text.trim().to_string()).filter(|text| !text.is_empty());
            let format = styles.format(cell.attr_u32("s")).map(|code| code.to_string());
            sheet.cells.push(Cell { column, row: row_index as u32, value, formula, format });
        }
    }

    if dropped > 0 {
        warnings.push(format!(
            "the sheet holds {dropped} cells or rows outside the {MAX_ROWS} × {MAX_COLS} grid; they were dropped"
        ));
    }
    sheet
}

/// The **value** of a cell, as the file holds it.
fn value_of(cell: &craftkit_xml::El, strings: &[String]) -> Value {
    let raw = cell.child("v").map(|v| v.text.trim().to_string());
    match cell.attr("t").unwrap_or("n") {
        "s" => match raw.as_deref().and_then(|text| text.parse::<usize>().ok()) {
            Some(index) => match strings.get(index) {
                Some(text) => Value::Text(text.clone()),
                None => Value::Empty,
            },
            None => Value::Empty,
        },
        "str" => match raw {
            Some(text) => Value::Text(text),
            None => Value::Empty,
        },
        "inlineStr" => match cell.child("is") {
            Some(item) => Value::Text(rich_text(item)),
            None => match raw {
                Some(text) => Value::Text(text),
                None => Value::Empty,
            },
        },
        "b" => match raw.as_deref() {
            Some(text) => Value::Bool(matches!(text, "1" | "true" | "TRUE")),
            None => Value::Empty,
        },
        "e" => match raw {
            Some(text) => Value::Error(text),
            None => Value::Empty,
        },
        _ => match raw.as_deref() {
            Some(text) if !text.is_empty() => match text.parse::<f64>() {
                Ok(number) if number.is_finite() => Value::Number(number),
                _ => Value::Empty,
            },
            _ => Value::Empty,
        },
    }
}

/// The place of a cell, from its address: `B7` is the column 1 and the
/// row 6.
fn address_of(address: &str) -> Option<(u32, u64)> {
    let split = address.find(|character: char| character.is_ascii_digit())?;
    let letters = &address[..split];
    let digits = &address[split..];
    let column = column_index(letters)?;
    let row = digits.parse::<u64>().ok()?;
    if row < 1 {
        return None;
    }
    Some((column, row - 1))
}

/// The column a name of letters says, counted from zero.
fn column_index(letters: &str) -> Option<u32> {
    if letters.is_empty() {
        return None;
    }
    let mut column: u32 = 0;
    for character in letters.chars() {
        if !character.is_ascii_alphabetic() {
            return None;
        }
        column = column * 26 + (character.to_ascii_uppercase() as u32 - 'A' as u32 + 1);
    }
    Some(column - 1)
}

/// The **styles** of a workbook: the format of each cell.
///
/// A workbook with no styles part has no formats, and that is not an
/// error: a file may be written without one.
fn read_styles_of(
    package: &mut craftkit_zip::Package<'_>,
    workbook_rels: &[Rel],
    warnings: &mut Vec<String>,
) -> Styles {
    let Some(relation) = workbook_rels.iter().find(|rel| rel.kind == "styles") else {
        return Styles::default();
    };
    let Some(root) = read_xml(package, &relation.target).ok().flatten() else {
        warnings.push(format!("the part {} could not be read", relation.target));
        return Styles::default();
    };
    read_styles(&root)
}

/// Read a part of the package as XML.
fn read_xml(package: &mut craftkit_zip::Package<'_>, part: &str) -> Result<Option<craftkit_xml::El>, Error> {
    let Some(bytes) = package.read(part).map_err(|error| Error::Package(error.to_string()))? else {
        return Ok(None);
    };
    craftkit_xml::parse(&bytes).map(Some).map_err(|error| Error::Xml(format!("{part}: {error}")))
}
