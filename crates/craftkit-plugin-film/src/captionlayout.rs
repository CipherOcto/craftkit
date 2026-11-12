//! The **layout** of a caption: where the caption goes on the frame.
//!
//! The tree's own: the size of a caption is relative to a frame of a
//! thousand and eighty lines, the lines of it are wrapped at ninety
//! per cent of the width of the frame, and the block of lines hangs
//! from the **anchor** of the style — the top, the middle or the
//! bottom — a margin away from it.
//!
//! # The rules
//!
//! - the **scale** is the height of the frame over the tree's own
//!   thousand and eighty lines;
//! - the **size** of the letters is the size of the style, held
//!   between the tree's own four and two hundred and fifty-six
//!   pixels, scaled, and then held between four pixels and the height
//!   of the frame;
//! - the **lines** are the lines of the text, each one trimmed, each
//!   one **wrapped** at ninety per cent of the width of the frame —
//!   the tree's rule: a word is never broken, and a line that is too
//!   wide starts a new one;
//! - the **height of a line** is the size of the letters times the
//!   line spacing of the style, held between the tree's own eight
//!   tenths and four;
//! - the **padding** of a line is three tenths of the size of the
//!   letters, rounded;
//! - the **align** is the align of the style, and the **anchor** is
//!   the anchor of the style: the block of lines hangs from the top,
//!   the middle or the bottom of the frame, a **margin** away from
//!   it — the margin is the margin of the style, held between zero
//!   and forty-five hundredths of the height of the frame;
//! - the **side** of the frame is five per cent of its width: a line
//!   that is aligned to the left is that far from the side, and a
//!   line that is aligned to the right is that far from the other;
//! - the **settings** of the cue — the tree's own `line:` and `align:`
//!   of a WebVTT cue — may say where the caption goes: a `line:` says
//!   the height of the block as a fraction of the frame, and an
//!   `align:` says the align;
//! - the **outline** of the style grows the box of the caption: the
//!   outline is scaled and held between zero and the tree's own
//!   sixty-four pixels, and the box grows by that much and one;
//! - a caption with **no words** has no place, and it is **refused**.

use craftkit_derive::commands;

use craftkit_font::{Font, Metrics};
use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The name of the command, in the messages.
const COMMAND: &str = "film.caption.layout";

/// The **height** of the frame the tree's sizes are relative to.
const BASE_HEIGHT: f64 = 1080.0;

/// The **smallest** a caption may be, in the tree's pixels.
const MIN_SIZE: f64 = 4.0;

/// The **largest** a caption may be, in the tree's pixels.
const MAX_SIZE: f64 = 256.0;

/// The **widest** an outline may be, in the tree's pixels.
const MAX_OUTLINE: f64 = 64.0;

/// The **widest** a line may be, as a fraction of the frame.
const MAX_WIDTH: f64 = 0.9;

/// The **side** of the frame, as a fraction of its width.
const SIDE: f64 = 0.05;

/// The **padding** of a line, as a fraction of the size of the
/// letters.
const PADDING: f64 = 0.3;

/// The **largest** a margin may be, as a fraction of the height of the
/// frame.
const MAX_MARGIN: f64 = 0.45;

/// The **smallest** a line spacing may be.
const MIN_LINE_SPACING: f64 = 0.8;

/// The **largest** a line spacing may be.
const MAX_LINE_SPACING: f64 = 4.0;

/// The **aligns** of a caption: the tree's own.
const ALIGNS: [&str; 3] = ["left", "center", "right"];

/// The **anchors** of a caption: the tree's own.
const ANCHORS: [&str; 3] = ["top", "middle", "bottom"];

/// Where a caption goes on a frame: the tree's own layout.
pub struct Placement {
    /// The lines of the caption.
    pub lines: Vec<String>,
    /// How wide each line is.
    pub widths: Vec<f64>,
    /// Where each line starts.
    pub x: Vec<f64>,
    /// The size of the letters.
    pub size: f64,
    /// The height of a line.
    pub line_height: f64,
    /// The top of the block of lines.
    pub top: f64,
    /// The padding of a line.
    pub padding: f64,
    /// The align of the caption.
    pub align: String,
    /// The anchor of the caption.
    pub anchor: String,
    /// The margin of the caption.
    pub margin: f64,
    /// The outline of the caption.
    pub outline: f64,
    /// The box of the caption.
    pub extent: [f64; 4],
    /// How high the letters rise above their baseline.
    pub ascent: f64,
    /// How far the letters go below their baseline.
    pub descent: f64,
}

/// The **style** of a caption: the tree's own defaults, and the style
/// of the track when it is said.
pub fn style_of(session: &Session, track: Option<NodeRef>) -> Result<Json, EngineError> {
    let Some(track) = track else { return Ok(crate::captionstyle::default_style()) };
    let Some(node) = session.document().node(track.0) else {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("there is no caption track `{}`", track.0),
        });
    };
    if node.kind != "film.caption.track" {
        return Err(EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the node `{}` is a `{}` and not a caption track", track.0, node.kind),
        });
    }
    Ok(node.data.get("style").cloned().unwrap_or_else(crate::captionstyle::default_style))
}

/// A **number** of the style, held between the two numbers.
fn number_in(style: &Json, name: &str, lowest: f64, highest: f64, fallback: f64) -> f64 {
    style
        .get(name)
        .and_then(Json::as_f64)
        .filter(|value| value.is_finite())
        .map(|value| value.clamp(lowest, highest))
        .unwrap_or(fallback)
}

/// A **word** of the style: one of the tree's own.
fn word_in(style: &Json, name: &str, words: &[&str], fallback: &str) -> String {
    let found = style.get(name).and_then(Json::as_str).unwrap_or_default().trim().to_ascii_lowercase();
    if words.contains(&found.as_str()) { found } else { fallback.to_string() }
}

/// The **settings** of a cue: the tree's own `line:` and `align:`.
fn settings_of(settings: &str) -> (Option<f64>, Option<String>) {
    let mut line = None;
    let mut align = None;
    for pair in settings.split_whitespace() {
        let Some((key, value)) = pair.split_once(':') else { continue };
        match key {
            "line" => {
                let value = value.split(',').next().unwrap_or("");
                if let Some(percent) = value.strip_suffix('%').and_then(|part| part.parse::<f64>().ok()) {
                    line = Some((percent / 100.0).clamp(0.0, 1.0));
                }
            }
            "align" => {
                align = match value {
                    "start" | "left" => Some("left".to_string()),
                    "end" | "right" => Some("right".to_string()),
                    "center" | "middle" => Some("center".to_string()),
                    _ => None,
                };
            }
            _ => {}
        }
    }
    (line, align)
}

/// **Wrap** one line to the width: the tree's own rule.
///
/// A word is never broken: a line that would be too wide with the
/// next word on it starts a new line.
fn wrap(line: &str, max_width: f64, measure: &dyn Fn(&str) -> f64) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in line.split(' ') {
        let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
        if !current.is_empty() && measure(&candidate) > max_width {
            out.push(std::mem::take(&mut current));
            current = word.to_string();
        } else {
            current = candidate;
        }
    }
    out.push(current);
    out
}

/// **Lay** a caption out on a frame of a given size.
///
/// The tree's own: the size of the letters is relative to a frame of
/// a thousand and eighty lines, the lines are wrapped at ninety per
/// cent of the width of the frame, and the block of them hangs from
/// the anchor of the style.
pub fn placement(
    text: &str,
    style: &Json,
    settings: &str,
    width: f64,
    height: f64,
) -> Result<Placement, String> {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Err("the frame of a caption has a width and a height that are positive".to_string());
    }
    // The **scale** and the **size** of the letters.
    let scale = height / BASE_HEIGHT;
    let size = number_in(style, "size", MIN_SIZE, MAX_SIZE, 54.0);
    let px = (size * scale).clamp(MIN_SIZE, height);
    let Some(metrics) = Metrics::new(px) else {
        return Err("the size of the letters of a caption is a number that is positive".to_string());
    };
    // The **lines**.
    let max_width = width * MAX_WIDTH;
    let font = Font::builtin();
    let measure = |line: &str| craftkit_font::width(line, font, &metrics);
    let mut lines: Vec<String> = Vec::new();
    for line in text.lines() {
        lines.extend(wrap(line.trim(), max_width, &measure));
    }
    lines.retain(|line| !line.is_empty());
    if lines.is_empty() {
        return Err("a caption with no words has no place".to_string());
    }
    // The **height of a line**, the **padding** and the **block**.
    let spacing = number_in(style, "line_spacing", MIN_LINE_SPACING, MAX_LINE_SPACING, 1.25);
    let line_height = px * spacing;
    let padding = (px * PADDING).round();
    let block_height = line_height * lines.len() as f64;
    // The **settings** of the cue.
    let (line_override, align_override) = settings_of(settings);
    let align = align_override.unwrap_or_else(|| word_in(style, "align", &ALIGNS, "center"));
    let anchor = word_in(style, "anchor", &ANCHORS, "bottom");
    let margin = number_in(style, "margin", 0.0, MAX_MARGIN, 0.08) * height;
    // The **top** of the block.
    let top = match (line_override, anchor.as_str()) {
        (Some(fraction), _) => (fraction * height).min(height - block_height),
        (None, "top") => margin,
        (None, "middle") => (height - block_height) / 2.0,
        _ => height - margin - block_height,
    }
    .max(0.0)
    .round();
    // The **side** of the frame and the place of each line.
    let side = width * SIDE;
    let widths: Vec<f64> = lines.iter().map(|line| measure(line)).collect();
    let xs: Vec<f64> = widths
        .iter()
        .map(|line_width| match align.as_str() {
            "left" => side + padding,
            "right" => width - side - padding - line_width,
            _ => (width - line_width) / 2.0,
        })
        .collect();
    // The **outline** grows the box.
    let outline = (number_in(style, "outline", 0.0, MAX_OUTLINE, 0.0) * scale).clamp(0.0, MAX_OUTLINE);
    let grow = outline.ceil() + 1.0;
    let left = xs.iter().zip(&widths).map(|(x, _)| x - padding - grow).fold(f64::MAX, f64::min).floor();
    let right = xs
        .iter()
        .zip(&widths)
        .map(|(x, line_width)| x + line_width + padding + grow)
        .fold(f64::MIN, f64::max)
        .ceil();
    let box_top = (top - grow).floor();
    let box_bottom = (top + block_height + grow).ceil();
    // The box is held inside the frame.
    let left = left.clamp(0.0, width);
    let right = right.clamp(0.0, width);
    let box_top = box_top.clamp(0.0, height);
    let box_bottom = box_bottom.clamp(0.0, height);
    if right <= left || box_bottom <= box_top {
        return Err("the caption has no room on the frame".to_string());
    }
    Ok(Placement {
        lines,
        widths,
        x: xs,
        size: px,
        line_height,
        top,
        padding,
        align,
        anchor,
        margin,
        outline,
        extent: [left, box_top, right, box_bottom],
        ascent: metrics.em,
        descent: metrics.descent,
    })
}

commands! {
    /// **Lay** a **caption** out on a frame of a given size.
    ///
    /// The tree's own: the size of the letters is relative to a frame of
    /// a thousand and eighty lines, the lines are wrapped at ninety per
    /// cent of the width of the frame, and the block of them hangs from
    /// the anchor of the style.
    film.caption.layout {
        area: "film",
        layer: "film",
        icon: "list",
        params: {
            /// The text of the caption.

            text: Text,
            /// The caption track that holds the style; the tree's own style
            /// when it is not said.

            track: Option<NodeRef>,
            /// The width of the frame.

            width: Number,
            /// The height of the frame.

            height: Number,
            /// The settings of the cue: `line:50% align:center`.

            settings: Option<Text>,
        },
        run: {

            let style = style_of(session, self.track)?;
            let settings = self.settings.as_ref().map(|text| text.0.clone()).unwrap_or_default();
            let laid = placement(&self.text.0, &style, &settings, self.width.0, self.height.0)
                .map_err(|message| EngineError::Failed { id: COMMAND.to_string(), message })?;
            Ok(Outcome::new("the caption is laid out on the frame".to_string()).with_data(json!({
                "lines": laid.lines,
                "widths": laid.widths,
                "x": laid.x,
                "size": laid.size,
                "line_height": laid.line_height,
                "top": laid.top,
                "align": laid.align,
                "anchor": laid.anchor,
                "margin": laid.margin,
                "outline": laid.outline,
                "box": laid.extent,
                "frame": [self.width.0, self.height.0],
            })))

        }
    }
}
