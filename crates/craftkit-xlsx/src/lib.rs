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
//! - It does not **shift** a formula: a cell that shares a formula
//!   with other cells is given the master's text and the distance
//!   from the master, and the battery that owns the spreadsheet model
//!   moves the text — the reader knows the file's words, and the
//!   moving is the model's.
//! - It reads the **drawings** of a sheet: the charts among them, and
//!   the **pictures** — the cell a picture is drawn in, the name the
//!   file gives it and its bytes. A picture that is in no cell is
//!   counted, and the reader says how many it left out.
//! - It does not read the **shapes** of a sheet: a shape is a drawing
//!   that is not a picture, and the tree has none.
//! - It does not read a **theme** of its own: a file that names a
//!   colour by the theme is read against the theme the file holds, and
//!   a file that holds no theme has no themed colour to read.
//! - It reads the **conditional formats** it has: a comparison, a
//!   formula, the words of a cell, the empty cells, the errors, the
//!   colour scale, the top ten, the average, the duplicates, the data
//!   bar and the icon set. The kind it does not have — the rule of the
//!   time period — is counted, and it says how many it left out.
//! - It reads the **tables** it can hold: a table has one header row
//!   here, and its fields are the text of that row, and so a table
//!   whose header row is off is not read. A table's **filter** and a
//!   column's **custom total** are the sheet's and the tree's own, and
//!   they are left out — and every one of them is said.
//! - It reads a workbook that is **encrypted**: such a file is a
//!   compound file and not a package, and the password opens it. The
//!   **container** is read (`cfb`), and the ciphers of [MS-OFFCRYPTO]
//!   open what is inside it: the **agile** one (`crypto`), and the
//!   **old** one of Excel 2007 (`standard`).

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod calculation;
mod featurebags;
mod phonetic;
mod rels;
mod styles;
mod theme;

pub mod cfb;
pub mod crypto;
pub mod rc4;
pub mod standard;

pub use calculation::{
    Calculation, DEFAULT_CHANGE, DEFAULT_ITERATIONS, MAX_ITERATIONS, Mode as CalculationMode,
    read as read_calculation,
};
pub use featurebags::{MAX_BAGS, checkbox_complements, xf_is_checkbox};
pub use phonetic::{Phonetic, PhoneticProps, PhoneticRun};
pub use rels::{Rel, rels_of, resolve, split_dir};
pub use styles::{
    Alignment, Border, Borders, Fill, Font, Protection, Styles, Underline, Vert, read_styles,
    read_styles_with,
};
pub use theme::{SLOTS, Theme, apply_tint, read_theme};

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

/// The most **levels** an outline may have: a file may say more, and
/// the tree's own outline is seven deep.
pub const MAX_OUTLINE: u32 = 7;

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
    /// The file is a **compound file**: it holds a workbook that is
    /// encrypted, or a workbook of the old format.
    Compound(String),
    /// The file is **encrypted**, and the password is not the password
    /// of the file — or the cipher could not open it.
    Encrypted(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Malformed(message) => write!(f, "malformed workbook: {message}"),
            Error::TooLarge(message) => write!(f, "limit exceeded: {message}"),
            Error::Package(message) => write!(f, "the package could not be read: {message}"),
            Error::Xml(message) => write!(f, "a part of the workbook is not readable xml: {message}"),
            Error::Compound(message) => write!(f, "the file is a compound file: {message}"),
            Error::Encrypted(message) => write!(f, "the workbook is encrypted: {message}"),
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
    /// The **kind** of the formula, when the file names one that is not
    /// an ordinary formula.
    ///
    /// A file says of a formula that it is an **array** — it fills a
    /// range of cells — or that it is a what-if **data table**. The
    /// text of such a formula is not the text of a formula of a cell.
    pub formula_kind: Option<FormulaKind>,
    /// The **range** of an array formula, when the file names one.
    pub array_range: Option<String>,
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
    /// The **shared formula** of the cell, when the cell shares a
    /// formula with other cells.
    ///
    /// A shared formula is written once in the cell at the top left of
    /// its group and the other cells of the group name it. The text
    /// here is the **master's** text, and the deltas say how far this
    /// cell is from the master.
    pub shared: Option<Shared>,
    /// Whether the cell is a **checkbox**: a cell control the file
    /// holds beside the cells rather than in them.
    ///
    /// The reader says that the cell is a checkbox; what a checkbox is
    /// in the tree that reads the file is that tree's own business.
    pub checkbox: bool,
    /// The **reading** of the text of the cell: the phonetic guide the
    /// file holds beside it.
    ///
    /// A reading is a fact of a text and not of a cell, but the file
    /// holds it beside the text and the reader reports it with the
    /// cell that carries it.
    pub phonetic: Option<Phonetic>,
}

/// The **kind** of a formula: what the file says the formula is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaKind {
    /// A formula that **fills a range of cells**: an array.
    Array,
    /// A what-if **data table**: a range of cells filled from a
    /// formula of the table.
    DataTable,
}

/// A **shared formula**: a group of cells that share one formula.
#[derive(Debug, Clone, PartialEq)]
pub struct Shared {
    /// The **text** of the master: the formula as the file writes it.
    pub text: String,
    /// How many rows this cell is **below** the master.
    pub row_delta: i64,
    /// How many columns this cell is to the **right** of the master.
    pub column_delta: i64,
}

/// A **sheet** of a workbook: a name and its cells.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Sheet {
    /// The name of the sheet.
    pub name: String,
    /// The **state** of the sheet: `hidden` or `veryHidden`, when the
    /// file hides it.
    ///
    /// A sheet that is not hidden says nothing, and that is not
    /// carried.
    pub state: Option<String>,
    /// The **colour** of the sheet's tab, when the file gives it one.
    pub tab_colour: Option<String>,
    /// The cells, in the order the file holds them.
    pub cells: Vec<Cell>,
    /// The **rows** of the sheet that say something of themselves: their
    /// height and whether they are hidden.
    pub rows: Vec<Row>,
    /// The **columns** of the sheet that say something of themselves:
    /// their width and whether they are hidden.
    pub columns: Vec<Column>,
    /// The **default width** of the sheet's columns, in characters,
    /// when the file says one.
    ///
    /// A file that says nothing has no default width: the width of a
    /// column that has no width of its own is then the tree's own, and
    /// that is not a fact of the file.
    pub default_width: Option<f64>,
    /// The **default height** of the sheet's rows, in points, when the
    /// file says one.
    ///
    /// A file that says nothing has no default height, for the same
    /// reason a file that says nothing has no default width.
    pub default_height: Option<f64>,
    /// The **merges** of the sheet: the ranges that are **one cell**.
    ///
    /// The ranges are the file's own words — `A1:B2` — and a range of
    /// one cell is not a merge and is not here.
    pub merges: Vec<String>,
    /// The **view** of the sheet: what is drawn of it, and what is not.
    ///
    /// A sheet with no view of its own has the default one, and the
    /// default one is not carried.
    pub view: Option<SheetView>,
    /// The **protection** of the sheet, when the sheet is protected.
    ///
    /// A sheet that is not protected has none: a sheet that is not
    /// protected says nothing about what may be done to it.
    pub protection: Option<SheetProtection>,
    /// The **conditional formats** of the sheet: the rules that are
    /// about a range of cells.
    pub conditionals: Vec<Conditional>,
    /// The **validations** of the sheet: the rules on what a cell may
    /// hold.
    pub validations: Vec<Validation>,
    /// The **area** the sheet prints, when the file names one.
    ///
    /// The area is the file's own words — `A1:C10` — without the sheet
    /// it belongs to and without the `$` of an absolute address.
    pub print_area: Option<String>,
    /// The **titles** the sheet prints on every page, when the file
    /// names them.
    ///
    /// The titles are the file's own text: the rows and the columns
    /// that repeat are said in the file's own way.
    pub print_titles: Option<String>,
    /// Whether the **summary row** of a group of rows sits **below**
    /// the group, when the file says.
    ///
    /// A file that says nothing has the summary below, and that is not
    /// carried: what a file does not say is not a fact of the file.
    pub summary_below: Option<bool>,
    /// Whether the **summary column** of a group of columns sits to
    /// the **right** of the group, when the file says.
    pub summary_right: Option<bool>,
    /// The **page setup** of the sheet: how it prints.
    ///
    /// The numbers are the **file's** own: the margins are in inches,
    /// the paper is the file's own number, and the scale is a percent.
    pub print: Option<Print>,
    /// The **header** and the **footer** of the pages of the sheet,
    /// when the file says them.
    pub header_footer: Option<HeaderFooter>,
    /// The **options** of the print: the marks it prints and where the
    /// sheet sits on the page.
    pub print_options: Option<PrintOptions>,
    /// The **edit ranges** of the sheet: the ranges that stay editable
    /// while the sheet is protected.
    pub edit_ranges: Vec<EditRange>,
    /// The **breaks** of the printing: the lines a page begins at, by
    /// the file's own numbering.
    ///
    /// The rows and the columns are two lists, and a break of the
    /// printing is a row or a column and not both.
    pub breaks: Breaks,
    /// The **sparklines** of the sheet: the little drawings in its
    /// cells.
    pub sparklines: Vec<Sparkline>,
    /// The **pivot tables** of the sheet: the pivots it holds.
    pub pivots: Vec<PivotTable>,
}

/// A **sparkline** of a sheet: a little drawing in a cell, taken from
/// a range of cells.
///
/// A sparkline lives in the **extension list** of the sheet, and the
/// names here are the file's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sparkline {
    /// The **cell** the sparkline is drawn in: the file's own address.
    pub cell: String,
    /// The **source**: the range it reads, as the file writes it.
    pub source: String,
    /// The **kind**: `line`, `column` or `stacked`.
    pub kind: String,
    /// The **colour** of the line, when the file names one.
    pub colour: Option<String>,
    /// Whether the **markers** are drawn.
    pub markers: bool,
}

/// A **pivot table** of a sheet: the cells it was made from, and the
/// fields it gathers them by.
///
/// The names of the fields are the names of the columns of the source,
/// as the file writes them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PivotTable {
    /// The **name** of the pivot, when the file gives it one.
    pub name: Option<String>,
    /// The **source**: the range the pivot reads, and the sheet it is
    /// in, as the file writes it — `Sheet1!$A$1:$C$10`.
    pub source: String,
    /// The **row fields**: the fields the rows are gathered by.
    pub row_fields: Vec<String>,
    /// The **column fields**: the fields the groups are spread over.
    pub column_fields: Vec<String>,
    /// The **value**: the field that is put together, and the way of
    /// putting it together.
    pub value: Option<(String, String)>,
}

/// The **parts** of a sheet: the things it holds in parts of their
/// own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Parts {
    /// The **filter** of the sheet: the rows it hides.
    pub filter: Option<AutoFilter>,
    /// The **links** of the sheet: the places its cells go to.
    pub links: Vec<Hyperlink>,
    /// The **comments** of the sheet: the notes its cells carry.
    pub comments: Vec<Comment>,
    /// The **tables** of the sheet.
    pub tables: Vec<Table>,
    /// The **charts** of the sheet.
    pub charts: Vec<Chart>,
    /// The **pictures** of the sheet.
    pub pictures: Vec<Picture>,
}

/// A **picture** of a sheet: an image the sheet shows, and the cell it
/// is drawn in.
///
/// The picture is in the file as **bytes**, and the bytes are given
/// back as they are: what they are is the business of the tree that
/// reads the file.
#[derive(Debug, Clone, PartialEq)]
pub struct Picture {
    /// The cell the picture is drawn in: `B3`.
    pub cell: String,
    /// The **name** the file gives the picture.
    pub name: String,
    /// The **alt text** of the picture: what is said when it cannot be
    /// shown.
    pub alt: String,
    /// The **format** of the bytes: the word the name of the file gives
    /// them — `png` or `jpeg`.
    pub format: String,
    /// The **bytes** of the picture.
    pub bytes: Vec<u8>,
}

/// A **chart** of a sheet: a drawing of some cells of it.
///
/// The names are the **file's** own, and the ranges are the file's
/// own words — `Sheet1!$B$2:$B$10` — because the file may read the
/// cells of another sheet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chart {
    /// The **kind** of the chart: `column`, `bar`, `line`, `pie`,
    /// `area`, `scatter`, `doughnut`, `bubble`, `radar` or `stock`.
    pub kind: String,
    /// The **stacking** of the chart: `stacked` or `percentStacked`,
    /// when the file says the chart is stacked.
    ///
    /// A chart that is not stacked has none: the file may say
    /// `clustered`, and that is the way of saying nothing.
    pub stacking: Option<String>,
    /// The **smooth line** of the chart.
    ///
    /// The file says of each of its series whether the line is
    /// smooth, and the tree's chart has one line for all of them:
    /// `Some(true)` when every series says so, `Some(false)` when
    /// none of them does, and **none** when the series do not all say
    /// the same — a chart the tree cannot draw as it is.
    pub smooth: Option<bool>,
    /// The **title** of the chart.
    pub title: Option<String>,
    /// The **legend**: where it is — `none`, `bottom`, `top`, `left`
    /// or `right`.
    pub legend: Option<String>,
    /// Whether the **data labels** are shown.
    pub data_labels: bool,
    /// Whether the **gridlines** are shown.
    pub gridlines: bool,
    /// The **title** of the axis of the categories.
    pub x_title: Option<String>,
    /// The **title** of the axis of the values.
    pub y_title: Option<String>,
    /// The **categories**: the range the chart names its categories by.
    pub categories: Option<String>,
    /// The **series**: the ranges the chart names its series by.
    pub series: Vec<String>,
    /// The **colours** of the series, as `#rrggbb`, in the order of the
    /// series.
    ///
    /// A series with no colour of its own has no colour here, and the
    /// list is shorter than the series.
    pub colours: Vec<String>,
}

/// Read the **charts** of a sheet.
///
/// A sheet names a **drawing** part, and the drawing holds the objects
/// the sheet shows: the charts among them, and the pictures. A chart is
/// in a part of its own, and the drawing names it; a picture is in the
/// drawing itself, and the drawing names its bytes.
fn read_drawings(
    package: &mut craftkit_zip::Package<'_>,
    root: &craftkit_xml::El,
    sheet_rels: &[Rel],
    theme: &Theme,
    warnings: &mut Vec<String>,
) -> (Vec<Chart>, Vec<Picture>) {
    let mut out = Vec::new();
    let mut pictures = Vec::new();
    let Some(drawing) = root.child("drawing") else { return (out, pictures) };
    let Some(id) = drawing.attr("id") else { return (out, pictures) };
    let Some(relation) = sheet_rels.iter().find(|relation| relation.id == id) else {
        return (out, pictures);
    };
    let Some(part) = read_xml(package, &relation.target).ok().flatten() else {
        return (out, pictures);
    };
    let drawing_rels = rels_of(package, &relation.target).unwrap_or_default();
    // The **objects** of the drawing: the frames that hold a chart.
    let mut frames = Vec::new();
    collect(&part, "graphicFrame", &mut frames);
    for frame in frames {
        let Some(id) = frame
            .child("graphic")
            .and_then(|graphic| graphic.child("graphicData"))
            .and_then(|data| data.child("chart"))
            .and_then(|chart| chart.attr("id"))
        else {
            continue;
        };
        let Some(relation) = drawing_rels.iter().find(|relation| relation.id == id) else { continue };
        let Some(root) = read_xml(package, &relation.target).ok().flatten() else { continue };
        let Some(chart) = read_chart(&root, theme) else { continue };
        if chart.smooth.is_none() {
            warnings.push(
                "a chart of the sheet has series that do not all say the same about the smooth line"
                    .to_string(),
            );
        }
        out.push(chart);
    }
    // The **pictures** of the drawing.
    let (read, unread) = read_pictures(package, &part, &drawing_rels);
    if unread > 0 {
        warnings.push(format!(
            "{unread} picture(s) of the sheet were not read: they are not in a cell, or they are too big"
        ));
    }
    pictures.extend(read);
    (out, pictures)
}

/// The **most** bytes of a picture that are read.
const MAX_PICTURE_BYTES: u64 = 40 * 1024 * 1024;

/// The **most** pictures a sheet may have.
const MAX_PICTURES: usize = 1_000;

/// Read the **pictures** of a drawing, and how many were not read.
///
/// A picture is in an **anchor**: the place on the sheet it is drawn in.
/// The anchor says the cell the picture starts at, and the picture names
/// the bytes it shows by a **relationship** of the drawing.
///
/// A picture that is in no anchor, or in one that does not say which cell
/// it is in, is not in a cell — and the tree's picture is the picture of
/// a cell. Such a picture is counted, and not read.
fn read_pictures(
    package: &mut craftkit_zip::Package<'_>,
    drawing: &craftkit_xml::El,
    drawing_rels: &[Rel],
) -> (Vec<Picture>, u64) {
    // The **pictures** of the drawing, and the **anchors** they are in.
    let mut all = Vec::new();
    collect(drawing, "pic", &mut all);
    let mut anchors = Vec::new();
    collect(drawing, "twoCellAnchor", &mut anchors);
    collect(drawing, "oneCellAnchor", &mut anchors);
    let mut out = Vec::new();
    for anchor in anchors {
        let Some(cell) = cell_of_anchor(anchor) else { continue };
        let mut framed = Vec::new();
        collect(anchor, "pic", &mut framed);
        for pic in framed {
            if out.len() >= MAX_PICTURES {
                break;
            }
            let Some(picture) = picture_of(package, pic, drawing_rels, &cell) else { continue };
            out.push(picture);
        }
    }
    let unread = (all.len() as u64).saturating_sub(out.len() as u64);
    (out, unread)
}

/// The **cell** an anchor draws its object in: the cell the object
/// starts at.
///
/// The anchor counts from zero, and the cell of the tree counts from
/// one.
fn cell_of_anchor(anchor: &craftkit_xml::El) -> Option<String> {
    let from = anchor.child("from")?;
    let column = from.child("col")?.text.trim().parse::<u32>().ok()?;
    let row = from.child("row")?.text.trim().parse::<u32>().ok()?;
    if column > 16_383 || row > 1_048_575 {
        return None;
    }
    Some(format!("{}{}", column_text(column), row + 1))
}

/// Read **one** picture of a drawing, when the tree can have it.
fn picture_of(
    package: &mut craftkit_zip::Package<'_>,
    pic: &craftkit_xml::El,
    drawing_rels: &[Rel],
    cell: &str,
) -> Option<Picture> {
    let properties = pic.child("nvPicPr").and_then(|nv| nv.child("cNvPr"));
    let name = properties.and_then(|property| property.attr("name")).unwrap_or("").to_string();
    let alt = properties.and_then(|property| property.attr("descr")).unwrap_or("").to_string();
    let id = pic.child("blipFill").and_then(|fill| fill.child("blip")).and_then(|blip| blip.attr("embed"))?;
    let relation = drawing_rels.iter().find(|relation| relation.id == id)?;
    let bytes = package.read_limited(&relation.target, MAX_PICTURE_BYTES).ok().flatten()?;
    Some(Picture { cell: cell.to_string(), name, alt, format: format_of_picture(&relation.target), bytes })
}

/// The **format** of a picture: the word the name of the file gives its
/// bytes.
fn format_of_picture(part: &str) -> String {
    let name = part.rsplit('/').next().unwrap_or_default().to_ascii_lowercase();
    let extension = name.rsplit('.').next().unwrap_or_default();
    match extension {
        "png" => "png",
        "jpg" | "jpeg" => "jpeg",
        "gif" => "gif",
        "bmp" => "bmp",
        "tif" | "tiff" => "tiff",
        "emf" => "emf",
        "wmf" => "wmf",
        "svg" => "svg",
        _ => "",
    }
    .to_string()
}

/// The column a name of letters says, counted from zero, as letters.
fn column_text(column: u32) -> String {
    let mut name = String::new();
    let mut column = column;
    loop {
        name.insert(0, (b'A' + (column % 26) as u8) as char);
        if column < 26 {
            break;
        }
        column = column / 26 - 1;
    }
    name
}

/// The **elements** of a part with a name, wherever they are.
fn collect<'a>(element: &'a craftkit_xml::El, name: &str, out: &mut Vec<&'a craftkit_xml::El>) {
    for child in &element.children {
        if child.name == name {
            out.push(child);
        }
        collect(child, name, out);
    }
}

/// Read the **pivot tables** of a sheet.
///
/// A pivot table of a file is in a part of its own, and it names a
/// **cache** of its own: the cache says where the cells come from and
/// what the fields are called.
fn read_pivots(
    package: &mut craftkit_zip::Package<'_>,
    sheet_rels: &[Rel],
    warnings: &mut Vec<String>,
) -> Vec<PivotTable> {
    let mut out = Vec::new();
    for relation in sheet_rels.iter().filter(|relation| relation.kind == "pivotTable") {
        let Some(root) = read_xml(package, &relation.target).ok().flatten() else { continue };
        // The **cache**: the part that says where the cells come from.
        let cache_rels = rels_of(package, &relation.target).unwrap_or_default();
        let Some(cache_relation) = cache_rels.iter().find(|relation| relation.kind == "pivotCacheDefinition")
        else {
            warnings.push("a pivot table has no cache of its own; it was not read".to_string());
            continue;
        };
        let Some(cache) = read_xml(package, &cache_relation.target).ok().flatten() else { continue };
        let Some(worksheet) = cache.child("cacheSource").and_then(|source| source.child("worksheetSource"))
        else {
            warnings.push("the cache of a pivot table has no worksheet source; it was not read".to_string());
            continue;
        };
        let range = worksheet.attr("ref").unwrap_or("").replace('$', "");
        if range.is_empty() {
            warnings.push("the source of a pivot table is not a range; it was not read".to_string());
            continue;
        }
        let sheet = worksheet.attr("sheet").unwrap_or("");
        let source = if sheet.is_empty() { range } else { format!("{sheet}!{range}") };
        // The **fields**: the names of the columns of the source, in
        // the order of the cache.
        let names: Vec<String> = cache
            .child("cacheFields")
            .map(|fields| {
                fields.kids("cacheField").map(|field| field.attr("name").unwrap_or("").to_string()).collect()
            })
            .unwrap_or_default();
        let name_of = |index: usize| -> String { names.get(index).cloned().unwrap_or_default() };
        // The **axes**: the fields the rows are gathered by and the
        // fields the groups are spread over.
        let indexes = |list: Option<&craftkit_xml::El>| -> Vec<usize> {
            list.map(|list| {
                list.children
                    .iter()
                    .filter_map(|field| field.attr_u32("x"))
                    .map(|index| index as usize)
                    .collect()
            })
            .unwrap_or_default()
        };
        let row_fields: Vec<String> =
            indexes(root.child("rowFields")).iter().map(|index| name_of(*index)).collect();
        let column_fields: Vec<String> =
            indexes(root.child("colFields")).iter().map(|index| name_of(*index)).collect();
        // The **value**: the field that is put together, and the way
        // of putting it together.
        let value = root.child("dataFields").and_then(|fields| fields.child("dataField")).map(|field| {
            let index = field.attr_u32("fld").unwrap_or(0) as usize;
            (name_of(index), field.attr("subtotal").unwrap_or("sum").to_string())
        });
        let name = root.attr("name").map(|name| name.to_string());
        out.push(PivotTable { name, source, row_fields, column_fields, value });
    }
    out
}

/// Read **one** chart of a file.
///
/// The chart is a **group** of series — a bar chart, a line chart, a
/// pie chart — and the group says of itself what kind of drawing it is.
fn read_chart(root: &craftkit_xml::El, theme: &Theme) -> Option<Chart> {
    let chart = root.child("chart")?;
    let plot = chart.child("plotArea")?;
    // The **group**: the kind of drawing and its series.
    let mut group: Option<(&craftkit_xml::El, &str)> = None;
    for name in [
        "barChart",
        "lineChart",
        "pieChart",
        "doughnutChart",
        "areaChart",
        "scatterChart",
        "bubbleChart",
        "radarChart",
        "stockChart",
    ] {
        if let Some(element) = plot.child(name) {
            group = Some((element, name));
            break;
        }
    }
    let (element, name) = group?;
    // The **stacking**: the way the series of the chart are laid on
    // one another. A chart that is not stacked says nothing, or says
    // `clustered`, and that is the way of saying nothing.
    let stacking = match element.child_val("grouping") {
        Some("stacked") => Some("stacked".to_string()),
        Some("percentStacked") => Some("percentStacked".to_string()),
        _ => None,
    };
    let kind = match name {
        "barChart" => {
            if element.child_val("barDir") == Some("bar") {
                "bar"
            } else {
                "column"
            }
        }
        "lineChart" => "line",
        "pieChart" => "pie",
        "doughnutChart" => "doughnut",
        "areaChart" => "area",
        "scatterChart" => "scatter",
        "bubbleChart" => "bubble",
        "radarChart" => "radar",
        _ => "stock",
    };
    // The **series**: each of them names its categories and its
    // values.
    let mut categories: Option<String> = None;
    let mut series: Vec<String> = Vec::new();
    // The **colours** of the series: the colour each one is drawn in.
    let mut colours: Vec<String> = Vec::new();
    for serie in element.kids("ser") {
        if categories.is_none() {
            categories = data_ref(serie.child("cat")).or_else(|| data_ref(serie.child("xVal")));
        }
        let values = data_ref(serie.child("val")).or_else(|| data_ref(serie.child("yVal")));
        if let Some(values) = values {
            series.push(values);
        }
        if let Some(colour) = colour_of_series(serie, theme) {
            colours.push(colour);
        }
    }
    // The **title**: the text of it, wherever it is written.
    let title = chart.child("title").map(all_text).filter(|text| !text.is_empty());
    // The **legend**: where it is.
    let legend = match chart.child("legend") {
        None => Some("none".to_string()),
        Some(legend) => Some(
            match legend.child_val("legendPos").unwrap_or("r") {
                "b" => "bottom",
                "t" => "top",
                "l" => "left",
                _ => "right",
            }
            .to_string(),
        ),
    };
    // The **axes**: the axis of the categories and the axis of the
    // values.
    let categories_axis = plot.child("catAx").or_else(|| plot.child("dateAx"));
    let values_axis = plot.child("valAx");
    let x_title =
        categories_axis.and_then(|axis| axis.child("title")).map(all_text).filter(|text| !text.is_empty());
    let y_title =
        values_axis.and_then(|axis| axis.child("title")).map(all_text).filter(|text| !text.is_empty());
    // The **smooth line**: the file says of each of its series
    // whether the line is smooth.
    let smooths: Vec<bool> = element
        .kids("ser")
        .map(|serie| matches!(serie.child_val("smooth"), Some("1") | Some("true")))
        .collect();
    let smooth = if smooths.is_empty() {
        Some(false)
    } else if smooths.iter().all(|smooth| *smooth) {
        Some(true)
    } else if smooths.iter().any(|smooth| *smooth) {
        None
    } else {
        Some(false)
    };
    // The **labels** of the data, and the **gridlines** of the values.
    let data_labels = element.child("dLbls").is_some_and(|labels| labels.child_val("showVal") == Some("1"))
        || element
            .kids("ser")
            .any(|serie| serie.child("dLbls").is_some_and(|labels| labels.child_val("showVal") == Some("1")));
    let gridlines = values_axis.is_some_and(|axis| axis.child("majorGridlines").is_some());
    Some(Chart {
        kind: kind.to_string(),
        stacking,
        smooth,
        title,
        legend,
        data_labels,
        gridlines,
        x_title,
        y_title,
        categories,
        series,
        colours,
    })
}

/// The **colour** a series is drawn in, when the file names one.
///
/// The colour is in the series' own shape: a **solid fill** of a
/// colour of the file — `srgbClr` — or a colour of the **theme**,
/// which the theme says what it is.
fn colour_of_series(serie: &craftkit_xml::El, theme: &Theme) -> Option<String> {
    let fill = serie.child("spPr").and_then(|shape| shape.child("solidFill"))?;
    if let Some(hex) = fill.child("srgbClr").and_then(|colour| colour.attr("val")) {
        return crate::styles::colour_of_hex(hex);
    }
    let scheme = fill.child("schemeClr").and_then(|colour| colour.attr("val"))?;
    let index = SLOTS.iter().position(|slot| *slot == scheme)?;
    theme.colour(index as u64).map(|colour| colour.to_string())
}

/// The **range** an element names, when it names one.
///
/// A file writes the range of a series as a reference, and the
/// reference is the file's own words.
fn data_ref(element: Option<&craftkit_xml::El>) -> Option<String> {
    let element = element?;
    for name in ["numRef", "strRef", "multiLvlStrRef"] {
        if let Some(reference) = element.child(name).and_then(|reference| reference.child("f")) {
            let text = reference.text.trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

/// The text of an element and of everything under it: the runs of a
/// rich text.
fn all_text(element: &craftkit_xml::El) -> String {
    let mut text = String::new();
    if let Some(run) = element.child("t") {
        text.push_str(&run.text);
    }
    for child in &element.children {
        text.push_str(&all_text(child));
    }
    text
}

/// The **breaks** of the printing of a sheet.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Breaks {
    /// The **rows** a page begins at.
    pub rows: Vec<u32>,
    /// The **columns** a page begins at.
    pub columns: Vec<u32>,
}

/// The **conditional formats** of the extension list of a sheet: the
/// rules a file puts beside the extension list.
///
/// These are not comparisons — they are bars of colour and marks of a
/// scale — and the tree's rule is a comparison, so they are not read.
/// The count is given, so that the reader may say how many it left
/// out.
fn extension_conditionals(root: &craftkit_xml::El) -> u64 {
    let mut count = 0u64;
    for list in root.kids("extLst") {
        for extension in list.kids("ext") {
            let Some(formats) = extension.child("conditionalFormattings") else { continue };
            for format in formats.kids("conditionalFormatting") {
                count += format.kids("cfRule").count() as u64;
            }
        }
    }
    count
}

/// Read the **sparklines** of a sheet: the little drawings in its
/// cells.
///
/// A file keeps them in the **extension list** of the sheet, and the
/// names here are the file's own.
pub fn read_sparklines(root: &craftkit_xml::El, theme: &Theme, palette: &[u32]) -> Vec<Sparkline> {
    let mut out = Vec::new();
    for list in root.kids("extLst") {
        for extension in list.kids("ext") {
            let Some(groups) = extension.child("sparklineGroups") else { continue };
            for group in groups.kids("sparklineGroup") {
                let kind = match group.attr("type") {
                    Some("column") => "column",
                    Some("stacked") => "stacked",
                    _ => "line",
                };
                let colour = group
                    .child("colorSeries")
                    .and_then(|colour| crate::styles::read_colour(colour, theme, palette));
                let markers = group.flag("markers", false);
                let Some(sparklines) = group.child("sparklines") else { continue };
                for sparkline in sparklines.kids("sparkline") {
                    let Some(cell) = sparkline.child("sqref") else { continue };
                    let cell = cell.text.trim().to_string();
                    if cell.is_empty() {
                        continue;
                    }
                    let source = sparkline.child("f").map(|f| f.text.trim().to_string()).unwrap_or_default();
                    out.push(Sparkline {
                        cell,
                        source,
                        kind: kind.to_string(),
                        colour: colour.clone(),
                        markers,
                    });
                }
            }
        }
    }
    out
}

/// The **page setup** of a sheet: the paper, the orientation, the
/// margins, the scale and the fit.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Print {
    /// The **paper** the sheet prints on, by name, when the file names
    /// one it knows.
    pub paper: Option<String>,
    /// The **orientation**: `portrait` or `landscape`.
    pub orientation: Option<String>,
    /// The **margins**, in inches: left, right, top, bottom, header
    /// and footer.
    pub margins: Option<[f64; 6]>,
    /// The **scale** of the print, in percent.
    pub scale: Option<u32>,
    /// The **fit**: the pages wide and the pages tall, when the file
    /// says it prints to fit.
    pub fit: Option<(u32, u32)>,
    /// Whether the sheet prints to **fit** rather than at a scale.
    pub fit_to_page: bool,
}

/// An **edit range** of a sheet: a range that stays editable while the
/// sheet is protected.
///
/// The **name** of the range and the **password** that opens it are
/// the file's own: the tree's edit ranges are ranges and nothing else.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EditRange {
    /// The **ranges** of cells, as the file writes them.
    pub ranges: Vec<String>,
    /// The **name** of the range, when the file names one.
    pub name: Option<String>,
    /// Whether the file names a **password** for the range.
    pub password: bool,
}

/// The **options** of the print of a sheet.
///
/// The **marks** are the gridlines and the headings, and the
/// **centring** is where the sheet sits on the page: in the middle of
/// it, sideways, down it, or both.
///
/// A file that says nothing has the marks off and the sheet in the
/// corner, and that is not carried: the flags are the file's own.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PrintOptions {
    /// Whether the **gridlines** are printed.
    pub gridlines: bool,
    /// Whether the **headings** are printed.
    pub headings: bool,
    /// Whether the sheet is centred **sideways** on the page.
    pub centred_horizontally: bool,
    /// Whether the sheet is centred **down** the page.
    pub centred_vertically: bool,
}

/// The **header** and the **footer** of the pages of a sheet.
///
/// The texts are the **file's** own words, with the file's own codes in
/// them: `&L`, `&C` and `&R` are the three sections, `&P` the page
/// number, `&N` the number of pages, and `&A` the name of the sheet.
///
/// The **kinds** of page are the file's own: the odd pages, the even
/// pages and the first page. A text the file does not say is not here.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeaderFooter {
    /// The header of the **odd** pages.
    pub odd_header: Option<String>,
    /// The footer of the **odd** pages.
    pub odd_footer: Option<String>,
    /// The header of the **even** pages.
    pub even_header: Option<String>,
    /// The footer of the **even** pages.
    pub even_footer: Option<String>,
    /// The header of the **first** page.
    pub first_header: Option<String>,
    /// The footer of the **first** page.
    pub first_footer: Option<String>,
}

/// A **validation** of a sheet: a rule on what a cell may hold.
///
/// The names are the **file's** own, and the ranges are the file's own
/// words — `A1:A9` — because the file may name several ranges for one
/// rule.
#[derive(Debug, Clone, PartialEq)]
pub struct Validation {
    /// The ranges the rule is over.
    pub ranges: Vec<String>,
    /// The **kind** of the rule: the file's own name — `whole`,
    /// `decimal`, `list`, `date`, `time`, `textLength`, `custom` or
    /// `none`.
    pub kind: String,
    /// The **operator**: the file's own name, when the file names one.
    pub operator: Option<String>,
    /// The **first formula**: the number, the word or the list.
    pub formula1: Option<String>,
    /// The **second formula**, for the operators that have two.
    pub formula2: Option<String>,
    /// Whether a **blank** is allowed.
    pub allow_blank: bool,
    /// Whether the list of a list rule is shown **in the cell**.
    pub in_cell_dropdown: bool,
    /// The **title** of the input message.
    pub input_title: String,
    /// The **input message** itself.
    pub input_message: String,
    /// Whether the input message is **shown**.
    pub show_input: bool,
    /// The **title** of the error message.
    pub error_title: String,
    /// The **error message** itself.
    pub error_message: String,
    /// The **style** of the error: `stop`, `warning` or `information`.
    pub error_style: String,
    /// Whether the error message is **shown**.
    pub show_error: bool,
}

/// The **top** of a range: the count highest values of it, or the count
/// lowest.
#[derive(Debug, Clone, PartialEq)]
pub struct Top {
    /// How many of the values.
    pub count: u32,
    /// Whether the **lowest** values are the ones and not the highest.
    pub bottom: bool,
    /// Whether the count is a **percent** of the numbers of the range.
    pub percent: bool,
}

/// The **average** of a range: the cells whose number is above it, or
/// the cells whose number is below it.
#[derive(Debug, Clone, PartialEq)]
pub struct Average {
    /// Whether the numbers **above** the average are the ones.
    pub above: bool,
}

/// The **duplicate** of a range: the cells whose value is repeated in
/// it, or the cells whose value is not.
#[derive(Debug, Clone, PartialEq)]
pub struct Duplicate {
    /// Whether the rule is about the values that are **not** repeated.
    pub unique: bool,
}

/// The **data bar** of a range: the bar behind a cell, and the colour
/// of it.
#[derive(Debug, Clone, PartialEq)]
pub struct DataBar {
    /// The colour of the bar.
    pub colour: String,
}

/// The **icon set** of a range: the three icons of it, and the two
/// percents that tell which one a cell has.
///
/// The percents are the **percentiles** of the numbers of the range: the
/// high icon for the numbers at or above the high percentile, the middle
/// one for the numbers at or above the low percentile.
#[derive(Debug, Clone, PartialEq)]
pub struct IconSet {
    /// The percent of the numbers that carry the high icon.
    pub high: f64,
    /// The percent of the numbers that carry the middle icon.
    pub low: f64,
}

/// A **colour scale** of a sheet: the colours of the scale, in order.
///
/// The colours are two — of the lowest number of the range and of the
/// highest — or three, and then the middle one stands for the middle
/// of the numbers.
#[derive(Debug, Clone, PartialEq)]
pub struct ColourScale {
    /// The colours of the scale, in order.
    pub colours: Vec<String>,
}

/// A **conditional format** of a sheet: a rule about a range of cells.
///
/// The rule is a **comparison** — the operation, the value and the
/// **look** of the cells that match — or a **formula** the cells are
/// judged by, or a kind of rule that is about the range as a whole: the
/// top ten, the above-the-average, the duplicates, the colour scale, the
/// data bar and the icon set. The kinds of rule this tree does not have
/// are not here, and the reader says how many it left out.
#[derive(Debug, Clone, PartialEq)]
pub struct Conditional {
    /// The range of cells the rule is about.
    pub range: String,
    /// The **operation** the cell's value is judged by: `=`, `!=`,
    /// `<`, `<=`, `>` or `>=` — and the four that name a cell by **what
    /// it holds**: `isEmpty`, `isNotEmpty`, `isError` and `isNotError`
    /// — and the four on the **text** of a cell: `beginsWith`,
    /// `endsWith`, `contains` and `notContains`.
    pub op: String,
    /// The **value** the cell's value is compared with, as the file
    /// writes it. An operation that names a cell by what it holds has
    /// no value, and this is empty.
    pub value: String,
    /// The **formula** the cells are judged by, when the rule is one.
    ///
    /// The formula is written as it is in the **top-left cell** of the
    /// range, and it is moved with the cell it is judged for.
    pub formula: Option<String>,
    /// The **colour scale** of the rule, when the rule is one: the
    /// colours of the scale, in order.
    pub colour_scale: Option<ColourScale>,
    /// The **top** of the range the rule is about, when the rule has
    /// one.
    pub top: Option<Top>,
    /// The **average** of the range the rule is about, when the rule has
    /// one.
    pub average: Option<Average>,
    /// The **duplicate** of the range the rule is about, when the rule
    /// has one.
    pub duplicate: Option<Duplicate>,
    /// The **data bar** of the range the rule is about, when the rule
    /// has one.
    pub data_bar: Option<DataBar>,
    /// The **icon set** of the range the rule is about, when the rule
    /// has one.
    pub icon_set: Option<IconSet>,
    /// The **number format** the matching cells are written with, when
    /// the rule names one.
    pub format: Option<String>,
    /// The **colour** behind the matching cells, as `#rrggbb`, when the
    /// rule names one.
    pub fill: Option<String>,
    /// The **colour** the text of the matching cells is written in, as
    /// `#rrggbb`, when the rule names one.
    pub colour: Option<String>,
    /// The **borders** the rule draws on the matching cells, by the
    /// edge they are on.
    pub borders: Borders,
    /// Whether the rule **stops** the walk when it matched.
    pub stop: Option<bool>,
    /// The **priority** of the rule, when it names one: the lower the
    /// number, the earlier the rule is tried.
    pub priority: Option<u32>,
}

/// The **view** of a sheet: what is drawn of it, and what is not.
///
/// The flags are the file's own, and the **frozen panes** are the rows
/// above and the columns to the left of a cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetView {
    /// Whether the **gridlines** are drawn.
    pub gridlines: bool,
    /// Whether the **headings** — the row numbers and the column
    /// letters — are drawn.
    pub headings: bool,
    /// Whether the **zeros** are shown.
    pub zeros: bool,
    /// Whether the sheet runs **right to left**.
    pub right_to_left: bool,
    /// Whether the **outline symbols** are drawn.
    pub outline_symbols: bool,
    /// Whether the **formulas** are shown instead of the values.
    pub show_formulas: bool,
    /// The **zoom** of the sheet, in percent, when the file names one.
    pub zoom: Option<u32>,
    /// The **frozen panes**: the rows above and the columns to the
    /// left, when the sheet freezes any.
    pub freeze: Option<(u32, u32)>,
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
    /// The **outline level** of the row: how deep in a group it is.
    ///
    /// A row that is in no group has the level **zero**.
    pub outline: u8,
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
    /// The **outline level** of the column: how deep in a group it is.
    ///
    /// A column that is in no group has the level **zero**.
    pub outline: u8,
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
/// A note of a newer file is a **thread**: it has replies under it.
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
    /// The **replies** under it, in the order they were written.
    pub replies: Vec<Reply>,
    /// Whether the thread is **resolved**.
    pub resolved: bool,
}

/// A **reply** to a comment: a note written under another note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
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

/// The **properties** of a workbook: what the file says about itself.
///
/// The properties are in the file's own parts — `docProps/core.xml`
/// and `docProps/app.xml` — and a file that has none has no
/// properties: they are not an error, they are nothing.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Properties {
    /// The **title** of the workbook.
    pub title: Option<String>,
    /// The **subject** of the workbook.
    pub subject: Option<String>,
    /// The **author** of the workbook: the one who created it.
    pub creator: Option<String>,
    /// The **keywords** of the workbook.
    pub keywords: Option<String>,
    /// The **description** of the workbook.
    pub description: Option<String>,
    /// The **last one to change** the workbook.
    pub last_modified_by: Option<String>,
    /// The **company** the workbook belongs to.
    pub company: Option<String>,
}

/// The **properties** of a workbook, as the file's own parts say them.
fn properties_of(package: &mut craftkit_zip::Package<'_>) -> Properties {
    let mut properties = Properties::default();
    if let Ok(Some(core)) = read_xml(package, "docProps/core.xml") {
        properties.title = text_of(&core, "title");
        properties.subject = text_of(&core, "subject");
        properties.creator = text_of(&core, "creator");
        properties.keywords = text_of(&core, "keywords");
        properties.description = text_of(&core, "description");
        properties.last_modified_by = text_of(&core, "lastModifiedBy");
    }
    if let Ok(Some(app)) = read_xml(package, "docProps/app.xml") {
        properties.company = text_of(&app, "Company");
    }
    properties
}

/// The **text** of a child of an element, when it is not nothing.
fn text_of(element: &craftkit_xml::El, name: &str) -> Option<String> {
    let text = element.child(name)?.text.trim().to_string();
    (!text.is_empty()).then_some(text)
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
    /// The **charts** of each sheet, by the name of the sheet.
    pub charts: Vec<(String, Vec<Chart>)>,
    /// The **pictures** of each sheet, by the name of the sheet.
    pub pictures: Vec<(String, Vec<Picture>)>,
    /// The date system the numbers of the cells are counted in.
    pub system: DateSystem,
    /// The **properties** of the workbook: what the file says about
    /// itself.
    pub properties: Properties,
    /// The **theme** of the workbook: the colours and the fonts it
    /// names its cells by.
    pub theme: Theme,
    /// The **protection** of the workbook: what the file locks, when
    /// it locks something.
    pub protection: Option<WorkbookProtection>,
    /// The **named styles** of the workbook: the styles the file names.
    pub named_styles: Vec<crate::styles::NamedStyle>,
    /// The **calculation settings** of the workbook: how the file says
    /// it calculates.
    pub calculation: Calculation,
    /// Whether the **objects** of the workbook are drawn.
    pub objects: ShowObjects,
    /// The **warnings**: what could not be read, and what was kept
    /// instead.
    ///
    /// A workbook that is damaged is still a workbook, and the reader
    /// says what it had to leave out rather than refusing the whole
    /// file.
    pub warnings: Vec<String>,
}

/// Whether the **objects** of a workbook are drawn.
///
/// A file says so in its `workbookPr`: the charts, the pictures, the
/// shapes and the slicers of the workbook are drawn (`all`), drawn as
/// **placeholders** (`placeholders`), or not drawn at all (`none`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShowObjects {
    /// The objects are drawn.
    #[default]
    All,
    /// The objects are drawn as placeholders.
    Placeholders,
    /// The objects are not drawn.
    None,
}

impl ShowObjects {
    /// The **name** of the way, as the tree says it.
    pub fn name(&self) -> &'static str {
        match self {
            ShowObjects::All => "all",
            ShowObjects::Placeholders => "placeholders",
            ShowObjects::None => "none",
        }
    }
}

/// The **protection** of a workbook: what the file locks.
///
/// The **password** is not kept: the tree's protection says what is
/// protected and nothing more, and a file's password is a hash of a
/// word nobody has.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkbookProtection {
    /// Whether the **structure** of the workbook is locked: its sheets
    /// may not be added, taken away, renamed or moved.
    pub structure: bool,
    /// Whether the file names a **password** for the protection.
    pub password: bool,
}

/// The **signature** of a compound file: the container a workbook that
/// is encrypted — or a workbook of the old format — is held in.
const COMPOUND: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// The name of the stream a **password-encrypted** workbook is held
/// in.
const ENCRYPTED_PACKAGE: &str = "EncryptedPackage";

/// Whether the file is a **compound file**.
fn is_compound(bytes: &[u8]) -> bool {
    bytes.starts_with(&COMPOUND)
}

/// Whether the compound file holds an **encrypted** workbook.
///
/// A compound file writes its names the way Windows writes them — in
/// two bytes each — and the name of the package of an encrypted
/// workbook is in the file.
fn holds_encrypted_package(bytes: &[u8]) -> bool {
    let name: Vec<u8> = ENCRYPTED_PACKAGE.encode_utf16().flat_map(|unit| unit.to_le_bytes()).collect();
    !name.is_empty() && bytes.windows(name.len()).any(|window| window == name.as_slice())
}

/// Read a workbook file.
///
/// The file is a package; the workbook part is the one the package's
/// own relationships name, and `xl/workbook.xml` is the fallback for a
/// package that does not say.
///
/// A file that is a **compound file** is not a package: it holds a
/// workbook that is encrypted, or a workbook of the old format, and
/// the file says which of the two it is. A workbook that is
/// **encrypted** is read with [`read_with_password`]; a workbook of the
/// old format is not read.
pub fn read(bytes: &[u8]) -> Result<Workbook, Error> {
    if is_compound(bytes) {
        return Err(Error::Compound(if holds_encrypted_package(bytes) {
            "the workbook is encrypted, and it is read with a password".to_string()
        } else {
            "the file holds a workbook of the old format, and this tree does not read it".to_string()
        }));
    }
    read_package(bytes)
}

/// Read a workbook file that is **encrypted**, with the password that
/// opens it.
///
/// The file is a compound file, and the cipher that opens it is the
/// one of [MS-OFFCRYPTO]: the password is checked against the verifier
/// of the file, and the workbook is the package inside it.
pub fn read_with_password(bytes: &[u8], password: &str) -> Result<Workbook, Error> {
    if !is_compound(bytes) {
        return Err(Error::Encrypted("the file is not encrypted".to_string()));
    }
    let compound = cfb::Compound::open(bytes).map_err(|error| Error::Compound(error.message))?;
    let plain = crypto::open(&compound, password).map_err(|error| Error::Encrypted(error.message))?;
    read_package(&plain)
}

/// Read a workbook from the bytes of its **package**.
fn read_package(bytes: &[u8]) -> Result<Workbook, Error> {
    let mut package =
        craftkit_zip::Package::open(bytes).map_err(|error| Error::Package(error.to_string()))?;
    let mut workbook = Workbook::default();

    let root = rels_of(&mut package, "")?;
    let part = root
        .iter()
        .find(|rel| rel.kind == "officeDocument")
        .map(|rel| rel.target.clone())
        .unwrap_or_else(|| "xl/workbook.xml".to_string());

    // The **properties** of the workbook: what the file says about
    // itself.
    workbook.properties = properties_of(&mut package);

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
    if let Some(properties) = workbook_xml.child("workbookPr") {
        if properties.flag("date1904", false) {
            workbook.system = DateSystem::Y1904;
        }
        // The **objects** of the workbook: whether they are drawn.
        workbook.objects = match properties.attr("showObjects") {
            Some("none") => ShowObjects::None,
            Some("placeholders") => ShowObjects::Placeholders,
            _ => ShowObjects::All,
        };
    }

    // The **protection** of the workbook: what the file locks.
    if let Some(protection) = workbook_xml.child("workbookProtection") {
        workbook.protection = Some(WorkbookProtection {
            structure: protection.flag("lockStructure", false),
            password: ["workbookPassword", "workbookHashValue", "workbookAlgorithmName"]
                .iter()
                .any(|name| protection.attr(name).is_some()),
        });
    }

    // The **calculation settings** of the workbook: how the file says it
    // calculates.
    let (calculation, said) = calculation::read(workbook_xml.child("calcPr"));
    workbook.calculation = calculation;
    workbook.warnings.extend(said);

    let sheet_names: Vec<String> = workbook_xml
        .child("sheets")
        .map(|sheets| {
            sheets.kids("sheet").map(|sheet| sheet.attr("name").unwrap_or("").to_string()).collect()
        })
        .unwrap_or_default();
    workbook.names = names_of(&workbook_xml, &sheet_names);
    // The **printing** of the sheets: the area a sheet prints and the
    // titles it repeats. A file says these with the built-in names.
    let prints = print_of(&workbook_xml, &sheet_names);

    let workbook_rels = rels_of(&mut package, &part)?;
    // The **date system** of the workbook, for the cells that hold a
    // date in the file's own words.
    let system = workbook.system;
    let strings = shared_strings(&mut package, &workbook_rels, &mut workbook.warnings)?;
    // The **theme**: the colours and the fonts the file names its
    // cells by. The styles come after it, because a style may name a
    // colour of the theme.
    let theme = read_theme_of(&mut package, &workbook_rels);
    workbook.theme = theme.clone();
    let styles = read_styles_of(&mut package, &workbook_rels, &mut workbook.warnings, &theme);
    // The **named styles** of the workbook: the styles the file names.
    workbook.named_styles = styles.named_styles().to_vec();

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
        let (mut sheet, parts) = read_sheet(
            &mut package,
            &relation.target,
            &strings,
            &styles,
            &theme,
            &workbook_rels,
            system,
            &mut workbook.warnings,
        );
        sheet.name = name.clone();
        // The **state** of the sheet: a file may hide a sheet, and it
        // may hide it very well.
        sheet.state = element.attr("state").map(str::to_string);
        if let Some((_, area, titles)) = prints.iter().find(|(sheet, _, _)| *sheet == name) {
            sheet.print_area = area.clone();
            sheet.print_titles = titles.clone();
        }
        if let Some(filter) = parts.filter {
            workbook.filters.push((sheet.name.clone(), filter));
        }
        if !parts.links.is_empty() {
            workbook.links.push((sheet.name.clone(), parts.links));
        }
        if !parts.comments.is_empty() {
            workbook.comments.push((sheet.name.clone(), parts.comments));
        }
        if !parts.tables.is_empty() {
            workbook.tables.push((sheet.name.clone(), parts.tables));
        }
        if !parts.charts.is_empty() {
            workbook.charts.push((sheet.name.clone(), parts.charts));
        }
        if !parts.pictures.is_empty() {
            workbook.pictures.push((sheet.name.clone(), parts.pictures));
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

/// The **printing** of the sheets: the area a sheet prints, and the
/// titles it repeats on every page.
///
/// A file says these with the **built-in names** — `_xlnm.Print_Area`
/// and `_xlnm.Print_Titles` — and they are not names a formula uses:
/// they are the printing of the sheet they belong to. The scope of a
/// built-in name is the sheet it is about, and the number the file
/// gives it counts the sheets of the file.
fn print_of(
    workbook_xml: &craftkit_xml::El,
    sheets: &[String],
) -> Vec<(String, Option<String>, Option<String>)> {
    let mut out: Vec<(String, Option<String>, Option<String>)> =
        sheets.iter().map(|name| (name.clone(), None, None)).collect();
    let Some(names) = workbook_xml.child("definedNames") else { return out };
    for element in names.kids("definedName") {
        let Some(name) = element.attr("name") else { continue };
        let lower = name.to_ascii_lowercase();
        if lower != "_xlnm.print_area" && lower != "_xlnm.print_titles" {
            continue;
        }
        let Some(scope) = element.attr_u32("localSheetId") else { continue };
        let Some(sheet) = sheets.get(scope as usize) else { continue };
        let Some(index) = out.iter().position(|(name, _, _)| name == sheet) else { continue };
        let text = element.text.trim();
        if text.is_empty() {
            continue;
        }
        if lower == "_xlnm.print_area" {
            out[index].1 = Some(area_text(text));
        } else {
            out[index].2 = Some(text.to_string());
        }
    }
    out
}

/// The **area** of a print area: the file's own words, without the
/// sheet it belongs to and without the `$` of an absolute address.
///
/// A file may name several areas, and the **first** is the one: the
/// others are the areas of a range that is not in one piece.
fn area_text(text: &str) -> String {
    let first = text.split(',').next().unwrap_or(text).trim();
    let own = first.rsplit_once('!').map(|(_, own)| own).unwrap_or(first);
    own.replace('$', "")
}

/// The **shared strings** of a workbook: the texts its cells share.
fn shared_strings(
    package: &mut craftkit_zip::Package<'_>,
    workbook_rels: &[Rel],
    warnings: &mut Vec<String>,
) -> Result<Vec<SharedString>, Error> {
    let Some(relation) = workbook_rels.iter().find(|rel| rel.kind == "sharedStrings") else {
        return Ok(Vec::new());
    };
    let Some(root) = read_xml(package, &relation.target)? else {
        warnings.push(format!("the part {} is not there", relation.target));
        return Ok(Vec::new());
    };
    let mut strings = Vec::new();
    let mut dropped = 0usize;
    for item in root.kids("si") {
        let text = rich_text(item);
        let (phonetic, said) = phonetic::read(item, &text);
        dropped += said;
        strings.push(SharedString { text, phonetic });
    }
    if dropped > 0 {
        warnings.push(format!(
            "{dropped} reading(s) of the shared strings were not read: their offsets do not fit the text"
        ));
    }
    Ok(strings)
}

/// The **reading** of the text of a cell: the phonetic guide the file
/// holds beside it, and how many readings were not read.
///
/// A cell that holds a **shared** text takes the reading of that text;
/// a cell that holds its text **in itself** takes the reading it holds
/// beside it.
fn reading_of(cell: &craftkit_xml::El, strings: &[SharedString]) -> (Option<Phonetic>, usize) {
    match cell.attr("t") {
        Some("s") => {
            let index = cell.child("v").and_then(|value| value.text.trim().parse::<usize>().ok());
            match index.and_then(|index| strings.get(index)) {
                Some(string) => (string.phonetic.clone(), 0),
                None => (None, 0),
            }
        }
        Some("inlineStr") => match cell.child("is") {
            Some(item) => {
                let (phonetic, dropped) = phonetic::read(item, &rich_text(item));
                (phonetic, dropped)
            }
            None => (None, 0),
        },
        _ => (None, 0),
    }
}

/// A **shared string** of a workbook: the text its cells share, and the
/// **reading** the file may hold beside it.
struct SharedString {
    /// The text itself.
    text: String,
    /// The reading of the text, when the file holds one.
    phonetic: Option<Phonetic>,
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
///
/// The arguments are the facts of the file that a sheet is read
/// against: the strings it may hold, the styles it is written in, the
/// theme the styles name their colours by, and the calendar its
/// numbers are counted in.
#[allow(clippy::too_many_arguments)]
fn read_sheet(
    package: &mut craftkit_zip::Package<'_>,
    part: &str,
    strings: &[SharedString],
    styles: &Styles,
    theme: &Theme,
    workbook_rels: &[Rel],
    system: DateSystem,
    warnings: &mut Vec<String>,
) -> (Sheet, Parts) {
    let mut sheet = Sheet::default();
    let Some(root) = read_xml(package, part).ok().flatten() else {
        warnings.push(format!("the part {part} could not be read; the sheet is empty"));
        return (sheet, Parts::default());
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
    let comments = read_comments(package, &sheet_rels, workbook_rels, warnings);
    // The **tables** of the sheet: they are in parts of their own
    // that the sheet names by a relationship.
    let tables = read_tables(package, &root, &sheet_rels, warnings);
    // The **charts** of the sheet: they are in parts of their own,
    // named by the drawing of the sheet.
    let (charts, pictures) = read_drawings(package, &root, &sheet_rels, theme, warnings);
    // The **pivot tables** of the sheet: they are in parts of their
    // own, and each of them names a cache of its own.
    let pivots = read_pivots(package, &sheet_rels, warnings);
    // The **protection** of the sheet: what may still be done to it.
    // A file protects a sheet with an element that says it is
    // protected, and the flags of that element say what is still
    // allowed.
    sheet.protection = read_sheet_protection(&root);
    // The **default size** of the sheet: the width of a column and the
    // height of a row that has no size of its own.
    if let Some(format) = root.child("sheetFormatPr") {
        sheet.default_width = format.attr_f64("defaultColWidth").filter(|width| *width > 0.0);
        sheet.default_height = format.attr_f64("defaultRowHeight").filter(|height| *height > 0.0);
    }
    // The **outline** of the sheet: where the summary of a group
    // sits. A file that says nothing about it has the summary below
    // and to the right, and that is not carried.
    if let Some(properties) = root.child("sheetPr")
        && let Some(outline) = properties.child("outlinePr")
    {
        sheet.summary_below = outline.attr_bool("summaryBelow");
        sheet.summary_right = outline.attr_bool("summaryRight");
    }
    // The **colour** of the sheet's tab, when the file gives it one.
    if let Some(properties) = root.child("sheetPr") {
        sheet.tab_colour = properties
            .child("tabColor")
            .and_then(|colour| crate::styles::read_colour(colour, theme, styles.palette()));
    }
    // The **page setup** of the sheet: how it prints.
    sheet.print = read_print(&root);
    // The **header** and the **footer** of the pages of the sheet.
    sheet.header_footer = read_header_footer(&root);
    // The **options** of the print: the marks it prints, and where the
    // sheet sits on the page.
    sheet.print_options = read_print_options(&root);
    // The **edit ranges** of the sheet: the ranges that stay editable
    // while the sheet is protected.
    sheet.edit_ranges = read_edit_ranges(&root);
    // The **breaks** of the printing: the lines a page begins at.
    sheet.breaks = read_breaks(&root);
    // The **sparklines** of the sheet: the little drawings in its
    // cells. They live in the extension list of the sheet.
    sheet.sparklines = read_sparklines(&root, theme, styles.palette());
    // The **conditional formats** of the extension list: they are not
    // comparisons, and the tree's rule is a comparison — so they are
    // not read, and the reader says how many it left out.
    let extension_conditionals = extension_conditionals(&root);
    if extension_conditionals > 0 {
        warnings.push(format!(
            "{extension_conditionals} conditional format(s) of the extension list were not read: \
             this tree has a rule but a comparison"
        ));
    }
    // The **columns** of the sheet: their width and whether they are
    // hidden. A file says a span of columns at once, so one span of
    // ten columns is ten columns here.
    sheet.columns = read_columns(&root);
    // The **merges** of the sheet: the ranges that are one cell. A
    // range of one cell is not a merge.
    sheet.merges = read_merges(&root);
    // The **view** of the sheet: what is drawn of it, and what is not.
    sheet.view = read_view(&root);
    // The **conditional formats** of the sheet: the rules that are
    // about a range of cells.
    sheet.conditionals = read_conditionals(&root, styles, warnings);
    // The **validations** of the sheet: the rules on what a cell may
    // hold.
    sheet.validations = read_validations(&root);
    // The **rows** of the sheet: their height and whether they are
    // hidden. A file says a row of its own, and this reader reads the
    // rows as it reads the cells.
    let mut rows: Vec<Row> = Vec::new();
    let Some(data) = root.child("sheetData") else {
        sheet.pivots = pivots;
        return (sheet, Parts { filter, links, comments, tables, charts, pictures });
    };

    // The row the reader is on: a row that does not say which row it is
    // is the row after the last one, as the tree's reader does.
    let mut next_row: u64 = 0;
    let mut dropped = 0u64;
    let mut next_column: u32;
    // The **masters** of the shared formulas: by the number of the
    // group, the cell that wrote the formula, and the formula.
    let mut masters: std::collections::BTreeMap<u32, (u32, u32, String)> = std::collections::BTreeMap::new();
    // The cells that **name** a shared formula without writing it.
    let mut pending: Vec<(usize, u32)> = Vec::new();

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
        // The **outline level** of the row: how deep in a group it is.
        let outline = row.attr_u32("outlineLevel").unwrap_or(0).min(MAX_OUTLINE) as u8;
        if height.is_some() || hidden || outline > 0 {
            rows.push(Row { row: row_index + 1, height, hidden, outline });
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
            let value = value_of(cell, strings, system);
            // The **reading** of the text: the phonetic guide the file
            // holds beside it.
            let (phonetic, said) = reading_of(cell, strings);
            if said > 0 {
                warnings.push(format!(
                    "{said} reading(s) of the sheet were not read: their offsets do not fit the text"
                ));
            }
            // The **formula** of the cell: the file's own text. A cell
            // that **shares** a formula with other cells writes the
            // formula once, in the cell at the top left of its group,
            // and the other cells name it; so a cell that names a
            // shared formula without writing it is remembered here,
            // and the group is filled in below.
            let shared_with = cell.child("f").and_then(|f| {
                if f.attr("t") != Some("shared") {
                    return None;
                }
                f.attr_u32("si")
            });
            let formula = cell.child("f").map(|f| f.text.trim().to_string()).filter(|text| !text.is_empty());
            // The **kind** of the formula: an **array** fills a range of
            // cells, and a **data table** is a what-if table of values.
            // The text of such a formula is not the text of a formula
            // of a cell, and the battery is told which it is.
            let formula_kind = cell.child("f").and_then(|f| match f.attr("t").unwrap_or("normal") {
                "array" => Some(FormulaKind::Array),
                "dataTable" => Some(FormulaKind::DataTable),
                _ => None,
            });
            let array_range = match formula_kind {
                Some(FormulaKind::Array) => {
                    cell.child("f").and_then(|f| f.attr("ref")).map(|range| range.replace('$', ""))
                }
                _ => None,
            };
            let shared_master = match (shared_with, &formula) {
                (Some(index), Some(text)) => {
                    masters.insert(index, (column, row_index as u32, text.clone()));
                    None
                }
                (Some(index), None) => Some(index),
                _ => None,
            };
            let style = cell.attr_u32("s");
            let format = styles.format(style).map(|code| code.to_string());
            // The **checkbox** of the cell: a cell control the file
            // holds beside its cells, and the cell's style is what
            // names it.
            let checkbox = styles.is_checkbox(style);
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
                formula_kind,
                array_range,
                format,
                font,
                fill,
                align,
                borders,
                protection,
                shared: None,
                checkbox,
                phonetic,
            });
            if let Some(index) = shared_master {
                // The cell names a shared formula: the group's master
                // may not have been read yet, so the name is kept and
                // the group is filled in below.
                pending.push((sheet.cells.len() - 1, index));
            }
        }
    }

    // The **shared formulas**: the cells that name a formula the
    // group's master wrote. The master's text and the distance from
    // the master are what the caller needs, and the shifting of the
    // text is the caller's own — the reader knows the file's words,
    // and the shifting is the model's.
    for (at, index) in pending {
        let Some((master_column, master_row, text)) = masters.get(&index) else {
            continue;
        };
        let cell = &sheet.cells[at];
        let shared = Shared {
            text: text.clone(),
            row_delta: cell.row as i64 - i64::from(*master_row),
            column_delta: cell.column as i64 - i64::from(*master_column),
        };
        sheet.cells[at].shared = Some(shared);
    }

    if dropped > 0 {
        warnings.push(format!(
            "the sheet holds {dropped} cells or rows outside the {MAX_ROWS} × {MAX_COLS} grid; they were dropped"
        ));
    }
    sheet.rows = rows;
    sheet.pivots = pivots;
    (sheet, Parts { filter, links, comments, tables, charts, pictures })
}

/// Read the **view** of a sheet: what is drawn of it, and what is not.
///
/// A file names the view of a sheet in an element of its own, and a
/// sheet with no such element has the default view.
///
/// The **frozen panes** are read from the pane of the view: a pane
/// that is frozen holds the number of the rows and the columns that
/// are frozen, and a pane that is split holds a measure in twips — a
/// different thing — and is not read.
fn read_view(root: &craftkit_xml::El) -> Option<SheetView> {
    let views = root.child("sheetViews")?;
    let view = views.child("sheetView")?;
    let mut freeze = None;
    if let Some(pane) = view.child("pane")
        && pane.attr("state") == Some("frozen")
    {
        let cols = pane.attr_f64("xSplit").unwrap_or(0.0).clamp(0.0, (MAX_COLS - 1) as f64) as u32;
        let rows = pane.attr_f64("ySplit").unwrap_or(0.0).clamp(0.0, (MAX_ROWS - 1) as f64) as u32;
        if rows > 0 || cols > 0 {
            freeze = Some((rows, cols));
        }
    }
    Some(SheetView {
        gridlines: view.flag("showGridLines", true),
        headings: view.flag("showRowColHeaders", true),
        zeros: view.flag("showZeros", true),
        right_to_left: view.flag("rightToLeft", false),
        outline_symbols: view.flag("showOutlineSymbols", true),
        show_formulas: view.flag("showFormulas", false),
        zoom: view.attr_u32("zoomScale").map(|zoom| zoom.clamp(10, 400)),
        freeze,
    })
}

/// Read the **conditional formats** of a sheet: the rules that are
/// about a range of cells.
///
/// A file names the ranges of a rule in one list, and this tree's rule
/// is about **one** range — so a rule over two ranges is two rules
/// here, with the same criterion and the same look.
///
/// The kinds of rule this tree has are read: a **comparison**, a
/// **formula**, the four that look for the **words** of a cell, the
/// four that name a cell by **what it holds**, and the kinds that are
/// about the range as a whole — the top ten, the above-the-average, the
/// duplicates, the colour scale, the data bar and the icon set. The
/// kind it does not have — the rule of the time period — is counted,
/// and the reader says how many it left out.
fn read_conditionals(
    root: &craftkit_xml::El,
    styles: &Styles,
    warnings: &mut Vec<String>,
) -> Vec<Conditional> {
    let mut formats = Vec::new();
    // The rules of the kinds this tree does not have: they are counted,
    // and the reader says how many there were.
    let mut unread = 0u64;
    // The **colour scales** this tree cannot read: the stops of them
    // are not the ones it has, and they are counted too.
    let mut unread_scales = 0u64;
    for block in root.kids("conditionalFormatting") {
        let Some(sqref) = block.attr("sqref") else { continue };
        let ranges: Vec<&str> = sqref.split_whitespace().collect();
        if ranges.is_empty() {
            continue;
        }
        for rule in block.kids("cfRule") {
            let kind = rule.attr("type").unwrap_or("cellIs");
            // The **icon set** of the rule: the tree has three icons,
            // and the two percents that tell which one a cell has. An
            // icon set that is not one the tree has is counted, and
            // not read.
            let mut icon_set: Option<IconSet> = None;
            if kind == "iconSet" {
                match read_icon_set(rule) {
                    Some(set) => icon_set = Some(set),
                    None => {
                        unread += 1;
                        continue;
                    }
                }
            }
            // The **criterion** of the rule: the operation and the
            // value, or the formula the cells are judged by, or the
            // colours of a scale.
            let (op, value, formula, colour_scale, top, average, duplicate, data_bar) = match kind {
                "cellIs" => {
                    let Some(op) = rule.attr("operator").and_then(conditional_op) else {
                        unread += 1;
                        continue;
                    };
                    let Some(value) =
                        rule.kids("formula").next().map(|formula| formula.text.trim().to_string())
                    else {
                        unread += 1;
                        continue;
                    };
                    (op.to_string(), value, None, None, None, None, None, None)
                }
                "expression" => {
                    let Some(formula) =
                        rule.kids("formula").next().map(|formula| formula.text.trim().to_string())
                    else {
                        unread += 1;
                        continue;
                    };
                    // The formula is written without the `=` the file
                    // does not write.
                    (
                        String::new(),
                        String::new(),
                        Some(formula.trim_start_matches('=').to_string()),
                        None,
                        None,
                        None,
                        None,
                        None,
                    )
                }
                "containsText" => (
                    "contains".to_string(),
                    rule.attr("text").unwrap_or("").to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
                "notContainsText" => (
                    "notContains".to_string(),
                    rule.attr("text").unwrap_or("").to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
                "beginsWith" => (
                    "beginsWith".to_string(),
                    rule.attr("text").unwrap_or("").to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
                "endsWith" => (
                    "endsWith".to_string(),
                    rule.attr("text").unwrap_or("").to_string(),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ),
                "containsBlanks" => {
                    ("isEmpty".to_string(), String::new(), None, None, None, None, None, None)
                }
                "notContainsBlanks" => {
                    ("isNotEmpty".to_string(), String::new(), None, None, None, None, None, None)
                }
                "containsErrors" => {
                    ("isError".to_string(), String::new(), None, None, None, None, None, None)
                }
                "notContainsErrors" => {
                    ("isNotError".to_string(), String::new(), None, None, None, None, None, None)
                }
                "top10" => {
                    // The **top ten**: the count of it is the value of
                    // the stop of the rule, and a stop that is a
                    // **percent** of the numbers is a percent.
                    let stop = rule.kids("cfvo").next();
                    let percent = stop
                        .and_then(|stop| stop.attr("type"))
                        .map(|kind| kind == "percent")
                        .unwrap_or_else(|| rule.attr("percent") == Some("1"));
                    let count = stop
                        .and_then(|stop| stop.attr("val"))
                        .and_then(|val| val.parse::<u32>().ok())
                        .or_else(|| rule.attr("rank").and_then(|rank| rank.parse::<u32>().ok()))
                        .unwrap_or(10);
                    let bottom = rule.attr("bottom") == Some("1");
                    (
                        String::new(),
                        String::new(),
                        None,
                        None,
                        Some(Top { count, bottom, percent }),
                        None,
                        None,
                        None,
                    )
                }
                "aboveAverage" => {
                    // The **above-the-average**: the tree's rule has no
                    // standard deviation and it does not count the
                    // numbers that are equal to the average.
                    let standard = rule.attr("stdDev").and_then(|dev| dev.parse::<f64>().ok()).unwrap_or(0.0);
                    let equal = rule.attr("equalAverage") == Some("1");
                    if standard != 0.0 || equal {
                        unread += 1;
                        continue;
                    }
                    let above = rule.attr("aboveAverage") != Some("0");
                    (String::new(), String::new(), None, None, None, Some(Average { above }), None, None)
                }
                "duplicateValues" => (
                    String::new(),
                    String::new(),
                    None,
                    None,
                    None,
                    None,
                    Some(Duplicate { unique: false }),
                    None,
                ),
                "uniqueValues" => (
                    String::new(),
                    String::new(),
                    None,
                    None,
                    None,
                    None,
                    Some(Duplicate { unique: true }),
                    None,
                ),
                "dataBar" => {
                    // The **data bar**: the stops of it are the lowest
                    // and the highest of the range, and it has a
                    // colour.
                    let Some(bar) = rule.kids("dataBar").next() else {
                        unread += 1;
                        continue;
                    };
                    let stops: Vec<&str> = bar.kids("cfvo").filter_map(|stop| stop.attr("type")).collect();
                    if stops.len() != 2 || stops[0] != "min" || stops[1] != "max" {
                        unread += 1;
                        continue;
                    }
                    let Some(rgb) = bar.kids("color").next().and_then(|colour| colour.attr("rgb")) else {
                        unread += 1;
                        continue;
                    };
                    let hex = rgb.trim();
                    let hex = hex.get(hex.len().saturating_sub(6)..).unwrap_or(hex);
                    (
                        String::new(),
                        String::new(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        Some(DataBar { colour: format!("#{}", hex.to_ascii_lowercase()) }),
                    )
                }
                "colorScale" => {
                    // A **scale**: the stops of it are the lowest and
                    // the highest of the range, and with three stops
                    // the middle one is the fiftieth percentile — the
                    // middle of the numbers.
                    let Some(scale) = rule.kids("colorScale").next() else {
                        unread += 1;
                        continue;
                    };
                    let stops: Vec<(&str, Option<&str>)> = scale
                        .kids("cfvo")
                        .map(|stop| (stop.attr("type").unwrap_or(""), stop.attr("val")))
                        .collect();
                    let two_stops = stops.len() == 2 && stops[0].0 == "min" && stops[1].0 == "max";
                    let middle_is_the_median = stops.len() == 3
                        && stops[0].0 == "min"
                        && stops[1].0 == "percentile"
                        && stops[1].1 == Some("50")
                        && stops[2].0 == "max";
                    if !two_stops && !middle_is_the_median {
                        unread_scales += 1;
                        continue;
                    }
                    let colours: Vec<String> = scale
                        .kids("color")
                        .filter_map(|colour| colour.attr("rgb"))
                        .map(|rgb| {
                            // The file writes the colour as `arrggbb`,
                            // and the tree's colour is `#rrggbb`.
                            let hex = rgb.trim();
                            let hex = hex.get(hex.len().saturating_sub(6)..).unwrap_or(hex);
                            format!("#{}", hex.to_ascii_lowercase())
                        })
                        .collect();
                    if colours.len() != stops.len() {
                        unread_scales += 1;
                        continue;
                    }
                    (
                        String::new(),
                        String::new(),
                        None,
                        Some(ColourScale { colours }),
                        None,
                        None,
                        None,
                        None,
                    )
                }
                "iconSet" => (String::new(), String::new(), None, None, None, None, None, None),
                _ => {
                    unread += 1;
                    continue;
                }
            };
            let dxf = styles.dxf(rule.attr_u32("dxfId"));
            let format = dxf.and_then(|dxf| dxf.format.clone());
            let fill = dxf.and_then(|dxf| dxf.fill.clone());
            let colour = dxf.and_then(|dxf| dxf.colour.clone());
            let borders = dxf.map(|dxf| dxf.borders.clone()).unwrap_or_default();
            let stop = rule.attr("stopIfTrue").map(|stop| stop != "0");
            let priority = rule.attr_u32("priority");
            for range in &ranges {
                formats.push(Conditional {
                    range: range.replace('$', ""),
                    op: op.clone(),
                    value: value.clone(),
                    formula: formula.clone(),
                    colour_scale: colour_scale.clone(),
                    top: top.clone(),
                    average: average.clone(),
                    duplicate: duplicate.clone(),
                    data_bar: data_bar.clone(),
                    icon_set: icon_set.clone(),
                    format: format.clone(),
                    fill: fill.clone(),
                    colour: colour.clone(),
                    borders: borders.clone(),
                    stop,
                    priority,
                });
            }
        }
    }
    if unread > 0 {
        warnings.push(format!(
            "{unread} conditional format(s) of a kind this tree does not have were not read: \
             a rule of the time period and more"
        ));
    }
    if unread_scales > 0 {
        warnings.push(format!(
            "{unread_scales} colour scale(s) were not read: the stops of them are not the lowest, \
             the middle and the highest"
        ));
    }
    formats
}

/// Read the **icon set** of a rule, when the tree has it.
///
/// The tree's icon set has **three** icons, and the two stops that tell
/// which one a cell has are **percents** of the numbers of the range:
/// the high icon for the numbers at or above the high percentile, the
/// middle one for the numbers at or above the low percentile.
///
/// A set that is **reversed**, or that hides the value of the cell, or
/// that has another count of icons, or whose stops are not percents, is
/// one this tree does not have.
fn read_icon_set(rule: &craftkit_xml::El) -> Option<IconSet> {
    let set = rule.child("iconSet")?;
    if set.attr("reverse") == Some("1") || set.attr("showValue") == Some("0") {
        return None;
    }
    if let Some(name) = set.attr("iconSet")
        && !name.starts_with('3')
    {
        return None;
    }
    let stops: Vec<Option<f64>> = set
        .kids("cfvo")
        .map(|stop| {
            if stop.attr("type") != Some("percent") {
                return None;
            }
            stop.attr("val").and_then(|val| val.trim().parse::<f64>().ok())
        })
        .collect();
    // Three stops: the bottom of the range, the middle icon and the
    // high one.
    if stops.len() != 3 || stops[0] != Some(0.0) {
        return None;
    }
    let low = stops[1]?;
    let high = stops[2]?;
    if !(0.0..=100.0).contains(&low) || !(0.0..=100.0).contains(&high) || high <= low {
        return None;
    }
    Some(IconSet { high, low })
}

/// The **operation** a rule compares with, as this tree's own.
///
/// The tree's rules are comparisons of two values — *between* and *not
/// between* — and this tree's criterion has one value, so those two
/// are not here.
fn conditional_op(operator: &str) -> Option<&'static str> {
    match operator {
        "equal" => Some("="),
        "notEqual" => Some("!="),
        "greaterThan" => Some(">"),
        "lessThan" => Some("<"),
        "greaterThanOrEqual" => Some(">="),
        "lessThanOrEqual" => Some("<="),
        _ => None,
    }
}

/// Read the **validations** of a sheet: the rules on what a cell may
/// hold.
///
/// A file names the ranges of a rule in one list, and this tree's rule
/// is over **one** range — so a rule over two ranges is two rules
/// here, with the same kind and the same formulas.
///
/// A file may hold its rules in the **extension list** of the sheet
/// instead of beside the cells, and those rules are read too.
fn read_validations(root: &craftkit_xml::El) -> Vec<Validation> {
    let mut validations = Vec::new();
    // The rules of the sheet itself.
    if let Some(list) = root.child("dataValidations") {
        for rule in list.kids("dataValidation") {
            read_validation(rule, &mut validations);
        }
    }
    // The rules of the **extension list**: a file may put a rule there
    // instead, and there the range is a child of the rule rather than
    // an attribute of it.
    for list in root.kids("extLst") {
        for extension in list.kids("ext") {
            let Some(rules) = extension.child("dataValidations") else { continue };
            for rule in rules.kids("dataValidation") {
                read_validation(rule, &mut validations);
            }
        }
    }
    validations
}

/// Read **one** validation of a sheet.
///
/// A file writes the range of a rule as an attribute — `sqref` — and
/// the extension list writes it as a child of the same name. A file
/// writes the formulas of a rule as children, and the extension list
/// writes them as a child of a child: the formula is in a `f`.
fn read_validation(rule: &craftkit_xml::El, validations: &mut Vec<Validation>) {
    let sqref = rule
        .attr("sqref")
        .map(|sqref| sqref.to_string())
        .or_else(|| rule.child("sqref").map(|sqref| sqref.text.trim().to_string()));
    let Some(sqref) = sqref else { return };
    let ranges: Vec<String> = sqref
        .split_whitespace()
        .map(|range| range.replace('$', ""))
        .filter(|range| !range.is_empty())
        .collect();
    if ranges.is_empty() {
        return;
    }
    let formula = |name: &str| {
        let formula = rule.child(name)?;
        let text = formula.child("f").map(|f| f.text.as_str()).unwrap_or(formula.text.as_str());
        let text = text.trim();
        if text.is_empty() { None } else { Some(text.to_string()) }
    };
    let kind = rule.attr("type").unwrap_or("none").to_string();
    let operator = rule.attr("operator").map(|operator| operator.to_string());
    let formula1 = formula("formula1");
    let formula2 = formula("formula2");
    let allow_blank = rule.flag("allowBlank", false);
    // A file's `showDropDown` **hides** the arrow of a list, and
    // the tree's flag is that the arrow is shown.
    let in_cell_dropdown = !rule.flag("showDropDown", false);
    let input_title = rule.attr("promptTitle").unwrap_or("").to_string();
    let input_message = rule.attr("prompt").unwrap_or("").to_string();
    let show_input = rule.flag("showInputMessage", false);
    let error_title = rule.attr("errorTitle").unwrap_or("").to_string();
    let error_message = rule.attr("error").unwrap_or("").to_string();
    let error_style = match rule.attr("errorStyle") {
        Some("warning") => "warning",
        Some("information") => "information",
        _ => "stop",
    };
    let show_error = rule.flag("showErrorMessage", false);
    for range in ranges {
        validations.push(Validation {
            ranges: vec![range],
            kind: kind.clone(),
            operator: operator.clone(),
            formula1: formula1.clone(),
            formula2: formula2.clone(),
            allow_blank,
            in_cell_dropdown,
            input_title: input_title.clone(),
            input_message: input_message.clone(),
            show_input,
            error_title: error_title.clone(),
            error_message: error_message.clone(),
            error_style: error_style.to_string(),
            show_error,
        });
    }
}

/// Read the **merges** of a sheet: the ranges that are one cell.
///
/// A range of **one** cell is not a merge — a cell is one cell already
/// — and it is not here. A range that is not a range of cells at all is
/// not here either.
fn read_merges(root: &craftkit_xml::El) -> Vec<String> {
    let mut merges = Vec::new();
    let Some(list) = root.child("mergeCells") else { return merges };
    for merge in list.kids("mergeCell") {
        let Some(range) = merge.attr("ref") else { continue };
        let range = range.trim().replace('$', "");
        let Some((first, second)) = range.split_once(':') else { continue };
        let (Some(first), Some(second)) = (address_of(first), address_of(second)) else { continue };
        if first == second {
            continue;
        }
        merges.push(range);
    }
    merges
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
        // The **outline level** of the columns: how deep in a group they
        // are.
        let outline = col.attr_u32("outlineLevel").unwrap_or(0).min(MAX_OUTLINE) as u8;
        if width.is_none() && !hidden && outline == 0 {
            continue;
        }
        for index in (min - 1)..max {
            columns.push(Column { column: index, width, hidden, outline });
        }
    }
    columns
}

/// Read the **page setup** of a sheet: the paper, the orientation, the
/// margins, the scale and the fit.
///
/// The numbers are the **file's** own: the margins are in inches, the
/// scale is a percent, and the paper is the file's own number.
fn read_print(root: &craftkit_xml::El) -> Option<Print> {
    let setup = root.child("pageSetup");
    let margins = root.child("pageMargins");
    if setup.is_none() && margins.is_none() {
        return None;
    }
    let mut print = Print::default();
    if let Some(setup) = setup {
        print.paper = setup.attr_u32("paperSize").and_then(paper_name).map(str::to_string);
        print.orientation = setup.attr("orientation").map(str::to_string);
        print.scale = setup.attr_u32("scale");
        let wide = setup.attr_u32("fitToWidth");
        let tall = setup.attr_u32("fitToHeight");
        if wide.is_some() || tall.is_some() {
            print.fit = Some((wide.unwrap_or(1), tall.unwrap_or(1)));
        }
    }
    if let Some(margins) = margins {
        let mut found = [0.0f64; 6];
        let mut any = false;
        for (index, name) in ["left", "right", "top", "bottom", "header", "footer"].iter().enumerate() {
            if let Some(value) = margins.attr_f64(name).filter(|value| *value >= 0.0) {
                found[index] = value;
                any = true;
            }
        }
        if any {
            print.margins = Some(found);
        }
    }
    // Whether the sheet prints to **fit**: the file says it in the
    // properties of the sheet.
    print.fit_to_page = root
        .path(&["sheetPr", "pageSetUpPr"])
        .map(|properties| properties.flag("fitToPage", false))
        .unwrap_or(false);
    Some(print)
}

/// Read the **edit ranges** of a sheet: the ranges that stay editable
/// while the sheet is protected.
///
/// A range of no cells is not a range, and it is not here.
fn read_edit_ranges(root: &craftkit_xml::El) -> Vec<EditRange> {
    let mut out: Vec<EditRange> = Vec::new();
    let Some(protected) = root.child("protectedRanges") else { return out };
    for range in protected.kids("protectedRange") {
        let ranges: Vec<String> = range
            .attr("sqref")
            .unwrap_or("")
            .split_whitespace()
            .filter(|text| !text.is_empty())
            .map(str::to_string)
            .collect();
        if ranges.is_empty() {
            continue;
        }
        out.push(EditRange {
            ranges,
            name: range.attr("name").map(str::to_string),
            password: ["password", "hashValue", "algorithmName", "saltValue", "spinCount"]
                .iter()
                .any(|name| range.attr(name).is_some()),
        });
    }
    out
}

/// Read the **options** of the print of a sheet: the marks it prints
/// and where the sheet sits on the page.
///
/// A file that says nothing about them has the marks off and the sheet
/// in the corner, and that is not carried.
fn read_print_options(root: &craftkit_xml::El) -> Option<PrintOptions> {
    let element = root.child("printOptions")?;
    Some(PrintOptions {
        gridlines: element.flag("gridLines", false),
        headings: element.flag("headings", false),
        centred_horizontally: element.flag("horizontalCentered", false),
        centred_vertically: element.flag("verticalCentered", false),
    })
}

/// Read the **header** and the **footer** of the pages of a sheet.
///
/// The texts are the file's own words: a text that is empty is not a
/// text, and a file that says no header at all has no header here.
fn read_header_footer(root: &craftkit_xml::El) -> Option<HeaderFooter> {
    let element = root.child("headerFooter")?;
    let text = |name: &str| {
        element.child(name).map(|child| child.text.trim().to_string()).filter(|text| !text.is_empty())
    };
    let header_footer = HeaderFooter {
        odd_header: text("oddHeader"),
        odd_footer: text("oddFooter"),
        even_header: text("evenHeader"),
        even_footer: text("evenFooter"),
        first_header: text("firstHeader"),
        first_footer: text("firstFooter"),
    };
    let empty = header_footer.odd_header.is_none()
        && header_footer.odd_footer.is_none()
        && header_footer.even_header.is_none()
        && header_footer.even_footer.is_none()
        && header_footer.first_header.is_none()
        && header_footer.first_footer.is_none();
    if empty { None } else { Some(header_footer) }
}

/// Read the **breaks** of the printing of a sheet: the rows and the
/// columns a page begins at.
///
/// The lines are the file's own numbering, and a break outside the
/// grid of the sheet is not a break.
fn read_breaks(root: &craftkit_xml::El) -> Breaks {
    let mut breaks = Breaks::default();
    let Some(rows) = root.child("rowBreaks") else { return breaks };
    for element in rows.kids("brk") {
        let Some(line) = element.attr_u32("id") else { continue };
        if line < MAX_ROWS as u32 {
            breaks.rows.push(line);
        }
    }
    if let Some(columns) = root.child("colBreaks") {
        for element in columns.kids("brk") {
            let Some(line) = element.attr_u32("id") else { continue };
            if line < MAX_COLS as u32 {
                breaks.columns.push(line);
            }
        }
    }
    breaks
}

/// The **papers** of the file format: the number a file writes, and the
/// name it means.
const PAPERS: [(u32, &str); 23] = [
    (1, "Letter"),
    (3, "Tabloid"),
    (4, "Ledger"),
    (5, "Legal"),
    (6, "Statement"),
    (7, "Executive"),
    (8, "A3"),
    (9, "A4"),
    (11, "A5"),
    (12, "B4"),
    (13, "B5"),
    (20, "Envelope #10"),
    (27, "Envelope DL"),
    (34, "Envelope B5"),
    (42, "B4 (ISO)"),
    (43, "Japanese Postcard"),
    (69, "Japanese Double Postcard"),
    (70, "A6"),
    (71, "Japanese Envelope Kaku #2"),
    (72, "Japanese Envelope Kaku #3"),
    (73, "Japanese Envelope #3"),
    (74, "Japanese Envelope #4"),
    (88, "B6"),
];

/// The name of the **paper** a number means, when it means one.
fn paper_name(number: u32) -> Option<&'static str> {
    PAPERS.iter().find(|(id, _)| *id == number).map(|(_, name)| *name)
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
    warnings: &mut Vec<String>,
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
        // The **header row**: a table of this tree has one, and the
        // fields of the table are the text of that row — so a table
        // whose header row is off is not a table of this tree, and it
        // is not read.
        if table.attr_u32("headerRowCount").unwrap_or(1) == 0 {
            warnings.push(format!(
                "the table `{name}` has no header row, and a table of this tree has one; it was not read"
            ));
            continue;
        }
        // The **filter** of the table: the filters of this tree are the
        // sheet's, and so a filter of a table is left out.
        if table.child("autoFilter").is_some() {
            warnings.push(format!(
                "the table `{name}` has a filter of its own, and the filters of this tree are the sheet's; it was not read"
            ));
        }
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
                let column_name = column.attr("name").unwrap_or("").to_string();
                let totals_way = totals_way(column.attr("totalsRowFunction"));
                let Some(totals_way) = totals_way else {
                    // A way this tree does not have is not a way, and
                    // the reader says that it left it out.
                    warnings.push(format!(
                        "the column `{column_name}` of the table `{name}` is totalled in a way this tree does not have; it was not read"
                    ));
                    continue;
                };
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
///
/// A newer file writes its notes a second way: as **threads**, with the
/// replies under them. Those are read too, and a thread is the truth
/// about its cell.
fn read_comments(
    package: &mut craftkit_zip::Package<'_>,
    sheet_rels: &[Rel],
    workbook_rels: &[Rel],
    warnings: &mut Vec<String>,
) -> Vec<Comment> {
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
            out.push(Comment { column, row: row as u32, author, text, replies: Vec::new(), resolved: false });
        }
    }
    // The **threads**: the notes of a newer file, with the replies
    // under them. The people who wrote them are named in a part of the
    // workbook's own.
    let persons = read_persons(package, workbook_rels);
    for relation in sheet_rels.iter().filter(|relation| relation.kind == "threadedComment") {
        let Some(root) = read_xml(package, &relation.target).ok().flatten() else { continue };
        let mut threads: Vec<(String, Comment)> = Vec::new();
        // The notes themselves: a thread with no parent is a note.
        for element in root.kids("threadedComment") {
            if element.attr("parentId").is_some() {
                continue;
            }
            let Some(address) = element.attr("ref") else { continue };
            let Some((column, row)) = address_of(address.trim()) else { continue };
            let Some(id) = element.attr("id") else { continue };
            let author =
                element.attr("personId").and_then(|person| persons.get(person)).cloned().unwrap_or_default();
            let text = element.child("text").map(thread_text).unwrap_or_default();
            let comment = Comment {
                column,
                row: row as u32,
                author,
                text,
                replies: Vec::new(),
                resolved: element.flag("done", false),
            };
            threads.push((id.to_string(), comment));
        }
        // The **replies**: a note under a note, in the order they were
        // written. A reply to a note that is not there is not a reply.
        for element in root.kids("threadedComment") {
            let Some(parent) = element.attr("parentId") else { continue };
            let Some((_, comment)) = threads.iter_mut().find(|(id, _)| id == parent) else {
                warnings.push("a reply to a note that is not there was not read".to_string());
                continue;
            };
            let author =
                element.attr("personId").and_then(|person| persons.get(person)).cloned().unwrap_or_default();
            let text = element.child("text").map(thread_text).unwrap_or_default();
            comment.replies.push(Reply { author, text });
        }
        for (_, comment) in threads {
            // A thread is the truth about its cell: the file writes the
            // older note beside it for the readers that do not know
            // threads, and the thread is the newer text.
            match out.iter_mut().find(|old| old.column == comment.column && old.row == comment.row) {
                Some(old) => *old = comment,
                None => out.push(comment),
            }
        }
    }
    out
}

/// The text of a **thread's** note: the text of the element itself, or
/// the text of its runs.
fn thread_text(element: &craftkit_xml::El) -> String {
    let mut text = element.text.clone();
    for run in element.kids("r") {
        if let Some(t) = run.child("t") {
            text.push_str(&t.text);
        }
    }
    text
}

/// The **people** of a workbook: the ones who wrote the notes of its
/// threads.
///
/// The people are in a part of the workbook's own, and each one has an
/// **id** that its notes name.
fn read_persons(
    package: &mut craftkit_zip::Package<'_>,
    workbook_rels: &[Rel],
) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    for relation in workbook_rels.iter().filter(|relation| relation.kind == "person") {
        let Some(root) = read_xml(package, &relation.target).ok().flatten() else { continue };
        for person in root.kids("person") {
            let Some(id) = person.attr("id") else { continue };
            let name = person.attr("displayName").unwrap_or("").to_string();
            out.insert(id.to_string(), name);
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
fn value_of(cell: &craftkit_xml::El, strings: &[SharedString], system: DateSystem) -> Value {
    let raw = cell.child("v").map(|v| v.text.trim().to_string());
    match cell.attr("t").unwrap_or("n") {
        "s" => match raw.as_deref().and_then(|text| text.parse::<usize>().ok()) {
            Some(index) => match strings.get(index) {
                Some(string) => Value::Text(string.text.clone()),
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
        // A **date**: the file writes it as an ISO text, and the
        // tree counts the days.
        "d" => match raw.as_deref().and_then(|text| parse_iso(text, system)) {
            Some(number) => Value::Number(number),
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

/// The **serial** of an ISO date or a time, when it is one.
///
/// A file writes a date in a cell as `2024-01-01`, or with a time
/// `2024-01-01T12:00:00`, or a time alone `12:00:00`. The number is
/// the **serial** of the day: the days are counted in the file's own
/// system.
fn parse_iso(text: &str, system: DateSystem) -> Option<f64> {
    let text = text.trim().trim_end_matches('Z');
    let (date, time) = match text.split_once('T') {
        Some((date, time)) => (date, Some(time)),
        None if text.contains(':') => ("", Some(text)),
        None => (text, None),
    };
    let mut serial = 0.0;
    if !date.is_empty() {
        let mut parts = date.split('-');
        let year = parts.next()?.parse::<i64>().ok()?;
        let month = parts.next()?.parse::<u32>().ok()?;
        let day = parts.next()?.parse::<u32>().ok()?;
        serial = serial_from_ymd(system, year, month, day)?;
    }
    if let Some(time) = time {
        // A time zone offset is dropped: the file's time is the time.
        let time = time.split('+').next().unwrap_or(time);
        let mut parts = time.split(':');
        let hour = parts.next()?.parse::<f64>().ok()?;
        let minute = parts.next().unwrap_or("0").parse::<f64>().ok()?;
        let second = parts.next().unwrap_or("0").parse::<f64>().ok()?;
        if !(0.0..24.0).contains(&hour) || !(0.0..60.0).contains(&minute) || !(0.0..61.0).contains(&second) {
            return None;
        }
        serial += (hour * 3600.0 + minute * 60.0 + second) / 86_400.0;
    }
    Some(serial)
}

/// The **serial** of a day, in the file's own system.
///
/// The 1900 system counts the days from the 31st of December 1899,
/// and it holds a day that never was — the 29th of February 1900 —
/// before the first of March. The 1904 system counts the days from
/// the first of January 1904.
fn serial_from_ymd(system: DateSystem, year: i64, month: u32, day: u32) -> Option<f64> {
    if !(1..=12).contains(&month) || day < 1 || day > days_in_month(year, month) {
        return None;
    }
    let days = days_from_civil(year, month, day);
    let serial = match system {
        DateSystem::Y1900 => days - DAYS_1899_12_30 - i64::from(year == 1900 && month <= 2),
        DateSystem::Y1904 => days - DAYS_1904_01_01,
    };
    if serial < 0 {
        return None;
    }
    Some(serial as f64)
}

/// The **days** of a month, in a year.
fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// The **days** from the first of January 1970 to a day.
///
/// The count of the days of the Gregorian calendar, which is the
/// count everybody counts.
const fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let month = month as i64;
    let day = day as i64;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

/// The days from the first of January 1970 to the 30th of December
/// 1899: the day before the first of the 1900 system.
const DAYS_1899_12_30: i64 = days_from_civil(1899, 12, 30);

/// The days from the first of January 1970 to the first of January
/// 1904: the first of the 1904 system.
const DAYS_1904_01_01: i64 = days_from_civil(1904, 1, 1);

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
/// The **theme** of the workbook.
///
/// A workbook with no theme part has no theme, and that is not an
/// error: a file may be written without one — and then a cell that
/// names a colour of the theme has no colour.
fn read_theme_of(package: &mut craftkit_zip::Package<'_>, workbook_rels: &[Rel]) -> Theme {
    let Some(relation) = workbook_rels.iter().find(|rel| rel.kind == "theme") else {
        return Theme::default();
    };
    let Some(root) = read_xml(package, &relation.target).ok().flatten() else {
        return Theme::default();
    };
    read_theme(&root)
}

/// A workbook with no styles part has no formats, and that is not an
/// error: a file may be written without one.
fn read_styles_of(
    package: &mut craftkit_zip::Package<'_>,
    workbook_rels: &[Rel],
    warnings: &mut Vec<String>,
    theme: &Theme,
) -> Styles {
    // The **checkboxes** of the file: the cell controls it holds beside
    // its cells. They are read before the styles, because a style is
    // what names them.
    let complements = read_complements_of(package, workbook_rels, warnings);
    let Some(relation) = workbook_rels.iter().find(|rel| rel.kind == "styles") else {
        return Styles::default();
    };
    let Some(root) = read_xml(package, &relation.target).ok().flatten() else {
        warnings.push(format!("the part {} could not be read", relation.target));
        return Styles::default();
    };
    styles::read_styles_with(&root, theme, &complements)
}

/// The **complements** of a workbook: for each one the file names,
/// whether it is a checkbox.
///
/// The cell controls of a file live in the **feature property bags**,
/// and the styles point at them. A workbook with no bags has no
/// complements, and that is not an error.
fn read_complements_of(
    package: &mut craftkit_zip::Package<'_>,
    workbook_rels: &[Rel],
    warnings: &mut Vec<String>,
) -> Vec<bool> {
    let Some(relation) = workbook_rels.iter().find(|rel| rel.kind == "FeaturePropertyBag") else {
        return Vec::new();
    };
    let Some(root) = read_xml(package, &relation.target).ok().flatten() else {
        warnings.push(format!("the part {} could not be read", relation.target));
        return Vec::new();
    };
    featurebags::checkbox_complements(&root)
}

/// Read a part of the package as XML.
fn read_xml(package: &mut craftkit_zip::Package<'_>, part: &str) -> Result<Option<craftkit_xml::El>, Error> {
    let Some(bytes) = package.read(part).map_err(|error| Error::Package(error.to_string()))? else {
        return Ok(None);
    };
    craftkit_xml::parse(&bytes).map(Some).map_err(|error| Error::Xml(format!("{part}: {error}")))
}
