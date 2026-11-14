//! `craftkit-pdf-writer` — the **PDF writer**: a file written as a PDF.
//!
//! # What this is
//!
//! A **drawing** of pages, and a file. The caller says "a page this
//! big", draws rectangles, lines, text and pictures on it, and the
//! writer gives back the bytes of a PDF 1.7 file.
//!
//! The file it writes is a **PDF 1.7** file: a catalogue, a page tree,
//! the pages, the fonts they use, the pictures they hold and the
//! cross-reference table that says where each of those is in the file.
//!
//! # What it can draw
//!
//! - **Vector drawing**: rectangles, lines (solid or dashed), polygons,
//!   circles (four cubic Béziers) and clipping.
//! - **Text**, in one of the twelve standard faces (the standard
//!   fourteen minus Symbol and ZapfDingbats), with the **advance
//!   widths** of the faces so that a caller can measure, align and wrap
//!   text before it draws it ([`text_width`]).
//! - **Pictures**, as RGB image XObjects with a soft mask for the alpha
//!   channel.
//! - **Links**: an area that goes to a URI, or to a place on another page.
//!
//! # The coordinates
//!
//! Points, with a **top-left origin**: `y` grows **downwards**, and the
//! `y` of a piece of text is its **baseline**. A page is as big as it
//! is said to be, and the writer flips the `y` into PDF's own
//! bottom-left convention on the way out.
//!
//! # The characters a face cannot hold
//!
//! The twelve faces are set in WinAnsiEncoding, which is a single byte
//! per character and covers Latin-1. A character outside it — a Chinese
//! one, a Japanese one, an emoji — has no byte, and it is set in a
//! **predefined, non-embedded Adobe-GB1 CID font** with the
//! `UniGB-UTF16-H` CMap, in which the text is written as UTF-16BE.
//! It is measured as a full-width glyph.
//!
//! **This is a limit, and it is named here:** Adobe-GB1 is a **Chinese**
//! collection. Chinese text is set correctly. Japanese, Korean and
//! emoji are written in that collection as well, and a reader may show
//! them wrongly. The alternative — writing them as `?` — is worse, and
//! this crate does not do it.
//!
//! # The file is written plainly
//!
//! The streams are **not compressed**: the content of a page is written
//! as it is, and the bytes of a picture are run-length encoded, which is
//! a filter every PDF reader has. So this crate needs **no
//! dependencies** at all.
//!
//! # Why it is a library and not a plugin
//!
//! Because a writer is a writer: it draws pages and gives back bytes,
//! and it knows nothing about a document, a canvas or the kit. What
//! draws a **sheet** into it — the sizes of the columns, the height of
//! the rows, the text a cell shows — is the grid battery's business,
//! and it is a plugin (§935).
//!
//! # What this does not do
//!
//! - It does not **read** a PDF. The PDF battery reads one (§771).
//! - It does not **embed** a font: the faces it uses are the ones a
//!   reader is required to have.
//! - It does not **encrypt**, and it does not write an **object
//!   stream**: the file it writes is a plain file.
//! - It does not **compress** the content of a page.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod metrics;

#[cfg(test)]
mod tests;

use std::fmt::Write as _;

/// An RGB colour, 0–255 per channel.
pub type Rgb = [u8; 3];

/// A point in page coordinates, measured from the top-left corner.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    /// The distance from the left edge of the page, in points.
    pub x: f32,
    /// The distance from the top edge of the page, in points.
    pub y: f32,
}

impl Point {
    /// A point at (`x`, `y`).
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A page-space rectangle: its top-left corner and its extent.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    /// The distance from the left edge of the page, in points.
    pub x: f32,
    /// The distance from the top edge of the page, in points.
    pub y: f32,
    /// The width, in points.
    pub width: f32,
    /// The height, in points.
    pub height: f32,
}

impl Rect {
    /// A rectangle at (`x`, `y`) of `width` × `height` points.
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self { x, y, width, height }
    }
}

/// One of the standard faces.
///
/// The standard fourteen faces, less Symbol and ZapfDingbats: a
/// reader is required to have the fourteen, so a file that uses them
/// needs no embedded font.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Font {
    /// Helvetica, upright.
    #[default]
    Helvetica,
    /// Helvetica, bold.
    HelveticaBold,
    /// Helvetica, oblique.
    HelveticaOblique,
    /// Helvetica, bold and oblique.
    HelveticaBoldOblique,
    /// Times Roman, upright.
    TimesRoman,
    /// Times Roman, bold.
    TimesBold,
    /// Times Roman, italic.
    TimesItalic,
    /// Times Roman, bold and italic.
    TimesBoldItalic,
    /// Courier, upright.
    Courier,
    /// Courier, bold.
    CourierBold,
    /// Courier, oblique.
    CourierOblique,
    /// Courier, bold and oblique.
    CourierBoldOblique,
}

/// A family of faces, for [`Font::styled`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Family {
    /// The sans faces: Helvetica.
    #[default]
    Sans,
    /// The serif faces: Times.
    Serif,
    /// The monospaced faces: Courier.
    Mono,
}

impl Font {
    /// Every face, in the order the writer numbers them.
    pub const ALL: [Font; 12] = [
        Font::Helvetica,
        Font::HelveticaBold,
        Font::HelveticaOblique,
        Font::HelveticaBoldOblique,
        Font::TimesRoman,
        Font::TimesBold,
        Font::TimesItalic,
        Font::TimesBoldItalic,
        Font::Courier,
        Font::CourierBold,
        Font::CourierOblique,
        Font::CourierBoldOblique,
    ];

    /// The PostScript name of the face.
    pub fn base_name(self) -> &'static str {
        match self {
            Font::Helvetica => "Helvetica",
            Font::HelveticaBold => "Helvetica-Bold",
            Font::HelveticaOblique => "Helvetica-Oblique",
            Font::HelveticaBoldOblique => "Helvetica-BoldOblique",
            Font::TimesRoman => "Times-Roman",
            Font::TimesBold => "Times-Bold",
            Font::TimesItalic => "Times-Italic",
            Font::TimesBoldItalic => "Times-BoldItalic",
            Font::Courier => "Courier",
            Font::CourierBold => "Courier-Bold",
            Font::CourierOblique => "Courier-Oblique",
            Font::CourierBoldOblique => "Courier-BoldOblique",
        }
    }

    fn index(self) -> usize {
        Font::ALL.iter().position(|f| *f == self).unwrap_or(0)
    }

    /// The face of `family` with the given weight and slant.
    pub fn styled(family: Family, bold: bool, italic: bool) -> Font {
        match (family, bold, italic) {
            (Family::Sans, false, false) => Font::Helvetica,
            (Family::Sans, true, false) => Font::HelveticaBold,
            (Family::Sans, false, true) => Font::HelveticaOblique,
            (Family::Sans, true, true) => Font::HelveticaBoldOblique,
            (Family::Serif, false, false) => Font::TimesRoman,
            (Family::Serif, true, false) => Font::TimesBold,
            (Family::Serif, false, true) => Font::TimesItalic,
            (Family::Serif, true, true) => Font::TimesBoldItalic,
            (Family::Mono, false, false) => Font::Courier,
            (Family::Mono, true, false) => Font::CourierBold,
            (Family::Mono, false, true) => Font::CourierOblique,
            (Family::Mono, true, true) => Font::CourierBoldOblique,
        }
    }

    /// The family a font name belongs to.
    ///
    /// A name with "serif" in it is a serif face, a name with "mono" in
    /// it is a monospaced one, and everything else is a sans face. A
    /// name with both — "sans serif" — is a sans face.
    pub fn family_for(name: &str) -> Family {
        let n = name.to_ascii_lowercase();
        const SERIF: [&str; 10] = [
            "times",
            "cambria",
            "georgia",
            "garamond",
            "serif",
            "book",
            "palatino",
            "baskerville",
            "tinos",
            "caladea",
        ];
        const MONO: [&str; 8] =
            ["courier", "consolas", "mono", "menlo", "lucida console", "cousine", "code", "typewriter"];
        if MONO.iter().any(|m| n.contains(m)) {
            Family::Mono
        } else if SERIF.iter().any(|s| n.contains(s)) && !n.contains("sans") {
            Family::Serif
        } else {
            Family::Sans
        }
    }

    /// The advance width of a character, in 1/1000 em.
    ///
    /// A character the face's table does not hold is measured as the
    /// character it most resembles, and a character nothing resembles is
    /// measured as the face's default width.
    pub fn char_width(self, c: char) -> u16 {
        let table: Option<&[u16; 95]> = match self {
            Font::Helvetica | Font::HelveticaOblique => Some(&metrics::HELVETICA),
            Font::HelveticaBold | Font::HelveticaBoldOblique => Some(&metrics::HELVETICA_BOLD),
            Font::TimesRoman | Font::TimesItalic => Some(&metrics::TIMES),
            Font::TimesBold | Font::TimesBoldItalic => Some(&metrics::TIMES_BOLD),
            _ => None,
        };
        if needs_cid(c) {
            return CID_WIDTH;
        }
        let Some(table) = table else { return 600 };
        let ascii = |c: char| -> Option<u16> {
            let i = (c as u32).checked_sub(32)? as usize;
            table.get(i).copied()
        };
        if let Some(w) = ascii(c) {
            return w;
        }
        match c {
            '—' | '…' | '‰' | '™' => 1000,
            _ => metrics::fold(c).and_then(ascii).unwrap_or(match self {
                Font::TimesRoman | Font::TimesItalic | Font::TimesBold | Font::TimesBoldItalic => 500,
                _ => 556,
            }),
        }
    }
}

/// The width of `text` in points, when it is set in `font` at `size` points.
pub fn text_width(font: Font, size: f32, text: &str) -> f32 {
    // u64: a u32 sum overflows for about eight million characters.
    let units: u64 = text.chars().map(|c| u64::from(font.char_width(c))).sum();
    units as f32 * size / 1000.0
}

/// A character as a WinAnsiEncoding (CP1252) byte; a character with no byte becomes `?`.
pub fn win_ansi(c: char) -> u8 {
    let u = c as u32;
    if (0x20..0x7F).contains(&u) || (0xA0..=0xFF).contains(&u) {
        return u as u8;
    }
    match c {
        '€' => 0x80,
        '‚' => 0x82,
        'ƒ' => 0x83,
        '„' => 0x84,
        '…' => 0x85,
        '†' => 0x86,
        '‡' => 0x87,
        'ˆ' => 0x88,
        '‰' => 0x89,
        'Š' => 0x8A,
        '‹' => 0x8B,
        'Œ' => 0x8C,
        'Ž' => 0x8E,
        '‘' => 0x91,
        '’' => 0x92,
        '“' => 0x93,
        '”' => 0x94,
        '•' => 0x95,
        '–' => 0x96,
        '—' => 0x97,
        '˜' => 0x98,
        '™' => 0x99,
        'š' => 0x9A,
        '›' => 0x9B,
        'œ' => 0x9C,
        'ž' => 0x9E,
        'Ÿ' => 0x9F,
        '\t' => b' ',
        _ => b'?',
    }
}

/// The slot of the CID font in a page's font flags, after the [`Font::ALL`] faces.
const CID_FONT: usize = Font::ALL.len();
/// The number of font resources a page may use.
const FONT_SLOTS: usize = CID_FONT + 1;
/// The base font of the CID font used for the characters WinAnsiEncoding lacks.
const CID_BASE_FONT: &str = "STSong-Light";
/// The default glyph width of the CID font (full width), in 1/1000 em.
const CID_WIDTH: u16 = 1000;

/// Whether `c` has no WinAnsiEncoding byte, and is therefore drawn with the CID font.
fn needs_cid(c: char) -> bool {
    !c.is_control() && c != '?' && win_ansi(c) == b'?'
}

/// A PDF hex string `< … >` of `text` in UTF-16BE, the `UniGB-UTF16-H` encoding.
fn pdf_utf16_string(text: &str) -> String {
    let mut s = String::with_capacity(text.len() * 4 + 2);
    s.push('<');
    for unit in text.encode_utf16() {
        let _ = write!(s, "{unit:04X}");
    }
    s.push('>');
    s
}

/// A PDF literal string `( … )` of `text` in WinAnsiEncoding.
fn pdf_string(text: &str) -> String {
    let mut s = String::with_capacity(text.len() + 2);
    s.push('(');
    for c in text.chars() {
        let b = win_ansi(c);
        match b {
            b'(' | b')' | b'\\' => {
                s.push('\\');
                s.push(b as char);
            }
            0x20..=0x7E => s.push(b as char),
            _ => {
                let _ = write!(s, "\\{b:03o}");
            }
        }
    }
    s.push(')');
    s
}

/// A PDF literal string of a URI: the non-ASCII bytes are percent-encoded first.
fn pdf_uri_string(uri: &str) -> String {
    let mut ascii = String::with_capacity(uri.len());
    for byte in uri.bytes() {
        if byte.is_ascii() {
            ascii.push(byte as char);
        } else {
            let _ = write!(ascii, "%{byte:02X}");
        }
    }
    pdf_string(&ascii)
}

/// A number for a content stream: finite, and short.
fn n(v: f32) -> String {
    let v = if v.is_finite() { v.clamp(-1.0e7, 1.0e7) } else { 0.0 };
    let r = (v * 1000.0).round() / 1000.0;
    if r == r.trunc() {
        format!("{}", r as i64)
    } else {
        let s = format!("{r:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Whether every value is finite: a NaN and an infinity are refused.
fn finite(v: &[f32]) -> bool {
    v.iter().all(|x| x.is_finite())
}

/// Whether a value is a positive, finite extent — a width, a height or a radius.
fn extent(v: f32) -> bool {
    v > 0.0 && v.is_finite()
}

/// A stroke width: a negative one, a NaN and an infinity become 0, the thinnest line.
fn stroke_width(w: f32) -> f32 {
    if w.is_finite() { w.max(0.0) } else { 0.0 }
}

/// The entries of a dash array (`[on, off, …]`, at most eight).
///
/// A pattern the PDF specification forbids — any non-finite entry, or
/// no entry that survives rounding to a positive number — becomes an
/// empty array, which is a solid line.
fn dash_values(dash: &[f32]) -> String {
    let d: Vec<f32> =
        dash.iter().take(8).map(|v| if v.is_finite() { v.max(0.0) } else { f32::NAN }).collect();
    if !finite(&d) || !d.iter().any(|v| (v.min(1.0e7) * 1000.0).round() > 0.0) {
        return String::new();
    }
    d.iter().map(|v| n(*v)).collect::<Vec<_>>().join(" ")
}

fn rgb_op(c: Rgb, stroke: bool) -> String {
    let f = |v: u8| n(v as f32 / 255.0);
    format!("{} {} {} {}", f(c[0]), f(c[1]), f(c[2]), if stroke { "RG" } else { "rg" })
}

/// A picture held by a page, waiting to be written.
struct ImageData {
    width: u32,
    height: u32,
    rgb: Vec<u8>,
    alpha: Option<Vec<u8>>,
}

/// Where a link goes.
enum LinkTarget {
    /// A URI outside the document.
    Uri(String),
    /// A place on another page of this document.
    Page { index: usize, x: f32, y: f32 },
}

/// An area of a page that goes somewhere.
struct LinkAnnotation {
    /// The rectangle, in PDF coordinates: the origin at the bottom left.
    rect: [f32; 4],
    /// Where it goes.
    target: LinkTarget,
}

/// A page under construction. Get one with [`PdfDoc::add_page`].
pub struct Page {
    width: f32,
    height: f32,
    content: String,
    fonts: [bool; FONT_SLOTS],
    images: Vec<ImageData>,
    links: Vec<LinkAnnotation>,
    depth: u32,
}

/// The most bytes of content one page may hold; past it, further drawing is dropped.
const MAX_PAGE_CONTENT: usize = 64 * 1024 * 1024;
/// The most pixels one embedded picture may hold.
const MAX_IMAGE_PIXELS: u64 = 40_000_000;
/// The most links one page may hold.
const MAX_PAGE_LINKS: usize = 4096;
/// The most bytes one link's URI may hold.
const MAX_LINK_URI_BYTES: usize = 4096;

impl Page {
    /// The width of the page, in points.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// The height of the page, in points.
    pub fn height(&self) -> f32 {
        self.height
    }

    /// The page's own `y`: the distance from the **top**, flipped into PDF's `y`.
    fn y(&self, y: f32) -> f32 {
        self.height - y
    }

    /// Whether the page has as much content as it may hold.
    fn full(&self) -> bool {
        self.content.len() > MAX_PAGE_CONTENT
    }

    /// Adds an operator to the content of the page, unless the page is full.
    fn op(&mut self, s: &str) {
        if !self.full() {
            self.content.push_str(s);
            self.content.push('\n');
        }
    }

    /// The rectangle of a link, in PDF coordinates, if it is one that may be drawn.
    fn link_rect(&self, rect: Rect) -> Option<[f32; 4]> {
        let Rect { x, y, width, height } = rect;
        if !finite(&[x, y, width, height])
            || !extent(width)
            || !extent(height)
            || self.links.len() >= MAX_PAGE_LINKS
        {
            return None;
        }
        let (right, bottom) = (x + width, y + height);
        if !finite(&[right, bottom]) {
            return None;
        }
        let (left, right) = (x.max(0.0), right.min(self.width));
        let (top, bottom) = (y.max(0.0), bottom.min(self.height));
        (right > left && bottom > top).then(|| [left, self.y(bottom), right, self.y(top)])
    }

    /// Adds an invisible clickable area that goes to a URI.
    ///
    /// A link to a scheme that is not `http`, `https` or `mailto`, a
    /// link whose URI is longer than 4096 bytes, and a link whose URI
    /// holds a control character are all refused.
    pub fn link_uri(&mut self, x: f32, y: f32, w: f32, h: f32, uri: &str) {
        let allowed = uri.split_once(':').is_some_and(|(scheme, rest)| {
            matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https" | "mailto") && !rest.is_empty()
        });
        if !allowed || uri.len() > MAX_LINK_URI_BYTES || uri.chars().any(char::is_control) {
            return;
        }
        if let Some(rect) = self.link_rect(Rect::new(x, y, w, h)) {
            self.links.push(LinkAnnotation { rect, target: LinkTarget::Uri(uri.to_string()) });
        }
    }

    /// Adds a link to a place on another page of this document.
    ///
    /// The page is named by its zero-based number; a link to a page that
    /// is not there is dropped when the document is written.
    pub fn link_page(&mut self, rect: Rect, page: usize, target: Point) {
        if !finite(&[target.x, target.y]) {
            return;
        }
        if let Some(rect) = self.link_rect(rect) {
            self.links.push(LinkAnnotation {
                rect,
                target: LinkTarget::Page { index: page, x: target.x, y: target.y },
            });
        }
    }

    /// Saves the graphics state; pair it with [`Page::restore`].
    pub fn save(&mut self) {
        self.depth = self.depth.saturating_add(1);
        self.content.push_str("q\n");
    }

    /// Restores the graphics state saved by [`Page::save`]; a call with nothing saved is ignored.
    pub fn restore(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
            self.content.push_str("Q\n");
        }
    }

    /// Intersects the clip with a rectangle, until the next [`Page::restore`].
    pub fn clip_rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        let s = format!("{} {} {} {} re W n", n(x), n(self.y(y + h)), n(w.max(0.0)), n(h.max(0.0)));
        self.op(&s);
    }

    /// Fills a rectangle with a colour.
    pub fn fill_rect(&mut self, x: f32, y: f32, w: f32, h: f32, color: Rgb) {
        if !(extent(w) && extent(h) && finite(&[x, y])) {
            return;
        }
        let s = format!("{}\n{} {} {} {} re f", rgb_op(color, false), n(x), n(self.y(y + h)), n(w), n(h));
        self.op(&s);
    }

    /// Draws the outline of a rectangle.
    pub fn rect_stroke(&mut self, x: f32, y: f32, w: f32, h: f32, width: f32, color: Rgb) {
        if !finite(&[x, y, w, h]) {
            return;
        }
        let s = format!(
            "{}\n{} w [] 0 d\n{} {} {} {} re S",
            rgb_op(color, true),
            n(stroke_width(width)),
            n(x),
            n(self.y(y + h)),
            n(w),
            n(h)
        );
        self.op(&s);
    }

    /// Draws a line.
    pub fn stroke_line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, width: f32, color: Rgb) {
        self.stroke_line_dashed(Point::new(x0, y0), Point::new(x1, y1), width, color, &[]);
    }

    /// Draws a line with a dash pattern (`[on, off, …]` in points; empty is solid).
    pub fn stroke_line_dashed(&mut self, start: Point, end: Point, width: f32, color: Rgb, dash: &[f32]) {
        if !finite(&[start.x, start.y, end.x, end.y]) {
            return;
        }
        let s = format!(
            "{}\n{} w [{}] 0 d\n{} {} m {} {} l S",
            rgb_op(color, true),
            n(stroke_width(width)),
            dash_values(dash),
            n(start.x),
            n(self.y(start.y)),
            n(end.x),
            n(self.y(end.y))
        );
        self.op(&s);
    }

    /// Draws an open polyline.
    pub fn polyline(&mut self, pts: &[[f32; 2]], width: f32, color: Rgb, dash: &[f32]) {
        if pts.len() < 2 || !pts.iter().all(|p| finite(p)) {
            return;
        }
        let mut s = format!(
            "{}\n{} w [{}] 0 d 1 j\n",
            rgb_op(color, true),
            n(stroke_width(width)),
            dash_values(dash)
        );
        self.path(&mut s, pts, false);
        s.push_str(" S");
        self.op(&s);
    }

    /// Writes the path of a polyline into the content.
    fn path(&self, s: &mut String, pts: &[[f32; 2]], close: bool) {
        for (i, p) in pts.iter().enumerate() {
            let _ = write!(s, "{} {} {} ", n(p[0]), n(self.y(p[1])), if i == 0 { "m" } else { "l" });
        }
        if close {
            s.push('h');
        }
    }

    /// Draws a closed polygon, filled and/or stroked.
    pub fn polygon(&mut self, pts: &[[f32; 2]], fill: Option<Rgb>, stroke: Option<(Rgb, f32)>) {
        if pts.len() < 2 || (fill.is_none() && stroke.is_none()) || !pts.iter().all(|p| finite(p)) {
            return;
        }
        let mut s = String::new();
        if let Some(f) = fill {
            s.push_str(&rgb_op(f, false));
            s.push('\n');
        }
        if let Some((c, w)) = stroke {
            let _ = writeln!(s, "{}\n{} w [] 0 d 1 j", rgb_op(c, true), n(stroke_width(w)));
        }
        self.path(&mut s, pts, true);
        s.push_str(match (fill.is_some(), stroke.is_some()) {
            (true, true) => " B",
            (true, false) => " f",
            _ => " S",
        });
        self.op(&s);
    }

    /// Draws a circle, as four cubic Béziers.
    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, fill: Option<Rgb>, stroke: Option<(Rgb, f32)>) {
        if !extent(r) || !finite(&[cx, cy]) || (fill.is_none() && stroke.is_none()) {
            return;
        }
        const K: f32 = 0.552_284_8;
        let k = r * K;
        let y = self.y(cy);
        let mut s = String::new();
        if let Some(f) = fill {
            s.push_str(&rgb_op(f, false));
            s.push('\n');
        }
        if let Some((c, w)) = stroke {
            let _ = writeln!(s, "{}\n{} w [] 0 d", rgb_op(c, true), n(stroke_width(w)));
        }
        let _ = write!(
            s,
            "{} {} m {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c {} {} {} {} {} {} c h",
            n(cx + r),
            n(y),
            n(cx + r),
            n(y + k),
            n(cx + k),
            n(y + r),
            n(cx),
            n(y + r),
            n(cx - k),
            n(y + r),
            n(cx - r),
            n(y + k),
            n(cx - r),
            n(y),
            n(cx - r),
            n(y - k),
            n(cx - k),
            n(y - r),
            n(cx),
            n(y - r),
            n(cx + k),
            n(y - r),
            n(cx + r),
            n(y - k),
            n(cx + r),
            n(y)
        );
        s.push_str(match (fill.is_some(), stroke.is_some()) {
            (true, true) => " B",
            (true, false) => " f",
            _ => " S",
        });
        self.op(&s);
    }

    /// Writes text, with its baseline starting at `(x, y)`.
    pub fn text(&mut self, x: f32, y: f32, size: f32, font: Font, color: Rgb, text: &str) {
        self.text_rotated(Point::new(x, y), size, font, color, text, 0.0);
    }

    /// Writes text turned by `angle` radians about its baseline origin.
    ///
    /// The angle is **clockwise on the page**, as the `y`-down
    /// coordinates of this crate are.
    pub fn text_rotated(&mut self, origin: Point, size: f32, font: Font, color: Rgb, text: &str, angle: f32) {
        if text.is_empty() || !extent(size) || !finite(&[origin.x, origin.y]) {
            return;
        }
        let fi = font.index();
        let size = n(size.min(10_000.0));
        let (sn, cs) = if angle.is_finite() { angle.sin_cos() } else { (0.0, 1.0) };
        // Clockwise in a y-down system is counter-clockwise negated in PDF's y-up one.
        let mut s = format!(
            "BT\n{}\n{} {} {} {} {} {} Tm\n",
            rgb_op(color, false),
            n(cs),
            n(-sn),
            n(sn),
            n(cs),
            n(origin.x),
            n(self.y(origin.y))
        );
        // A run of WinAnsi text uses `font`; a run it cannot encode uses the CID
        // font. Each `Tj` advances the text position, so the runs follow each
        // other along the baseline.
        let mut rest = text;
        while let Some(first) = rest.chars().next() {
            let cid = needs_cid(first);
            let end = rest.char_indices().find(|(_, c)| needs_cid(*c) != cid).map_or(rest.len(), |(i, _)| i);
            let (run, tail) = rest.split_at(end);
            let (slot, string) = if cid { (CID_FONT, pdf_utf16_string(run)) } else { (fi, pdf_string(run)) };
            if let Some(f) = self.fonts.get_mut(slot) {
                *f = true;
            }
            let _ = writeln!(s, "/F{} {} Tf\n{} Tj", slot + 1, size, string);
            rest = tail;
        }
        s.push_str("ET");
        self.op(&s);
    }

    /// Draws a picture — `pw` × `ph` pixels, row-major, straight alpha — into
    /// the box `(x, y, w, h)`.
    ///
    /// A buffer that is too short, and a picture of more than forty
    /// million pixels, are refused.
    pub fn image_rgba(&mut self, destination: Rect, rgba: &[u8], pixel_size: [u32; 2]) {
        let Rect { x, y, width: w, height: h } = destination;
        let [pw, ph] = pixel_size;
        let px = pw as u64 * ph as u64;
        if pw == 0
            || ph == 0
            || px > MAX_IMAGE_PIXELS
            || (rgba.len() as u64) < px * 4
            || !(extent(w) && extent(h) && finite(&[x, y]))
        {
            return;
        }
        let px = px as usize;
        let mut rgb = Vec::with_capacity(px * 3);
        let mut alpha = Vec::with_capacity(px);
        for p in rgba.as_chunks::<4>().0.iter().take(px) {
            rgb.extend_from_slice(&[p[0], p[1], p[2]]);
            alpha.push(p[3]);
        }
        let alpha = if alpha.iter().all(|a| *a == 255) { None } else { Some(alpha) };
        let idx = self.images.len();
        self.images.push(ImageData { width: pw, height: ph, rgb, alpha });
        let s = format!("q {} 0 0 {} {} {} cm /Im{} Do Q", n(w), n(h), n(x), n(self.y(y + h)), idx + 1);
        self.op(&s);
    }
}

/// What the file says about itself.
#[derive(Clone, Debug, Default)]
pub struct Info {
    /// The title of the document.
    pub title: String,
    /// The author of the document.
    pub author: String,
    /// The program that made the file. Without one, the writer's own name is used.
    pub creator: String,
}

/// A PDF document under construction.
#[derive(Default)]
pub struct PdfDoc {
    pages: Vec<Page>,
    /// What the file says about itself.
    pub info: Info,
}

/// The run-length encoding of PDF's `RunLengthDecode` filter.
fn run_length(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 2 + 16);
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        let mut run = 1;
        while i + run < data.len() && run < 128 && data[i + run] == b {
            run += 1;
        }
        if run >= 2 {
            out.push((257 - run) as u8);
            out.push(b);
            i += run;
            continue;
        }
        // A literal run, up to the next repeat of three or more.
        let start = i;
        let mut len = 0;
        while i < data.len() && len < 128 {
            if i + 2 < data.len() && data[i] == data[i + 1] && data[i] == data[i + 2] {
                break;
            }
            i += 1;
            len += 1;
        }
        if len == 0 {
            // The next bytes are a repeat; the loop above will take them.
            continue;
        }
        out.push((len - 1) as u8);
        out.extend_from_slice(&data[start..start + len]);
    }
    out.push(128);
    out
}

/// The most characters one Info string may hold; a longer one is cut on a character boundary.
const MAX_INFO_CHARS: usize = 1024;

/// A PDF text string (an Info entry) of `text`.
///
/// A printable ASCII string is a literal `( … )`; any other is UTF-16BE
/// with a byte-order mark, as a hex string (ISO 32000-1 7.9.2.2).
fn pdf_text_string(text: &str) -> String {
    let text: String = text.chars().take(MAX_INFO_CHARS).collect();
    if text.chars().all(|c| matches!(c, ' '..='~')) {
        return pdf_string(&text);
    }
    let mut s = String::with_capacity(text.len() * 4 + 6);
    s.push_str("<FEFF");
    for unit in text.encode_utf16() {
        let _ = write!(s, "{unit:04X}");
    }
    s.push('>');
    s
}

impl PdfDoc {
    /// A document with no pages.
    pub fn new() -> PdfDoc {
        PdfDoc::default()
    }

    /// Adds a page of `width_pt` × `height_pt` points, and gives it back to draw on.
    ///
    /// A page that is not a sensible size is made one: a Letter page.
    pub fn add_page(&mut self, width_pt: f32, height_pt: f32) -> &mut Page {
        let width = if width_pt.is_finite() { width_pt.clamp(3.0, 14_400.0) } else { 612.0 };
        let height = if height_pt.is_finite() { height_pt.clamp(3.0, 14_400.0) } else { 792.0 };
        self.pages.push(Page {
            width,
            height,
            content: String::new(),
            fonts: [false; FONT_SLOTS],
            images: Vec::new(),
            links: Vec::new(),
            depth: 0,
        });
        let i = self.pages.len() - 1;
        &mut self.pages[i]
    }

    /// The number of pages.
    pub fn page_count(&self) -> usize {
        self.pages.len()
    }

    /// The page at `i`, to keep drawing on it.
    pub fn page_mut(&mut self, i: usize) -> Option<&mut Page> {
        self.pages.get_mut(i)
    }

    /// Writes the document out as the bytes of a PDF file.
    ///
    /// A document with no pages gets one blank Letter page.
    pub fn finish(mut self) -> Vec<u8> {
        if self.pages.is_empty() {
            self.add_page(612.0, 792.0);
        }
        // The numbering of the objects: 1 the catalogue, 2 the page tree,
        // 3 the info, then the fonts, then the pages.
        let mut used = [false; FONT_SLOTS];
        for p in &self.pages {
            for (u, f) in used.iter_mut().zip(p.fonts.iter()) {
                *u |= *f;
            }
        }
        let mut next = 4u32;
        let mut font_ids = [0u32; FONT_SLOTS];
        for (i, u) in used.iter().enumerate() {
            if *u {
                font_ids[i] = next;
                // The CID font needs its descendant font and its font descriptor too.
                next += if i == CID_FONT { 3 } else { 1 };
            }
        }
        let mut objects: Vec<(u32, Vec<u8>)> = Vec::new();
        let mut kids = Vec::new();
        let mut annotations = Vec::new();
        let page_count = self.pages.len();
        let mut font_dict = String::new();
        for (i, id) in font_ids.iter().enumerate() {
            if *id != 0 {
                let _ = write!(font_dict, "/F{} {} 0 R ", i + 1, id);
                if i == CID_FONT {
                    let (cid_id, desc_id) = (id + 1, id + 2);
                    objects.push((
                        *id,
                        format!(
                            "<< /Type /Font /Subtype /Type0 /BaseFont /{CID_BASE_FONT} /Encoding /UniGB-UTF16-H /DescendantFonts [{cid_id} 0 R] >>"
                        )
                        .into_bytes(),
                    ));
                    objects.push((
                        cid_id,
                        format!(
                            "<< /Type /Font /Subtype /CIDFontType0 /BaseFont /{CID_BASE_FONT} /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 4 >> /FontDescriptor {desc_id} 0 R /DW {CID_WIDTH} >>"
                        )
                        .into_bytes(),
                    ));
                    objects.push((
                        desc_id,
                        format!(
                            "<< /Type /FontDescriptor /FontName /{CID_BASE_FONT} /Flags 6 /FontBBox [-25 -254 1000 880] /ItalicAngle 0 /Ascent 880 /Descent -120 /CapHeight 880 /StemV 93 >>"
                        )
                        .into_bytes(),
                    ));
                    continue;
                }
                let name = Font::ALL.get(i).copied().unwrap_or_default().base_name();
                let enc = if name.starts_with("Symbol") { "" } else { " /Encoding /WinAnsiEncoding" };
                objects.push((
                    *id,
                    format!("<< /Type /Font /Subtype /Type1 /BaseFont /{name}{enc} >>").into_bytes(),
                ));
            }
        }
        for page in &mut self.pages {
            while page.depth > 0 {
                page.restore();
            }
            let mut xobjects = String::new();
            for (k, img) in page.images.iter().enumerate() {
                let img_id = next;
                next += 1;
                let smask = match &img.alpha {
                    Some(a) => {
                        let id = next;
                        next += 1;
                        let data = run_length(a);
                        let mut o = format!(
                            "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /RunLengthDecode /Length {} >>\nstream\n",
                            img.width,
                            img.height,
                            data.len()
                        )
                        .into_bytes();
                        o.extend_from_slice(&data);
                        o.extend_from_slice(b"\nendstream");
                        objects.push((id, o));
                        format!(" /SMask {id} 0 R")
                    }
                    None => String::new(),
                };
                let data = run_length(&img.rgb);
                let mut o = format!(
                    "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8{} /Filter /RunLengthDecode /Length {} >>\nstream\n",
                    img.width,
                    img.height,
                    smask,
                    data.len()
                )
                .into_bytes();
                o.extend_from_slice(&data);
                o.extend_from_slice(b"\nendstream");
                objects.push((img_id, o));
                let _ = write!(xobjects, "/Im{} {} 0 R ", k + 1, img_id);
            }
            let content_id = next;
            let page_id = next + 1;
            next += 2;
            let body = std::mem::take(&mut page.content);
            let mut o = format!("<< /Length {} >>\nstream\n", body.len()).into_bytes();
            o.extend_from_slice(body.as_bytes());
            o.extend_from_slice(b"endstream");
            objects.push((content_id, o));
            let xo = if xobjects.is_empty() { String::new() } else { format!(" /XObject << {xobjects}>>") };
            let mut link_refs = String::new();
            for link in std::mem::take(&mut page.links) {
                if matches!(&link.target, LinkTarget::Page { index, .. } if *index >= page_count) {
                    continue;
                }
                let id = next;
                next += 1;
                let _ = write!(link_refs, "{id} 0 R ");
                annotations.push((id, link));
            }
            let annots = if link_refs.is_empty() { String::new() } else { format!(" /Annots [{link_refs}]") };
            objects.push((
                page_id,
                format!(
                    "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /ProcSet [/PDF /Text /ImageB /ImageC] /Font << {}>>{} >> /Contents {} 0 R{} >>",
                    n(page.width),
                    n(page.height),
                    font_dict,
                    xo,
                    content_id,
                    annots
                )
                .into_bytes(),
            ));
            kids.push(page_id);
        }
        for (id, link) in annotations {
            let [x0, y0, x1, y1] = link.rect;
            let action = match link.target {
                LinkTarget::Uri(uri) => format!("/A << /S /URI /URI {} >>", pdf_uri_string(&uri)),
                LinkTarget::Page { index, x, y } => {
                    let Some(&page_id) = kids.get(index) else { continue };
                    let height = self.pages.get(index).map_or(0.0, |p| p.height);
                    format!("/Dest [{page_id} 0 R /XYZ {} {} null]", n(x), n(height - y))
                }
            };
            objects.push((
                id,
                format!(
                    "<< /Type /Annot /Subtype /Link /Rect [{} {} {} {}] /Border [0 0 0] {action} >>",
                    n(x0),
                    n(y0),
                    n(x1),
                    n(y1)
                )
                .into_bytes(),
            ));
        }
        objects.push((1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));
        let kids_s: Vec<String> = kids.iter().map(|k| format!("{k} 0 R")).collect();
        objects.push((
            2,
            format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids_s.join(" "), kids.len()).into_bytes(),
        ));
        let creator = if self.info.creator.is_empty() { "craftkit" } else { &self.info.creator };
        let mut info =
            format!("<< /Producer {} /Creator {}", pdf_text_string("craftkit"), pdf_text_string(creator));
        if !self.info.title.is_empty() {
            let _ = write!(info, " /Title {}", pdf_text_string(&self.info.title));
        }
        if !self.info.author.is_empty() {
            let _ = write!(info, " /Author {}", pdf_text_string(&self.info.author));
        }
        info.push_str(" >>");
        objects.push((3, info.into_bytes()));
        objects.sort_by_key(|(id, _)| *id);

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(b"%PDF-1.7\n%\xE2\xE3\xCF\xD3\n");
        let mut offsets = vec![0usize; next as usize];
        for (id, body) in &objects {
            if let Some(slot) = offsets.get_mut(*id as usize) {
                *slot = out.len();
            }
            out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
            out.extend_from_slice(body);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        let mut x = format!("xref\n0 {}\n0000000000 65535 f \n", next);
        for off in offsets.iter().skip(1) {
            let _ = writeln!(x, "{off:010} 00000 n ");
        }
        let _ =
            write!(x, "trailer\n<< /Size {} /Root 1 0 R /Info 3 0 R >>\nstartxref\n{}\n%%EOF\n", next, xref);
        out.extend_from_slice(x.as_bytes());
        out
    }
}
