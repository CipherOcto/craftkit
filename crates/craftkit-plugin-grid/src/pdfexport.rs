//! The **export** of a sheet as a **PDF file**.
//!
//! A sheet is printed on paper: a size, an orientation and four
//! margins, and the sheet is broken into **pages** — the rows that go
//! on one page and the columns that go on it. The pages are
//! **derived**: they are worked out from the paper and the sheet as they
//! are, and they are never written down (the module `printing`).
//!
//! This module **draws** those pages and writes them out as a PDF file.
//!
//! The rules:
//!
//! - the file is written where the caller says, and the answer says how
//!   many pages it has;
//! - a sheet that holds nothing has **no pages**, and nothing is
//!   written;
//! - the **size** of a cell on the page is the size of the row and of
//!   the column it is in, at the **scale** the sheet prints at;
//! - the **text** of a cell is the text the cell **shows**: its value
//!   written with its format;
//! - the **look** of a cell is its own word first and the word of the
//!   style it names second — the tree's rule, and the same rule the
//!   cell's own reading uses;
//! - a **number** that does not fit in its cell is drawn as a row of
//!   `#`, as a spreadsheet draws it, and **text** that does not fit runs
//!   over the cells beside it when they are empty;
//! - a **merged** cell is drawn over the whole of its region, and the
//!   cells it covers are not drawn at all.

use craftkit_derive::commands;

use craftkit_pdf_writer::{Family, Font as PdfFont, Page as PdfPage, PdfDoc};
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Colour, EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::{Align, AlignH, AlignV, CellRef, Font, Value};
use crate::printing::{Pages, Setup};

/// The **size of the text** of a cell, when the cell names none: in points.
const DEFAULT_FONT_SIZE: f64 = 11.0;

/// The **inset** of the text in its cell, in points.
const PAD: f64 = 2.0;

/// The **height of a line** of text, as a share of the size of the font.
const LINE: f64 = 1.2;

/// The **colour of the gridlines**: the tree's own grey.
const GRIDLINE: [u8; 3] = [176, 176, 176];

/// The **most** lines one cell's text may be broken into.
const MAX_LINES: usize = 200;

/// The **most** characters of a cell's text that are drawn.
const MAX_CHARS: usize = 10_000;

/// The **most** cells a cell's text may run over.
const MAX_SPILL: u32 = 64;

/// The **fill of a heading**: the tree's own pale grey.
const HEADING_FILL: [u8; 3] = [242, 242, 242];

/// The **line around a heading**.
const HEADING_LINE: [u8; 3] = [160, 160, 160];

/// The **text of a heading**.
const HEADING_TEXT: [u8; 3] = [64, 64, 64];

/// The **size of the text of a heading**, in points.
const HEADING_SIZE: f64 = 9.0;

/// The **size of the text of a header or of a footer**, in points.
const HEADER_SIZE: f64 = 10.0;

/// The **most** lines a header or a footer may hold.
const MAX_HEADER_LINES: usize = 20;

/// A **colour** of the tree, as the three bytes a PDF wants.
fn rgb_of(text: &str) -> Option<[u8; 3]> {
    let colour = Colour::from_hex(text).ok()?;
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some([channel(colour.0[0]), channel(colour.0[1]), channel(colour.0[2])])
}

/// The **face** a cell's text is drawn in.
fn face_of(font: Option<&Font>) -> PdfFont {
    let family = font.and_then(|font| font.name.as_deref()).map(PdfFont::family_for).unwrap_or(Family::Sans);
    PdfFont::styled(family, font.is_some_and(|font| font.bold), font.is_some_and(|font| font.italic))
}

/// The **size of the text** of a cell, in points: the font's own size, or
/// the sheet's, at the scale the sheet prints at.
fn size_of(font: Option<&Font>, scale: f64) -> f64 {
    let size = font
        .and_then(|font| font.size)
        .filter(|size| size.is_finite() && *size > 0.0 && *size <= 400.0)
        .unwrap_or(DEFAULT_FONT_SIZE);
    (size * scale).clamp(1.0, 400.0)
}

/// The **width** of a text, in points, as the face measures it.
fn width_of(face: PdfFont, size: f64, text: &str) -> f64 {
    craftkit_pdf_writer::text_width(face, size as f32, text) as f64
}

/// The text of a cell, **broken into lines** that fit in `width`.
fn wrap_lines(text: &str, face: PdfFont, size: f64, width: f64) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split(' ') {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if line.is_empty() || width_of(face, size, &candidate) <= width {
                line = candidate;
            } else {
                out.push(std::mem::take(&mut line));
                line = word.to_string();
            }
            // A word that is longer than the cell is broken by characters.
            while width_of(face, size, &line) > width && line.chars().count() > 1 {
                let characters: Vec<char> = line.chars().collect();
                let mut cut = characters.len() - 1;
                while cut > 1 && width_of(face, size, &characters[..cut].iter().collect::<String>()) > width {
                    cut -= 1;
                }
                out.push(characters[..cut].iter().collect());
                line = characters[cut..].iter().collect();
            }
            if out.len() >= MAX_LINES {
                return out;
            }
        }
        out.push(line);
        if out.len() >= MAX_LINES {
            break;
        }
    }
    out
}

/// The **look** of a cell: its font, its alignment and its fill.
///
/// The tree's rule: the cell's own word comes first, and the word of
/// the style it names comes second. The fill of a **conditional rule**
/// is on top of both, as the rule is on top of the cell.
struct Look {
    font: Option<Font>,
    align: Option<Align>,
    fill: Option<String>,
}

/// The look of a cell, as the tree's own rule says it.
fn look_of(
    session: &Session,
    sheet: u64,
    node: &Node,
    cells: &crate::formula::Cells,
    cell: CellRef,
    value: &Value,
) -> Look {
    let styles = crate::styles_of(session, sheet);
    let style = cells
        .get(cell)
        .and_then(|entry| entry.style.as_deref())
        .and_then(|name| styles.iter().find(|style| style.name == name));
    let own = cells.get(cell);
    let looks = crate::conditional::looks_for(node, cell, value);
    Look {
        font: own.and_then(|entry| entry.font.clone()).or_else(|| style.and_then(|style| style.font.clone())),
        align: own.and_then(|entry| entry.align).or_else(|| style.and_then(|style| style.align)),
        fill: looks.fill.clone().or_else(|| own.and_then(|entry| entry.fill.clone())),
    }
}

/// The **value** of a cell: the one it holds, or the one its formula
/// computes.
fn value_of(session: &Session, name: &str, cells: &crate::formula::Cells, cell: CellRef) -> Value {
    match cells.get(cell) {
        Some(entry) => match (&entry.value, &entry.formula) {
            (Some(value), _) => value.clone(),
            (None, Some(_)) => crate::book_of(session.document()).value_of(name, cell),
            (None, None) => Value::Empty,
        },
        None => Value::Empty,
    }
}

/// The **horizontal** place of the text of a cell.
///
/// The tree's rule: a cell that is **general** puts a number to the
/// right, a truth or an error in the middle, and text to the left.
fn horizontal(align: AlignH, value: &Value) -> AlignH {
    match align {
        AlignH::General => match value {
            Value::Number(_) => AlignH::Right,
            Value::Bool(_) | Value::Error(_) => AlignH::Center,
            _ => AlignH::Left,
        },
        AlignH::CenterAcross => AlignH::Center,
        AlignH::Justify | AlignH::Fill => AlignH::Left,
        other => other,
    }
}

/// The **sheet** as it is drawn: everything a page needs to know.
struct Drawing<'a> {
    session: &'a Session,
    sheet: u64,
    node: &'a Node,
    cells: &'a crate::formula::Cells,
    setup: &'a Setup,
    pages: &'a Pages,
    /// The number of this page, from one.
    number: usize,
    /// The number of pages of the sheet.
    count: usize,
}

/// The **place of the rows and the columns** of a page.
struct Places<'a> {
    pages: &'a Pages,
    node: &'a Node,
    rows: &'a [u32],
    columns: &'a [u32],
    xs: &'a [f64],
    ys: &'a [f64],
    /// The left margin: where the strip of the row numbers is.
    left: f64,
    /// The top margin: where the strip of the column names is.
    top: f64,
}

/// Draws the **headings** of the sheet: the number of each row down the
/// left and the name of each column across the top.
///
/// The headings take their room from the page, so they are drawn in the
/// room the page set aside for them.
fn draw_headings(page: &mut PdfPage, places: &Places<'_>) {
    let Places { pages, node, rows, columns, xs, ys, left, top } = places;
    let (pages, node) = (*pages, *node);
    let (left, top) = (*left, *top);
    let (heading_width, heading_height) = pages.heading_room;
    if heading_width <= 0.0 || heading_height <= 0.0 {
        return;
    }
    let scale = pages.scale;
    let size = (HEADING_SIZE * scale).clamp(1.0, 400.0);
    let face = PdfFont::styled(Family::Sans, false, false);
    let line = (0.35 * scale).clamp(0.1, 0.5);
    // The **columns**: their names across the top.
    for (index, column) in columns.iter().enumerate() {
        let x = xs.get(index).copied().unwrap_or(left);
        let width = crate::printing::column_width(node, *column) * scale;
        page.fill_rect(x as f32, top as f32, width as f32, heading_height as f32, HEADING_FILL);
        page.rect_stroke(
            x as f32,
            top as f32,
            width as f32,
            heading_height as f32,
            line as f32,
            HEADING_LINE,
        );
        let text = CellRef::column_text(*column);
        let measured = width_of(face, size, &text);
        page.text(
            (x + (width - measured) / 2.0) as f32,
            (top + heading_height / 2.0 + size * 0.35) as f32,
            size as f32,
            face,
            HEADING_TEXT,
            &text,
        );
    }
    // The **rows**: their numbers down the left.
    for (index, row) in rows.iter().enumerate() {
        let y = ys.get(index).copied().unwrap_or(top);
        let height = crate::printing::row_height(node, *row) * scale;
        page.fill_rect(left as f32, y as f32, heading_width as f32, height as f32, HEADING_FILL);
        page.rect_stroke(
            left as f32,
            y as f32,
            heading_width as f32,
            height as f32,
            line as f32,
            HEADING_LINE,
        );
        let text = (row + 1).to_string();
        let measured = width_of(face, size, &text);
        page.text(
            (left + (heading_width - measured) / 2.0) as f32,
            (y + height / 2.0 + size * 0.35) as f32,
            size as f32,
            face,
            HEADING_TEXT,
            &text,
        );
    }
}

/// Draws the **header** and the **footer** of a page.
///
/// The header is above the grid, inside the top margin, and the footer
/// is below it, inside the bottom margin. The three sections of each are
/// at the left, in the middle and at the right of the page.
fn draw_header_footer(
    page: &mut PdfPage,
    node: &Node,
    number: usize,
    count: usize,
    setup: &Setup,
    page_width: f64,
    page_height: f64,
) {
    let (header, footer) = crate::headers::of_page(node, number, count);
    let margins = setup.margins;
    let size = HEADER_SIZE;
    let left = margins[0];
    let right = page_width - margins[1];
    // The **header**: the last of its lines sits just above the grid.
    for (index, section) in [&header.left, &header.centre, &header.right].iter().enumerate() {
        if section.text.trim().is_empty() {
            continue;
        }
        let face = PdfFont::styled(Family::Sans, section.bold, section.italic);
        let lines: Vec<&str> = section.text.split('\n').take(MAX_HEADER_LINES).collect();
        let count = lines.len() as f64;
        for (line_number, line) in lines.iter().enumerate() {
            let measured = width_of(face, size, line);
            let x = match index {
                0 => left,
                1 => (page_width - measured) / 2.0,
                _ => right - measured,
            };
            let y = margins[2] - size * 0.22 - (count - 1.0 - line_number as f64) * size * 1.2;
            page.text(x as f32, y as f32, size as f32, face, [0, 0, 0], line);
        }
    }
    // The **footer**: the first of its lines sits just below the grid.
    for (index, section) in [&footer.left, &footer.centre, &footer.right].iter().enumerate() {
        if section.text.trim().is_empty() {
            continue;
        }
        let face = PdfFont::styled(Family::Sans, section.bold, section.italic);
        let lines: Vec<&str> = section.text.split('\n').take(MAX_HEADER_LINES).collect();
        for (line_number, line) in lines.iter().enumerate() {
            let measured = width_of(face, size, line);
            let x = match index {
                0 => left,
                1 => (page_width - measured) / 2.0,
                _ => right - measured,
            };
            let y = page_height - margins[3] + size * 0.8 + line_number as f64 * size * 1.2;
            page.text(x as f32, y as f32, size as f32, face, [0, 0, 0], line);
        }
    }
}

/// Draws one **page** of the sheet.
fn draw_page(page: &mut PdfPage, drawing: &Drawing<'_>, band: ((u32, u32), (u32, u32))) {
    let Drawing { session, sheet, node, cells, setup, pages, number, count } = drawing;
    let (session, sheet, node, cells, setup, pages) = (*session, *sheet, *node, *cells, *setup, *pages);
    let (number, count) = (*number, *count);
    let scale = pages.scale;
    let name = node.name.clone();
    let margins = setup.margins;
    let origin_x = margins[0] + pages.heading_room.0;
    let origin_y = margins[2] + pages.heading_room.1;
    // The **rows** and the **columns** of the page: the titles first,
    // and then the band of the page.
    let (band_rows, band_columns) = band;
    let (title_rows, title_columns) = &pages.titles;
    let mut rows: Vec<u32> = title_rows.clone();
    rows.extend(band_rows.0..=band_rows.1);
    let mut columns: Vec<u32> = title_columns.clone();
    columns.extend(band_columns.0..=band_columns.1);
    // The place of each of them on the page.
    let mut xs: Vec<f64> = Vec::with_capacity(columns.len());
    let mut x = origin_x;
    for column in &columns {
        xs.push(x);
        x += crate::printing::column_width(node, *column) * scale;
    }
    let mut ys: Vec<f64> = Vec::with_capacity(rows.len());
    let mut y = origin_y;
    for row in &rows {
        ys.push(y);
        y += crate::printing::row_height(node, *row) * scale;
    }
    let block_width = x - origin_x;
    let block_height = y - origin_y;
    // The **gridlines**, when the sheet says they are printed.
    if pages.gridlines {
        let width = (0.35 * scale).clamp(0.1, 0.5) as f32;
        for x in &xs {
            page.stroke_line(
                *x as f32,
                origin_y as f32,
                *x as f32,
                (origin_y + block_height) as f32,
                width,
                GRIDLINE,
            );
        }
        for y in &ys {
            page.stroke_line(
                origin_x as f32,
                *y as f32,
                (origin_x + block_width) as f32,
                *y as f32,
                width,
                GRIDLINE,
            );
        }
        page.stroke_line(
            x as f32,
            origin_y as f32,
            x as f32,
            (origin_y + block_height) as f32,
            width,
            GRIDLINE,
        );
        page.stroke_line(
            origin_x as f32,
            y as f32,
            (origin_x + block_width) as f32,
            y as f32,
            width,
            GRIDLINE,
        );
    }
    // The **headings**: the numbers of the rows and the names of the
    // columns, in the room the page set aside for them.
    if pages.headings {
        let places = Places {
            pages,
            node,
            rows: &rows,
            columns: &columns,
            xs: &xs,
            ys: &ys,
            left: margins[0],
            top: margins[2],
        };
        draw_headings(page, &places);
    }
    // The **header** and the **footer** of the page.
    let (page_width, page_height) = setup.page();
    draw_header_footer(page, node, number, count, setup, page_width, page_height);
    // The **merges** of the sheet: a merged cell is drawn over the whole
    // of its region, and the cells it covers are not drawn at all.
    let merges = crate::merges::merges_of(node);
    let place_of = |column: u32, columns: &[u32], xs: &[f64]| -> f64 {
        match columns.iter().position(|other| *other == column) {
            Some(index) => xs[index],
            None => {
                let mut place = origin_x;
                for other in columns {
                    if *other >= column {
                        break;
                    }
                    place += crate::printing::column_width(node, *other) * scale;
                }
                place
            }
        }
    };
    for (row_index, row) in rows.iter().enumerate() {
        for (column_index, column) in columns.iter().enumerate() {
            let cell = CellRef { col: *column, row: *row };
            let covered = crate::merges::covering(&merges, cell);
            if covered.is_some() {
                continue;
            }
            let merged = merges.iter().find(|area| area.start == cell).copied();
            // The **box** of the cell on the page.
            let mut left = xs[column_index];
            let mut top = ys[row_index];
            let mut right = left + crate::printing::column_width(node, *column) * scale;
            let mut bottom = top + crate::printing::row_height(node, *row) * scale;
            if let Some(area) = merged {
                left = place_of(area.start.col, &columns, &xs);
                right = place_of(area.end.col, &columns, &xs)
                    + crate::printing::column_width(node, area.end.col) * scale;
                top = ys[row_index];
                bottom = top + crate::printing::row_height(node, area.end.row) * scale
                    - crate::printing::row_height(node, area.start.row) * scale;
            }
            // The box of a cell is inside the block of the page.
            let left = left.clamp(origin_x, origin_x + block_width);
            let right = right.clamp(left, origin_x + block_width);
            let top = top.clamp(origin_y, origin_y + block_height);
            let bottom = bottom.clamp(top, origin_y + block_height);
            let width = right - left;
            let height = bottom - top;
            if width <= 0.0 || height <= 0.0 {
                continue;
            }
            let value = value_of(session, &name, cells, cell);
            let look = look_of(session, sheet, node, cells, cell, &value);
            // The **fill** of the cell.
            if let Some(fill) = look.fill.as_deref().and_then(rgb_of) {
                page.fill_rect(left as f32, top as f32, width as f32, height as f32, fill);
            }
            // The **text** of the cell.
            let text = crate::finding::text_of(session, sheet, &name, cells, cell, false);
            if text.trim().is_empty() {
                continue;
            }
            let text: String = text.chars().take(MAX_CHARS).collect();
            let face = face_of(look.font.as_ref());
            let size = size_of(look.font.as_ref(), scale);
            let colour = look
                .font
                .as_ref()
                .and_then(|font| font.colour.as_deref())
                .and_then(rgb_of)
                .unwrap_or([0, 0, 0]);
            let align = look.align.unwrap_or_else(Align::plain);
            let is_number = matches!(value, Value::Number(_));
            let pad = PAD;
            let available = (width - 2.0 * pad).max(0.0);
            let mut text = text;
            if !align.wrap {
                text = text.replace(['\n', '\r'], " ");
            }
            let mut measured = width_of(face, size, &text);
            // A **number** that does not fit is drawn as a row of `#`.
            if is_number && !align.wrap && measured > available + 0.01 {
                let hash = width_of(face, size, "#").max(0.1);
                let count = ((available / hash).floor().max(1.0) as usize).min(255);
                text = "#".repeat(count);
                measured = width_of(face, size, &text);
            }
            let lines =
                if align.wrap { wrap_lines(&text, face, size, available.max(size)) } else { vec![text] };
            let line_height = size * LINE;
            let count = lines.len().max(1) as f64;
            let first_baseline = match align.vertical {
                AlignV::Top => top + pad + size * 0.8,
                AlignV::Middle => {
                    top + (height - count * line_height) / 2.0 + line_height * 0.5 + size * 0.33
                }
                AlignV::Bottom => top + height - pad - size * 0.22 - (count - 1.0) * line_height,
            };
            // The text runs over the cells beside it when they are empty.
            let mut clip_left = left;
            let mut clip_right = right;
            if !align.wrap && !is_number && measured + 2.0 * pad > width {
                let mut need = measured + 2.0 * pad - width;
                for (spilled, other) in columns.iter().skip(column_index + 1).enumerate() {
                    if need <= 0.0 || spilled as u32 >= MAX_SPILL {
                        break;
                    }
                    let neighbour = CellRef { col: *other, row: *row };
                    if crate::merges::covering(&merges, neighbour).is_some() {
                        break;
                    }
                    let empty = cells.get(neighbour).is_none_or(|entry| {
                        entry.value.as_ref().is_none_or(|value| value == &Value::Empty)
                            && entry.formula.is_none()
                    });
                    if !empty {
                        break;
                    }
                    let room = crate::printing::column_width(node, *other) * scale;
                    clip_right += room;
                    need -= room;
                }
                clip_right = clip_right.min(origin_x + block_width);
                clip_left = clip_left.max(origin_x);
            }
            page.save();
            page.clip_rect(
                clip_left as f32,
                top as f32,
                (clip_right - clip_left).max(0.0) as f32,
                height as f32,
            );
            let place = horizontal(align.horizontal, &value);
            for (index, line) in lines.iter().enumerate() {
                let line_width = width_of(face, size, line);
                let x = match place {
                    AlignH::Right => right - pad - line_width,
                    AlignH::Center => left + (width - line_width) / 2.0,
                    _ => left + pad,
                };
                let baseline = first_baseline + index as f64 * line_height;
                if baseline - size > bottom || baseline < top - line_height {
                    continue;
                }
                page.text(x as f32, baseline as f32, size as f32, face, colour, line);
                // The **underline** and the **strike** of the text.
                if let Some(font) = look.font.as_ref() {
                    let thickness = (size * 0.06).max(0.2);
                    if font.underline != crate::formula::Underline::None {
                        page.stroke_line(
                            x as f32,
                            (baseline + size * 0.12) as f32,
                            (x + line_width) as f32,
                            (baseline + size * 0.12) as f32,
                            thickness as f32,
                            colour,
                        );
                    }
                    if font.strike {
                        page.stroke_line(
                            x as f32,
                            (baseline - size * 0.3) as f32,
                            (x + line_width) as f32,
                            (baseline - size * 0.3) as f32,
                            thickness as f32,
                            colour,
                        );
                    }
                }
            }
            page.restore();
        }
    }
}

commands! {
    /// **Write** a sheet as a **PDF file**.
    ///
    /// The file is written where the caller says, and the answer says how
    /// many pages it has. The sheet is written as it **prints**: the
    /// paper, the margins, the scale and the pages are the sheet's own.
    grid.export.pdf {
        area: "grid",
        layer: "grid",
        icon: "file",
        params: {
            /// The sheet to write.
            sheet: NodeRef,
            /// Where to write the file.
            path: Text,
        },
        run: {
            let id = "grid.export.pdf".to_string();
            let sheet = self.sheet.0;
            let path = self.path.0.trim().to_string();
            if path.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a file has a path".to_string(),
                });
            }
            let node = crate::printing::sheet_of(session, &id, sheet)?;
            let name = node.name.clone();
            let cells = crate::cells_of(&node);
            let setup = Setup::of(&node);
            // The **pages**: they are worked out in one place, and a
            // sheet that holds nothing has none.
            let Some(pages) = crate::printing::pages_of(&node, &setup, &cells, &id)? else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the sheet `{name}` holds nothing to export"),
                });
            };
            // The **file**: a page for each page of the sheet.
            let mut doc = PdfDoc::new();
            doc.info.title = name.clone();
            let (width, height) = setup.page();
            let count = pages.bands.len();
            for (number, band) in pages.bands.iter().enumerate() {
                let page = doc.add_page(width as f32, height as f32);
                let drawing = Drawing {
                    session,
                    sheet,
                    node: &node,
                    cells: &cells,
                    setup: &setup,
                    pages: &pages,
                    number: number + 1,
                    count,
                };
                draw_page(page, &drawing, *band);
            }
            let bytes = doc.finish();
            std::fs::write(&path, &bytes).map_err(|error| EngineError::Failed {
                id: id.clone(),
                message: format!("the file cannot be written to `{path}`: {error}"),
            })?;
            Ok(Outcome::new(format!("wrote {} page(s)", pages.bands.len())).with_data(json!({
                "sheet": sheet,
                "path": path,
                "pages": pages.bands.len(),
                "area": pages.area.to_text(),
                "paper": setup.paper,
                "scale": (pages.scale * 10_000.0).round() / 100.0,
                "bytes": bytes.len(),
            })))

        }
    }
}
