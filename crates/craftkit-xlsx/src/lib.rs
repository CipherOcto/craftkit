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
pub use styles::{Alignment, Border, Borders, Fill, Font, Protection, Styles, Underline, Vert, read_styles};

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
    /// The **font** of the cell, when the cell names a style of its
    /// own.
    ///
    /// A cell that names no style has no font here: the font of a cell
    /// that names nothing is the sheet's own, and a sheet's own is not
    /// a cell's.
    pub font: Option<Font>,
    /// The **fill** of the cell: the colour behind its text, when the
    /// cell names a style of its own and that style has a colour
    /// behind the text.
    pub fill: Option<Fill>,
    /// The **alignment** of the cell, when the cell names a style of
    /// its own and that style says how the text lies.
    pub align: Option<Alignment>,
    /// The **borders** of the cell: the lines on its edges, when the
    /// cell names a style of its own and that style has lines.
    pub borders: Option<Borders>,
    /// The **protection** of the cell: whether it is locked and
    /// whether its text is hidden, when the cell names a style of its
    /// own and that style says so.
    pub protection: Option<Protection>,
}

/// A **sheet** of a workbook: a name and its cells.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sheet {
    /// The name of the sheet.
    pub name: String,
    /// The cells, in the order the file holds them.
    pub cells: Vec<Cell>,
    /// The **rows** of the sheet that say something of themselves: their
    /// height and whether they are hidden.
    pub rows: Vec<Row>,
    /// The **columns** of the sheet that say something of themselves:
    /// their width and whether they are hidden.
    pub columns: Vec<Column>,
    /// The **protection** of the sheet, when the sheet is protected.
    ///
    /// A sheet that is not protected has none: a sheet that is not
    /// protected says nothing about what may be done to it.
    pub protection: Option<SheetProtection>,
}

/// A **row** of a sheet: its height, and whether it is hidden.
///
/// The height is in **points**, as the file writes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Row {
    /// The **row**, counted from one.
    pub row: u64,
    /// The **height** of the row, in points, when the file gives one.
    pub height: Option<f64>,
    /// Whether the row is **hidden**.
    pub hidden: bool,
}

/// A **column** of a sheet: its width, and whether it is hidden.
///
/// The width is the **file's** own: the number of characters of the
/// face of the sheet, with the padding of the file in it. This tree's
/// own column width is the number of characters of the **cell's** face,
/// and the two are not the same measure — so the number is carried as
/// the file gives it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Column {
    /// The **column**, counted from zero.
    pub column: u32,
    /// The **width** of the column, when the file gives one.
    pub width: Option<f64>,
    /// Whether the column is **hidden**.
    pub hidden: bool,
}

/// The **protection** of a sheet: what may still be done to it.
///
/// The tree's own sheet protection is a dozen flags, and this one has
/// four: the sorting, the filtering, the formatting and the lock of the
/// cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SheetProtection {
    /// Whether the **sorting** is still allowed.
    pub sort: bool,
    /// Whether the **filtering** is still allowed.
    pub filter: bool,
    /// Whether the **formatting** is still allowed.
    pub format: bool,
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

/// A **table** of a sheet: a range of cells with a name and a way of
/// being totalled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// The name of the table.
    pub name: String,
    /// The range it reaches, as the file writes it.
    pub range: String,
    /// Whether the table has a **total row**.
    pub totals: bool,
    /// The **way** the columns are totalled in, and the ways of the
    /// columns that have one of their own: the index of the column in
    /// the range and the way it is totalled in.
    pub way: String,
    /// The ways of the columns that have one of their own.
    pub ways: Vec<(u32, String)>,
    /// The **label** of the total row: the text it shows in its first
    /// cell.
    pub label: Option<String>,
}

/// A **comment** of a sheet: a note a cell carries.
///
/// A comment has an **author** and a **text**, and it is on one cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comment {
    /// The column of the cell, counted from zero.
    pub column: u32,
    /// The row of the cell, counted from zero.
    pub row: u32,
    /// Who wrote it.
    pub author: String,
    /// What it says.
    pub text: String,
}

/// A **link** of a sheet: a place a cell goes to.
///
/// The link is over a **range** of cells: a file may put one link on
/// many cells at once, and the tree's reader copies it to each of
/// them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hyperlink {
    /// The first cell of the range.
    pub column: u32,
    /// The row of the first cell.
    pub row: u64,
    /// The last cell of the range: the same column and row when the
    /// link is on one cell.
    pub last_column: u32,
    /// The row of the last cell.
    pub last_row: u64,
    /// Where the link goes.
    pub target: String,
    /// The text shown when the mouse rests on it.
    pub tooltip: Option<String>,
    /// Whether it goes **outside** the workbook.
    pub external: bool,
}

/// A **criterion** of a filter: what the cells of a column are
/// judged by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterCriterion {
    /// A **list** of the values that are kept, and whether an empty
    /// cell is kept with them.
    Values {
        /// The values that are kept.
        values: Vec<String>,
        /// Whether an empty cell is kept.
        blanks: bool,
    },
    /// One comparison, or two joined by **and** or by **or**.
    Custom {
        /// The first comparison: the operation and the value.
        first: (String, String),
        /// The second comparison, when there is one.
        second: Option<(String, String)>,
        /// Whether the two are joined by **and** and not by **or**.
        and: bool,
    },
    /// The **top** rows of a column, or the bottom ones.
    Top10 {
        /// Whether they are the bottom ones and not the top ones.
        bottom: bool,
        /// Whether the count is a **percentage** and not a number of
        /// rows.
        percent: bool,
        /// How many.
        count: u32,
    },
    /// The rows **above** the average of the column, or below it.
    AboveAverage(bool),
}

/// A **filter** of a sheet: the rows it hides.
///
/// The filter is over a **range**, and each of its columns has a
/// criterion of its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoFilter {
    /// The range the filter is over.
    pub range: Option<String>,
    /// The criteria, by the **offset** of their column in the range.
    pub criteria: Vec<(u32, FilterCriterion)>,
}

/// A **name** of a workbook: a word that stands for a range of cells.
///
/// The name is the file's own, and so is the text it stands for: a
/// file writes that text as a formula, and what it names is read from
/// the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The name itself.
    pub name: String,
    /// The **scope** of the name: the sheet it belongs to, when it
    /// belongs to one, and nothing when it belongs to the workbook.
    ///
    /// The scope is a sheet's **name**, and not a sheet's number: the
    /// number in the file counts the sheets the file holds, and a file
    /// may hold sheets this reader does not read.
    pub scope: Option<String>,
    /// The text the name stands for.
    pub refers_to: String,
}

/// A **workbook**: its sheets and its date system.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Workbook {
    /// The sheets, in the order the file holds them.
    pub sheets: Vec<Sheet>,
    /// The **names** of the workbook: the words the formulas use for
    /// the ranges they name.
    pub names: Vec<Name>,
    /// The **filter** of each sheet, by the name of the sheet.
    pub filters: Vec<(String, AutoFilter)>,
    /// The **links** of each sheet, by the name of the sheet.
    pub links: Vec<(String, Vec<Hyperlink>)>,
    /// The **comments** of each sheet, by the name of the sheet.
    pub comments: Vec<(String, Vec<Comment>)>,
    /// The **tables** of each sheet, by the name of the sheet.
    pub tables: Vec<(String, Vec<Table>)>,
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

    let sheet_names: Vec<String> = workbook_xml
        .child("sheets")
        .map(|sheets| {
            sheets.kids("sheet").map(|sheet| sheet.attr("name").unwrap_or("").to_string()).collect()
        })
        .unwrap_or_default();
    workbook.names = names_of(&workbook_xml, &sheet_names);

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
        let (mut sheet, filter, links, comments, tables) =
            read_sheet(&mut package, &relation.target, &strings, &styles, &mut workbook.warnings);
        sheet.name = name;
        if let Some(filter) = filter {
            workbook.filters.push((sheet.name.clone(), filter));
        }
        if !links.is_empty() {
            workbook.links.push((sheet.name.clone(), links));
        }
        if !comments.is_empty() {
            workbook.comments.push((sheet.name.clone(), comments));
        }
        if !tables.is_empty() {
            workbook.tables.push((sheet.name.clone(), tables));
        }
        workbook.sheets.push(sheet);
        let _ = index;
    }

    Ok(workbook)
}

/// The **names** of a workbook: the words its formulas use.
///
/// The built-in names — the ones a file gives to the print area, the
/// print titles and the filter — are not names a formula uses, and
/// they are not read: this reader gives back the workbook's shape, and
/// the shape is the sheets, the cells and the names.
fn names_of(workbook_xml: &craftkit_xml::El, sheets: &[String]) -> Vec<Name> {
    let Some(names) = workbook_xml.child("definedNames") else { return Vec::new() };
    let mut out = Vec::new();
    for element in names.kids("definedName") {
        let Some(name) = element.attr("name") else { continue };
        // A **built-in** name is not a name: it says something about
        // the printing of a sheet or the filter of one.
        if name.to_ascii_lowercase().starts_with("_xlnm.") {
            continue;
        }
        let refers_to = element.text.trim().to_string();
        if refers_to.is_empty() {
            continue;
        }
        // The **scope**: the sheet the name belongs to, by the number
        // the file gives it, and the number counts the sheets of the
        // file and not the sheets this reader reads.
        let scope = element.attr_u32("localSheetId").and_then(|number| sheets.get(number as usize).cloned());
        out.push(Name { name: name.to_string(), scope, refers_to });
    }
    out
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
) -> (Sheet, Option<AutoFilter>, Vec<Hyperlink>, Vec<Comment>, Vec<Table>) {
    let mut sheet = Sheet::default();
    let Some(root) = read_xml(package, part).ok().flatten() else {
        warnings.push(format!("the part {part} could not be read; the sheet is empty"));
        return (sheet, None, Vec::new(), Vec::new(), Vec::new());
    };
    // The **filter** of the sheet: the rows it hides.
    let filter = root.child("autoFilter").map(read_filter);
    // The **links** of the sheet: the places its cells go to. A link
    // that names a **relationship** goes to what that relationship
    // points at, and the relationships of a sheet are in the sheet's
    // own part.
    let sheet_rels = rels_of(package, part).unwrap_or_default();
    let links = read_links(&root, &sheet_rels);
    // The **comments** of the sheet: they are in a part of their own
    // that the sheet is related to, and not in the sheet itself.
    let comments = read_comments(package, &sheet_rels);
    // The **tables** of the sheet: they are in parts of their own
    // that the sheet names by a relationship.
    let tables = read_tables(package, &root, &sheet_rels);
    // The **protection** of the sheet: what may still be done to it.
    // A file protects a sheet with an element that says it is
    // protected, and the flags of that element say what is still
    // allowed.
    sheet.protection = read_sheet_protection(&root);
    // The **columns** of the sheet: their width and whether they are
    // hidden. A file says a span of columns at once, so one span of
    // ten columns is ten columns here.
    sheet.columns = read_columns(&root);
    // The **rows** of the sheet: their height and whether they are
    // hidden. A file says a row of its own, and this reader reads the
    // rows as it reads the cells.
    let mut rows: Vec<Row> = Vec::new();
    let Some(data) = root.child("sheetData") else {
        return (sheet, filter, links, comments, tables);
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
        // The **row** itself: its height and whether it is hidden. A
        // row that says nothing of itself is not a row here: the
        // height of a row that says nothing is the sheet's own.
        let height = row.attr_f64("ht").filter(|height| *height > 0.0);
        let hidden = row.flag("hidden", false);
        if height.is_some() || hidden {
            rows.push(Row { row: row_index + 1, height, hidden });
        }

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
            let style = cell.attr_u32("s");
            let format = styles.format(style).map(|code| code.to_string());
            // The **font**, the **fill** and the **alignment** of the
            // cell: only when the cell names a style of its own. A
            // cell that names no style has the sheet's own face, and
            // the sheet's own face is not the cell's.
            let (font, fill, align, borders, protection) = if style.is_some() {
                (
                    styles.font(style).cloned(),
                    styles.fill(style).cloned(),
                    styles.align(style).cloned(),
                    styles.borders(style).cloned(),
                    styles.protection(style).cloned(),
                )
            } else {
                (None, None, None, None, None)
            };
            sheet.cells.push(Cell {
                column,
                row: row_index as u32,
                value,
                formula,
                format,
                font,
                fill,
                align,
                borders,
                protection,
            });
        }
    }

    if dropped > 0 {
        warnings.push(format!(
            "the sheet holds {dropped} cells or rows outside the {MAX_ROWS} × {MAX_COLS} grid; they were dropped"
        ));
    }
    sheet.rows = rows;
    (sheet, filter, links, comments, tables)
}

/// Read the **columns** of a sheet: their width and whether they are
/// hidden.
///
/// A file says a **span** of columns at once, and a span of ten
/// columns is ten columns here. A span that is out of the grid is
/// clipped to it, as the tree's reader clips it.
fn read_columns(root: &craftkit_xml::El) -> Vec<Column> {
    let mut columns = Vec::new();
    let Some(cols) = root.child("cols") else { return columns };
    for col in cols.kids("col") {
        let (Some(min), Some(max)) = (col.attr_u32("min"), col.attr_u32("max")) else { continue };
        if min == 0 || max < min {
            continue;
        }
        let max = max.min(MAX_COLS as u32);
        if u64::from(min) > MAX_COLS {
            continue;
        }
        let width = col.attr_f64("width").filter(|width| *width > 0.0);
        let hidden = col.flag("hidden", false);
        if width.is_none() && !hidden {
            continue;
        }
        for index in (min - 1)..max {
            columns.push(Column { column: index, width, hidden });
        }
    }
    columns
}

/// Read the **tables** of a sheet.
///
/// The tables are in parts of their own, and the sheet names them in
/// its `tableParts`. A table has a name, a range, a header row, a
/// total row, and a **way** each column is totalled in.
fn read_tables(
    package: &mut craftkit_zip::Package<'_>,
    root: &craftkit_xml::El,
    sheet_rels: &[Rel],
) -> Vec<Table> {
    let Some(parts) = root.child("tableParts") else { return Vec::new() };
    let mut out = Vec::new();
    for part in parts.kids("tablePart") {
        let Some(relation) = part.attr("id").and_then(|id| sheet_rels.iter().find(|rel| rel.id == id)) else {
            continue;
        };
        let Some(table) = read_xml(package, &relation.target).ok().flatten() else { continue };
        let Some(range) = table.attr("ref") else { continue };
        let name = table.attr("displayName").or_else(|| table.attr("name")).unwrap_or("Table").to_string();
        let totals = table.attr_u32("totalsRowCount").unwrap_or(0) > 0;
        // The **way** each column is totalled in, and the way of the
        // table itself: the first column that has a way of its own
        // gives the table its way, and every column that has one is
        // kept.
        let mut way = String::new();
        let mut ways: Vec<(u32, String)> = Vec::new();
        let mut label = None;
        if let Some(columns) = table.child("tableColumns") {
            for (index, column) in columns.kids("tableColumn").enumerate() {
                let totals_way = totals_way(column.attr("totalsRowFunction"));
                let Some(totals_way) = totals_way else { continue };
                if totals_way != "none" {
                    ways.push((index as u32, totals_way.clone()));
                    if way.is_empty() {
                        way = totals_way;
                    }
                }
                // The **label** of the total row is the text in its
                // first cell: the first column that has a label
                // gives it.
                if label.is_none()
                    && let Some(text) = column.attr("totalsRowLabel")
                {
                    label = Some(text.to_string());
                }
            }
        }
        if way.is_empty() {
            way = "sum".to_string();
        }
        out.push(Table { name, range: range.to_string(), totals, way, ways, label });
    }
    out
}

/// The **way** a column is totalled in, as this tree writes it.
fn totals_way(function: Option<&str>) -> Option<String> {
    let way = match function.unwrap_or("none") {
        "average" => "average",
        "count" => "count",
        "countNums" => "count-numbers",
        "max" => "max",
        "min" => "min",
        "sum" => "sum",
        "stdDev" => "standard-deviation",
        "var" => "variance",
        "none" => "none",
        // A way this tree does not have is not a way: the column is
        // left out rather than given a name that means nothing here.
        _ => return None,
    };
    Some(way.to_string())
}

/// Read the **protection** of a sheet.
///
/// A file protects a sheet with an element that says it is protected,
/// and the flags of that element say what is **not** allowed: a flag
/// that is there and true forbids the thing, and a flag that is not
/// there forbids nothing. This tree's protection is the other way
/// round — it says what **is** allowed — so the flags are turned
/// about here.
fn read_sheet_protection(root: &craftkit_xml::El) -> Option<SheetProtection> {
    let element = root.child("sheetProtection")?;
    if !element.flag("sheet", false) {
        return None;
    }
    Some(SheetProtection {
        sort: !element.flag("sort", true),
        filter: !element.flag("autoFilter", true),
        format: !element.flag("formatCells", true),
    })
}

/// Read the **comments** of a sheet.
///
/// The comments are in a part of their own, and the sheet is related
/// to that part. A comment has an author and a text, and the text of a
/// note written by a spreadsheet program often begins with the author
/// and a colon — and that beginning is dropped, because the author is
/// kept beside it.
fn read_comments(package: &mut craftkit_zip::Package<'_>, sheet_rels: &[Rel]) -> Vec<Comment> {
    let mut out = Vec::new();
    for relation in sheet_rels.iter().filter(|relation| relation.kind == "comments") {
        let Some(root) = read_xml(package, &relation.target).ok().flatten() else { continue };
        let authors: Vec<String> = root
            .child("authors")
            .map(|authors| authors.kids("author").map(|author| author.text.clone()).collect())
            .unwrap_or_default();
        let Some(list) = root.child("commentList") else { continue };
        for comment in list.kids("comment") {
            let Some(address) = comment.attr("ref") else { continue };
            let Some((column, row)) = address_of(address.trim()) else { continue };
            let author = comment
                .attr_u32("authorId")
                .and_then(|index| authors.get(index as usize))
                .cloned()
                .unwrap_or_default();
            let mut text = comment.child("text").map(rich_text).unwrap_or_default();
            if !author.is_empty()
                && let Some(rest) = text.strip_prefix(&format!("{author}:"))
            {
                text = rest.strip_prefix('\n').unwrap_or(rest).to_string();
            }
            out.push(Comment { column, row: row as u32, author, text });
        }
    }
    out
}

/// Read the **links** of a sheet: the places its cells go to.
///
/// A link names a **range** of cells, and it may name a place inside
/// the workbook or a place outside it. A link with no place at all is
/// not a link, and it is left out.
fn read_links(root: &craftkit_xml::El, sheet_rels: &[Rel]) -> Vec<Hyperlink> {
    let Some(list) = root.child("hyperlinks") else { return Vec::new() };
    let mut out = Vec::new();
    for link in list.kids("hyperlink") {
        let Some(range) = link.attr("ref") else { continue };
        let Some((first, last)) = range_of(range) else { continue };
        // A link that names a **relationship** goes outside the
        // workbook; a link that names a **location** goes inside it.
        let relation = link.attr("id").and_then(|id| sheet_rels.iter().find(|rel| rel.id == id));
        let outside = relation.is_some();
        let mut target = relation.map(|rel| rel.target.clone()).unwrap_or_default();
        if let Some(location) = link.attr("location").filter(|location| !location.is_empty()) {
            if target.is_empty() {
                target = location.trim_start_matches('#').to_string();
            } else {
                target = format!("{target}#{location}");
            }
        }
        if target.is_empty() {
            continue;
        }
        out.push(Hyperlink {
            column: first.0,
            row: first.1,
            last_column: last.0,
            last_row: last.1,
            target,
            tooltip: link.attr("tooltip").map(|tooltip| tooltip.to_string()),
            external: outside,
        });
    }
    out
}

/// The **first** and the **last** cell of a range, as the columns and
/// the rows they are.
fn range_of(range: &str) -> Option<((u32, u64), (u32, u64))> {
    let mut parts = range.split(':');
    let first = address_of(parts.next()?.trim())?;
    let last = match parts.next() {
        Some(text) => address_of(text.trim())?,
        None => first,
    };
    Some((first, last))
}

/// Read the **filter** of a sheet: the range it is over, and the
/// criterion of each of its columns.
///
/// A column of the filter may hold a criterion this reader does not
/// know — a colour, an icon — and such a column is left out: a filter
/// with a column missing is a filter that hides fewer rows, and a
/// filter that hides rows for a reason nobody can see is worse.
fn read_filter(element: &craftkit_xml::El) -> AutoFilter {
    let mut criteria = Vec::new();
    for column in element.kids("filterColumn") {
        let Some(offset) = column.attr_u32("colId") else { continue };
        let criterion = if let Some(filters) = column.child("filters") {
            FilterCriterion::Values {
                values: filters
                    .kids("filter")
                    .filter_map(|filter| filter.attr("val"))
                    .map(|value| value.to_string())
                    .collect(),
                blanks: filters.flag("blank", false),
            }
        } else if let Some(custom) = column.child("customFilters") {
            let mut comparisons = custom.kids("customFilter").map(|filter| {
                (filter_op(filter.attr("operator")).to_string(), filter.attr("val").unwrap_or("").to_string())
            });
            let Some(first) = comparisons.next() else { continue };
            FilterCriterion::Custom { first, second: comparisons.next(), and: custom.flag("and", false) }
        } else if let Some(top) = column.child("top10") {
            FilterCriterion::Top10 {
                bottom: !top.flag("top", true),
                percent: top.flag("percent", false),
                count: top.attr_f64("val").unwrap_or(10.0).clamp(0.0, 1e9) as u32,
            }
        } else if let Some(dynamic) = column.child("dynamicFilter") {
            match dynamic.attr("type") {
                Some("aboveAverage") => FilterCriterion::AboveAverage(true),
                Some("belowAverage") => FilterCriterion::AboveAverage(false),
                _ => continue,
            }
        } else {
            continue;
        };
        criteria.push((offset, criterion));
    }
    AutoFilter { range: element.attr("ref").map(|range| range.to_string()), criteria }
}

/// The **operation** a filter compares with.
fn filter_op(operation: Option<&str>) -> &'static str {
    match operation.unwrap_or("equal") {
        "lessThan" => "<",
        "lessThanOrEqual" => "<=",
        "notEqual" => "<>",
        "greaterThanOrEqual" => ">=",
        "greaterThan" => ">",
        _ => "=",
    }
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
