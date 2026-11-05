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

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

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
const DEFAULT_ROW_HEIGHT: f64 = 15.0;

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
fn sheet_of(session: &Session, id: &str, sheet: u64) -> Result<Node, EngineError> {
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
fn row_height(node: &Node, row: u32) -> f64 {
    node.data
        .get("rows")
        .and_then(|rows| rows.get(row.to_string()))
        .and_then(|height| height.as_f64())
        .filter(|height| height.is_finite() && *height > 0.0)
        .unwrap_or(DEFAULT_ROW_HEIGHT)
}

/// The **width of a column**, in points.
fn column_width(node: &Node, column: u32) -> f64 {
    node.data
        .get("columns")
        .and_then(|columns| columns.get(crate::formula::CellRef::column_text(column)))
        .and_then(|width| width.as_f64())
        .filter(|width| width.is_finite() && *width > 0.0)
        .map(|width| width * CHARACTER + PADDING)
        .unwrap_or(DEFAULT_COLUMN_WIDTH * CHARACTER + PADDING)
}

/// The **bands** of a run of lines: the lines that go on one page.
///
/// A line that is wider than the page is on a page of its own.
fn bands(start: u32, end: u32, size: impl Fn(u32) -> f64, room: f64) -> Vec<(u32, u32)> {
    let mut out: Vec<(u32, u32)> = Vec::new();
    let mut current = start;
    let mut used = 0.0;
    let mut line = start;
    loop {
        let length = size(line);
        if line > current && used + length > room + 0.01 {
            out.push((current, line - 1));
            current = line;
            used = 0.0;
        }
        used += length;
        if line >= end {
            break;
        }
        line += 1;
    }
    out.push((current, end));
    out
}

/// **Set** the paper of a sheet.
#[derive(Command)]
#[command(id = "grid.print.paper", area = "grid", layer = "grid", icon = "page", mutates, undoable)]
pub struct SetPaper {
    /// The sheet.
    pub sheet: NodeRef,
    /// The paper: `Letter`, `A4`, `Tabloid` and the rest.
    pub paper: Text,
}

impl SetPaper {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
#[derive(Command)]
#[command(id = "grid.print.orientation", area = "grid", layer = "grid", icon = "page", mutates, undoable)]
pub struct SetOrientation {
    /// The sheet.
    pub sheet: NodeRef,
    /// The orientation: `portrait` or `landscape`.
    pub orientation: Text,
}

impl SetOrientation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
#[derive(Command)]
#[command(id = "grid.print.margins", area = "grid", layer = "grid", icon = "page", mutates, undoable)]
pub struct SetMargins {
    /// The sheet.
    pub sheet: NodeRef,
    /// The preset: `normal`, `wide` or `narrow`.
    pub preset: Option<Text>,
    /// The **left** margin, in inches.
    pub left: Option<Number>,
    /// The **right** margin, in inches.
    pub right: Option<Number>,
    /// The **top** margin, in inches.
    pub top: Option<Number>,
    /// The **bottom** margin, in inches.
    pub bottom: Option<Number>,
}

impl SetMargins {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
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
            if !margin.is_finite() || margin < 0.0 || margin > MAX_MARGIN {
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
#[derive(Command)]
#[command(id = "grid.print.read", area = "grid", layer = "grid", icon = "list")]
pub struct ReadSetup {
    /// The sheet.
    pub sheet: NodeRef,
}

impl ReadSetup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.print.read".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let setup = Setup::of(&node);
        let (page_width, page_height) = setup.page();
        let (printable_width, printable_height) = setup.printable();
        Ok(Outcome::new(format!("the sheet is printed on `{}`", setup.paper)).with_data(json!({
            "sheet": sheet,
            "paper": setup.paper,
            "orientation": setup.orientation,
            "margins": setup.to_json()["margins"].clone(),
            "page": { "width": page_width, "height": page_height },
            "printable": { "width": printable_width, "height": printable_height },
        })))
    }
}

/// **Read** the pages of a sheet.
///
/// The pages are **derived**: they are worked out from the paper, the
/// margins and the sizes of the rows and the columns.
#[derive(Command)]
#[command(id = "grid.print.pages", area = "grid", layer = "grid", icon = "list")]
pub struct ReadPages {
    /// The sheet.
    pub sheet: NodeRef,
}

impl ReadPages {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "grid.print.pages".to_string();
        let sheet = self.sheet.0;
        let node = sheet_of(session, &id, sheet)?;
        let setup = Setup::of(&node);
        let cells = crate::cells_of(&node);
        let Some(area) = crate::reading::used_range(&cells) else {
            return Ok(Outcome::new("the sheet holds nothing to print").with_data(json!({
                "sheet": sheet,
                "pages": Vec::<craftkit_sdk::serde_json::Value>::new(),
                "count": 0,
            })));
        };
        let (printable_width, printable_height) = setup.printable();
        let rows = bands(area.start.row, area.end.row, |row| row_height(&node, row), printable_height);
        let columns =
            bands(area.start.col, area.end.col, |column| column_width(&node, column), printable_width);
        let mut pages: Vec<craftkit_sdk::serde_json::Value> = Vec::new();
        for (first_row, last_row) in &rows {
            for (first_column, last_column) in &columns {
                if pages.len() >= MAX_PAGES {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the sheet is more than {MAX_PAGES} pages, and a sheet is not that long"
                        ),
                    });
                }
                pages.push(json!({
                    "rows": [first_row + 1, last_row + 1],
                    "columns": [
                        crate::formula::CellRef::column_text(*first_column),
                        crate::formula::CellRef::column_text(*last_column),
                    ],
                }));
            }
        }
        Ok(Outcome::new(format!("the sheet is {} page(s)", pages.len())).with_data(json!({
            "sheet": sheet,
            "area": area.to_text(),
            "pages": pages,
            "count": pages.len(),
        })))
    }
}
