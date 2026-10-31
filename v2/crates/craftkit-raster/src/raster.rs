//! The **canvas**: the kit's painting seam, over a picture.

use craftkit_font::{Font, Metrics};
use craftkit_sdk::camera::Camera;
use craftkit_sdk::document::{Canvas, Colour, FillRule, Path, Point, Rect, Segment, Stroke, SubPath};

use crate::image::Image;

/// The most pixels a picture may hold.
///
/// A picture is memory, and a picture that is asked to be enormous is a
/// program that runs out of memory. The number is this crate's, and a
/// program that wants a different one says so.
pub const MAX_IMAGE_PIXELS: usize = 16_777_216;

/// The longest side a picture may have.
///
/// A picture that is one pixel wide and sixteen million tall is within
/// the pixels and is not a picture anybody wants; the bound is a shape
/// and not only a size.
pub const MAX_IMAGE_SIDE: usize = 8192;

/// How many pieces a curve is cut into.
///
/// A cubic is not a line, and a line is what a rasteriser can draw. The
/// number is a number: a hundredth of a curve is a curve to the eye.
const CURVE_STEPS: usize = 24;

/// How wide a character is, in the world's units, before the zoom.
///
/// This is the **cell**: every character takes one, a letter and a
/// space alike, so a line of text is monospaced.
const ADVANCE: f64 = 8.0;

/// How tall a character is, in the world's units, before the zoom.
const LINE_HEIGHT: f64 = 12.0;

/// The **em** of the canvas's text: the height of the capitals, in the
/// world's units.
///
/// It is under the line height, because a line of text needs room below
/// the baseline for the letters that hang there — a `g`, a `p` — and
/// the line height is that room.
const EM: f64 = LINE_HEIGHT * 2.0 / 3.0;

/// How wide the pen is, in the world's units.
///
/// A letter is a line, and a line has to be thick enough to be seen.
const PEN: f64 = 0.8;

/// A canvas that draws into a picture.
///
/// The text takes the colour it is given, like everything else: the
/// seam has one colour for everything that is drawn, and a canvas that
/// chose its own would be a canvas that could not draw a layer's text.
#[derive(Debug, Clone)]
pub struct Raster {
    image: Image,
    camera: Camera,
}

impl Raster {
    /// A canvas of that size, with that camera.
    ///
    /// The size is refused when it is out of bounds: a picture that is
    /// asked to be enormous is refused and not made smaller, because a
    /// picture nobody asked for is not the picture that was asked for.
    pub fn new(width: usize, height: usize, camera: Camera) -> Result<Self, String> {
        if width > MAX_IMAGE_SIDE || height > MAX_IMAGE_SIDE {
            return Err(format!(
                "the picture is {width} by {height}, and the longest side is {MAX_IMAGE_SIDE}"
            ));
        }
        if width.saturating_mul(height) > MAX_IMAGE_PIXELS {
            return Err(format!(
                "the picture holds {} pixels, and the limit is {MAX_IMAGE_PIXELS}",
                width.saturating_mul(height)
            ));
        }
        Ok(Raster { image: Image::new(width, height, Colour::WHITE), camera })
    }

    /// The picture that has been drawn.
    pub fn image(&self) -> &Image {
        &self.image
    }

    /// The camera the picture is seen through.
    pub fn camera(&self) -> Camera {
        self.camera
    }

    /// Draw a picture into the picture.
    ///
    /// The samples are the **decoder's**: interleaved by pixel — one
    /// byte for a grey picture, three for a colour one — and the rows
    /// are top to bottom. The picture is scaled into the rectangle, and
    /// the scaling is the simplest one there is: the **nearest** pixel
    /// is taken, and there is no filtering.
    ///
    /// A picture that does not fit the samples it was given is drawn as
    /// far as it goes: a picture with fewer samples than it says is a
    /// picture that is not all there.
    pub fn blit(&mut self, samples: &[u8], width: u32, height: u32, components: u8, rect: Rect) {
        if width == 0 || height == 0 || components == 0 {
            return;
        }
        let (x0, y0) = self.to_pixels(Point::new(rect.x, rect.y + rect.h));
        let (x1, y1) = self.to_pixels(Point::new(rect.x + rect.w, rect.y));
        let left = x0.min(x1);
        let right = x0.max(x1);
        let top = y0.min(y1);
        let bottom = y0.max(y1);
        let first = (left.floor().max(0.0)) as usize;
        let last = (right.ceil().min(self.image.width() as f64)) as usize;
        let top_row = (top.floor().max(0.0)) as usize;
        let bottom_row = (bottom.ceil().min(self.image.height() as f64)) as usize;
        let span_x = right - left;
        let span_y = bottom - top;
        if span_x <= 0.0 || span_y <= 0.0 {
            return;
        }
        for row in top_row..bottom_row {
            let source_y = (((row as f64 + 0.5 - top) / span_y) * height as f64) as u32;
            let source_y = source_y.min(height - 1);
            for column in first..last {
                let source_x = (((column as f64 + 0.5 - left) / span_x) * width as f64) as u32;
                let source_x = source_x.min(width - 1);
                let at = ((source_y as usize * width as usize) + source_x as usize) * components as usize;
                let Some(pixel) = samples.get(at..at + components as usize) else { continue };
                let colour = match components {
                    1 => Colour([
                        pixel[0] as f32 / 255.0,
                        pixel[0] as f32 / 255.0,
                        pixel[0] as f32 / 255.0,
                        1.0,
                    ]),
                    3 => Colour([
                        pixel[0] as f32 / 255.0,
                        pixel[1] as f32 / 255.0,
                        pixel[2] as f32 / 255.0,
                        1.0,
                    ]),
                    4 => Colour([
                        pixel[0] as f32 / 255.0,
                        pixel[1] as f32 / 255.0,
                        pixel[2] as f32 / 255.0,
                        1.0,
                    ]),
                    _ => continue,
                };
                self.image.blend(column, row, colour);
            }
        }
    }

    /// Where a point of the world is on the picture.
    ///
    /// The world's **up** is the picture's **down**: the document is
    /// drawn the way it is looked at, and not the way it is stored.
    fn to_pixels(&self, point: Point) -> (f64, f64) {
        // The camera's own mapping, and only the picture's own size on
        // top of it: a front end that clicks with a different mapping
        // clicks on the wrong thing.
        let screen = self.camera.to_screen(point);
        let centre_x = self.image.width() as f64 / 2.0;
        let centre_y = self.image.height() as f64 / 2.0;
        (screen.x + centre_x, centre_y - screen.y)
    }

    /// Fill a shape that has already been turned into a polygon.
    fn fill_polygon(&mut self, points: &[(f64, f64)], colour: Colour, rule: FillRule) {
        if points.len() < 3 || colour.0[3] <= 0.0 {
            return;
        }
        let mut lowest = f64::INFINITY;
        let mut highest = f64::NEG_INFINITY;
        for (_, y) in points {
            lowest = lowest.min(*y);
            highest = highest.max(*y);
        }
        let first = (lowest.floor().max(0.0)) as usize;
        let last = (highest.ceil().min(self.image.height() as f64)) as usize;
        for row in first..last {
            // The sample is the middle of the row: a pixel is either
            // painted or it is not, and the middle is where it is.
            let sample = row as f64 + 0.5;
            let mut crossings: Vec<(f64, i32)> = Vec::new();
            for (index, &(x0, y0)) in points.iter().enumerate() {
                let (x1, y1) = points[(index + 1) % points.len()];
                if (y0 <= sample) == (y1 <= sample) {
                    continue;
                }
                let t = (sample - y0) / (y1 - y0);
                let x = x0 + t * (x1 - x0);
                let direction = if y1 > y0 { 1 } else { -1 };
                crossings.push((x, direction));
            }
            crossings
                .sort_by(|left, right| left.0.partial_cmp(&right.0).unwrap_or(std::cmp::Ordering::Equal));
            let mut winding = 0;
            let mut start = 0.0;
            for (x, direction) in crossings {
                let was_inside = match rule {
                    FillRule::NonZero => winding != 0,
                    FillRule::EvenOdd => winding % 2 != 0,
                };
                winding += direction;
                let is_inside = match rule {
                    FillRule::NonZero => winding != 0,
                    FillRule::EvenOdd => winding % 2 != 0,
                };
                if !was_inside && is_inside {
                    start = x;
                }
                if was_inside && !is_inside {
                    self.fill_span(row, start, x, colour);
                }
            }
        }
    }

    /// Fill a run of a row, from one edge to another.
    fn fill_span(&mut self, row: usize, from: f64, to: f64, colour: Colour) {
        if to <= from {
            return;
        }
        let first = (from.floor().max(0.0)) as usize;
        let last = (to.ceil().min(self.image.width() as f64)) as usize;
        for column in first..last {
            self.image.blend(column, row, colour);
        }
    }
}

impl Canvas for Raster {
    /// Draw a rectangle.
    fn rect(&mut self, rect: Rect, fill: Colour) {
        if fill.0[3] <= 0.0 {
            return;
        }
        let (x0, y0) = self.to_pixels(Point::new(rect.x, rect.y + rect.h));
        let (x1, y1) = self.to_pixels(Point::new(rect.x + rect.w, rect.y));
        let left = x0.min(x1);
        let right = x0.max(x1);
        let top = y0.min(y1);
        let bottom = y0.max(y1);
        let first = (left.floor().max(0.0)) as usize;
        let last = (right.ceil().min(self.image.width() as f64)) as usize;
        let top_row = (top.floor().max(0.0)) as usize;
        let bottom_row = (bottom.ceil().min(self.image.height() as f64)) as usize;
        for row in top_row..bottom_row {
            for column in first..last {
                self.image.blend(column, row, fill);
            }
        }
    }

    /// Draw a path: filled, and stroked on its outline.
    fn path(&mut self, path: &Path, fill: Colour, stroke: Option<Stroke>) {
        let contours: Vec<Vec<(f64, f64)>> =
            path.subpaths.iter().map(|subpath| self.flatten(subpath)).collect();
        if fill.0[3] > 0.0 {
            for contour in &contours {
                self.fill_polygon(contour, fill, path.rule);
            }
        }
        let Some(stroke) = stroke else { return };
        if stroke.colour.0[3] <= 0.0 || stroke.width <= 0.0 {
            return;
        }
        for contour in &contours {
            self.stroke_contour(contour, stroke);
        }
    }

    /// Draw a line of text.
    ///
    /// The letters are the kit's **own**: the typeface is a table of
    /// strokes drawn in code (§128), and a character is drawn by its
    /// strokes and not by the box it would occupy.
    ///
    /// The **advance** is still one cell for every character — a space
    /// takes a cell and a letter takes a cell — so a line of text is
    /// still monospaced; what changed is that the cell now holds a
    /// letter and not a rectangle.
    fn text(&mut self, at: Point, text: &str, colour: Colour) {
        let font = Font::builtin();
        let Some(metrics) = Metrics::new(EM) else { return };
        let unit = metrics.unit();
        for (index, character) in text.chars().enumerate() {
            let x = at.x + index as f64 * ADVANCE;
            let Some(glyph) = font.glyph(character) else { continue };
            let natural = glyph.advance * unit;
            // The letter is **stretched** to the cell: the cell says how
            // wide the character is and the typeface says what it looks
            // like.
            let scale = if natural > 0.0 { ADVANCE / natural } else { 1.0 };
            for stroke in &glyph.strokes {
                if stroke.len() < 2 {
                    continue;
                }
                let mut segments = Vec::with_capacity(stroke.len() - 1);
                for point in stroke.iter().skip(1) {
                    segments.push(Segment::Line {
                        to: Point::new(x + point[0] * unit * scale, at.y + point[1] * unit),
                    });
                }
                let start = stroke[0];
                let path = Path::single(SubPath {
                    start: Point::new(x + start[0] * unit * scale, at.y + start[1] * unit),
                    segments,
                    closed: false,
                });
                Canvas::path(self, &path, Colour::NONE, Some(Stroke { colour, width: PEN }));
            }
        }
    }
}

impl Raster {
    /// Turn a contour into points on the picture.
    fn flatten(&self, subpath: &craftkit_sdk::document::SubPath) -> Vec<(f64, f64)> {
        let mut points = vec![self.to_pixels(subpath.start)];
        let mut at = subpath.start;
        for segment in &subpath.segments {
            match segment {
                Segment::Line { to } => {
                    points.push(self.to_pixels(*to));
                    at = *to;
                }
                Segment::Cubic { c1, c2, to } => {
                    for step in 1..=CURVE_STEPS {
                        let t = step as f64 / CURVE_STEPS as f64;
                        let point = cubic(at, *c1, *c2, *to, t);
                        points.push(self.to_pixels(point));
                    }
                    at = *to;
                }
            }
        }
        if subpath.closed && points.len() > 1 {
            let first = points[0];
            let last = points[points.len() - 1];
            if (first.0 - last.0).abs() > f64::EPSILON || (first.1 - last.1).abs() > f64::EPSILON {
                points.push(first);
            }
        }
        points
    }

    /// Draw the outline of a contour.
    ///
    /// Each segment is a **quad**: the line, widened to the stroke's
    /// width. The joints are squares, because a square joint is what a
    /// machine can draw and a round one is not.
    fn stroke_contour(&mut self, contour: &[(f64, f64)], stroke: Stroke) {
        if contour.len() < 2 {
            return;
        }
        let half = stroke.width * self.camera.zoom() / 2.0;
        for window in contour.windows(2) {
            let (x0, y0) = window[0];
            let (x1, y1) = window[1];
            let dx = x1 - x0;
            let dy = y1 - y0;
            let length = (dx * dx + dy * dy).sqrt();
            if length <= f64::EPSILON {
                continue;
            }
            let nx = -dy / length * half;
            let ny = dx / length * half;
            let quad = [(x0 + nx, y0 + ny), (x1 + nx, y1 + ny), (x1 - nx, y1 - ny), (x0 - nx, y0 - ny)];
            self.fill_polygon(&quad, stroke.colour, FillRule::NonZero);
        }
        // The joints: a square at every corner, so that the outline has
        // no gaps in it.
        for point in contour {
            let square = [
                (point.0 - half, point.1 - half),
                (point.0 + half, point.1 - half),
                (point.0 + half, point.1 + half),
                (point.0 - half, point.1 + half),
            ];
            self.fill_polygon(&square, stroke.colour, FillRule::NonZero);
        }
    }
}

/// A point on a cubic curve.
fn cubic(from: Point, c1: Point, c2: Point, to: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let x = u * u * u * from.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * to.x;
    let y = u * u * u * from.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * to.y;
    Point::new(x, y)
}
