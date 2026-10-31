//! `craftkit-font` — **the kit's own typeface**, drawn in code.
//!
//! # What this is
//!
//! A typeface is a table of **glyphs**, and a glyph is a set of
//! **strokes** and a width. That is all it is: the strokes of a capital
//! `A`, the strokes of a small `g`, and how far the writing moves after
//! each one.
//!
//! This crate is that table, and it is the tree's **own**: the letters
//! are drawn here, in a source file, from coordinates chosen for this
//! tree. There is no font file, no artwork and no asset of any kind —
//! the same rule the icons follow, and for the same reason.
//!
//! # Why it is a library and not a plugin
//!
//! Because a typeface is **data**: strokes and numbers. It knows
//! nothing about a document, a canvas, a layer or the kit. The caller
//! brings the **size** and the **place**, and the library brings the
//! **outlines**.
//!
//! # The grid
//!
//! Every glyph is drawn on the same grid, so that the letters line up
//! with each other and a caller does not have to know where a letter
//! starts:
//!
//! - the **baseline** is `y = 0`;
//! - the **cap height** is `y = 2` — the top of a capital letter;
//! - the **x-height** is `y = 1.5` — the top of a small letter;
//! - the **descender** is `y = -0.5` — the bottom of a `g` or a `p`.
//!
//! The **em** is the size: the em is the cap height, because that is
//! the number a caller has in mind when it says how big a letter is.
//! One grid unit is therefore `em / 2` of the caller's units.
//!
//! # What this does not do
//!
//! - **It does not kern.** A pair of letters is not adjusted against
//!   each other; the **advance** is the width of the letter and the
//!   next letter starts there. A caller that wants kerning applies it
//!   to the advance, which is what a metric is for.
//! - **It does not shape.** There is no ligature, no substitution and
//!   no complex script: one character, one glyph.
//! - **It does not fill.** A glyph is a set of **strokes**, and a
//!   stroke is a line. A caller that wants a filled letter has to
//!   trace the outline of the strokes, and this crate does not do that
//!   for it.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod glyphs;

pub use glyphs::{Entry, Glyph, TABLE, entries};

use std::collections::HashMap;
use std::sync::OnceLock;

/// The **cap height**, in grid units: the top of a capital letter.
pub const CAP_HEIGHT: f64 = 2.0;

/// The **x-height**, in grid units: the top of a small letter.
pub const X_HEIGHT: f64 = 1.5;

/// The **descender**, in grid units: the bottom of a `g` or a `p`.
pub const DESCENT: f64 = -0.5;

/// The distance from one baseline to the next, in grid units.
///
/// It is the cap height plus the descender: a line of letters needs
/// room for the letters that hang below the baseline, and the line
/// height is where the next line's letters start.
pub const LINE_HEIGHT: f64 = CAP_HEIGHT - DESCENT;

/// The **size** of a typeface at a given size: the numbers a caller
/// needs to place a line of text.
///
/// The em is the **cap height** in the caller's units, so a caller that
/// says "ten" gets letters whose capitals are ten tall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Metrics {
    /// The em: the cap height, in the caller's units.
    pub em: f64,
    /// The x-height, in the caller's units.
    pub x_height: f64,
    /// How far a letter may go **below** the baseline.
    pub descent: f64,
    /// The distance from one baseline to the next.
    pub line_height: f64,
}

impl Metrics {
    /// The metrics of a typeface at a size.
    ///
    /// A size that is **not a number**, or that is not positive, has no
    /// metrics: the caller is told so rather than being handed a size
    /// of zero and a drawing of nothing.
    pub fn new(em: f64) -> Option<Metrics> {
        if !em.is_finite() || em <= 0.0 {
            return None;
        }
        let unit = em / CAP_HEIGHT;
        Some(Metrics {
            em,
            x_height: X_HEIGHT * unit,
            descent: -DESCENT * unit,
            line_height: LINE_HEIGHT * unit,
        })
    }

    /// One grid unit, in the caller's units.
    pub fn unit(&self) -> f64 {
        self.em / CAP_HEIGHT
    }
}

/// A typeface: the glyphs it has, and what they are worth.
///
/// The kit's typeface is [`Font::builtin`]. A caller that wants a
/// different one builds its own: the table is a list of [`Entry`]s and
/// nothing more.
#[derive(Clone, Debug)]
pub struct Font {
    glyphs: HashMap<char, Glyph>,
}

impl Font {
    /// A typeface built from a table.
    pub fn from_entries(entries: &[Entry]) -> Font {
        let mut glyphs = HashMap::with_capacity(entries.len());
        for entry in entries {
            // The **last** entry for a character wins: a table that says
            // the same thing twice is a table with an answer, and the
            // answer is the last one.
            glyphs.insert(entry.character, entry.glyph.clone());
        }
        Font { glyphs }
    }

    /// The kit's own typeface.
    pub fn builtin() -> &'static Font {
        static FONT: OnceLock<Font> = OnceLock::new();
        FONT.get_or_init(|| Font::from_entries(&glyphs::entries()))
    }

    /// The glyph for a character, if the typeface has one.
    pub fn glyph(&self, character: char) -> Option<&Glyph> {
        self.glyphs.get(&character)
    }

    /// How wide a character is, in grid units, if the typeface has it.
    pub fn advance(&self, character: char) -> Option<f64> {
        self.glyphs.get(&character).map(|glyph| glyph.advance)
    }

    /// The characters the typeface has, in order.
    pub fn characters(&self) -> Vec<char> {
        let mut characters: Vec<char> = self.glyphs.keys().copied().collect();
        characters.sort_unstable();
        characters
    }

    /// The glyph's **outline**, scaled to a size and moved into place.
    ///
    /// The strokes come back in the caller's units, with the baseline at
    /// `baseline` and the writing starting at `x`.
    pub fn outline(
        &self,
        character: char,
        metrics: &Metrics,
        x: f64,
        baseline: f64,
    ) -> Option<Vec<Vec<[f64; 2]>>> {
        let glyph = self.glyph(character)?;
        Some(glyph.outline(metrics.unit(), x, baseline))
    }
}

impl Glyph {
    /// The glyph's strokes, scaled to a size and moved into place.
    ///
    /// The size is **one grid unit** in the caller's units, and the
    /// baseline is where the letters sit.
    pub fn outline(&self, unit: f64, x: f64, baseline: f64) -> Vec<Vec<[f64; 2]>> {
        self.strokes
            .iter()
            .map(|stroke| {
                stroke.iter().map(|point| [x + point[0] * unit, baseline + point[1] * unit]).collect()
            })
            .collect()
    }

    /// The glyph's **box**: where it is, in grid units.
    ///
    /// The box is the glyph's own width — not its advance — and its
    /// height runs from the descender to the cap height. A letter that
    /// hangs below the baseline hangs below the box of a letter that
    /// does not, and the box says so.
    pub fn box_of(&self) -> Option<([f64; 2], [f64; 2])> {
        if self.strokes.is_empty() {
            return None;
        }
        let mut left = f64::INFINITY;
        let mut right = f64::NEG_INFINITY;
        let mut top = f64::INFINITY;
        let mut bottom = f64::NEG_INFINITY;
        for stroke in &self.strokes {
            for point in stroke {
                left = left.min(point[0]);
                right = right.max(point[0]);
                top = top.min(point[1]);
                bottom = bottom.max(point[1]);
            }
        }
        Some(([left, bottom], [right, top]))
    }
}

/// A **placed** character: where it is, and how wide it is.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placed {
    /// The character.
    pub character: char,
    /// Where it starts, in the caller's units, from the origin.
    pub x: f64,
    /// How wide it is, in the caller's units.
    pub advance: f64,
}

/// The characters of a string, placed one after another.
///
/// A character the typeface **does not have** is left out: there is
/// nothing to draw and nothing to advance by, and a caller that wants
/// to know can ask the typeface.
pub fn layout(text: &str, font: &Font, metrics: &Metrics) -> Vec<Placed> {
    let unit = metrics.unit();
    let mut placed = Vec::with_capacity(text.chars().count());
    let mut x = 0.0;
    for character in text.chars() {
        let Some(glyph) = font.glyph(character) else { continue };
        let advance = glyph.advance * unit;
        placed.push(Placed { character, x, advance });
        x += advance;
    }
    placed
}

/// How wide a string is, in the caller's units.
pub fn width(text: &str, font: &Font, metrics: &Metrics) -> f64 {
    let unit = metrics.unit();
    let mut width = 0.0;
    for character in text.chars() {
        let Some(glyph) = font.glyph(character) else { continue };
        width += glyph.advance * unit;
    }
    width
}
