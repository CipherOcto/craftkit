//! Drawing a paragraph: the **font**'s outlines and the **metric**'s
//! widths.
//!
//! The two are not the same thing and the difference matters:
//!
//! - the **typeface** says what a letter *looks* like — its strokes,
//!   in the kit's own grid;
//! - the **metric** says how wide each character *is* in this
//!   paragraph's setting.
//!
//! So a letter is drawn in the typeface's shape and placed at the
//! metric's width: the glyph is **stretched** by whatever the metric
//! says against what the typeface would have had. A metric that agrees
//! with the typeface stretches nothing; a metric that is monospaced
//! stretches every letter to the same width, and that is what
//! monospaced text looks like.
//!
//! What is **not** here: kerning and ligatures. The breaker measures a
//! word with the metric's runs and pairs, so a metric with a kerning
//! pair measures the word a little narrower than the letters are drawn.
//! The drawing places one glyph per character, and says so.

use crate::{Metric, break_lines, hyphenate_of, items_for, measure_of, metric_of, words_of};
use craftkit_font::{Font, Metrics};
use craftkit_sdk::document::{Colour, Path, Point, Rect, Segment, Stroke, SubPath};
use craftkit_sdk::{Document, Node};

/// The size of a metric that does not say: the **em**, in millimetres.
///
/// It is about eleven points, which is a size a body of text is set in.
/// A metric that says nothing is not refused: a metric is a table of
/// widths, and the size is a separate thing that a caller may not care
/// about.
pub const DEFAULT_EM_MM: f64 = 4.0;

/// How wide the strokes of a letter are, as a part of the em.
///
/// A letter is a line, and a line has to be thick enough to be seen and
/// thin enough to be a letter.
pub const STROKE_EM: f64 = 0.06;

/// What a paragraph is drawn as: the **paths** of its letters, the
/// **strokes** they are made of, and the **cells** the letters sit in.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// The strokes of every letter, one path each.
    pub paths: Vec<Path>,
    /// The same strokes, in the caller's units: the lines a pointer can
    /// be **on**.
    ///
    /// A letter is a line and not a box: the middle of an `o` is a hole
    /// and the middle of an `l` is empty, and a pointer in the middle
    /// of an `o` is on nothing.
    pub strokes: Vec<Vec<[f64; 2]>>,
    /// The **cell** of every letter: the space it takes, from the pen
    /// to the next pen.
    ///
    /// The cell is the advance, and the ink is usually narrower than
    /// the cell: that is what the space at the side of a letter is.
    pub cells: Vec<Rect>,
}

/// Draw a paragraph: every letter of every line, as paths.
///
/// A paragraph with no measure, no metric or no words is **not
/// drawn**: it has no place yet, and a thing with no place is not
/// there.
pub fn draw(node: &Node, document: &Document) -> Option<Drawn> {
    let metric = metric_of(document, node)?;
    let words = words_of(node)?;
    let measure = measure_of(node);
    if measure <= 0.0 || words.is_empty() {
        return None;
    }
    let metrics = Metrics::new(metric.em_mm)?;
    let breaks = break_lines(node, Some(&metric)).ok()?;
    let typeset = items_for(&words, &metric, hyphenate_of(node));
    let font = Font::builtin();
    let top = breaks.lines.len() as f64 * metrics.line_height;
    let mut drawn = Drawn::default();
    for (index, line) in breaks.lines.iter().enumerate() {
        let baseline = top - metrics.em - index as f64 * metrics.line_height;
        let mut x = 0.0;
        for (offset, item) in typeset.items[line.start..line.end].iter().enumerate() {
            let item_index = line.start + offset;
            match item.kind {
                craftkit_text_layout::Kind::Box => {
                    // A box holds a **piece** of a word: the whole word,
                    // or the piece a broken word was cut into.
                    let Some(piece) = typeset.text_of.get(item_index) else { break };
                    if piece.is_empty() {
                        break;
                    }
                    place_word(piece, x, baseline, &metric, &metrics, font, &mut drawn);
                    x += item.width;
                }
                craftkit_text_layout::Kind::Glue => {
                    x += glue_width(item.width, item.stretch, item.shrink, line.ratio);
                }
                craftkit_text_layout::Kind::Penalty => {}
            }
        }
        // A line that ended at a **hyphenation point** ends in a
        // hyphen, and the hyphen is drawn: a picture of a line without
        // the hyphen is a picture of another paragraph.
        if line.flagged {
            place_word("-", x, baseline, &metric, &metrics, font, &mut drawn);
        }
    }
    Some(drawn)
}

/// Where a paragraph **is**: the block of its lines.
///
/// The block is as wide as the measure and as tall as the lines, and it
/// stands at the origin: a paragraph is a column of text, and the
/// column is where the paragraph is.
pub fn block(node: &Node, document: &Document) -> Option<Rect> {
    let metric = metric_of(document, node)?;
    let words = words_of(node)?;
    let measure = measure_of(node);
    if measure <= 0.0 || words.is_empty() {
        return None;
    }
    let metrics = Metrics::new(metric.em_mm)?;
    let breaks = break_lines(node, Some(&metric)).ok()?;
    let height = breaks.lines.len() as f64 * metrics.line_height;
    Some(Rect::new(0.0, 0.0, measure, height))
}

/// How wide a glue is on a line.
///
/// A line that is too loose **stretches** its glues and a line that is
/// too tight **shrinks** them, and the ratio says how much: the same
/// ratio the breaker worked out.
fn glue_width(width: f64, stretch: f64, shrink: f64, ratio: f64) -> f64 {
    if ratio >= 0.0 { width + ratio * stretch } else { width + ratio * shrink }
}

/// Place one word: its letters, one after another.
///
/// Each letter is drawn in the typeface's shape and takes the **metric**
///'s width, so a letter is stretched by the difference between the two.
fn place_word(
    word: &str,
    x: f64,
    baseline: f64,
    metric: &Metric,
    metrics: &Metrics,
    font: &Font,
    drawn: &mut Drawn,
) {
    let unit = metrics.unit();
    let mut cursor = x;
    for character in word.chars() {
        let Some(glyph) = font.glyph(character) else { continue };
        let natural = glyph.advance * unit;
        let wanted = metric.width_of(character, natural);
        let scale = if natural > 0.0 { wanted / natural } else { 1.0 };
        let mut ink: Vec<[f64; 2]> = Vec::with_capacity(glyph.strokes.len());
        for stroke in &glyph.strokes {
            let mut points: Vec<[f64; 2]> = Vec::with_capacity(stroke.len());
            for point in stroke {
                points.push([cursor + point[0] * unit * scale, baseline + point[1] * unit]);
            }
            ink.extend(points.iter().copied());
            if points.len() < 2 {
                continue;
            }
            let mut segments = Vec::with_capacity(points.len() - 1);
            for point in points.iter().skip(1) {
                segments.push(Segment::Line { to: Point::new(point[0], point[1]) });
            }
            let path = Path::single(SubPath {
                start: Point::new(points[0][0], points[0][1]),
                segments,
                closed: false,
            });
            drawn.paths.push(path);
        }
        if !ink.is_empty() {
            drawn.strokes.push(ink);
        }
        // The **cell** of the letter: the space it takes.
        drawn.cells.push(Rect::new(cursor, baseline - metrics.descent, wanted, metrics.em + metrics.descent));
        cursor += wanted;
    }
}

/// The stroke of a letter, in the kit's colours.
pub fn stroke(colour: Colour) -> Stroke {
    Stroke { colour, width: STROKE_EM }
}

/// Whether a point is **on** the drawing.
///
/// A letter is a line, and the line is as wide as the stroke that
/// draws it: a point is on the letter when it is within half the
/// stroke's width of one of the letter's lines.
pub fn hits(drawn: &Drawn, point: Point) -> bool {
    let radius = STROKE_EM * DEFAULT_EM_MM / 2.0;
    drawn
        .strokes
        .iter()
        .any(|stroke| stroke.windows(2).any(|pair| distance_to_line(point, pair[0], pair[1]) <= radius))
}

/// How far a point is from a **line** between two points.
///
/// The line is a segment, so the answer is the distance to the segment
/// and not to the infinite line: a point beyond the end of a stroke is
/// not on the stroke.
fn distance_to_line(point: Point, from: [f64; 2], to: [f64; 2]) -> f64 {
    let dx = to[0] - from[0];
    let dy = to[1] - from[1];
    let length = (dx * dx + dy * dy).sqrt();
    if length == 0.0 {
        let ex = point.x - from[0];
        let ey = point.y - from[1];
        return (ex * ex + ey * ey).sqrt();
    }
    let t = (((point.x - from[0]) * dx + (point.y - from[1]) * dy) / (length * length)).clamp(0.0, 1.0);
    let ex = point.x - (from[0] + t * dx);
    let ey = point.y - (from[1] + t * dy);
    (ex * ex + ey * ey).sqrt()
}
