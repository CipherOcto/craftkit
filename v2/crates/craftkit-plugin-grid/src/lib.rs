//! `craftkit-plugin-grid` — the document model of a **spreadsheet**.
//!
//! # What is ported
//!
//! A spreadsheet is the hardest thing in the family to put in a kit,
//! because it is not a tree and its graph is not a graph of things:
//!
//! - a **workbook** of **sheets**;
//! - a sheet's **cells**, which are far too many to be nodes: a sheet
//!   with a million cells is not a tree with a million nodes;
//! - a **formula** in a cell, which reads **areas** of cells rather than
//!   cells: `SUM(A1:B4)` reads sixteen cells and one range;
//! - the **value** of a formula cell, which the application computes and
//!   **stores** beside the formula.
//! - the **functions** a formula may call: `SUM`, `AVERAGE`, `MIN`,
//!   `MAX`, `COUNT`, `COUNTA`, `TEXT` and `SUBTOTAL`.
//!
//! And a formula may ask about the **shape** of a set of numbers — the
//! middle, the most frequent, the spread, the order statistics and the
//! place of a number in a list (the module `statistics`).
//!
//! And a formula may do **arithmetic**: round a number, cut it, take
//! the rest of a division, take its root, raise it to a power, and
//! find the next whole number that is even or odd — with the tree's
//! own careful rules (the module `arithmetic`).
//!
//! And a formula may ask about an **angle**: its sine and its cosine,
//! its tangent and their inverses, the hyperbolic ones and the turn of
//! an angle from degrees to radians (the module `trigonometry`).
//!
//! And a formula may ask what a sum of money is worth now and later:
//! the payment of a loan, its present and future worth, the rate that
//! makes it work out, the interest of one payment and the rate a sum
//! grows at — with the tree's own rules (the module `finance`).
//!
//! And a formula may ask what a thing that wears out is worth now, a
//! period at a time: the straight line, the sum of the years, the
//! declining balance and the double of it (the module `depreciation`).
//!
//! And a formula may ask the **price** of a security that is sold at a
//! discount and redeemed at its value — the discount, the price, the
//! yield, the interest and the bill (the module `securities`).
//!
//! And a formula may ask the **measure** of a thing: a number in
//! another unit, with the tree's units and its prefixes (the module
//! `units`).
//!
//! And a formula may ask the **deviations** of a set, its **means**,
//! its **skew**, its **ranks** and its **percentiles** — the tree's two
//! ways of each (the module `statistics`).
//!
//! And a formula may ask about **two sets** of numbers at once: the
//! sums of their squares, the difference of the squares and the
//! squares of the differences (the module `statistics`).
//!
//! And a formula may ask the **words** of a number and the **letters**
//! of a text: Roman numerals, another base, a character and its code,
//! and a text written for a URL (the module `words`).
//!
//! And a formula may ask for the **value of a text**: a text that
//! holds a number, a date or a time — with the tree's careful rules
//! about signs, currencies, separators and parentheses (the module
//! `textvalue`).
//!
//! And a formula may ask about the **cash flows** of a plan: the rate
//! at which a series of payments is worth nothing, with dates or
//! without them, what a sum grows to through a schedule, and the
//! interest and the principal of a run of payments (the module
//! `cashflows`).
//!
//! And a formula may ask the **shape** of a distribution: the normal,
//! the gamma, the error function, Student's t, the chi-square, the F,
//! the binomial, the Poisson, the exponential, the Weibull, the
//! lognormal and the beta (the module `distributions`, and the
//! maths of the module `special`).
//!
//! And a formula may ask how two sets of numbers **move together**:
//! the correlation, the slope of the line, where it crosses the axis,
//! its error and its prediction (the module `regression`).
//!
//! And a formula may ask about the **truth** of things and may choose
//! between them: `AND`, `OR`, `XOR`, `NOT`, `IFS`, `IFERROR`, `IFNA`,
//! `SWITCH` and `CHOOSE` (the module `logic`).
//!
//! And a formula may ask what a **value** is: whether it is a number
//! or text or an error, whether it is even or odd, and what kind of a
//! value it is (the module `information`).
//!
//! And a formula may ask about a number in another **base** — two,
//! eight and sixteen — or about the **bits** of a number (the module
//! `engineering`).
//!
//! And a formula may ask how many ways there are to **choose**: the
//! factorial, the combinations with and without repetition, the
//! permutations and the multinomial (the module `combinations`).
//!
//! And a formula may ask about a bond whose first or last coupon
//! period is **not a whole one** — the price and the yield of it (the
//! module `bonds`).
//!
//! And a formula may ask what interest a bond has **earned** between
//! two dates — over a year fraction, or over the coupon periods it
//! has been through (the module `bonds`).
//!
//! And a formula may ask about a **bond**: the days around its coupon
//! period, the price of it, the yield, the duration and the price of
//! one that is paid out (the module `bonds`). The **days** between two
//! dates — the tree's five bases — are the module `daycounts`, and the
//! securities and the bonds share them.
//!
//! And a formula may **choose**: `IF` takes a condition and the two
//! values it chooses between, and only the branch that is taken is
//! computed (the module `formula`).
//!
//! And a formula may **count** what meets a condition: `COUNTIF` and
//! `SUMIF` take a range of cells and a condition, and the condition is
//! a value — a text condition is a condition written out, with the
//! tree's wildcards in it (the module `criteria`).
//!
//! And a formula may speak of a **table**: a database of records and a
//! table of criteria, and the field it aggregates — `DSUM`,
//! `DAVERAGE`, `DCOUNT`, `DCOUNTA`, `DGET`, `DMAX`, `DMIN`,
//! `DPRODUCT`, `DVAR`, `DVARP`, `DSTDEV` and `DSTDEVP` (the module
//! `database`).
//!
//! And a formula may **test a hypothesis**: a t-test, a chi-square
//! test, an F-test and a z-test — the questions two sets of numbers
//! ask of each other (the module `hypotheses`).
//!
//! And a sheet may be **searched**: the cells whose text holds what
//! was looked for, with the tree's wildcards and the tree's rule about
//! the case (the module `finding`).
//!
//! And a cell may carry a **note**: a piece of text that says
//! something about the cell and is not the cell's value. The note
//! belongs to the sheet and names the cell, as the tree's comment
//! does (the module `notes`).
//!
//! And a cell may carry a **link**: a target that something else may
//! open, and whether the target is inside this workbook is derived
//! (the module `links`).
//!
//! And a formula may **shape** a text: take a part of it, count it,
//! fold it or clean it — `LEFT`, `RIGHT`, `MID`, `LEN`, `TRIM`,
//! `UPPER`, `LOWER` and `PROPER` (the module `texts`).
//!
//! And a formula may **find** a value in a range — `MATCH`, `VLOOKUP`
//! and `INDEX` — and the search is the **exact** one; the tree's
//! approximate search is not here, and a lookup that asks for it says
//! so (the module `lookups`).
//!
//! And a formula may **make a date** and take it apart — `DATE`,
//! `YEAR`, `MONTH` and `DAY` — in the date system of the workbook,
//! and may ask for the **week** of a day in its year (the module
//! `datetimes`).
//!
//! And a range may be **sorted**: the rows of it are put in order by
//! the values of one of its columns, and what a row holds travels
//! with it — the format, the style, the formula with its relative
//! references, the note and the link (the module `sorting`).
//!
//! And a sheet may be **protected**: then its cells are locked and
//! the sorting, the filtering and the formatting are the ones the
//! protection says (the module `protection`).
//!
//! And a workbook may carry **names**: a word that stands for a range of
//! cells, and a formula may use the word where it would use the range
//! (the module `names`).
//!
//! And a name may be **read**: the range it stands for and the values
//! of the cells of it, and the values are derived (the module
//! `namereading`).
//!
//! And a **sheet** may be read whole: its name, its used range, its
//! view, its zoom, the rows and columns that are hidden, its
//! protection and the calendar it counts its dates in (the module
//! `sheetreading`).
//!
//! And a **workbook** may be read whole: its sheets, its names, its
//! styles, its lists of words and its calendar (the module
//! `workbookreading`).
//!
//! And a sheet may carry **merges**: a range of cells that is one cell,
//! whose value is the value of the cell at the top left (the module
//! `merges`).
//!
//! And a sheet's rows have a **height** and its columns have a
//! **width** (the module `sizes`).
//!
//! And a row and a column may be **hidden** (the module `hiding`).
//!
//! And a sheet may carry a **filter**: a column and a criterion, and
//! the rows the filter hides are derived from it (the module
//! `filters`).
//!
//! And a sheet may carry **conditional formats**: a range, a criterion
//! and a format, and the cells that match are written with it (the
//! module `conditional`).
//!
//! # The mapping
//!
//! | the application | v2 |
//! |---|---|
//! | a workbook | a **node** of the kind `grid.workbook` |
//! | a sheet | a **node** of the kind `grid.sheet` |
//! | the cells | the sheet's **payload**: a sparse map |
//! | a formula | the cell's **payload**: the formula's text |
//! | a cell's value | **derived**: the evaluator computes it |
//! | the dependency graph | the sheet's **payload**, and the plugin's business |
//!
//! And a **range** may be **filled** from a cell or from a range of
//! cells: the tree's AutoFill, in its four modes — `copy`, `series`,
//! `extend` and `alternate` — which differ in what they do with a lone
//! number, with the tree's rule that the target holds the source at a
//! corner (the module `fill`).
//!
//! And a fill of a **formula** copies the formula and **shifts** its
//! references — the tree's `shift_relative`, and the tree's `$`: an
//! anchored reference does not move (the module `formula`).
//!
//! And a cell may carry a **number format**, and the **text** it shows
//! is derived from its value and that format: a format is the cell's
//! own, where the tree's format is a style and a style is shared (the
//! module `format`).
//!
//! And a format may be given by its **code** or by its **name**: the
//! tree's ribbon names its formats, and a name here says a code of
//! this tree — the names this tree does not write are refused with the
//! reason (the module `formatnames`).
//!
//! And a **style** is a named format of the workbook, and it is
//! **shared**: a cell may name a style, and a hundred cells that name
//! one style carry one format — and changing the style changes every
//! cell that names it, because the format of a cell is **derived**
//! (the module `styles`).
//!
//! And a cell may hold a **date**, which is a **number** and a
//! **format**: the number says how many days have passed and the
//! format says that the number is a date. The tree's calendar is here,
//! and it has the day in it that never happened (the module `dates`).
//!
//! And a column may be **flash filled**: the rows that are filled are
//! the **examples**, and what the examples say is a **program** —
//! literals, tokens of the source and the first letter of a token —
//! that is then run on the rows that are empty (the module `flash`).
//!
//! And a workbook may carry **lists of your own** — the words a fill
//! steps through — and the tree's own lists are the days of the week,
//! the months and the quarters (the module `fill`).
//!
//! And a workbook says which **calendar** its serials are counted in:
//! the 1900 system, with the day in it that never happened, or the
//! 1904 system, without one. The system is a fact of the **workbook**,
//! and every date in every sheet of it is read in that calendar (the
//! module `dates`).
//!
//! And a cell may hold a **time**, which is a **fraction of a day**
//! and a **format**: the number says how much of the day has gone and
//! the format says that the number is a time and how it is written.
//! The tree's clock shapes and its elapsed shapes are both here (the
//! modules `dates` and `format`).
//!
//! And a sheet has a **view** of its own: whether the gridlines and
//! the headings are drawn, whether the zeros are shown, whether the
//! sheet runs right to left, and how far it is zoomed — and it may
//! have **frozen panes**, the rows above a cell and the columns to its
//! left. The tree keeps these with the sheet and saves them with the
//! file, and the tree's freeze is the cell's own: freezing at `B2`
//! freezes the row above it and the column to its left, and freezing
//! at `A1` freezes nothing (the module `view`).
//!
//! And a sheet may be **hidden**, and its tab may be **coloured**: a
//! workbook must keep one visible sheet — the tree's rule, and a
//! workbook with no sheet to look at is not a workbook — and the
//! colour is a hex colour and not a name (the module `tabs`).
//!
//! And a range of cells may carry a **validation rule**: what may be
//! typed into it — a whole number, a decimal, a word from a list, a
//! date, a time, a length of text — with an operator and the
//! formulas that go with it. A rule is **written for the first cell**
//! of its range, and the relative references in it move with the cell
//! (the module `validation`).
//!
//! # The three things the port found
//!
//! - **A grid is not a tree, and the cells are in the payload.** A cell
//!   is not a node: a sheet has too many of them, and they have no
//!   children and no relations. So the sheet carries them, and the
//!   **payload is where the size is** — which is a thing a kit must be
//!   able to say out loud, because a document with a huge payload is a
//!   different kind of document from one with a huge tree.
//! - **A precedent is an area, not a thing.** The graph of a
//!   spreadsheet is a graph of **areas**: a formula reads a *range* of
//!   cells, and "what depends on this cell" is a question about areas.
//!   A kit whose relations are between nodes cannot hold that graph in
//!   its relations, so the graph lives in the payload and the plugin
//!   owns it — the same bargain the compositing battery made with its
//!   paths.
//! - **The value of a formula is derived, and the application stores
//!   it.** The application keeps the cached result beside the formula,
//!   because a spreadsheet shows numbers and must show them at once.
//!   Here the cell keeps **only** the formula, and the evaluator
//!   computes the value every time — so a cell's value cannot be wrong,
//!   because it is never written down. The price is that every value is
//!   computed every time, and the price is paid knowingly.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod arithmetic;
mod bonds;
mod cashflows;
mod cellread;
mod clearing;
mod combinations;
mod conditional;
mod criteria;
mod database;
mod dates;
mod datetimes;
mod daycounts;
mod depreciation;
mod distributions;
mod engineering;
mod fill;
mod filters;
mod finance;
mod finding;
mod flash;
mod format;
mod formatnames;
mod formula;
mod fractions;
mod hiding;
mod hypotheses;
mod information;
mod links;
mod logic;
mod lookups;
mod merges;
mod namereading;
mod names;
mod notes;
mod protection;
mod reading;
mod regression;
mod removal;
mod sections;
mod securities;
mod sheetnames;
mod sheetreading;
mod sizes;
mod sorting;
mod special;
mod statistics;
mod styles;
mod tabs;
mod tabsorder;
mod texts;
mod textvalue;
mod trigonometry;
mod units;
mod validation;
mod view;
mod words;
mod workbookreading;

use craftkit_derive::{Command, Evaluator, Layer, NodeKind, Panel};
use craftkit_sdk::serde_json::{self, Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Session, Text};

pub use cellread::ReadCell;
pub use clearing::{ClearContents, ClearFormat};
pub use conditional::{CheckConditional, ClearConditional, SetConditional};
pub use criteria::Condition;
use fill::FillRange;
pub use filters::{ClearFilter, FilteredRows, SetFilter};
pub use finding::FindCells;
use flash::FlashFill;
use format::{DateStyle, Format, MAX_FORMAT_CHARS, TimeStyle};
use formula::{Book, CellRef, Cells, Value};
pub use hiding::{HideColumn, HideRow, ShowColumn, ShowRow};
pub use links::{DeleteLink, GetLink, SetLink};
pub use merges::{ClearMerge, SetMerge};
pub use namereading::ReadName;
pub use names::{DeleteName, SetName};
pub use notes::{DeleteNote, GetNote, SetNote};
pub use protection::{ProtectSheet, UnprotectSheet};
pub use reading::ReadRange;
pub use removal::RemoveSheet;
pub use sheetnames::RenameSheet;
pub use sheetreading::ReadSheet;
pub use sizes::{SetColumnWidth, SetRowHeight};
pub use sorting::SortRange;
pub use styles::{AddStyle, RemoveStyle, SetCellStyle, SetStyle};
pub use tabs::{HideSheet, SetSheetColor, UnhideSheet};
pub use tabsorder::MoveSheet;
pub use validation::{CheckValidation, ClearValidation, SetValidation};
pub use view::{FreezePanes, SetView, SetZoom, UnfreezePanes};
pub use workbookreading::ReadWorkbook;

/// The grid layer.
///
/// It needs the core, and it says so.
#[derive(Layer)]
#[layer(id = "grid", name = "Grid", requires("core"))]
pub struct GridLayer;

/// A **workbook**.
#[derive(NodeKind)]
#[node_kind(kind = "grid.workbook", label = "Workbook", layer = "grid", evaluates)]
pub struct WorkbookKind;

impl WorkbookKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a workbook **means**: its sheets.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({
            "sheets": node.children.iter().filter(|child| child.kind == "grid.sheet").count(),
        }))
    }
}

/// A **sheet**: a grid of cells, held in the payload.
#[derive(NodeKind)]
#[node_kind(kind = "grid.sheet", label = "Sheet", layer = "grid", evaluates)]
pub struct SheetKind;

impl SheetKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a sheet **means**: how many cells it has, and how many of
    /// them hold a formula.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        let cells = cells_of(node);
        let formulas = cells.iter().filter(|(_, cell)| cell.formula.is_some()).count();
        Some(json!({
            "cells": cells.len(),
            "formulas": formulas,
        }))
    }
}

/// The **workbook**: the sheets of the document, by name.
///
/// A formula may read another sheet, and the value of a cell is then a
/// fact about the workbook and not about the sheet: the sheets are
/// gathered here, once, and the evaluation reads them.
fn book_of(document: &Document) -> Book {
    // The **date system** is a fact of the workbook, and every sheet
    // of it is read in that calendar.
    let system = document
        .walk()
        .find(|node| node.kind == "grid.workbook")
        .and_then(|node| node.data.get("date_system").and_then(|system| system.as_str()))
        .and_then(dates::System::parse)
        .unwrap_or_default();
    let mut book = Book::in_system(system);
    for node in document.walk().filter(|node| node.kind == "grid.sheet") {
        book.add(&node.name, cells_of(node));
        // The **hidden rows** of the sheet: a subtotal that is the
        // hundred-and-more form leaves them out.
        let hidden: Vec<u64> = node
            .data
            .get("hidden_rows")
            .and_then(|rows| rows.as_array())
            .map(|rows| rows.iter().filter_map(|row| row.as_u64()).collect())
            .unwrap_or_default();
        book.hide_rows(&node.name, hidden);
        // The **filter** of the sheet: the rows it hides are derived
        // from it.
        if let Some(filter) = node.data.get("filter") {
            let column =
                filter.get("column").and_then(|column| column.as_str()).and_then(CellRef::column_index);
            let op = filter.get("op").and_then(|op| op.as_str()).unwrap_or("=").to_string();
            let value = filter.get("value").and_then(|value| value.as_str()).unwrap_or("").to_string();
            if let Some(column) = column {
                book.set_filter(
                    &node.name,
                    formula::Filter { column, criterion: formula::Criterion { op, value } },
                );
            }
        }
    }
    // The **names** of the workbook: what a name stands for.
    if let Some(workbook) = document.walk().find(|node| node.kind == "grid.workbook")
        && let Some(names) = workbook.data.get("names").and_then(|names| names.as_object())
    {
        for (name, refers_to) in names {
            let Some(text) = refers_to.as_str() else { continue };
            let Ok((sheet, area)) = names::parse_refers_to(text) else { continue };
            book.add_name(name, formula::Name { sheet, area });
        }
    }
    book
}

/// The cells of a sheet, from its payload.
fn cells_of(node: &Node) -> Cells {
    let mut cells = Cells::new();
    let Some(map) = node.data.get("cells").and_then(|cells| cells.as_object()) else {
        return cells;
    };
    for (address, entry) in map {
        let Some(cell) = CellRef::parse(address) else { continue };
        let formula = entry.get("formula").and_then(|formula| formula.as_str()).map(|text| text.to_string());
        let value = entry.get("value").map(Value::from_json);
        let format = entry.get("format").and_then(|format| format.as_str()).map(|text| text.to_string());
        let style = entry.get("style").and_then(|style| style.as_str()).map(|text| text.to_string());
        match (value, formula) {
            (_, Some(formula)) => cells.set_formula(cell, formula),
            (Some(value), None) => cells.set_value(cell, value),
            (None, None) => {}
        }
        if let Some(format) = format {
            cells.set_format(cell, Some(format));
        }
        if let Some(style) = style {
            cells.set_style(cell, Some(style));
        }
    }
    cells
}

/// The cells of a sheet, into its payload.
pub(crate) fn cells_into(cells: &Cells) -> Json {
    let mut map = serde_json::Map::new();
    for (cell, entry) in cells.iter() {
        let mut one = serde_json::Map::new();
        if let Some(formula) = &entry.formula {
            one.insert("formula".to_string(), Json::String(formula.clone()));
        }
        if let Some(value) = &entry.value {
            one.insert("value".to_string(), value.to_json());
        }
        if let Some(format) = &entry.format {
            one.insert("format".to_string(), Json::String(format.clone()));
        }
        if let Some(style) = &entry.style {
            one.insert("style".to_string(), Json::String(style.clone()));
        }
        map.insert(cell.to_text(), Json::Object(one));
    }
    Json::Object(map)
}

/// The cell at an address, or the error that says there is no such cell.
pub(crate) fn cell_at(text: &str, id: &str) -> Result<CellRef, EngineError> {
    CellRef::parse(text.trim())
        .ok_or_else(|| EngineError::Failed { id: id.to_string(), message: format!("`{text}` is not a cell") })
}

/// The cells of a sheet, from a node, to change them.
pub(crate) fn sheet_cells_mut(node: &mut Node) -> Cells {
    cells_of(node)
}

/// The **custom lists** of the workbook a sheet is in.
///
/// A custom list is a list of words a fill may step through, and the
/// lists belong to the **workbook** — as they do in the tree.
pub fn custom_lists_of(session: &Session, sheet: u64) -> Vec<Vec<String>> {
    let Some(book) =
        session.document().walk().find(|node| node.children.iter().any(|child| child.id == sheet))
    else {
        return Vec::new();
    };
    book.data
        .get("custom_lists")
        .and_then(|lists| lists.as_array())
        .map(|lists| {
            lists
                .iter()
                .filter_map(|list| {
                    let words: Vec<String> = list
                        .as_array()?
                        .iter()
                        .filter_map(|word| word.as_str().map(|word| word.to_string()))
                        .collect();
                    Some(words)
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **date system** of the workbook a sheet is in.
///
/// The system is a fact of the **workbook**, as it is in the tree: the
/// workbook says which calendar its serials are counted in, and every
/// date in every sheet of it is read in that calendar.
pub fn system_of(session: &Session, sheet: u64) -> dates::System {
    let Some(book) =
        session.document().walk().find(|node| node.children.iter().any(|child| child.id == sheet))
    else {
        return dates::System::default();
    };
    book.data
        .get("date_system")
        .and_then(|system| system.as_str())
        .and_then(dates::System::parse)
        .unwrap_or_default()
}

/// The **styles** of the workbook a sheet is in: their names and the
/// formats they carry.
///
/// A style is a **named format** of the workbook, and it is shared: a
/// hundred cells that name one style carry one format.
pub fn styles_of(session: &Session, sheet: u64) -> Vec<(String, String)> {
    let Some(book) =
        session.document().walk().find(|node| node.children.iter().any(|child| child.id == sheet))
    else {
        return Vec::new();
    };
    book.data
        .get("styles")
        .and_then(|styles| styles.as_array())
        .map(|styles| {
            styles
                .iter()
                .filter_map(|style| {
                    let name = style.get("name")?.as_str()?.to_string();
                    let format = style.get("format")?.as_str()?.to_string();
                    Some((name, format))
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The **format** a cell is written with.
///
/// A cell may name a **style** of the workbook, and the style carries a
/// format: the cell is then written with the style's format.
///
/// The tree's rule: a style whose format is *General* does not change
/// the cell's format — the cell keeps its own. And a cell that names a
/// style that is not there keeps its own format as well.
pub fn format_of(session: &Session, sheet: u64, cells: &Cells, cell: CellRef) -> Format {
    match format_code_of(session, sheet, cells, cell) {
        Some(code) => Format::parse(&code).unwrap_or(Format::General),
        None => Format::General,
    }
}

/// The **format code** a cell is written with.
///
/// The same rule as the format itself: the code of the style the cell
/// names, if it names one that has a format, and the cell's own code
/// otherwise.
pub fn format_code_of(session: &Session, sheet: u64, cells: &Cells, cell: CellRef) -> Option<String> {
    if let Some(name) = cells.style(cell) {
        let styles = styles_of(session, sheet);
        if let Some((_, code)) = styles.iter().find(|(style, _)| style == name) {
            let format = Format::parse(code).unwrap_or(Format::General);
            if format != Format::General {
                return Some(code.clone());
            }
        }
    }
    cells.format(cell).map(|code| code.to_string())
}

/// Put a **workbook** in the document.
#[derive(Command)]
#[command(id = "grid.workbook.add", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct AddWorkbook {
    /// What to call it.
    pub name: Text,
}

impl AddWorkbook {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.clone();
        if name.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "grid.workbook.add".to_string(),
                message: "a workbook has a name".to_string(),
            });
        }
        // A workbook carries the **date system** its serials are
        // counted in, as the tree's workbook does.
        let id = session.document_mut()?.add_node_with_under(
            None,
            name.clone(),
            "grid.workbook",
            None,
            json!({
                "date_system": dates::System::default().name(),
                "custom_lists": Vec::<String>::new(),
                "styles": Vec::<String>::new(),
            }),
        );
        Ok(Outcome::changed(format!("added the workbook `{name}`"))
            .with_data(json!({ "id": id, "name": name })))
    }
}

/// **Add** a **custom list** to a workbook.
///
/// A custom list is a list of words a fill may step through: the
/// tree's own lists are the days of the week, the months and the
/// quarters, and a custom list is one of your own.
///
/// The words are given **one per line**, because a word is a line and
/// a line may not be two words. A list of **one** word is not a list,
/// and two words that are the same are one word twice — so both are
/// refused.
#[derive(Command)]
#[command(id = "grid.workbook.list.add", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct AddCustomList {
    /// The workbook the list belongs to.
    pub workbook: NodeRef,
    /// The words of the list, one per line.
    pub words: Text,
}

impl AddCustomList {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.workbook.list.add".to_string();
        let workbook = self.workbook.0;
        let Some(node) = session.document().node(workbook) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no workbook `{workbook}`"),
            });
        };
        if node.kind != "grid.workbook" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
            });
        }
        let words: Vec<String> = self
            .words
            .0
            .lines()
            .map(|line| line.trim().to_string())
            .filter(|line| !line.is_empty())
            .collect();
        if words.len() < 2 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a list of one word is not a list".to_string(),
            });
        }
        // The tree's list is found by **membership**, so two words
        // that are the same are one word twice: the fill would find
        // the first and never the second.
        for (index, word) in words.iter().enumerate() {
            if words[..index].iter().any(|other| other.eq_ignore_ascii_case(word)) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{word}` is in the list twice, and a fill would find the first of them"
                    ),
                });
            }
        }
        let mut lists =
            node.data.get("custom_lists").and_then(|lists| lists.as_array()).cloned().unwrap_or_default();
        lists.push(json!(words));
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["custom_lists"] = json!(lists);
        }
        Ok(Outcome::changed(format!("the workbook `{workbook}` has a list of {} words", words.len()))
            .with_data(json!({ "workbook": workbook, "words": words })))
    }
}

/// **Take away** a **custom list** of a workbook.
///
/// The list is named by its **place**: the first list, the second
/// list, and so on.
#[derive(Command)]
#[command(id = "grid.workbook.list.remove", area = "grid", layer = "grid", icon = "minus", mutates, undoable)]
pub struct RemoveCustomList {
    /// The workbook the list belongs to.
    pub workbook: NodeRef,
    /// Which list: the first is 1, the second is 2, and so on.
    pub list: Number,
}

impl RemoveCustomList {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.workbook.list.remove".to_string();
        let workbook = self.workbook.0;
        let Some(node) = session.document().node(workbook) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no workbook `{workbook}`"),
            });
        };
        if node.kind != "grid.workbook" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
            });
        }
        let which = self.list.0;
        if !which.is_finite() || which.fract() != 0.0 || which < 1.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a list is named by its place, and the first list is the first".to_string(),
            });
        }
        let which = which as usize - 1;
        let mut lists =
            node.data.get("custom_lists").and_then(|lists| lists.as_array()).cloned().unwrap_or_default();
        if which >= lists.len() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the workbook `{workbook}` has {} lists", lists.len()),
            });
        }
        let removed = lists.remove(which);
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["custom_lists"] = json!(lists);
        }
        Ok(Outcome::changed(format!("the workbook `{workbook}` has one list less"))
            .with_data(json!({ "workbook": workbook, "list": which + 1, "removed": removed })))
    }
}

/// **Set** the **date system** of a workbook.
///
/// The system is the calendar the workbook's serials are counted in:
/// the `1900` system counts from the 31st of December 1899 and has the
/// 29th of February of 1900 in it — the day that never happened — and
/// the `1904` system counts from the 1st of January 1904 and has no
/// such day.
///
/// The two are 1462 days apart, so every date in the workbook moves.
#[derive(Command)]
#[command(
    id = "grid.workbook.date.system",
    area = "grid",
    layer = "grid",
    icon = "pencil",
    mutates,
    undoable
)]
pub struct SetDateSystem {
    /// The workbook.
    pub workbook: NodeRef,
    /// `1900` or `1904`.
    pub system: Text,
}

impl SetDateSystem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.workbook.date.system".to_string();
        let workbook = self.workbook.0;
        let Some(node) = session.document().node(workbook) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no workbook `{workbook}`"),
            });
        };
        if node.kind != "grid.workbook" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{workbook}` is a `{}` and not a workbook", node.kind),
            });
        }
        let system = dates::System::parse(&self.system.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!(
                "`{}` is not a date system: a date system is `1900` or `1904`",
                self.system.0.trim()
            ),
        })?;
        if let Some(node) = session.document_mut()?.node_mut(workbook) {
            node.data["date_system"] = json!(system.name());
        }
        Ok(Outcome::changed(format!(
            "the workbook `{workbook}` counts its days in the {} system",
            system.name()
        ))
        .with_data(json!({ "workbook": workbook, "system": system.name() })))
    }
}

/// Put a **sheet** in a workbook.
#[derive(Command)]
#[command(id = "grid.sheet.add", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct AddSheet {
    /// The workbook it goes in.
    pub workbook: NodeRef,
    /// What to call it.
    pub name: Text,
}

impl AddSheet {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        let workbook = self.workbook.0;
        // The **rules** of a sheet's name are the rules of the name and
        // not of the command that writes it: the tree's `check_sheet_name`
        // is used when a sheet is added and when it is renamed, and here
        // they are one rule with two doors.
        //
        // The uniqueness is the tree's: a name is taken **in the
        // workbook**, and two workbooks may hold sheets of one name.
        let taken: Vec<String> = session
            .document()
            .node(workbook)
            .map(|node| {
                node.children
                    .iter()
                    .filter(|child| child.kind == "grid.sheet")
                    .map(|child| child.name.clone())
                    .collect()
            })
            .unwrap_or_default();
        let taken = taken.iter().find(|other| other.eq_ignore_ascii_case(&name)).cloned();
        if let Some(reason) = sheetnames::why_not(&name, taken.as_deref()) {
            return Err(EngineError::Failed { id: "grid.sheet.add".to_string(), message: reason });
        }
        if session.document().node(workbook).is_none() {
            return Err(EngineError::Failed {
                id: "grid.sheet.add".to_string(),
                message: format!("there is no workbook `{workbook}`"),
            });
        }
        let id = session.document_mut()?.add_node_with_under(
            Some(workbook),
            name.clone(),
            "grid.sheet",
            None,
            json!({
                "cells": {},
                "view": crate::view::SheetView::default().to_json(),
                "zoom": 100,
            }),
        );
        Ok(Outcome::changed(format!("added the sheet `{name}`")).with_data(json!({ "id": id, "name": name })))
    }
}

/// Put a **value** in a cell.
#[derive(Command)]
#[command(id = "grid.cell.set", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct SetCell {
    /// The sheet it goes in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// The value: a number, a text or a boolean.
    pub value: Json,
}

impl SetCell {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, "grid.cell.set")?;
        let value = Value::from_json(&self.value);
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: "grid.cell.set".to_string(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        if node.kind != "grid.sheet" {
            return Err(EngineError::Failed {
                id: "grid.cell.set".to_string(),
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        merges::refuse_covered(node, &[cell], "grid.cell.set")?;
        crate::protection::refuse_locked(node, &[cell], "grid.cell.set")?;
        let mut cells = sheet_cells_mut(node);
        cells.set_value(cell, value.clone());
        node.data["cells"] = cells_into(&cells);
        Ok(Outcome::changed(format!("set the cell {}", cell.to_text()))
            .with_data(json!({ "sheet": sheet, "cell": cell.to_text(), "value": value.to_json() })))
    }
}

/// Put a **number format** on a cell.
///
/// The formats this tree knows are `general`, `0`, `0.00`, `#,##0`,
/// `#,##0.00`, `0%` and `0.00%`. A format that is **not one of them**
/// is refused: a format that is quietly ignored is a format the user
/// believes in and the program does not have.
///
/// A cell may be formatted **before it has a value**, and the format
/// is the cell's own: the tree's format is a style and a style is
/// shared, and here a hundred cells with one format carry a hundred
/// formats.
#[derive(Command)]
#[command(id = "grid.cell.format", area = "grid", layer = "grid", icon = "pencil", mutates, undoable)]
pub struct SetFormat {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// The format: `general`, `0`, `0.00`, `#,##0`, `#,##0.00`, `0%` or
    /// `0.00%` — or the **name** of a format, such as `currency` or
    /// `scientific`.
    pub format: Text,
}

impl SetFormat {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.format".to_string();
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, &id)?;
        let given = self.format.0.trim().to_string();
        if given.chars().count() > MAX_FORMAT_CHARS {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a format is at most {MAX_FORMAT_CHARS} characters long"),
            });
        }
        // A format is given by its **code** or by its **name**, and the
        // code is kept: the name is a way of saying the code.
        let code = match formatnames::resolve(&given) {
            Ok(code) => code,
            Err(refusal) => {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: formatnames::message(&given, &refusal),
                });
            }
        };
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        if node.kind != "grid.sheet" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        crate::protection::refuse_format(node, &id)?;
        let mut cells = sheet_cells_mut(node);
        cells.set_format(cell, Some(code.clone()));
        node.data["cells"] = cells_into(&cells);
        Ok(Outcome::changed(format!("the cell {} is formatted `{code}`", cell.to_text()))
            .with_data(json!({ "sheet": sheet, "cell": cell.to_text(), "format": code })))
    }
}

/// Put a **date** in a cell.
///
/// A date in a spreadsheet is a **number** and a **format**: the number
/// is the number of days since the 30th of December 1899, and the
/// format says that the number is a date and how it is written. This
/// command puts both of them in the cell.
///
/// The date is written as `2026-10-26`, and a date that did not
/// happen — the 30th of February — is refused.
#[derive(Command)]
#[command(id = "grid.cell.date", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct SetDate {
    /// The sheet it goes in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// The date, as `2026-10-26`.
    pub date: Text,
    /// The format the date is written with. The default is
    /// `yyyy-mm-dd`, and a format that is not a date format is
    /// refused.
    pub format: Option<Text>,
}

impl SetDate {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.date".to_string();
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, &id)?;
        // The **date system** of the workbook the sheet is in: the
        // date is written in that calendar and the serial is that
        // calendar's.
        let system = system_of(session, sheet);
        let (date, shape) = dates::parse(&self.date.0, system).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!(
                "`{}` is not a date: a date is written as 2026-10-26, 10/26/2026 or 4-Mar-2026, and it is a day that happened",
                self.date.0.trim()
            ),
        })?;
        let serial = dates::to_serial(date.year, date.month, date.day, system).ok_or_else(|| {
            EngineError::Failed { id: id.clone(), message: format!("`{}` is not a date", self.date.0.trim()) }
        })?;
        let style = match &self.format {
            Some(code) => {
                // A date format is given by its **code** or by its
                // **name**, and the name says a date shape.
                let given = code.0.trim().to_string();
                let said = match crate::formatnames::resolve(&given) {
                    Ok(code) => code,
                    Err(refusal) => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: crate::formatnames::message(&given, &refusal),
                        });
                    }
                };
                DateStyle::parse(&said).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{said}` is not a date format: the date formats are `yyyy-mm-dd`, `dd/mm/yyyy`, `m/d/yyyy`, `d-mmm-yyyy`, `mmm`, `mmmm`, `ddd`, `dddd` and `yyyy`"
                    ),
                })?
            }
            // The **shape** the date was written in is the format it is
            // written with, as the tree has it: a date written as
            // `10/26/2026` is written as `10/26/2026`.
            None => DateStyle::parse(shape).unwrap_or(DateStyle::Iso),
        };
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        if node.kind != "grid.sheet" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        merges::refuse_covered(node, &[cell], &id)?;
        crate::protection::refuse_locked(node, &[cell], &id)?;
        let mut cells = sheet_cells_mut(node);
        cells.set_value(cell, Value::Number(serial));
        cells.set_format(cell, Some(style.code().to_string()));
        node.data["cells"] = cells_into(&cells);
        Ok(Outcome::changed(format!("set the cell {} to the date {}", cell.to_text(), date.to_text()))
            .with_data(json!({
                "sheet": sheet,
                "cell": cell.to_text(),
                "date": date.to_text(),
                "serial": serial,
                "format": style.code(),
            })))
    }
}

/// Put a **time** in a cell.
///
/// A time in a spreadsheet is a **number** and a **format**: the number
/// is the fraction of the day the time is, and the format says that the
/// number is a time and how it is written.
///
/// The time is written as `13:45`, `13:45:30` or `1:45 pm`, and a time
/// that did not happen — the 13th of the 60th minute — is refused.
#[derive(Command)]
#[command(id = "grid.cell.time", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct SetTime {
    /// The sheet it goes in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// The time, as `13:45` or `1:45 pm`.
    pub time: Text,
    /// The format the time is written with. The default is the format
    /// the time was written in, and a format that is not a time format
    /// is refused.
    pub format: Option<Text>,
}

impl SetTime {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.time".to_string();
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, &id)?;
        let (time, shape) = dates::parse_time(&self.time.0).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!(
                "`{}` is not a time: a time is written as 13:45, 13:45:30 or 1:45 pm, and it is a time that happened",
                self.time.0.trim()
            ),
        })?;
        let style = match &self.format {
            Some(code) => {
                // A time format is given by its **code** or by its
                // **name**, and the name says a clock shape.
                let given = code.0.trim().to_string();
                let said = match crate::formatnames::resolve(&given) {
                    Ok(code) => code,
                    Err(refusal) => {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: crate::formatnames::message(&given, &refusal),
                        });
                    }
                };
                TimeStyle::parse(&said).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{said}` is not a time format: the time formats are `h`, `hh`, `h:mm`, `hh:mm`, `h:mm:ss`, `hh:mm:ss`, `h AM/PM`, `h:mm AM/PM`, `h:mm:ss AM/PM`, `hh:mm:ss AM/PM`, `[h]:mm`, `[h]:mm:ss` and `[mm]:ss`"
                    ),
                })?
            }
            // The **shape** the time was written in is the format it is
            // written with, as the tree has it: a time written as
            // `1:45 pm` is written as `1:45 pm`.
            None => TimeStyle::parse(shape).unwrap_or(TimeStyle::HourMinute),
        };
        let serial = dates::time_fraction(time.hour, time.minute, time.second);
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        if node.kind != "grid.sheet" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        merges::refuse_covered(node, &[cell], &id)?;
        crate::protection::refuse_locked(node, &[cell], &id)?;
        let mut cells = sheet_cells_mut(node);
        cells.set_value(cell, Value::Number(serial));
        cells.set_format(cell, Some(style.code().to_string()));
        node.data["cells"] = cells_into(&cells);
        Ok(Outcome::changed(format!("set the cell {} to the time {}", cell.to_text(), time.to_text()))
            .with_data(json!({
                "sheet": sheet,
                "cell": cell.to_text(),
                "time": time.to_text(),
                "serial": serial,
                "format": style.code(),
            })))
    }
}

/// The **text** a cell shows.
///
/// The text is the **value** of the cell written with its format, and
/// the value of a formula cell is computed: a cell that holds a
/// formula has no value until it is calculated, and this is the
/// calculation.
#[derive(Command)]
#[command(id = "grid.cell.text", area = "grid", layer = "grid", icon = "list")]
pub struct CellText {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
}

impl CellText {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.cell.text".to_string();
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, &id)?;
        let Some(node) = session.document().node(sheet) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        if node.kind != "grid.sheet" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        let name = node.name.clone();
        let cells = cells_of(node);
        // The **value**: the one the cell holds, or the one its formula
        // computes.
        let value = match cells.get(cell) {
            Some(entry) => match (&entry.value, &entry.formula) {
                (Some(value), _) => value.clone(),
                (None, Some(_)) => {
                    let book = book_of(session.document());
                    book.value_of(&name, cell)
                }
                (None, None) => Value::Empty,
            },
            None => Value::Empty,
        };
        // The **format** the cell is written with: the **conditional**
        // format that matches it, if there is one — a rule is on top
        // of a fact — and otherwise the format of the style it names,
        // if it names one, and its own otherwise.
        let format = match conditional::format_for(node, cell, &value) {
            Some(code) => Format::parse(&code).unwrap_or(Format::General),
            None => format_of(session, sheet, &cells, cell),
        };
        let text = format.text(&value, system_of(session, sheet));
        Ok(Outcome::new(format!("{} shows `{text}`", cell.to_text())).with_data(
            json!({ "sheet": sheet, "cell": cell.to_text(), "text": text, "value": value.to_json() }),
        ))
    }
}

/// Put a **formula** in a cell.
///
/// A formula that does not parse is refused, and so is one that reads
/// itself: the cycle is found **here**, and not later in the middle of
/// a recalculation.
#[derive(Command)]
#[command(id = "grid.formula.set", area = "grid", layer = "grid", icon = "plus", mutates, undoable)]
pub struct SetFormula {
    /// The sheet it goes in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
    /// The formula, with or without its `=`.
    pub formula: Text,
}

impl SetFormula {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, "grid.formula.set")?;
        let text = self.formula.0.trim().to_string();
        let expression = formula::parse(&text).map_err(|error| EngineError::Failed {
            id: "grid.formula.set".to_string(),
            message: error.message,
        })?;
        let Some(node) = session.document().node(sheet) else {
            return Err(EngineError::Failed {
                id: "grid.formula.set".to_string(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        if node.kind != "grid.sheet" {
            return Err(EngineError::Failed {
                id: "grid.formula.set".to_string(),
                message: format!("the node `{sheet}` is not a sheet"),
            });
        }
        merges::refuse_covered(node, &[cell], "grid.formula.set")?;
        crate::protection::refuse_locked(node, &[cell], "grid.formula.set")?;
        let name = node.name.clone();
        let mut cells = cells_of(node);
        // The cycle check: with the new formula in place, does the cell
        // read itself — in this sheet or in another?
        cells.set_formula(cell, text.clone());
        let mut book = book_of(session.document());
        if let Some(sheet_cells) = book.sheet_mut(&name) {
            *sheet_cells = cells.clone();
        }
        if book.reads_itself(&name, cell) {
            return Err(EngineError::Failed {
                id: "grid.formula.set".to_string(),
                message: format!("the formula in {name}!{} reads itself", cell.to_text()),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: "grid.formula.set".to_string(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        node.data["cells"] = cells_into(&cells);
        let precedents = formula::precedents(&expression, book.names());
        Ok(Outcome::changed(format!("set the formula in {}", cell.to_text())).with_data(json!({
            "sheet": sheet,
            "cell": cell.to_text(),
            "formula": text,
            "precedents": precedents
                .iter()
                .map(|area| area.to_text())
                .collect::<Vec<_>>(),
        })))
    }
}

/// Empty a **cell**.
#[derive(Command)]
#[command(id = "grid.cell.clear", area = "grid", layer = "grid", icon = "minus", mutates, undoable)]
pub struct ClearCell {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
}

impl ClearCell {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, "grid.cell.clear")?;
        let Some(node) = session.document_mut()?.node_mut(sheet) else {
            return Err(EngineError::Failed {
                id: "grid.cell.clear".to_string(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        crate::protection::refuse_locked(node, &[cell], "grid.cell.clear")?;
        let mut cells = sheet_cells_mut(node);
        let was_there = cells.clear(cell);
        node.data["cells"] = cells_into(&cells);
        // The **note** and the **link** go with the cell: the tree's
        // *clear all* takes the comment and the hyperlink of the cell
        // with it.
        let mut notes = notes::notes_of(node);
        let had_note = notes.remove(&cell.to_text()).is_some();
        node.data["notes"] = notes::notes_into(&notes);
        let mut links = links::links_of(node);
        let had_link = links.remove(&cell.to_text()).is_some();
        node.data["links"] = links::links_into(&links);
        Ok(Outcome::changed(format!("emptied the cell {}", cell.to_text())).with_data(json!({
            "sheet": sheet,
            "cell": cell.to_text(),
            "was_there": was_there,
            "had_note": had_note,
            "had_link": had_link,
        })))
    }
}

/// The **precedents** of a cell: the areas its formula reads.
#[derive(Command)]
#[command(id = "grid.precedents", area = "grid", layer = "grid", icon = "list")]
pub struct ReadPrecedents {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
}

impl ReadPrecedents {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, "grid.precedents")?;
        let Some(node) = session.document().node(sheet) else {
            return Err(EngineError::Failed {
                id: "grid.precedents".to_string(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let cells = cells_of(node);
        let Some(entry) = cells.get(cell) else {
            return Err(EngineError::Failed {
                id: "grid.precedents".to_string(),
                message: format!("the cell {} is empty", cell.to_text()),
            });
        };
        let Some(formula) = &entry.formula else {
            return Err(EngineError::Failed {
                id: "grid.precedents".to_string(),
                message: format!("the cell {} holds no formula", cell.to_text()),
            });
        };
        let expression = formula::parse(formula).map_err(|error| EngineError::Failed {
            id: "grid.precedents".to_string(),
            message: error.message,
        })?;
        let book = book_of(session.document());
        let areas = formula::precedents(&expression, book.names());
        Ok(Outcome::new(format!("{} area(s)", areas.len())).with_data(json!({
            "sheet": sheet,
            "cell": cell.to_text(),
            "precedents": areas
                .iter()
                .map(|reference| json!({
                    "area": reference.area.to_text(),
                    "sheet": reference.sheet,
                    "cells": reference.area.count(),
                }))
                .collect::<Vec<_>>(),
        })))
    }
}

/// The **dependents** of a cell: the cells whose formulas read it.
#[derive(Command)]
#[command(id = "grid.dependents", area = "grid", layer = "grid", icon = "list")]
pub struct ReadDependents {
    /// The sheet it is in.
    pub sheet: NodeRef,
    /// The cell's address.
    pub cell: Text,
}

impl ReadDependents {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let sheet = self.sheet.0;
        let cell = cell_at(&self.cell.0, "grid.dependents")?;
        let Some(node) = session.document().node(sheet) else {
            return Err(EngineError::Failed {
                id: "grid.dependents".to_string(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let name = node.name.clone();
        // The dependents of a cell are the cells that read it, in this
        // sheet **or in another**: a formula in `Sheet2` that reads
        // `Sheet1!A1` depends on it.
        let book = book_of(session.document());
        let mut dependents: Vec<Json> = Vec::new();
        for sheet_node in session.document().walk().filter(|node| node.kind == "grid.sheet") {
            let cells = cells_of(sheet_node);
            for (other, entry) in cells.iter() {
                let Some(formula) = &entry.formula else { continue };
                let Ok(expression) = formula::parse(formula) else { continue };
                for reference in formula::precedents(&expression, book.names()) {
                    // A reference in another sheet does not read this cell.
                    if reference.sheet.as_deref().is_some_and(|other| other != name) {
                        continue;
                    }
                    if reference.area.covers(cell) {
                        dependents.push(json!({
                            "sheet": sheet_node.name,
                            "cell": other.to_text(),
                            "reads": reference.to_text(),
                        }));
                        break;
                    }
                }
            }
        }
        let count = dependents.len();
        Ok(Outcome::new(format!("{count} dependent(s)")).with_data(json!({
            "sheet": sheet,
            "cell": cell.to_text(),
            "dependents": dependents,
        })))
    }
}

/// The **values** of a sheet: every formula, computed.
///
/// The same thing the evaluator says, as a command: an agent asks for
/// the numbers.
#[derive(Command)]
#[command(id = "grid.calc", area = "grid", layer = "grid", icon = "list")]
pub struct CalcSheet {
    /// The sheet to calculate.
    pub sheet: NodeRef,
}

impl CalcSheet {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let sheet = self.sheet.0;
        let Some(node) = session.document().node(sheet) else {
            return Err(EngineError::Failed {
                id: "grid.calc".to_string(),
                message: format!("there is no sheet `{sheet}`"),
            });
        };
        let name = node.name.clone();
        let cells = cells_of(node);
        let book = book_of(session.document());
        let mut values = serde_json::Map::new();
        for (cell, entry) in cells.iter() {
            if entry.formula.is_none() {
                continue;
            }
            values.insert(cell.to_text(), book.value_of(&name, cell).to_json());
        }
        let count = values.len();
        Ok(Outcome::new(format!("{count} value(s)")).with_data(json!({
            "sheet": sheet,
            "cells": Json::Object(values),
        })))
    }
}

/// The evaluator: the **values** of a sheet.
///
/// Every formula is computed, every time, from the cells it reads. No
/// value is stored, so no value can be wrong.
#[derive(Evaluator)]
#[evaluator(id = "grid.calc", label = "Calc", layer = "grid")]
pub struct CalcEvaluator;

impl CalcEvaluator {
    /// The values of every sheet that has a formula in it.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for node in document.walk() {
            if node.kind != "grid.sheet" {
                continue;
            }
            let cells = cells_of(node);
            let book = book_of(document);
            let mut values = serde_json::Map::new();
            for (cell, entry) in cells.iter() {
                if entry.formula.is_none() {
                    continue;
                }
                let value = book.value_of(&node.name, cell);
                values.insert(cell.to_text(), value.to_json());
            }
            plan.items.push(PlanItem {
                node: node.id,
                kind: "grid.calc".to_string(),
                value: json!({ "cells": Json::Object(values) }),
            });
        }
        plan
    }
}

/// The panel that shows the workbook.
#[derive(Panel)]
#[panel(
    id = "panel.grid",
    label = "Grid",
    layer = "grid",
    commands(
        "grid.workbook.add",
        "grid.sheet.add",
        "grid.cell.set",
        "grid.formula.set",
        "grid.calc",
        "grid.range.fill"
    )
)]
pub struct GridPanel;

// The whole of the battery, on every target.
craftkit_sdk::plugin_anchor!(
    GridLayer,
    WorkbookKind,
    SheetKind,
    AddWorkbook,
    AddSheet,
    RenameSheet,
    MoveSheet,
    ClearContents,
    ClearFormat,
    DeleteLink,
    DeleteNote,
    FindCells,
    GetLink,
    GetNote,
    ReadCell,
    ReadRange,
    ProtectSheet,
    SetLink,
    SetNote,
    SortRange,
    UnprotectSheet,
    RemoveSheet,
    SetCell,
    SetFormula,
    SetName,
    DeleteName,
    SetMerge,
    ClearMerge,
    SetRowHeight,
    SetColumnWidth,
    HideRow,
    ShowRow,
    HideColumn,
    ShowColumn,
    SetFilter,
    ClearFilter,
    FilteredRows,
    SetConditional,
    ClearConditional,
    CheckConditional,
    ClearCell,
    ReadPrecedents,
    ReadDependents,
    CalcSheet,
    FillRange,
    FlashFill,
    SetFormat,
    SetTime,
    CellText,
    CalcEvaluator,
    GridPanel
);
