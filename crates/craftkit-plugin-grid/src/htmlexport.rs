//! The **export of a sheet as an HTML page**.
//!
//! A page is a **table**: a row of the sheet is a row of the table and
//! a cell is a cell of it, and a cell is written as it is **shown** —
//! the tree's own rule of the CSV (§557).
//!
//! The page carries the **look** of the sheet: the bold of a cell, the
//! colour of its text, the colour behind it and the way its text lies.
//! A **merged cell** spans its region, a **hidden row** is not written
//! at all, and a sheet that is read **right to left** says so.
//!
//! The text is **given** and not written to a file: the writing is the
//! caller's business, as it is for the CSV — and a command of the tree
//! never reaches outside the program.
//!
//! # What the port found
//!
//! - **The reference writes the file** and the tree gives the text.
//!   The reference's export is a "save as a web page": it names a path
//!   and writes a file there. Here the export is a **query**: it gives
//!   the text of the page, and the writing is the caller's business —
//!   the tree's own rule of the CSV (§557).
//! - **The reference draws the pictures and the charts over the
//!   table**, as inline SVG, and gives the table the sheet's own
//!   widths when it does. Here the page is the **table alone**: a
//!   picture or a chart of the sheet is not drawn beside it.
//! - **The reference's page is one long line** and the look of its
//!   cell is the look of the cell's **style**. Here the page is line
//!   by line, and the look of a cell is the cell's own, or the
//!   style's when the cell names none — the tree's own rules.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::formula::{AlignH, Area, CellRef, Cells, Value};

/// The **most** rows a page may hold: the tree's own bound.
///
/// A page of endless rows is not a page, and the tree **refuses** it
/// rather than cutting it short.
const MAX_ROWS: u64 = 100_001;

/// The **most** cells a page may hold: the tree's own bound.
///
/// A sparse sheet with one cell far to the right would otherwise be
/// gigabytes of empty cells.
const MAX_CELLS: u64 = 4_000_000;

commands! {
    /// **Export** a sheet as an HTML page.
    ///
    /// The page is the sheet's own **table**: a row of the sheet is a
    /// row of the table and a cell is a cell of it, written as it is
    /// **shown**.
    ///
    /// The page carries the **look** of the sheet: the bold of a cell,
    /// the colour of its text, the colour behind it and the way its
    /// text lies. A **merged cell** spans its region, and a **hidden
    /// row** is not written at all.
    ///
    /// The text is **given** and not written to a file: the writing is
    /// the caller's business, as it is for the CSV.
    grid.export.html {
        area: "grid",
        layer: "grid",
        icon: "file",
        params: {
            /// The sheet to export.
            sheet: NodeRef,
            /// The range to export. Without it, the used range is exported.
            range: Option<Text>,
        },
        run: {
            let id = "grid.export.html".to_string();
            let sheet = self.sheet.0;
            let Some(node) = session.document().node(sheet) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no sheet `{sheet}`"),
                });
            };
            if node.kind != "grid.sheet" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{sheet}` is a `{}` and not a sheet", node.kind),
                });
            }
            let name = node.name.clone();
            let cells = crate::cells_of(node);
            // The **range**: the one that was named, or the used range.
            let area = match &self.range {
                Some(range) => crate::fill::area_at(&range.0, &id)?,
                None => match crate::reading::used_range(&cells) {
                    Some(area) => area,
                    None => {
                        return Ok(Outcome::new(format!("{name} holds nothing to export")).with_data(json!({
                            "sheet": sheet,
                            "text": "",
                            "rows": 0,
                            "columns": 0,
                        })));
                    }
                },
            };
            // The **bounds**: a page of endless rows is not a page.
            let rows = u64::from(area.end.row - area.start.row + 1);
            let columns = u64::from(area.end.col - area.start.col + 1);
            if rows > MAX_ROWS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the sheet is too tall for a page: {rows} rows, at most {MAX_ROWS}"
                    ),
                });
            }
            if rows * columns > MAX_CELLS {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the sheet is too wide for a page: {} cells, at most {MAX_CELLS}",
                        rows * columns
                    ),
                });
            }
            let text = page_of(session, node, sheet, &name, &cells, &area);
            Ok(Outcome::new(format!("exported {rows} row(s)")).with_data(json!({
                "sheet": sheet,
                "range": area.to_text(),
                "rows": rows,
                "columns": columns,
                "text": text,
            })))

        }
    }
}

/// The **page** of a sheet: its table, with the look of the sheet.
fn page_of(session: &Session, node: &Node, sheet: u64, name: &str, cells: &Cells, area: &Area) -> String {
    let styles = crate::styles_of(session, sheet);
    let merges = crate::merges::merges_of(node);
    // The **hidden rows** are the tree's own numbering, and the cell of
    // the sheet is counted from zero.
    let hidden = crate::hiding::hidden_rows(node);
    let right_to_left = crate::view::view_of(node).right_to_left;
    let book = crate::book_of(session.document());
    let mut out = String::new();
    out.push_str("<!doctype html>\n<html>\n<head>\n<meta charset=\"utf-8\">\n");
    out.push_str(&format!("<title>{}</title>\n", escape(name)));
    out.push_str("<style>\n");
    out.push_str(
        "table{border-collapse:collapse;font-family:Calibri,Carlito,Arial,sans-serif;font-size:11pt}\n",
    );
    out.push_str("td{border:1px solid #d4d4d4;padding:2px 4px;white-space:nowrap}\n");
    out.push_str("</style>\n</head>\n<body>\n");
    out.push_str(&format!("<table{}>\n", if right_to_left { " dir=\"rtl\"" } else { "" }));
    for row in area.start.row..=area.end.row {
        if hidden.contains(&u64::from(row + 1)) {
            continue;
        }
        out.push_str("<tr>");
        for column in area.start.col..=area.end.col {
            let cell = CellRef { col: column, row };
            // A cell that a **merge** covers is not written: the cell
            // that covers it spans the region.
            if merges.iter().any(|merge| merge.covers(cell) && merge.start != cell) {
                continue;
            }
            let span = merges.iter().find(|merge| merge.start == cell);
            // The **region** of the merge: the cell spans all of it,
            // and the merge is a fact of the sheet and not of the
            // range that is exported.
            let (rows_span, columns_span) = match span {
                None => (1, 1),
                Some(merge) => (
                    u64::from(merge.end.row - merge.start.row + 1),
                    u64::from(merge.end.col - merge.start.col + 1),
                ),
            };
            out.push_str(&cell_of(
                session,
                &book,
                sheet,
                name,
                cells,
                &styles,
                cell,
                rows_span,
                columns_span,
                right_to_left,
            ));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n</body>\n</html>\n");
    out
}

/// The **cell** of the page: its text, its look and its place in the
/// table.
#[allow(clippy::too_many_arguments)]
fn cell_of(
    session: &Session,
    book: &crate::formula::Book,
    sheet: u64,
    name: &str,
    cells: &Cells,
    styles: &[crate::styles::Style],
    cell: CellRef,
    rows_span: u64,
    columns_span: u64,
    right_to_left: bool,
) -> String {
    let entry = cells.get(cell);
    let text = crate::finding::text_of(session, sheet, name, cells, cell, false);
    // The **look** of the cell: its own, or the style it names.
    let style = entry
        .and_then(|entry| entry.style.as_deref())
        .and_then(|named| styles.iter().find(|style| style.name == named));
    let font =
        entry.and_then(|entry| entry.font.as_ref()).or_else(|| style.and_then(|style| style.font.as_ref()));
    let align =
        entry.and_then(|entry| entry.align.as_ref()).or_else(|| style.and_then(|style| style.align.as_ref()));
    let fill = entry.and_then(|entry| entry.fill.as_ref());
    // The **value** of the cell: a number is written to the right and
    // the text to the left, unless the cell says otherwise.
    let value = match entry.and_then(|entry| entry.value.clone()) {
        Some(value) => value,
        None => {
            if entry.and_then(|entry| entry.formula.as_ref()).is_some() {
                book.value_of(name, cell)
            } else {
                Value::Empty
            }
        }
    };
    let number = matches!(value, Value::Number(_));
    let mut css = String::new();
    if let Some(font) = font {
        if font.bold {
            css.push_str("font-weight:bold;");
        }
        if font.italic {
            css.push_str("font-style:italic;");
        }
        if let Some(colour) = &font.colour {
            css.push_str(&format!("color:{colour};"));
        }
    }
    if let Some(fill) = fill {
        css.push_str(&format!("background:{fill};"));
    }
    let horizontal = align.map(|align| align.horizontal).unwrap_or(AlignH::General);
    let alignment = match horizontal {
        AlignH::Center | AlignH::CenterAcross => Some("center"),
        AlignH::Right => Some("right"),
        AlignH::Left => Some("left"),
        AlignH::Fill | AlignH::Justify => None,
        AlignH::General => {
            if number {
                Some(if right_to_left { "left" } else { "right" })
            } else {
                None
            }
        }
    };
    if let Some(alignment) = alignment {
        css.push_str(&format!("text-align:{alignment};"));
    }
    // The **direction** of the text: the letters of a right-to-left
    // language follow their first strong character, and a number on a
    // right-to-left sheet stays left to right, so that `-5` does not
    // read `5-`.
    let direction = if number {
        if right_to_left { " dir=\"ltr\"" } else { "" }
    } else if right_to_left || text.chars().any(is_right_to_left_letter) {
        " dir=\"auto\""
    } else {
        ""
    };
    let mut out = String::from("<td");
    if rows_span > 1 {
        out.push_str(&format!(" rowspan=\"{rows_span}\""));
    }
    if columns_span > 1 {
        out.push_str(&format!(" colspan=\"{columns_span}\""));
    }
    if !css.is_empty() {
        out.push_str(&format!(" style=\"{css}\""));
    }
    out.push_str(direction);
    out.push('>');
    out.push_str(&escape(&text));
    out.push_str("</td>");
    out
}

/// The **text** of the page, with the marks of HTML in it escaped.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(character),
        }
    }
    out
}

/// Hebrew, Arabic, Syriac, Thaana, N'Ko and their presentation forms.
fn is_right_to_left_letter(character: char) -> bool {
    matches!(character, '\u{0590}'..='\u{08FF}' | '\u{FB1D}'..='\u{FDFF}' | '\u{FE70}'..='\u{FEFF}')
}
