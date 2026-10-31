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
    /// Where every **character** of the text stands.
    ///
    /// The characters are the text's: the words with a space between
    /// them, and every one of them is somewhere on a line. The letters
    /// are drawn, but the *character* is more than the letter: a space
    /// has no ink and is still a character, and the caret stands
    /// before one.
    pub chars: Vec<CharPlace>,
}

/// Where one **character** of the paragraph's text stands.
///
/// The character is counted in the text — the words with a space between
/// them — and this is where that character ended up when the
/// paragraph was broken into lines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CharPlace {
    /// The character's index in the paragraph's text.
    pub index: usize,
    /// Where the character starts, on the baseline.
    pub x: f64,
    /// The baseline the character stands on.
    pub baseline: f64,
    /// How wide the character is.
    pub width: f64,
    /// The top of the line the character stands on.
    pub top: f64,
    /// The bottom of that line.
    pub bottom: f64,
}

/// Where the **caret** stands: a line, at a place on a baseline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Caret {
    /// The place of the line, on the baseline.
    pub x: f64,
    /// The top of the line.
    pub top: f64,
    /// The bottom of the line.
    pub bottom: f64,
}

impl Caret {
    /// The caret's **box**.
    ///
    /// A caret is a line and not a box, so its width is nothing: what
    /// it has is a place on a baseline and the two ends of the line.
    pub fn rect(&self) -> Rect {
        Rect::new(self.x, self.top, 0.0, self.bottom - self.top)
    }
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
    let ink = Ink { metric: &metric, metrics: &metrics, font };
    let top = breaks.lines.len() as f64 * metrics.line_height;
    let mut drawn = Drawn::default();
    // Where each word's first character is, in the paragraph's text.
    let starts = starts_of(&words);
    // How many of each word's characters have been placed: a word that
    // was broken is more than one box, and the pieces are the word's
    // characters in order.
    let mut consumed: Vec<usize> = vec![0; words.len()];
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
                    let word = typeset.word_of.get(item_index).copied().unwrap_or(0);
                    let start =
                        starts.get(word).copied().unwrap_or(0) + consumed.get(word).copied().unwrap_or(0);
                    place_word(piece, Some(start), x, baseline, &ink, &mut drawn);
                    if let Some(count) = consumed.get_mut(word) {
                        *count += piece.chars().count();
                    }
                    x += item.width;
                }
                craftkit_text_layout::Kind::Glue => {
                    // A glue is the **space** between two words, and the
                    // space is a character of the text: it has no ink,
                    // and it is where the caret stands between two
                    // words.
                    let word = typeset.word_of.get(item_index).copied().unwrap_or(0);
                    let space = starts.get(word).copied().unwrap_or(0).saturating_sub(1);
                    let width = glue_width(item.width, item.stretch, item.shrink, line.ratio);
                    drawn.chars.push(CharPlace {
                        index: space,
                        x,
                        baseline,
                        width,
                        top: baseline - metrics.descent,
                        bottom: baseline + metrics.em,
                    });
                    x += width;
                }
                craftkit_text_layout::Kind::Penalty => {}
            }
        }
        // The **break**: a glue that was taken is a space that the
        // line swallowed, and the space is still a character of the
        // text.
        if let Some(item) = typeset.items.get(line.end)
            && item.kind == craftkit_text_layout::Kind::Glue
        {
            let word = typeset.word_of.get(line.end).copied().unwrap_or(0);
            let space = starts.get(word).copied().unwrap_or(0).saturating_sub(1);
            drawn.chars.push(CharPlace {
                index: space,
                x,
                baseline,
                width: 0.0,
                top: baseline - metrics.descent,
                bottom: baseline + metrics.em,
            });
        }
        // A line that ended at a **hyphenation point** ends in a
        // hyphen, and the hyphen is drawn: a picture of a line without
        // the hyphen is a picture of another paragraph.
        //
        // The hyphen is **not** a character of the text: the word was
        // broken, and the break is the breaker's business.
        if line.flagged {
            place_word("-", None, x, baseline, &ink, &mut drawn);
        }
    }
    Some(drawn)
}

/// Where each word's first character is, in the paragraph's text.
///
/// The text is the words with a **space** between them, so a word
/// starts after the words before it and after the spaces between them.
pub(crate) fn starts_of(words: &[String]) -> Vec<usize> {
    let mut starts = Vec::with_capacity(words.len());
    let mut at = 0usize;
    for word in words {
        starts.push(at);
        at += word.chars().count() + 1;
    }
    starts
}

/// Where the **part** of the paragraph that is picked out is.
///
/// The part is a range of **characters**, counted in the text, and the
/// characters are placed as the paragraph is drawn. A range that
/// crosses a line is more than one piece: one on each line, and the
/// pieces are in the order the lines are in.
pub fn part(drawn: &Drawn, from: usize, to: usize) -> Vec<Rect> {
    let mut pieces: Vec<(f64, Rect)> = Vec::new();
    for place in &drawn.chars {
        if place.index < from || place.index >= to {
            continue;
        }
        match pieces.last_mut() {
            // The same line: the piece grows to the right.
            Some((baseline, rect)) if *baseline == place.baseline => {
                rect.w = place.x + place.width - rect.x;
            }
            // A new line: a new piece.
            _ => pieces
                .push((place.baseline, Rect::new(place.x, place.top, place.width, place.bottom - place.top))),
        }
    }
    pieces.into_iter().map(|(_, rect)| rect).collect()
}

/// Where the caret that stands before the character at `character` is.
///
/// The caret stands at the **place of the character it stands before**:
/// the character before it is the last one the pen passed, and the
/// caret is where the pen stands now. A caret at the end of the text
/// is at the end of the last line, and a caret at a space that a line
/// break swallowed is at the end of the line before it.
pub fn caret(drawn: &Drawn, character: usize) -> Option<Caret> {
    let (x, top, bottom) = match drawn.chars.iter().find(|place| place.index == character) {
        Some(place) => (place.x, place.top, place.bottom),
        None => {
            let before =
                drawn.chars.iter().filter(|place| place.index < character).max_by_key(|place| place.index)?;
            (before.x + before.width, before.top, before.bottom)
        }
    };
    Some(Caret { x, top, bottom })
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

/// The things a letter is drawn **with**: the typeface that says what a
/// letter looks like, the metric that says how wide it is, and the size
/// the two of them are at.
struct Ink<'a> {
    /// The metric: the widths of the characters.
    metric: &'a Metric,
    /// The size: the em, the line's height and the descent.
    metrics: &'a Metrics,
    /// The typeface: the shape of a letter.
    font: &'a Font,
}

/// Place one word: its letters, one after another.
///
/// Each letter is drawn in the typeface's shape and takes the **metric**
///'s width, so a letter is stretched by the difference between the two.
///
/// The `start` is where the word's first character is in the paragraph's
/// text, and every character of the word is placed after it. A word
/// with no `start` — a hyphen, which the breaker drew and the text
/// does not hold — is drawn and is not placed.
fn place_word(word: &str, start: Option<usize>, x: f64, baseline: f64, ink: &Ink<'_>, drawn: &mut Drawn) {
    let metric = ink.metric;
    let metrics = ink.metrics;
    let font = ink.font;
    let unit = metrics.unit();
    let mut cursor = x;
    let mut places: Vec<CharPlace> = Vec::with_capacity(word.chars().count());
    for (offset, character) in word.chars().enumerate() {
        let glyph = font.glyph(character);
        let natural = glyph.map(|glyph| glyph.advance * unit).unwrap_or(0.0);
        let wanted = metric.width_of(character, natural);
        if let Some(glyph) = glyph {
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
            drawn.cells.push(Rect::new(
                cursor,
                baseline - metrics.descent,
                wanted,
                metrics.em + metrics.descent,
            ));
        }
        // The **place** of the character: the caret stands before one,
        // and a character the typeface has no glyph for is still a
        // character and still takes its width.
        places.push(CharPlace {
            index: start.map(|start| start + offset).unwrap_or(offset),
            x: cursor,
            baseline,
            width: wanted,
            top: baseline - metrics.descent,
            bottom: baseline + metrics.em,
        });
        cursor += wanted;
    }
    if start.is_some() {
        drawn.chars.extend(places);
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
