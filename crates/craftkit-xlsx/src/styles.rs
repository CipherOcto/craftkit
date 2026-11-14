//! The **styles** of a workbook: the format of each cell.
//!
//! A workbook has a part that says what its cells look like: the
//! **styles** part. It holds
//!
//! - the **number formats** the workbook names by their own number —
//!   the code of each, and the number it has;
//! - the **cell formats**, each of which says which number format its
//!   cell is written with.
//!
//! A cell names one of them by its **style** — the `s` attribute of
//! the cell — and so the format of a cell is found by two steps: the
//! style of the cell, and the number format of that style.
//!
//! # The built-in formats
//!
//! Some formats have no number of their own: they are **built in**, and
//! their number says which one they are. The table is the tree's own
//! table, and the numbers that are not in it — the East Asian ones —
//! read as the general format.
//!
//! # What this does not do
//!
//! - It does not read the **fonts**, the **fills** or the **borders**:
//!   this reader gives back the shape of a workbook, and the shape is
//!   the values and their formats.
//! - It does not decide what a format **means**: the battery that owns
//!   the spreadsheet model has its own format language, and it is the
//!   one that reads a code.

use craftkit_xml::El;

/// The **built-in** number formats, by the number they have.
///
/// The tree's own table, in the tree's own order. The numbers that are
/// not in it — 23 to 36, which are East Asian locale formats — read as
/// the general format.
const BUILTIN_FORMATS: [(u32, &str); 36] = [
    (0, "General"),
    (1, "0"),
    (2, "0.00"),
    (3, "#,##0"),
    (4, "#,##0.00"),
    (5, "\"$\"#,##0_);\\(\"$\"#,##0\\)"),
    (6, "\"$\"#,##0_);[Red]\\(\"$\"#,##0\\)"),
    (7, "\"$\"#,##0.00_);\\(\"$\"#,##0.00\\)"),
    (8, "\"$\"#,##0.00_);[Red]\\(\"$\"#,##0.00\\)"),
    (9, "0%"),
    (10, "0.00%"),
    (11, "0.00E+00"),
    (12, "# ?/?"),
    (13, "# ??/??"),
    (14, "m/d/yyyy"),
    (15, "d-mmm-yy"),
    (16, "d-mmm"),
    (17, "mmm-yy"),
    (18, "h:mm AM/PM"),
    (19, "h:mm:ss AM/PM"),
    (20, "h:mm"),
    (21, "h:mm:ss"),
    (22, "m/d/yyyy h:mm"),
    (37, "#,##0 ;(#,##0)"),
    (38, "#,##0 ;[Red](#,##0)"),
    (39, "#,##0.00;(#,##0.00)"),
    (40, "#,##0.00;[Red](#,##0.00)"),
    (41, "_(* #,##0_);_(* \\(#,##0\\);_(* \"-\"_);_(@_)"),
    (42, "_(\"$\"* #,##0_);_(\"$\"* \\(#,##0\\);_(\"$\"* \"-\"_);_(@_)"),
    (43, "_(* #,##0.00_);_(* \\(#,##0.00\\);_(* \"-\"??_);_(@_)"),
    (44, "_(\"$\"* #,##0.00_);_(\"$\"* \\(#,##0.00\\);_(\"$\"* \"-\"??_);_(@_)"),
    (45, "mm:ss"),
    (46, "[h]:mm:ss"),
    (47, "mmss.0"),
    (48, "##0.0E+0"),
    (49, "@"),
];

/// The format of the **general** format: the value as it is.
const GENERAL: &str = "General";

/// The **underline** of a font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Underline {
    /// There is no underline.
    #[default]
    None,
    /// A single line under the text.
    Single,
    /// Two lines under the text.
    Double,
    /// A single line under the text, and the line is as long as the
    /// numbers of the cell are wide.
    SingleAccounting,
    /// Two lines under the text, and the lines are as long as the
    /// numbers of the cell are wide.
    DoubleAccounting,
}

/// Where the text of a cell sits **above or below** its line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Vert {
    /// On the line of the text.
    #[default]
    Baseline,
    /// Above the line of the text.
    Superscript,
    /// Below the line of the text.
    Subscript,
}

/// The **font** of a cell: the face of its text, the size of it, and
/// the marks that are put on it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Font {
    /// The name of the face, when the font names one.
    pub name: Option<String>,
    /// The size of the text, when the font names one.
    pub size: Option<f64>,
    /// Whether the text is **bold**.
    pub bold: bool,
    /// Whether the text is **italic**.
    pub italic: bool,
    /// Whether the text is **struck through**.
    pub strike: bool,
    /// The **underline** of the text.
    pub underline: Underline,
    /// Where the text sits above or below its line.
    pub vert: Vert,
    /// The **colour** of the text, as `#rrggbb`, when the font names
    /// one.
    pub colour: Option<String>,
}

/// The **fill** of a cell: the colour behind its text.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Fill {
    /// The **pattern** the fill is drawn in: `solid`, or the name the
    /// file gives it.
    pub pattern: String,
    /// The **colour** of the fill, as `#rrggbb`, when the fill names
    /// one. Only a **solid** fill has a colour here.
    pub colour: Option<String>,
}

/// A **border**: a line on one edge of a cell, of a style, in a
/// colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Border {
    /// The **edge** the line is on: `top`, `right`, `bottom` or
    /// `left`.
    pub edge: String,
    /// The **style** of the line: the tree's own names, which are the
    /// file's own names too.
    pub style: String,
    /// The **colour** of the line, as `#rrggbb`, when the file names
    /// one.
    pub colour: Option<String>,
}

/// The **borders** of a cell: the lines on its four edges.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Borders {
    /// The lines of the cell, by the edge they are on.
    pub edges: Vec<Border>,
}

/// The **alignment** of a cell: how its text lies across it and how
/// it lies down it.
///
/// The names are the **tree's** own, and a name the tree does not
/// have is **nothing**: a file may name an alignment the tree has not
/// heard of, and that alignment is not carried.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Alignment {
    /// How the text lies across the cell, when the tree has that way.
    pub horizontal: Option<String>,
    /// How the text lies down the cell, when the tree has that way.
    pub vertical: Option<String>,
    /// How far the text is indented, in steps of the width of a
    /// character.
    pub indent: u8,
    /// Whether the text wraps at the width of the cell.
    pub wrap: bool,
}

/// The **protection** of a cell: whether it is **locked**, and whether
/// its text is **hidden**.
///
/// A cell that says nothing is locked, as the tree's reader has it: the
/// lock of a cell is the lock of its style, and the style of a cell
/// that names nothing is locked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Protection {
    /// Whether the cell is **locked**.
    pub locked: bool,
    /// Whether the **text** of the cell is hidden.
    pub hidden: bool,
}

/// A **differential format**: the look of a rule that is about a range
/// of cells and not about a cell.
///
/// A differential format is the tree's `dxf`: the fill behind the
/// cells a rule matched, and the number format they are written with.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Dxf {
    /// The **number format** the cells are written with, when the
    /// differential format names one.
    pub format: Option<String>,
    /// The **colour** behind the cells, as `#rrggbb`, when the
    /// differential format names one.
    pub fill: Option<String>,
}

/// The **styles** of a workbook: the format of each cell, by the
/// number of the cell format.
#[derive(Debug, Clone, Default)]
pub struct Styles {
    /// The format of each cell format, in the order the file holds
    /// them. The first one is the normal format, and every cell with
    /// no style of its own has it.
    formats: Vec<String>,
    /// The **font** of each cell format, by the number of the cell
    /// format: the number of the font it names.
    font_ids: Vec<u32>,
    /// The **fill** of each cell format, by the number of the cell
    /// format: the number of the fill it names.
    fill_ids: Vec<u32>,
    /// The **alignment** of each cell format, by the number of the cell
    /// format.
    aligns: Vec<Option<Alignment>>,
    /// The **borders** of each cell format, by the number of the cell
    /// format: the number of the borders it names.
    border_ids: Vec<u32>,
    /// The **protection** of each cell format, by the number of the
    /// cell format.
    protections: Vec<Option<Protection>>,
    /// The **fonts** of the workbook, in the order the file holds them.
    fonts: Vec<Font>,
    /// The **fills** of the workbook, in the order the file holds them.
    fills: Vec<Fill>,
    /// The **borders** of the workbook, in the order the file holds
    /// them.
    borders: Vec<Borders>,
    /// The **differential formats** of the workbook, in the order the
    /// file holds them: the looks of the rules that are about a range
    /// of cells.
    dxfs: Vec<Dxf>,
}

impl Styles {
    /// The **format** of the style of that number.
    ///
    /// A style the file does not have has no format: the general
    /// format is the format of a cell that names nothing.
    pub fn format(&self, style: Option<u32>) -> Option<&str> {
        let index = style? as usize;
        let code = self.formats.get(index)?;
        if code.eq_ignore_ascii_case(GENERAL) { None } else { Some(code.as_str()) }
    }

    /// The **font** of the style of that number.
    ///
    /// A cell with no style of its own has the **first** font of the
    /// workbook, as the tree's reader has it.
    pub fn font(&self, style: Option<u32>) -> Option<&Font> {
        let index = style.unwrap_or(0) as usize;
        let number = self.font_ids.get(index).copied().unwrap_or(0) as usize;
        self.fonts.get(number)
    }

    /// The **fill** of the style of that number.
    pub fn fill(&self, style: Option<u32>) -> Option<&Fill> {
        let index = style.unwrap_or(0) as usize;
        let number = self.fill_ids.get(index).copied().unwrap_or(0) as usize;
        self.fills.get(number)
    }

    /// The **alignment** of the style of that number.
    pub fn align(&self, style: Option<u32>) -> Option<&Alignment> {
        let index = style.unwrap_or(0) as usize;
        self.aligns.get(index).and_then(|align| align.as_ref())
    }

    /// The **borders** of the style of that number.
    pub fn borders(&self, style: Option<u32>) -> Option<&Borders> {
        let index = style.unwrap_or(0) as usize;
        let number = self.border_ids.get(index).copied().unwrap_or(0) as usize;
        self.borders.get(number)
    }

    /// The **protection** of the style of that number.
    pub fn protection(&self, style: Option<u32>) -> Option<&Protection> {
        let index = style.unwrap_or(0) as usize;
        self.protections.get(index).and_then(|protection| protection.as_ref())
    }

    /// The **differential format** of that number: the look of a rule
    /// that is about a range of cells.
    pub fn dxf(&self, number: Option<u32>) -> Option<&Dxf> {
        let index = number? as usize;
        self.dxfs.get(index)
    }

    /// Whether there are **no** styles at all: a workbook with no
    /// styles part has no formats.
    pub fn is_empty(&self) -> bool {
        self.formats.is_empty()
    }
}

/// Read the **styles** part of a workbook.
pub fn read_styles(root: &El) -> Styles {
    // The **custom** number formats: the ones the workbook names by
    // their own number.
    let mut custom: Vec<(u32, String)> = Vec::new();
    if let Some(formats) = root.child("numFmts") {
        for format in formats.kids("numFmt") {
            let Some(id) = format.attr_u32("numFmtId") else { continue };
            let Some(code) = format.attr("formatCode") else { continue };
            custom.push((id, code.to_string()));
        }
    }
    // The **fonts** of the workbook, in the order the file holds them.
    let fonts: Vec<Font> =
        root.child("fonts").map(|fonts| fonts.kids("font").map(read_font).collect()).unwrap_or_default();
    // The **fills** of the workbook, in the order the file holds them.
    let fills: Vec<Fill> =
        root.child("fills").map(|fills| fills.kids("fill").map(read_fill).collect()).unwrap_or_default();
    // The **borders** of the workbook, in the order the file holds them.
    let borders: Vec<Borders> = root
        .child("borders")
        .map(|borders| borders.kids("border").map(read_borders).collect())
        .unwrap_or_default();
    // The **differential formats** of the workbook: the looks of the
    // rules that are about a range of cells.
    let dxfs: Vec<Dxf> =
        root.child("dxfs").map(|dxfs| dxfs.kids("dxf").map(read_dxf).collect()).unwrap_or_default();
    // The **cell formats**: each says which number format its cell is
    // written with, which font it is written in, which fill is behind
    // it and how its text lies.
    let mut formats = Vec::new();
    let mut font_ids = Vec::new();
    let mut fill_ids = Vec::new();
    let mut aligns = Vec::new();
    let mut border_ids = Vec::new();
    let mut protections = Vec::new();
    if let Some(cells) = root.child("cellXfs") {
        for format in cells.kids("xf") {
            let id = format.attr_u32("numFmtId").unwrap_or(0);
            let code = custom
                .iter()
                .find(|(number, _)| *number == id)
                .map(|(_, code)| code.clone())
                .or_else(|| builtin_format(id).map(|code| code.to_string()))
                .unwrap_or_else(|| GENERAL.to_string());
            formats.push(code);
            font_ids.push(format.attr_u32("fontId").unwrap_or(0));
            fill_ids.push(format.attr_u32("fillId").unwrap_or(0));
            aligns.push(format.child("alignment").map(read_alignment));
            border_ids.push(format.attr_u32("borderId").unwrap_or(0));
            protections.push(format.child("protection").map(read_protection));
        }
    }
    Styles { formats, font_ids, fill_ids, aligns, border_ids, protections, fonts, fills, borders, dxfs }
}

/// Read a **differential format**: the look of a rule that is about a
/// range of cells.
///
/// A differential format names its fill the other way round: the colour
/// is in the **background** of the fill, and the background of a fill
/// that has no pattern is its colour.
fn read_dxf(element: &El) -> Dxf {
    let format = element.child("numFmt").and_then(|format| {
        format.attr("formatCode").map(|code| code.to_string()).or_else(|| {
            let id = format.attr_u32("numFmtId")?;
            builtin_format(id).map(|code| code.to_string())
        })
    });
    let fill = element.child("fill").and_then(|fill| {
        let pattern = fill.child("patternFill")?;
        let kind = pattern.attr("patternType").unwrap_or("none");
        if kind != "none" && kind != "solid" {
            return None;
        }
        pattern
            .child("bgColor")
            .and_then(read_colour)
            .or_else(|| pattern.child("fgColor").and_then(read_colour))
    });
    Dxf { format, fill }
}

/// Read the **protection** of a cell format.
///
/// A cell format that says nothing about its protection has no
/// protection here: the lock of a cell that says nothing is the lock
/// of its style, and a style that says nothing is locked.
fn read_protection(element: &El) -> Protection {
    Protection { locked: element.flag("locked", true), hidden: element.flag("hidden", false) }
}

/// The **styles** a border may have — the tree's own.
const BORDER_STYLES: [&str; 13] = [
    "thin",
    "medium",
    "thick",
    "dashed",
    "dotted",
    "double",
    "hair",
    "mediumDashed",
    "dashDot",
    "mediumDashDot",
    "dashDotDot",
    "mediumDashDotDot",
    "slantDashDot",
];

/// Read the **borders** of a cell: the lines on its four edges.
///
/// A line with no style is no line, and a style this tree does not
/// have is no line either.
fn read_borders(element: &El) -> Borders {
    let mut edges = Vec::new();
    for edge in ["top", "right", "bottom", "left"] {
        let Some(line) = element.child(edge) else { continue };
        let Some(style) = line.attr("style") else { continue };
        if !BORDER_STYLES.contains(&style) {
            continue;
        }
        edges.push(Border {
            edge: edge.to_string(),
            style: style.to_string(),
            colour: line.child("color").and_then(read_colour),
        });
    }
    Borders { edges }
}

/// Read a **font** of a workbook.
fn read_font(element: &El) -> Font {
    // A property that is **there** is true, unless it says otherwise.
    let flag = |name: &str| element.child(name).is_some_and(|child| child.flag("val", true));
    Font {
        name: element
            .child("name")
            .or_else(|| element.child("rFont"))
            .and_then(|name| name.attr("val"))
            .map(|name| name.to_string()),
        size: element.child("sz").and_then(|size| size.attr_f64("val")).filter(|size| *size > 0.0),
        bold: flag("b"),
        italic: flag("i"),
        strike: flag("strike"),
        underline: match element.child("u").and_then(|line| line.attr("val")) {
            Some("double") => Underline::Double,
            Some("singleAccounting") => Underline::SingleAccounting,
            Some("doubleAccounting") => Underline::DoubleAccounting,
            Some("none") => Underline::None,
            Some(_) | None => Underline::Single,
        },
        vert: match element.child_val("vertAlign") {
            Some("superscript") => Vert::Superscript,
            Some("subscript") => Vert::Subscript,
            _ => Vert::Baseline,
        },
        colour: element.child("color").and_then(read_colour),
    }
}

/// Read a **fill** of a workbook.
///
/// A fill is a **pattern**, and only a **solid** one is a colour here:
/// a pattern of stripes is a drawing, and a drawing is not a fact of
/// the sheet.
fn read_fill(element: &El) -> Fill {
    let Some(pattern) = element.child("patternFill") else { return Fill::default() };
    let kind = pattern.attr("patternType").unwrap_or("none").to_string();
    let colour = if kind == "solid" { pattern.child("fgColor").and_then(read_colour) } else { None };
    Fill { pattern: kind, colour }
}

/// Read the **colour** of an element, as `#rrggbb`.
///
/// A file names a colour in three ways, and this tree has two of
/// them: a colour in **hex** — with an alpha byte in front of it,
/// which is dropped — and a colour by its **number** in the old
/// palette, which is the tree's own table. A **theme** colour is not
/// a colour here: this tree has no theme.
fn read_colour(element: &El) -> Option<String> {
    if element.flag("auto", false) {
        return None;
    }
    if let Some(hex) = element.attr("rgb") {
        return colour_of_hex(hex);
    }
    if let Some(number) = element.attr_u32("indexed") {
        let rgb = INDEXED.get(number as usize)?;
        return Some(format!("#{rgb:06x}"));
    }
    None
}

/// A colour of **hex**, as `#rrggbb`.
///
/// The hex of a file is eight digits: the alpha byte first, and the
/// colour after it. A hex of six digits is the colour itself.
fn colour_of_hex(hex: &str) -> Option<String> {
    let hex = hex.trim().trim_start_matches('#');
    let digits = match hex.len() {
        8 => hex.get(2..)?,
        6 => hex,
        _ => return None,
    };
    if !digits.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("#{}", digits.to_ascii_lowercase()))
}

/// The **old palette** of colours: the numbers a file may name a
/// colour by.
///
/// The tree's own table, in the tree's own order.
const INDEXED: [u32; 64] = [
    0x000000, 0xFFFFFF, 0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF, 0x000000, 0xFFFFFF,
    0xFF0000, 0x00FF00, 0x0000FF, 0xFFFF00, 0xFF00FF, 0x00FFFF, 0x800000, 0x008000, 0x000080, 0x808000,
    0x800080, 0x008080, 0xC0C0C0, 0x808080, 0x9999FF, 0x993366, 0xFFFFCC, 0xCCFFFF, 0x660066, 0xFF8080,
    0x0066CC, 0xCCCCFF, 0x000080, 0xFF00FF, 0xFFFF00, 0x00FFFF, 0x800080, 0x800000, 0x008080, 0x0000FF,
    0x00CCFF, 0xCCFFFF, 0xCCFFCC, 0xFFFF99, 0x99CCFF, 0xFF99CC, 0xCC99FF, 0xFFCC99, 0x3366FF, 0x33CCCC,
    0x99CC00, 0xFFCC00, 0xFF9900, 0xFF6600, 0x666699, 0x969696, 0x003366, 0x339966, 0x003300, 0x333300,
    0x993300, 0x993366, 0x333399, 0x333333,
];

/// Read the **alignment** of a cell format.
///
/// A name the tree does not have is **nothing**: the file named a way
/// of lying that this tree has not heard of, and that way is not
/// carried.
fn read_alignment(element: &El) -> Alignment {
    let horizontal = match element.attr("horizontal").unwrap_or("general") {
        "general" => Some("general".to_string()),
        "left" => Some("left".to_string()),
        "center" => Some("center".to_string()),
        "right" => Some("right".to_string()),
        "fill" => Some("fill".to_string()),
        "justify" => Some("justify".to_string()),
        "centerContinuous" => Some("center-across".to_string()),
        _ => None,
    };
    let vertical = match element.attr("vertical").unwrap_or("bottom") {
        "top" => Some("top".to_string()),
        "center" => Some("middle".to_string()),
        "bottom" => Some("bottom".to_string()),
        _ => None,
    };
    let indent = element.attr_u32("indent").unwrap_or(0).min(250);
    Alignment {
        horizontal,
        vertical,
        indent: u8::try_from(indent).unwrap_or(0),
        wrap: element.flag("wrapText", false),
    }
}

/// The **built-in** format of that number, if it has one.
fn builtin_format(id: u32) -> Option<&'static str> {
    BUILTIN_FORMATS.iter().find(|(number, _)| *number == id).map(|(_, code)| *code)
}
