//! The **burn-in** of a caption: the caption drawn into the frame.
//!
//! The tree's own: the caption is laid out on the frame, and the
//! letters of it are drawn — over the **background** of the style, and
//! over the **outline** of it.
//!
//! # The rules
//!
//! - the **layout** is the tree's own: the size of the letters is
//!   relative to a frame of a thousand and eighty lines, the lines are
//!   wrapped at ninety per cent of the width of the frame, and the
//!   block of them hangs from the anchor of the style;
//! - the **background** of the style is the box of each line — the
//!   line and the padding around it — and it is drawn under the
//!   letters;
//! - the **outline** of the style is under the letters and over the
//!   background;
//! - the **letters** are the colour of the style;
//! - the picture is a **PNG**, and it is written where the caller says.
//!
//! # What is not the tree's
//!
//! The tree **fills** the letters of its font, and this tree draws the
//! **strokes** of the kit's face: the width of the stroke is a tenth of
//! the size of the letters, and the outline is a wider stroke of the
//! same letters beneath them, where the tree widens the filled letters
//! themselves.

use craftkit_derive::Command;

use craftkit_font::Font;
use craftkit_raster::Image;
use craftkit_sdk::document::{BlendMode, Colour};

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

use crate::captionlayout;

/// The name of the command, in the messages.
const COMMAND: &str = "film.caption.burn";

/// The **width** of the stroke of the letters: a tenth of the size of
/// the letters.
///
/// The tree fills the letters of its font; this tree draws the strokes
/// of the kit's face, and the width of the stroke is this tree's own.
const STROKE: f64 = 0.1;

/// A **colour** of the style, as the tree writes it: `#rrggbb` or
/// `#rrggbbaa`.
fn colour_of(style: &Json, name: &str, fallback: &str) -> Colour {
    let text = style.get(name).and_then(Json::as_str).unwrap_or(fallback).trim();
    let hex = text.strip_prefix('#').unwrap_or(text);
    let pair = |at: usize| u8::from_str_radix(&hex[at..at + 2], 16).unwrap_or(0);
    if hex.len() != 6 && hex.len() != 8 {
        return Colour([0.0, 0.0, 0.0, 0.0]);
    }
    let alpha = if hex.len() == 8 { pair(6) } else { 255 };
    Colour([
        f32::from(pair(0)) / 255.0,
        f32::from(pair(2)) / 255.0,
        f32::from(pair(4)) / 255.0,
        f32::from(alpha) / 255.0,
    ])
}

/// **Fill** a **disc** of the picture: the stroke of a letter is a
/// disc that walks along the line.
fn fill_disc(image: &mut Image, x: f64, y: f64, radius: f64, colour: Colour) {
    let left = (x - radius).floor() as i64;
    let right = (x + radius).ceil() as i64;
    let top = (y - radius).floor() as i64;
    let bottom = (y + radius).ceil() as i64;
    for py in top..=bottom {
        if py < 0 {
            continue;
        }
        for px in left..=right {
            if px < 0 {
                continue;
            }
            let dx = px as f64 + 0.5 - x;
            let dy = py as f64 + 0.5 - y;
            if dx * dx + dy * dy <= radius * radius {
                image.blend(px as usize, py as usize, colour, BlendMode::Normal);
            }
        }
    }
}

/// **Draw** a **line** of the picture: the stroke walks along it.
fn draw_line(image: &mut Image, points: &[[f64; 2]], width: f64, colour: Colour) {
    let radius = (width / 2.0).max(0.5);
    if points.len() == 1 {
        fill_disc(image, points[0][0], points[0][1], radius, colour);
        return;
    }
    for pair in points.windows(2) {
        let (x0, y0) = (pair[0][0], pair[0][1]);
        let (x1, y1) = (pair[1][0], pair[1][1]);
        let length = ((x1 - x0).powi(2) + (y1 - y0).powi(2)).sqrt();
        let steps = ((length / radius).ceil() as usize).max(1);
        for step in 0..=steps {
            let part = step as f64 / steps as f64;
            fill_disc(image, x0 + (x1 - x0) * part, y0 + (y1 - y0) * part, radius, colour);
        }
    }
}

/// **Fill** the **box** of the picture: the background of a line.
fn fill_box(image: &mut Image, left: f64, top: f64, right: f64, bottom: f64, colour: Colour) {
    for py in top.floor() as i64..=bottom.ceil() as i64 {
        if py < 0 {
            continue;
        }
        for px in left.floor() as i64..=right.ceil() as i64 {
            if px < 0 {
                continue;
            }
            image.blend(px as usize, py as usize, colour, BlendMode::Normal);
        }
    }
}

/// **Burn** a caption into a frame: the caption is drawn, and the
/// picture of it is written.
///
/// The tree's own: the layout is the tree's, and the letters are drawn
/// over the background of the style and over its outline.
#[derive(Command)]
#[command(id = "film.caption.burn", area = "film", layer = "film", icon = "image")]
pub struct BurnCaption {
    /// The text of the caption.
    pub text: Text,
    /// The caption track that holds the style; the tree's own style
    /// when it is not said.
    pub track: Option<NodeRef>,
    /// The width of the frame.
    pub width: Number,
    /// The height of the frame.
    pub height: Number,
    /// The settings of the cue: `line:50% align:center`.
    pub settings: Option<Text>,
    /// Where the picture is written.
    pub to: Text,
}

impl BurnCaption {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let style = captionlayout::style_of(session, self.track)?;
        let settings = self.settings.as_ref().map(|text| text.0.clone()).unwrap_or_default();
        let laid = captionlayout::placement(&self.text.0, &style, &settings, self.width.0, self.height.0)
            .map_err(|message| EngineError::Failed { id: COMMAND.to_string(), message })?;
        let width = self.width.0;
        let height = self.height.0;
        // The **frame** is a picture of its own size, and the size of
        // it is the kit's own bound.
        let pixels = width * height;
        if !pixels.is_finite() || pixels <= 0.0 || pixels > craftkit_raster::MAX_IMAGE_PIXELS as f64 {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: format!(
                    "the frame of a caption holds at most {} pixels",
                    craftkit_raster::MAX_IMAGE_PIXELS
                ),
            });
        }
        let mut image = Image::new(width as usize, height as usize, Colour([0.0, 0.0, 0.0, 0.0]));
        // The **colours** of the style.
        let text_colour = colour_of(&style, "colour", "#ffffffff");
        let background_colour = colour_of(&style, "background_colour", "#000000bf");
        let outline_colour = colour_of(&style, "outline_colour", "#000000ff");
        let background = style.get("background").and_then(Json::as_bool).unwrap_or(true);
        // The **letters**, and where each line of them stands.
        let font = Font::builtin();
        let Some(metrics) = craftkit_font::Metrics::new(laid.size) else {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the size of the letters of a caption is a number that is positive".to_string(),
            });
        };
        let stroke = laid.size * STROKE;
        for (index, line) in laid.lines.iter().enumerate() {
            let line_top = laid.top + laid.line_height * index as f64;
            let baseline = line_top + (laid.line_height - (laid.ascent + laid.descent)) / 2.0 + laid.ascent;
            let left = laid.x[index] - laid.padding;
            let right = laid.x[index] + laid.widths[index] + laid.padding;
            // The **background** of the line.
            if background && background_colour.0[3] > 0.0 {
                fill_box(&mut image, left, line_top, right, line_top + laid.line_height, background_colour);
            }
            // The **letters** of the line, and the **outline** under
            // them.
            let placed = craftkit_font::layout(line, font, &metrics);
            for character in placed {
                let Some(strokes) =
                    font.outline(character.character, &metrics, laid.x[index] + character.x, baseline)
                else {
                    continue;
                };
                if laid.outline > 0.0 {
                    for stroke_points in &strokes {
                        draw_line(&mut image, stroke_points, stroke + laid.outline * 2.0, outline_colour);
                    }
                }
                for stroke_points in &strokes {
                    draw_line(&mut image, stroke_points, stroke, text_colour);
                }
            }
        }
        // The **picture** is written where the caller says.
        let path = self.to.0.trim().to_string();
        if path.is_empty() {
            return Err(EngineError::Failed {
                id: COMMAND.to_string(),
                message: "the picture of the caption goes somewhere".to_string(),
            });
        }
        let png = image.to_png();
        std::fs::write(&path, &png).map_err(|error| EngineError::Failed {
            id: COMMAND.to_string(),
            message: format!("the picture cannot be written: {error}"),
        })?;
        Ok(Outcome::new("the caption is burnt into the frame".to_string()).with_data(json!({
            "lines": laid.lines,
            "size": laid.size,
            "top": laid.top,
            "box": laid.extent,
            "frame": [width, height],
            "path": path,
            "bytes": png.len(),
        })))
    }
}
