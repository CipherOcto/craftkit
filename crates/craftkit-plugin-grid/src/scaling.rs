//! The **scale** of a print.
//!
//! A sheet is printed at a **scale** — a percentage of its own size,
//! from ten to four hundred — or it is printed to **fit**: a number of
//! pages wide and a number of pages tall, and the scale that does that
//! is **derived**.
//!
//! The rules:
//!
//! - a **scale** is a whole number of percent from ten to four
//!   hundred; the tree clamps it, and a scale of a thousand percent is
//!   not a scale, so it is refused;
//! - a **fit** that is zero pages wide *and* zero pages tall is
//!   refused: a fit that fits everywhere is not a fit;
//! - a fit that **cannot be met** is refused: a sheet that does not
//!   fit into one page wide even at ten percent does not fit, and the
//!   tree would print it on more pages than it was told to;
//! - a fit and a plain scale are **two ways of saying one thing**, so
//!   setting one takes the other away.
//!
//! # What the port found
//!
//! - **The tree keeps a scale and a fit at the same time**, and lets
//!   the fit win. Here one of them is one fact: a sheet prints at a
//!   scale, and setting the scale takes the fit away.
//! - **The tree shrinks the fit by three percent at a time** until the
//!   pages fit, which is a number nobody chose. Here the scale is
//!   **worked out** — the largest scale at which the pages fit — and it
//!   is reported.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome};

use crate::printing::{Setup, bands, column_width, row_height, sheet_of};

/// The **least** a scale may be, in percent: the tree's own bound.
pub const MIN_SCALE: f64 = 10.0;

/// The **most** a scale may be, in percent: the tree's own bound.
pub const MAX_SCALE: f64 = 400.0;

/// The **least** a fit may shrink the sheet to: a tenth of its size.
pub const MIN_FIT_SCALE: f64 = 0.1;

/// The **most** pages a fit may ask for, on either axis.
pub const MAX_FIT_PAGES: u32 = 1000;

/// The **content** of a sheet that is printed: the rows and the
/// columns, less the titles, which are printed on every page.
#[derive(Debug, Clone)]
pub struct Content {
    /// The rows that are printed.
    pub rows: Vec<u32>,
    /// The columns that are printed.
    pub columns: Vec<u32>,
}

impl Content {
    /// The content of a sheet: the area that is printed, or the range
    /// the sheet uses, less the titles.
    pub fn of(node: &Node) -> Content {
        let cells = crate::cells_of(node);
        let area = match crate::printing::area_of(node) {
            Some(area) => Some(area),
            None => crate::reading::used_range(&cells),
        };
        let Some(area) = area else { return Content { rows: Vec::new(), columns: Vec::new() } };
        let (title_rows, title_columns) = crate::printing::titles_of(node);
        Content {
            rows: (area.start.row..=area.end.row).filter(|row| !title_rows.contains(row)).collect(),
            columns: (area.start.col..=area.end.col)
                .filter(|column| !title_columns.contains(column))
                .collect(),
        }
    }
}

/// The **scale** a sheet prints at, as a fraction of its size, and
/// whether a fit is met.
///
/// A sheet that is **fitted** prints at the scale that fits it into the
/// pages it was told to fit; a sheet that is not fitted prints at its
/// own scale.
pub fn effective_scale(setup: &Setup, node: &Node, content: &Content) -> (f64, bool) {
    let Some((wide, tall)) = setup.fit else { return (setup.scale / 100.0, true) };
    fit_scale(setup, node, content, wide, tall)
}

/// The **scale** that fits the content into the pages asked for, and
/// whether it fits at all.
///
/// The scale is the **largest** one at which the pages fit: the
/// content is as big as it may be and still fit. A fit that is not met
/// even at the smallest scale is reported as not met.
pub fn fit_scale(setup: &Setup, node: &Node, content: &Content, wide: u16, tall: u16) -> (f64, bool) {
    let (page_width, page_height) = setup.printable();
    let total_width: f64 = content.columns.iter().map(|column| column_width(node, *column)).sum();
    let total_height: f64 = content.rows.iter().map(|row| row_height(node, *row)).sum();
    // The **ceiling**: the scale at which the content would fit if it
    // were spread evenly over the pages. The pages are not spread
    // evenly, so the scale is worked out below.
    let mut ceiling = 1.0f64;
    if wide > 0 && total_width > 0.0 {
        ceiling = ceiling.min(page_width * f64::from(wide) / total_width);
    }
    if tall > 0 && total_height > 0.0 {
        ceiling = ceiling.min(page_height * f64::from(tall) / total_height);
    }
    if !ceiling.is_finite() || ceiling <= 0.0 {
        return (MIN_FIT_SCALE, false);
    }
    // A fit that cannot be met **at the smallest scale** is not met at
    // all: the sheet is too big for the pages it was told to fit.
    if ceiling < MIN_FIT_SCALE {
        return (MIN_FIT_SCALE, false);
    }
    let fits = |scale: f64| -> bool {
        if wide > 0 {
            let columns =
                bands(&content.columns, |column| column_width(node, column), page_width / scale, &[]);
            if columns.len() > usize::from(wide) {
                return false;
            }
        }
        if tall > 0 {
            let rows = bands(&content.rows, |row| row_height(node, row), page_height / scale, &[]);
            if rows.len() > usize::from(tall) {
                return false;
            }
        }
        true
    };
    if fits(ceiling) {
        return (ceiling, true);
    }
    if !fits(MIN_FIT_SCALE) {
        return (MIN_FIT_SCALE, false);
    }
    // The **largest** scale that fits: the pages grow as the scale
    // grows, so the scale that fits is found by halving the distance
    // to the ceiling, a fixed number of times.
    let mut low = MIN_FIT_SCALE;
    let mut high = ceiling;
    for _ in 0..40 {
        let middle = (low + high) / 2.0;
        if fits(middle) {
            low = middle;
        } else {
            high = middle;
        }
    }
    (low, true)
}

commands! {
    /// **Set** the scale a sheet prints at.
    ///
    /// The scale is a whole number of percent, from ten to four hundred.
    /// Setting it takes the **fit** away: a fit is another way of saying
    /// the scale, and a sheet prints at one of them.
    grid.print.scale {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.
            sheet: NodeRef,
            /// The scale, in percent: a whole number from 10 to 400.
            percent: Number,
        },
        run: {
            let id = "grid.print.scale".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let percent = self.percent.0;
            if !percent.is_finite() || percent.fract() != 0.0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a scale is a whole number of percent: {percent} is not one"),
                });
            }
            if !(MIN_SCALE..=MAX_SCALE).contains(&percent) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "a scale is a number of percent from {MIN_SCALE} to {MAX_SCALE}: {percent} is not one"
                    ),
                });
            }
            let mut setup = Setup::of(&node);
            setup.scale = percent;
            setup.fit = None;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["print"] = setup.to_json();
            }
            Ok(Outcome::changed(format!("the sheet is printed at {percent} percent")).with_data(json!({
                "sheet": sheet,
                "scale": percent,
            })))

        }
    }

    /// **Print** a sheet to fit: so many pages wide and so many pages tall.
    ///
    /// The scale that does it is **derived** — the largest scale at which
    /// the pages fit — and it is reported. A fit that cannot be met is
    /// refused: the sheet does not fit, and saying otherwise would be a
    /// lie.
    grid.print.fit {
        area: "grid",
        layer: "grid",
        icon: "page",
        mutates,
        undoable,
        params: {
            /// The sheet.
            sheet: NodeRef,
            /// The number of pages the sheet is **wide**: one, two or three.
            /// Zero means as many as it takes.
            wide: Number,
            /// The number of pages the sheet is **tall**. Zero means as many
            /// as it takes.
            tall: Number,
        },
        run: {
            let id = "grid.print.fit".to_string();
            let sheet = self.sheet.0;
            let node = sheet_of(session, &id, sheet)?;
            let pages = |value: f64, what: &str| -> Result<u32, EngineError> {
                if !value.is_finite() || value.fract() != 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the number of pages {what} is a whole number: {value} is not one"),
                    });
                }
                if value < 0.0 || value > f64::from(MAX_FIT_PAGES) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "the number of pages {what} is from zero to {MAX_FIT_PAGES}: {value} is not one"
                        ),
                    });
                }
                Ok(value as u32)
            };
            let wide = pages(self.wide.0, "wide")?;
            let tall = pages(self.tall.0, "tall")?;
            if wide == 0 && tall == 0 {
                return Err(EngineError::Failed {
                    id,
                    message: "a fit of no pages at all is not a fit: it is the whole sheet".to_string(),
                });
            }
            let mut setup = Setup::of(&node);
            let content = Content::of(&node);
            let (scale, met) = fit_scale(&setup, &node, &content, wide as u16, tall as u16);
            if !met {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "the sheet does not fit into {wide} page(s) wide and {tall} page(s) tall, even at ten percent"
                    ),
                });
            }
            setup.fit = Some((wide as u16, tall as u16));
            setup.scale = 100.0;
            if let Some(node) = session.document_mut()?.node_mut(sheet) {
                node.data["print"] = setup.to_json();
            }
            Ok(Outcome::changed(format!("the sheet is printed to fit {wide} by {tall}")).with_data(json!({
                "sheet": sheet,
                "fit": { "wide": wide, "tall": tall },
                "scale": (scale * 10_000.0).round() / 100.0,
            })))

        }
    }
}
