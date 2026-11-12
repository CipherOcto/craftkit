//! The **printing** of a sheet.
//!
//! A sheet is printed on **paper**: a size, an orientation and four
//! margins. And the sheet is broken into **pages** — the rows that go
//! on one page and the columns that go on it — and the pages are
//! **derived**: they are worked out from the paper and the sheet as
//! they are, and they are never written down.
//!
//! The **units** are the tree's own:
//!
//! - the paper is named by the tree's names — `Letter`, `A4`,
//!   `Tabloid` — and its size is in **points**, a seventy-second of an
//!   inch;
//! - the margins are in **inches**, because that is what the tree's
//!   dialogue asks for;
//! - a row's height is in points and a column's width is in
//!   characters, as they are everywhere else in this battery.
//!
//! The rules:
//!
//! - a paper that is **not one of the tree's papers** is refused: the
//!   tree falls back on Letter, and a list that says Letter when it
//!   was told A4 is a list that is wrong;
//! - a margin that is **not a number between zero and twenty inches**
//!   is refused: the tree clamps it, and a margin of thirty inches
//!   is not a margin;
//! - a sheet with **nothing in it** has **no pages**: the answer is
//!   nothing, and nothing is written as nothing.
//!
//! # What the port found
//!
//! - **The tree's paper is a name and its size is a table.** The tree
//!   falls back on Letter for a name it does not know. Here the
//!   names are the tree's names, the sizes are the tree's sizes, and
//!   a name that is not one of them is refused.
//! - **The tree's margins are clamped and here they are refused.** The
//!   tree's dialogue cannot say a margin of thirty inches, so the
//!   tree clamps it to twenty; here a margin outside the range is
//!   refused, because a margin that is not the one that was asked for
//!   is a margin that is wrong.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::formula::Area;

/// The **paper** the tree prints on: its name and its size in points.
///
/// The names are the tree's names, and the sizes are the standard
/// sizes of the papers they name.
pub const PAPERS: [(&str, f64, f64); 24] = [
    ("Letter", 612.0, 792.0),
    ("Legal", 612.0, 1008.0),
    ("A4", 595.28, 841.89),
    ("A3", 841.89, 1190.55),
    ("A5", 419.53, 595.28),
    ("A6", 297.64, 419.53),
    ("Tabloid", 792.0, 1224.0),
    ("Ledger", 1224.0, 792.0),
    ("Executive", 522.0, 756.0),
    ("B4", 728.5, 1031.81),
    ("B5", 515.91, 728.5),
    ("B6", 362.83, 515.91),
    ("B4 (ISO)", 708.66, 1000.63),
    ("B5 (ISO)", 498.9, 708.66),
    ("B6 (ISO)", 354.33, 498.9),
    ("Japanese Postcard", 283.46, 419.53),
    ("Japanese Double Postcard", 419.53, 566.93),
    ("Japanese Envelope #3", 340.16, 666.14),
    ("Japanese Envelope #4", 255.12, 581.1),
    ("Japanese Envelope Kaku #2", 680.31, 941.1),
    ("Japanese Envelope Kaku #3", 612.28, 785.2),
    ("A4 Extra", 620.0, 842.0),
    ("A3 Extra", 906.0, 1191.0),
    ("B4 Extra", 1016.0, 1452.0),
];

/// The **default** paper: the tree's default paper.
pub const DEFAULT_PAPER: &str = "Letter";

/// The **default** margins, in inches: the tree's **normal** preset.
pub const DEFAULT_MARGINS: [f64; 4] = [0.7, 0.7, 0.75, 0.75];

/// The **most** a margin may be, in inches: the tree's own bound.
pub const MAX_MARGIN: f64 = 20.0;

/// The **most** pages a sheet may be broken into.
const MAX_PAGES: usize = 4096;

/// A row's **height**, when it has not been set: in points.
pub(crate) const DEFAULT_ROW_HEIGHT: f64 = 15.0;

/// A column's **width**, when it has not been set: in characters.
const DEFAULT_COLUMN_WIDTH: f64 = 8.5;

/// The **width of a character**, in points: the tree's own.
const CHARACTER: f64 = 7.0;

/// The **padding of a cell**, in points: the tree's own.
const PADDING: f64 = 5.0;

/// The **setup** a sheet prints with.
#[derive(Debug, Clone)]
pub struct Setup {
    /// The name of the paper.
    pub paper: String,
    /// The width of the paper, in points, in portrait.
    pub width: f64,
    /// The height of the paper, in points, in portrait.
    pub height: f64,
    /// The orientation: `portrait` or `landscape`.
    pub orientation: String,
    /// The margins, in points: left, right, top and bottom.
    pub margins: [f64; 4],
    /// The **scale** of the print, in percent: from ten to four hundred.
    pub scale: f64,
    /// The **fit**: the number of pages wide and the number of pages
    /// tall, when the sheet is printed to fit rather than at a scale.
    pub fit: Option<(u16, u16)>,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup {
            paper: DEFAULT_PAPER.to_string(),
            width: 612.0,
            height: 792.0,
            orientation: "portrait".to_string(),
            margins: [
                DEFAULT_MARGINS[0] * 72.0,
                DEFAULT_MARGINS[1] * 72.0,
                DEFAULT_MARGINS[2] * 72.0,
                DEFAULT_MARGINS[3] * 72.0,
            ],
            scale: 100.0,
            fit: None,
        }
    }
}

impl Setup {
    /// The **setup** of a sheet, as it is kept.
    pub fn of(node: &Node) -> Setup {
        let mut setup = Setup::default();
        let Some(stored) = node.data.get("print") else { return setup };
        if let Some(paper) = stored.get("paper").and_then(|paper| paper.as_str())
            && let Some((_, width, height)) = paper_of(paper)
        {
            setup.paper = paper.to_string();
            setup.width = width;
            setup.height = height;
        }
        if let Some(orientation) = stored.get("orientation").and_then(|orientation| orientation.as_str())
            && (orientation == "portrait" || orientation == "landscape")
        {
            setup.orientation = orientation.to_string();
        }
        let margins = stored.get("margins");
        let number = |name: &str| {
            margins
                .and_then(|margins| margins.get(name))
                .and_then(|value| value.as_f64())
                .filter(|value| value.is_finite())
        };
        for (index, name) in ["left", "right", "top", "bottom"].iter().enumerate() {
            if let Some(margin) = number(name) {
                setup.margins[index] = margin;
            }
        }
        if let Some(scale) = stored.get("scale").and_then(|scale| scale.as_f64())
            && scale.is_finite()
            && (crate::scaling::MIN_SCALE..=crate::scaling::MAX_SCALE).contains(&scale)
        {
            setup.scale = scale;
        }
        if let Some(fit) = stored.get("fit")
            && let (Some(wide), Some(tall)) = (
                fit.get("wide").and_then(|wide| wide.as_u64()),
                fit.get("tall").and_then(|tall| tall.as_u64()),
            )
            && wide <= u64::from(u16::MAX)
            && tall <= u64::from(u16::MAX)
        {
            setup.fit = Some((wide as u16, tall as u16));
        }
        setup
    }

    /// The **setup**, as it is kept.
    pub fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        json!({
            "paper": self.paper,
            "orientation": self.orientation,
            "margins": {
                "left": self.margins[0],
                "right": self.margins[1],
                "top": self.margins[2],
                "bottom": self.margins[3],
            },
            "scale": self.scale,
            "fit": match self.fit {
                Some((wide, tall)) => json!({ "wide": wide, "tall": tall }),
                None => Json::Null,
            },
        })
    }

    /// The **size of the page**, in points: the paper as it is put on
    /// the table.
    pub fn page(&self) -> (f64, f64) {
        if self.orientation == "landscape" { (self.height, self.width) } else { (self.width, self.height) }
    }

    /// The **printable** size of the page, in points: the page less
    /// the margins.
    pub fn printable(&self) -> (f64, f64) {
        let (width, height) = self.page();
        (
            (width - self.margins[0] - self.margins[1]).max(36.0),
            (height - self.margins[2] - self.margins[3]).max(36.0),
        )
    }
}

/// The **paper** a name says, if it is one of the tree's papers.
///
/// The name is matched the way the tree matches it: spaces, hyphens,
/// underscores and brackets do not matter, so `b5`, `B5 (JIS)` and
/// `b5-jis` are one name.
pub fn paper_of(name: &str) -> Option<(&'static str, f64, f64)> {
    let wanted = name.trim().to_ascii_lowercase().replace([' ', '-', '_', '(', ')'], "");
    PAPERS
        .iter()
        .find(|(paper, _, _)| {
            let known = paper.to_ascii_lowercase().replace([' ', '-', '_', '(', ')'], "");
            known == wanted
        })
        .map(|(paper, width, height)| (*paper, *width, *height))
}

/// The **sheet** a command is about, and whether it is a sheet.
pub(crate) fn sheet_of(session: &Session, id: &str, sheet: u64) -> Result<Node, EngineError> {
    let Some(node) = session.document().node(sheet) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no sheet `{sheet}`"),
        });
    };
    if node.kind != "grid.sheet" {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
        });
    }
    Ok(node.clone())
}

/// The **height of a row**, in points.
pub(crate) fn row_height(node: &Node, row: u32) -> f64 {
    node.data
        .get("rows")
        .and_then(|rows| rows.get(row.to_string()))
        .and_then(|height| height.as_f64())
        .filter(|height| height.is_finite() && *height > 0.0)
        .unwrap_or(DEFAULT_ROW_HEIGHT)
}

/// The **width of a column**, in points.
pub(crate) fn column_width(node: &Node, column: u32) -> f64 {
    node.data
        .get("columns")
        .and_then(|columns| columns.get(crate::formula::CellRef::column_text(column)))
        .and_then(|width| width.as_f64())
        .filter(|width| width.is_finite() && *width > 0.0)
        .map(|width| width * CHARACTER + PADDING)
        .unwrap_or(DEFAULT_COLUMN_WIDTH * CHARACTER + PADDING)
}

/// The **breaks** of a sheet: the rows and the columns a page begins
/// at, put there by hand.
pub fn breaks_of(node: &Node) -> (Vec<u32>, Vec<u32>) {
    let list = |name: &str| -> Vec<u32> {
        node.data
            .get(name)
            .and_then(|breaks| breaks.as_array())
            .map(|breaks| breaks.iter().filter_map(|line| line.as_u64()).map(|line| line as u32).collect())
            .unwrap_or_default()
    };
    (list("row_breaks"), list("column_breaks"))
}

/// The **titles** of a sheet: the rows and the columns that are
/// printed on every page.
pub fn titles_of(node: &Node) -> (Vec<u32>, Vec<u32>) {
    let list = |name: &str| -> Vec<u32> {
        node.data
            .get(name)
            .and_then(|titles| titles.as_array())
            .map(|titles| titles.iter().filter_map(|line| line.as_u64()).map(|line| line as u32).collect())
            .unwrap_or_default()
    };
    (list("title_rows"), list("title_columns"))
}

/// The **area** a sheet prints: the area that was set, or nothing.
pub fn area_of(node: &Node) -> Option<Area> {
    let text = node.data.get("print_area").and_then(|area| area.as_str())?;
    let (_, area) = crate::merges::parse_area(text)?;
    Some(area)
}

/// The **bands** of a set of lines: the lines that go on one page.
///
/// A line that is wider than the page is on a page of its own, and a
/// **break** put in by hand starts a page there.
pub(crate) fn bands(lines: &[u32], size: impl Fn(u32) -> f64, room: f64, breaks: &[u32]) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    let mut index = 0usize;
    while index < lines.len() {
        let start = lines[index];
        let mut used = 0.0;
        let mut last = start;
        let mut taken = 0usize;
        while index + taken < lines.len() {
            let line = lines[index + taken];
            let length = size(line);
            let manual = taken > 0 && breaks.contains(&line);
            if taken > 0 && (manual || used + length > room + 0.01) {
                break;
            }
            used += length;
            last = line;
            taken += 1;
        }
        out.push((start, last));
        index += taken.max(1);
    }
    out
}

commands! {
    /// **Set** the paper of a sheet.
    grid.print.paper {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The paper: `Letter`, `A4`, `Tabloid` and the rest.

            paper: Text,
        },
        run: {

            let id = "grid.print.paper".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let Some((paper, _, _)) = paper_of(&self.paper.0) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("`{}` is not a paper this tree has", self.paper.0.trim()),
                });
            };
            let mut setup = Setup::of(&node);
            setup.paper = paper.to_string();
            let (width, height) =
                paper_of(paper).map(|(_, width, height)| (width, height)).unwrap_or((612.0, 792.0));
            setup.width = width;
            setup.height = height;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["print"] = setup.to_json();
            }
            Ok(Outcome::changed(format!("the sheet is printed on `{paper}`")).with_data(json!({
                "sheet": sheet,
                "paper": paper,
            })))

        }
    }
    /// **Set** the orientation of a sheet.
    grid.print.orientation {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The orientation: `portrait` or `landscape`.

            orientation: Text,
        },
        run: {

            let id = "grid.print.orientation".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let orientation = self.orientation.0.trim().to_ascii_lowercase();
            if orientation != "portrait" && orientation != "landscape" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("`{orientation}` is not an orientation: it is `portrait` or `landscape`"),
                });
            }
            let mut setup = Setup::of(&node);
            setup.orientation = orientation.clone();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["print"] = setup.to_json();
            }
            Ok(Outcome::changed(format!("the sheet is printed `{orientation}`")).with_data(json!({
                "sheet": sheet,
                "orientation": orientation,
            })))

        }
    }
    /// **Set** the margins of a sheet.
    ///
    /// The margins are in **inches**, the way the tree's dialogue asks for
    /// them, and a preset is the tree's own: `normal`, `wide` or
    /// `narrow`.
    grid.print.margins {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The preset: `normal`, `wide` or `narrow`.

            preset: Option<Text>,
            /// The **left** margin, in inches.

            left: Option<Number>,
            /// The **right** margin, in inches.

            right: Option<Number>,
            /// The **top** margin, in inches.

            top: Option<Number>,
            /// The **bottom** margin, in inches.

            bottom: Option<Number>,
        },
        run: {

            let id = "grid.print.margins".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let mut setup = Setup::of(&node);
            if let Some(preset) = &self.preset {
                let preset = preset.0.trim().to_ascii_lowercase();
                let margins: [f64; 4] = match preset.as_str() {
                    "normal" => [0.7, 0.7, 0.75, 0.75],
                    "wide" => [1.0, 1.0, 1.0, 1.0],
                    "narrow" => [0.25, 0.25, 0.75, 0.75],
                    _ => {
                        return Err(EngineError::Failed {
                            id,
                            message: format!("`{preset}` is not a preset: it is `normal`, `wide` or `narrow`"),
                        });
                    }
                };
                for (index, margin) in margins.iter().enumerate() {
                    setup.margins[index] = margin * 72.0;
                }
            }
            for (index, margin) in [self.left, self.right, self.top, self.bottom].iter().enumerate() {
                let Some(margin) = margin else { continue };
                let margin = margin.0;
                if !margin.is_finite() || !(0.0..=MAX_MARGIN).contains(&margin) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "{margin} is not a margin: a margin is a number between zero and {MAX_MARGIN} inches"
                        ),
                    });
                }
                setup.margins[index] = margin * 72.0;
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["print"] = setup.to_json();
            }
            Ok(Outcome::changed("the margins of the sheet are set".to_string()).with_data(json!({
                "sheet": sheet,
                "margins": setup.to_json()["margins"].clone(),
            })))

        }
    }
    /// **Read** the setup of a sheet.
    grid.print.read {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet.

            sheet: NodeRef,
        },
        run: {

            let id = "grid.print.read".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let setup = Setup::of(&node);
            let (page_width, page_height) = setup.page();
            let (printable_width, printable_height) = setup.printable();
            let (row_breaks, column_breaks) = breaks_of(&node);
            let (title_rows, title_columns) = titles_of(&node);
            let title_rows: Vec<u32> = title_rows.iter().map(|row| row + 1).collect();
            let title_columns: Vec<String> =
                title_columns.iter().map(|column| crate::formula::CellRef::column_text(*column)).collect();
            // The **scale** a fit works out to, when a fit is set.
            let fit_scale = crate::scaling::effective_scale(&setup, &node, &crate::scaling::Content::of(&node));
            Ok(Outcome::new(format!("the sheet is printed on `{}`", setup.paper)).with_data(json!({
                "sheet": sheet,
                "paper": setup.paper,
                "orientation": setup.orientation,
                "margins": setup.to_json()["margins"].clone(),
                "scale": setup.scale,
                "fit": match setup.fit {
                    Some((wide, tall)) => json!({ "wide": wide, "tall": tall }),
                    None => Json::Null,
                },
                "fit_scale": (fit_scale.0 * 10_000.0).round() / 100.0,
                "header": crate::headers::to_json(&node)["header"].clone(),
                "footer": crate::headers::to_json(&node)["footer"].clone(),
                "gridlines": crate::marks::gridlines_of(&node),
                "headings": crate::marks::headings_of(&node),
                "page": { "width": page_width, "height": page_height },
                "printable": { "width": printable_width, "height": printable_height },
                "area": area_of(&node).map(|area| json!(area.to_text())).unwrap_or(Json::Null),
                "title_rows": title_rows,
                "title_columns": title_columns,
                "row_breaks": row_breaks.iter().map(|row| row + 1).collect::<Vec<u32>>(),
                "column_breaks": column_breaks
                    .iter()
                    .map(|column| crate::formula::CellRef::column_text(*column))
                    .collect::<Vec<String>>(),
            })))

        }
    }
    /// **Read** the pages of a sheet.
    ///
    /// The pages are **derived**: they are worked out from the paper, the
    /// margins and the sizes of the rows and the columns.
    grid.print.pages {
        area: "grid",
        layer: "grid",
        icon: "list",
        params: {
            /// The sheet.

            sheet: NodeRef,
        },
        run: {

            let id = "grid.print.pages".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let setup = Setup::of(&node);
            let cells = crate::cells_of(&node);
            // The **area** that is printed: the one that was set, or the
            // used range of the sheet.
            let area = match area_of(&node) {
                Some(area) => area,
                None => match crate::reading::used_range(&cells) {
                    Some(area) => area,
                    None => {
                        return Ok(Outcome::new("the sheet holds nothing to print").with_data(json!({
                            "sheet": sheet,
                            "pages": Vec::<craftkit_sdk::serde_json::Value>::new(),
                            "count": 0,
                        })));
                    }
                },
            };
            let (printable_width, printable_height) = setup.printable();
            let (row_breaks, column_breaks) = breaks_of(&node);
            // The **titles**: the rows and the columns that are printed on
            // every page. They are not part of the pages themselves, so
            // they are left out of the bands.
            let (title_rows, title_columns) = titles_of(&node);
            let content_rows: Vec<u32> =
                (area.start.row..=area.end.row).filter(|row| !title_rows.contains(row)).collect();
            let content_columns: Vec<u32> =
                (area.start.col..=area.end.col).filter(|column| !title_columns.contains(column)).collect();
            // The **scale** the sheet prints at: the scale of the setup, or
            // the scale that fits it into the pages it was told to fit.
            let content = crate::scaling::Content { rows: content_rows, columns: content_columns };
            let (scale, fit_met) = crate::scaling::effective_scale(&setup, &node, &content);
            // A sheet that is **fitted** is broken by the fit and not by
            // hand: the fit is the way the pages are chosen.
            let (row_breaks, column_breaks) =
                if setup.fit.is_some() { (Vec::new(), Vec::new()) } else { (row_breaks, column_breaks) };
            // The **headings** take room from the page: the rows are one
            // heading shorter and the columns one heading narrower.
            let digits = (area.end.row + 1).to_string().len() as u32;
            let (heading_width, heading_height) = crate::marks::room_of(&node, digits);
            let rows = bands(
                &content.rows,
                |row| row_height(&node, row),
                (printable_height / scale - heading_height).max(1.0),
                &row_breaks,
            );
            let columns = bands(
                &content.columns,
                |column| column_width(&node, column),
                (printable_width / scale - heading_width).max(1.0),
                &column_breaks,
            );
            let mut bands_of_pages: Vec<((u32, u32), (u32, u32))> = Vec::new();
            for (first_row, last_row) in &rows {
                for (first_column, last_column) in &columns {
                    if bands_of_pages.len() >= MAX_PAGES {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the sheet is more than {MAX_PAGES} pages, and a sheet is not that long"
                            ),
                        });
                    }
                    bands_of_pages.push(((*first_row, *last_row), (*first_column, *last_column)));
                }
            }
            // The **header** and the **footer** of each page: the codes in
            // them are worked out for the page they are on.
            let count = bands_of_pages.len();
            let mut pages: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
            for (number, ((first_row, last_row), (first_column, last_column))) in
                bands_of_pages.iter().enumerate()
            {
                let (header, footer) = crate::headers::of_page(&node, number + 1, count);
                pages.push(json!({
                    "number": number + 1,
                    "rows": [first_row + 1, last_row + 1],
                    "columns": [
                        crate::formula::CellRef::column_text(*first_column),
                        crate::formula::CellRef::column_text(*last_column),
                    ],
                    "title_rows": title_rows.iter().map(|row| row + 1).collect::<Vec<u32>>(),
                    "title_columns": title_columns
                        .iter()
                        .map(|column| crate::formula::CellRef::column_text(*column))
                        .collect::<Vec<String>>(),
                    "header": header.to_json(),
                    "footer": footer.to_json(),
                    "gridlines": crate::marks::gridlines_of(&node),
                    "headings": crate::marks::headings_of(&node),
                }));
            }
            Ok(Outcome::new(format!("the sheet is {} page(s)", pages.len())).with_data(json!({
                "sheet": sheet,
                "area": area.to_text(),
                "scale": (scale * 10_000.0).round() / 100.0,
                "fit_met": fit_met,
                "pages": pages,
                "count": pages.len(),
            })))

        }
    }
    /// **Set** the area a sheet prints.
    ///
    /// Without a range the area is taken away, and the sheet prints the
    /// range it uses.
    grid.print.area {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The range that is printed: `A1:F50`. Without it, the sheet
            /// prints the range it uses.

            range: Option<Text>,
        },
        run: {

            let id = "grid.print.area".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, &id, sheet)?;
            let area = match &self.range {
                Some(range) => {
                    let text = range.0.trim().to_string();
                    let area = crate::fill::area_at(&text, &id)?;
                    if area.start == area.end {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!("the range {text} is one cell, and an area of one cell is a cell"),
                        });
                    }
                    Some(area)
                }
                None => None,
            };
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["print_area"] = match area {
                    Some(area) => json!(area.to_text()),
                    None => Json::Null,
                };
            }
            let text = area.map(|area| area.to_text()).unwrap_or_else(|| "the used range".to_string());
            Ok(Outcome::changed(format!("the sheet prints {text}")).with_data(json!({
                "sheet": sheet,
                "area": area.map(|area| json!(area.to_text())).unwrap_or(Json::Null),
            })))

        }
    }
    /// **Put** a break in a sheet, or take it away.
    ///
    /// A break is a place where a page **begins**, put there by hand. The
    /// row break is the row the page begins at and the column break is
    /// the column it begins at.
    grid.print.break {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The row the page begins at. Without it, the column is meant.

            row: Option<Number>,
            /// The column the page begins at, as `A`.

            column: Option<Text>,
            /// Whether the break is **taken away**.

            clear: Option<craftkit_sdk::Bool>,
        },
        run: {

            let id = "grid.print.break".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let clear = self.clear.map(|clear| clear.0).unwrap_or(false);
            match (&self.row, &self.column) {
                (Some(_), Some(_)) => {
                    return Err(EngineError::Failed {
                        id,
                        message: "a break is a row or a column and not both".to_string(),
                    });
                }
                (None, None) => {
                    return Err(EngineError::Failed { id, message: "a break is a row or a column".to_string() });
                }
                _ => {}
            }
            let (name, line_at) = if let Some(row) = &self.row {
                let row = row.0;
                if !row.is_finite() || row < 1.0 || row.fract() != 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("a row is counted from one: {row} is not one"),
                    });
                }
                ("row_breaks", row as u32 - 1)
            } else {
                let column = self.column.as_ref().map(|column| column.0.trim().to_string()).unwrap_or_default();
                let Some(index) = crate::formula::CellRef::column_index(&column.to_ascii_uppercase()) else {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("`{column}` is not a column: a column is `A`, `B` or `AA`"),
                    });
                };
                ("column_breaks", index)
            };
            let breaks = node.data.get(name).and_then(|breaks| breaks.as_array()).cloned().unwrap_or_default();
            let mut lines: Vec<u64> =
                breaks.iter().filter_map(|line| line.as_u64()).filter(|line| *line != line_at as u64).collect();
            if !clear {
                lines.push(line_at as u64);
                lines.sort_unstable();
            }
            let list: Vec<craftkit_sdk::serde_json::Value> = lines.into_iter().map(|line| json!(line)).collect();
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data[name] = craftkit_sdk::serde_json::Value::Array(list.clone());
            }
            Ok(Outcome::changed(format!(
                "the break is {} in the sheet `{sheet}`",
                if clear { "taken away" } else { "set" }
            ))
            .with_data(json!({ "sheet": sheet, "break": list, "kind": name })))

        }
    }
    /// **Set** the titles of a sheet: the rows and the columns that are
    /// printed on **every** page.
    ///
    /// The titles are not part of the pages: they are printed above them,
    /// again and again.
    grid.print.titles {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.

            sheet: NodeRef,
            /// The rows that are printed on every page, counted from one. The
            /// first row is the first title.

            rows: Option<Text>,
            /// The columns that are printed on every page, as `A`. The first
            /// column is the first title.

            columns: Option<Text>,
        },
        run: {

            let id = "grid.print.titles".to_string();
            let sheet = self.sheet.0;
            sheet_of(session, &id, sheet)?;
            let mut rows: Vec<u32> = Vec::new();
            if let Some(text) = &self.rows {
                let text = text.0.trim().to_string();
                if !text.is_empty() {
                    for part in text.split(',') {
                        let part = part.trim();
                        let Some((first, last)) = part.split_once(':') else {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("`{part}` is not a range of rows"),
                            });
                        };
                        let number = |text: &str| -> Result<u32, EngineError> {
                            let row = text.trim().parse::<u64>().map_err(|_| EngineError::Failed {
                                id: id.clone(),
                                message: format!("`{text}` is not a row"),
                            })?;
                            if row == 0 {
                                return Err(EngineError::Failed {
                                    id: id.clone(),
                                    message: "a row is counted from one: 0 is not one".to_string(),
                                });
                            }
                            Ok(row as u32 - 1)
                        };
                        let first = number(first)?;
                        let last = number(last)?;
                        let (low, high) = (first.min(last), first.max(last));
                        for row in low..=high {
                            if !rows.contains(&row) {
                                rows.push(row);
                            }
                        }
                    }
                    rows.sort_unstable();
                }
            }
            let mut columns: Vec<u32> = Vec::new();
            if let Some(text) = &self.columns {
                let text = text.0.trim().to_string();
                if !text.is_empty() {
                    for part in text.split(',') {
                        let part = part.trim().to_ascii_uppercase();
                        let Some((first, last)) = part.split_once(':') else {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("`{part}` is not a range of columns"),
                            });
                        };
                        let Some(first) = crate::formula::CellRef::column_index(first.trim()) else {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("`{first}` is not a column"),
                            });
                        };
                        let Some(last) = crate::formula::CellRef::column_index(last.trim()) else {
                            return Err(EngineError::Failed {
                                id: id.clone(),
                                message: format!("`{last}` is not a column"),
                            });
                        };
                        let (low, high) = (first.min(last), first.max(last));
                        for column in low..=high {
                            if !columns.contains(&column) {
                                columns.push(column);
                            }
                        }
                    }
                    columns.sort_unstable();
                }
            }
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["title_rows"] = json!(rows.clone());
                node.data["title_columns"] = json!(columns.clone());
            }
            Ok(Outcome::changed("the titles of the sheet are set".to_string()).with_data(json!({
                "sheet": sheet,
                "rows": rows.iter().map(|row| row + 1).collect::<Vec<u32>>(),
                "columns": columns
                    .iter()
                    .map(|column| crate::formula::CellRef::column_text(*column))
                    .collect::<Vec<String>>(),
            })))

        }
    }
}
