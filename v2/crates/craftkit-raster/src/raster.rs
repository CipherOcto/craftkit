//! The **canvas**: the kit's painting seam, over a picture.

use craftkit_font::{Font, Metrics};
use craftkit_sdk::camera::Camera;
use craftkit_sdk::document::{
    BlendMode, Canvas, Cap, Colour, Fill, FillRule, Gradient, GradientGeometry, Join, Path, Point, Rect,
    Segment, Stroke, SubPath,
};

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

/// How many pieces a **round** corner or end is cut into.
///
/// A circle is not a polygon, and a polygon is what a rasteriser can
/// fill. The number is a number: a twelfth of a circle is a circle to
/// the eye.
const ARC_STEPS: usize = 12;

/// How long a **miter** may be, in widths of the stroke.
///
/// A corner that is nearly straight makes a miter that is very long,
/// and a long spike of ink is not what a corner means. A miter longer
/// than four widths is a bevel.
const MITER_LIMIT: f64 = 4.0;

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
                self.image.blend(column, row, colour, BlendMode::Normal);
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
    fn fill_polygon(&mut self, points: &[(f64, f64)], fill: &Fill, blend: BlendMode, rule: FillRule) {
        if points.len() < 3 || fill.is_none() {
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
                    self.fill_span(row, start, x, fill, blend);
                }
            }
        }
    }

    /// Fill a run of a row, from one edge to another.
    ///
    /// A **colour** is the same in every pixel of the span, and a
    /// **gradient** is not: every pixel is asked where it is along the
    /// gradient, and the answer is its colour.
    fn fill_span(&mut self, row: usize, from: f64, to: f64, fill: &Fill, blend: BlendMode) {
        if to <= from {
            return;
        }
        let first = (from.floor().max(0.0)) as usize;
        let last = (to.ceil().min(self.image.width() as f64)) as usize;
        match fill {
            Fill::Solid(colour) => {
                for column in first..last {
                    self.image.blend(column, row, *colour, blend);
                }
            }
            Fill::Gradient(gradient) => {
                for column in first..last {
                    let colour = gradient.colour_at_parameter(self.parameter_of(gradient, column, row));
                    self.image.blend(column, row, colour, blend);
                }
            }
        }
    }

    /// Where a pixel is **along** a gradient.
    ///
    /// The gradient is in the world's units and the pixel is in the
    /// picture's, so the gradient's own geometry is put into pixels
    /// once and the answer is worked out there.
    fn parameter_of(&self, gradient: &Gradient, column: usize, row: usize) -> f64 {
        let pixel = (column as f64 + 0.5, row as f64 + 0.5);
        match gradient.geometry {
            GradientGeometry::Linear { from, to } => {
                let start = self.to_pixels(from);
                let end = self.to_pixels(to);
                let dx = end.0 - start.0;
                let dy = end.1 - start.1;
                let length = dx * dx + dy * dy;
                if length <= 0.0 {
                    return 0.0;
                }
                (((pixel.0 - start.0) * dx + (pixel.1 - start.1) * dy) / length).clamp(0.0, 1.0)
            }
            GradientGeometry::Radial { centre, radius } => {
                let middle = self.to_pixels(centre);
                let reach = radius * self.camera.zoom();
                if reach <= 0.0 {
                    return 0.0;
                }
                let dx = pixel.0 - middle.0;
                let dy = pixel.1 - middle.1;
                ((dx * dx + dy * dy).sqrt() / reach).clamp(0.0, 1.0)
            }
        }
    }
}

impl Canvas for Raster {
    /// Draw a rectangle.
    fn rect(&mut self, rect: Rect, fill: Fill, blend: BlendMode) {
        if fill.is_none() {
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
                let colour = match &fill {
                    Fill::Solid(colour) => *colour,
                    Fill::Gradient(gradient) => {
                        gradient.colour_at_parameter(self.parameter_of(gradient, column, row))
                    }
                };
                self.image.blend(column, row, colour, blend);
            }
        }
    }

    /// Draw a path: filled, and stroked on its outline.
    fn path(&mut self, path: &Path, fill: Fill, blend: BlendMode, stroke: Option<Stroke>) {
        let contours: Vec<Vec<(f64, f64)>> =
            path.subpaths.iter().map(|subpath| self.flatten(subpath)).collect();
        if !fill.is_none() {
            for contour in &contours {
                self.fill_polygon(contour, &fill, blend, path.rule);
            }
        }
        let Some(stroke) = stroke else { return };
        if stroke.colour.0[3] <= 0.0 || stroke.width <= 0.0 {
            return;
        }
        for contour in &contours {
            self.stroke_contour(contour, stroke, blend);
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
    /// Draw a **picture**: its pixels, in a rectangle.
    ///
    /// The picture is **scaled** to fill the rectangle: every pixel of
    /// the picture is read and the nearest one is put in each place of
    /// the rectangle. There is no smoothing, because the kit has no
    /// anti-aliasing (§67) and a picture that is smoothed and a picture
    /// that is not are two different pictures.
    fn picture(&mut self, at: Rect, picture: &craftkit_sdk::picture::Picture) {
        if picture.is_empty() {
            return;
        }
        let (left, top) = self.to_pixels(Point::new(at.x, at.y + at.h));
        let (right, bottom) = self.to_pixels(Point::new(at.x + at.w, at.y));
        let first_column = left.floor().max(0.0) as usize;
        let last_column = (right.ceil().max(0.0) as usize).min(self.image.width());
        let first_row = top.floor().max(0.0) as usize;
        let last_row = (bottom.ceil().max(0.0) as usize).min(self.image.height());
        let width = right - left;
        let height = bottom - top;
        if width <= 0.0 || height <= 0.0 {
            return;
        }
        for row in first_row..last_row {
            for column in first_column..last_column {
                let along = (column as f64 + 0.5 - left) / width;
                let down = (row as f64 + 0.5 - top) / height;
                let x = (along * f64::from(picture.width)) as u32;
                let y = (down * f64::from(picture.height)) as u32;
                let Some([r, g, b, a]) = picture.pixel(x.min(picture.width.saturating_sub(1)), y) else {
                    continue;
                };
                let colour = Colour([
                    f32::from(r) / 255.0,
                    f32::from(g) / 255.0,
                    f32::from(b) / 255.0,
                    f32::from(a) / 255.0,
                ]);
                self.image.blend(column, row, colour, BlendMode::Normal);
            }
        }
    }

    fn text(&mut self, at: Point, text: &str, colour: Colour, size: f64) {
        let font = Font::builtin();
        let Some(metrics) = Metrics::new(size) else { return };
        let unit = metrics.unit();
        let mut pen = at.x;
        for character in text.chars() {
            let Some(glyph) = font.glyph(character) else { continue };
            for stroke in &glyph.strokes {
                if stroke.len() < 2 {
                    continue;
                }
                let mut segments = Vec::with_capacity(stroke.len() - 1);
                for point in stroke.iter().skip(1) {
                    segments.push(Segment::Line {
                        to: Point::new(pen + point[0] * unit, at.y + point[1] * unit),
                    });
                }
                let start = stroke[0];
                let path = Path::single(SubPath {
                    start: Point::new(pen + start[0] * unit, at.y + start[1] * unit),
                    segments,
                    closed: false,
                });
                Canvas::path(
                    self,
                    &path,
                    Fill::Solid(Colour::NONE),
                    BlendMode::Normal,
                    Some(Stroke { colour, width: PEN, ..Default::default() }),
                );
            }
            pen += glyph.advance * unit;
        }
    }
}

/// The **paint** a line is laid down with: the colour, and how it
/// mixes with what is under it.
#[derive(Clone, Copy)]
struct Paint {
    /// The colour of the line.
    colour: Colour,
    /// How the line mixes with what is under it.
    blend: BlendMode,
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
    /// width. The **corners** and the **ends** are drawn from the
    /// stroke's own join and cap, because that is what they are for.
    fn stroke_contour(&mut self, contour: &[(f64, f64)], stroke: Stroke, blend: BlendMode) {
        if contour.len() < 2 {
            return;
        }
        let half = stroke.width * self.camera.zoom() / 2.0;
        if half <= 0.0 {
            return;
        }
        let paint = Paint { colour: stroke.colour, blend };
        // A **closed** contour says where it starts again at the end.
        let closed = contour.len() > 2
            && (contour[0].0 - contour[contour.len() - 1].0).abs() <= f64::EPSILON
            && (contour[0].1 - contour[contour.len() - 1].1).abs() <= f64::EPSILON;
        let points: Vec<(f64, f64)> =
            if closed { contour[..contour.len() - 1].to_vec() } else { contour.to_vec() };
        let count = points.len();
        if count < 2 {
            return;
        }
        // The **segments**: the line, widened.
        let segments = if closed { count } else { count - 1 };
        for index in 0..segments {
            let from = points[index];
            let to = points[(index + 1) % count];
            // A **square** end carries the line on past its last point.
            let before = if !closed && index == 0 && stroke.cap == Cap::Square { half } else { 0.0 };
            let after =
                if !closed && index == segments - 1 && stroke.cap == Cap::Square { half } else { 0.0 };
            self.stroke_segment(from, to, half, before, after, paint);
        }
        // The **corners**: the joints between two segments.
        let corners = if closed { count } else { count - 2 };
        for index in 0..corners {
            let corner_at = if closed { index } else { index + 1 };
            let before = points[(corner_at + count - 1) % count];
            let corner = points[corner_at];
            let after = points[(corner_at + 1) % count];
            self.stroke_joint(before, corner, after, half, stroke.join, paint);
        }
        // The **ends**: the caps of an open line.
        if !closed && stroke.cap == Cap::Round {
            self.stroke_cap(points[0], points[1], half, true, paint);
            self.stroke_cap(points[count - 1], points[count - 2], half, false, paint);
        }
    }

    /// Draw one **segment** of a contour: the line, widened.
    fn stroke_segment(
        &mut self,
        from: (f64, f64),
        to: (f64, f64),
        half: f64,
        before: f64,
        after: f64,
        paint: Paint,
    ) {
        let Some((ux, uy)) = unit(from, to) else { return };
        let start = (from.0 - ux * before, from.1 - uy * before);
        let end = (to.0 + ux * after, to.1 + uy * after);
        let nx = -uy * half;
        let ny = ux * half;
        let quad = [
            (start.0 + nx, start.1 + ny),
            (end.0 + nx, end.1 + ny),
            (end.0 - nx, end.1 - ny),
            (start.0 - nx, start.1 - ny),
        ];
        self.fill_polygon(&quad, &Fill::Solid(paint.colour), paint.blend, FillRule::NonZero);
    }

    /// Draw the **corner** where two segments meet.
    ///
    /// The joint fills the gap on the **outside** of the turn: a
    /// pointed corner, a round one or a cut-off one.
    fn stroke_joint(
        &mut self,
        before: (f64, f64),
        corner: (f64, f64),
        after: (f64, f64),
        half: f64,
        join: Join,
        paint: Paint,
    ) {
        let Some((u0x, u0y)) = unit(before, corner) else { return };
        let Some((u1x, u1y)) = unit(corner, after) else { return };
        let cross = u0x * u1y - u0y * u1x;
        if cross.abs() <= f64::EPSILON {
            return;
        }
        // The **outside** of the turn: the side the two edges do not
        // share.
        let sign = if cross > 0.0 { -1.0 } else { 1.0 };
        let o0 = (sign * -u0y, sign * u0x);
        let o1 = (sign * -u1y, sign * u1x);
        let p0 = (corner.0 + o0.0 * half, corner.1 + o0.1 * half);
        let p1 = (corner.0 + o1.0 * half, corner.1 + o1.1 * half);
        match join {
            Join::Bevel => {
                self.fill_polygon(
                    &[p0, p1, corner],
                    &Fill::Solid(paint.colour),
                    paint.blend,
                    FillRule::NonZero,
                );
            }
            Join::Round => {
                // The **arc** on the outside of the turn, from one edge
                // to the other.
                let start = angle_of(p0, corner);
                let finish = angle_of(p1, corner);
                let mut sector = Vec::with_capacity(ARC_STEPS + 2);
                sector.push(corner);
                for step in 0..=ARC_STEPS {
                    let angle = start + shortest(start, finish) * step as f64 / ARC_STEPS as f64;
                    sector.push((corner.0 + angle.cos() * half, corner.1 + angle.sin() * half));
                }
                self.fill_polygon(&sector, &Fill::Solid(paint.colour), paint.blend, FillRule::NonZero);
            }
            Join::Miter => {
                // The two edges are carried on until they meet, and the
                // meeting point is the **miter**. A miter that is too
                // long is a bevel: the limit is four widths.
                let Some(miter) = meet(p0, u0x, u0y, p1, u1x, u1y) else {
                    self.fill_polygon(
                        &[p0, p1, corner],
                        &Fill::Solid(paint.colour),
                        paint.blend,
                        FillRule::NonZero,
                    );
                    return;
                };
                let length = ((miter.0 - corner.0).powi(2) + (miter.1 - corner.1).powi(2)).sqrt();
                if length > MITER_LIMIT * half {
                    self.fill_polygon(
                        &[p0, p1, corner],
                        &Fill::Solid(paint.colour),
                        paint.blend,
                        FillRule::NonZero,
                    );
                } else {
                    self.fill_polygon(
                        &[p0, miter, p1],
                        &Fill::Solid(paint.colour),
                        paint.blend,
                        FillRule::NonZero,
                    );
                }
            }
        }
    }

    /// Draw the **end** of an open line: the round cap.
    ///
    /// The half circle is put on the end, away from the line.
    fn stroke_cap(&mut self, end: (f64, f64), towards: (f64, f64), half: f64, first: bool, paint: Paint) {
        let Some((ux, uy)) = unit(towards, end) else { return };
        // The way the line comes: from the point towards the end, and
        // the cap is behind it.
        let (dx, dy) = if first { (-ux, -uy) } else { (ux, uy) };
        let left = (-dy, dx);
        let right = (dy, -dx);
        let start = angle_of((end.0 + left.0 * half, end.1 + left.1 * half), end);
        let finish = angle_of((end.0 + right.0 * half, end.1 + right.1 * half), end);
        let middle = angle_of((end.0 + dx * half, end.1 + dy * half), end);
        // The half circle is the way round that goes **through** the
        // direction the line comes from.
        let step = if angle_distance(start, middle) + angle_distance(middle, finish) <= std::f64::consts::PI {
            1.0
        } else {
            -1.0
        };
        let mut cap = Vec::with_capacity(ARC_STEPS + 2);
        for index in 0..=ARC_STEPS {
            let angle = start + (finish - start) * step * index as f64 / ARC_STEPS as f64;
            cap.push((end.0 + angle.cos() * half, end.1 + angle.sin() * half));
        }
        self.fill_polygon(&cap, &Fill::Solid(paint.colour), paint.blend, FillRule::NonZero);
    }
}

/// The **unit** vector from one point to another.
fn unit(from: (f64, f64), to: (f64, f64)) -> Option<(f64, f64)> {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f64::EPSILON {
        return None;
    }
    Some((dx / length, dy / length))
}

/// The **angle** of a point about a centre.
fn angle_of(point: (f64, f64), centre: (f64, f64)) -> f64 {
    (point.1 - centre.1).atan2(point.0 - centre.0)
}

/// The **shortest** way round from one angle to another.
fn shortest(from: f64, to: f64) -> f64 {
    let mut difference = to - from;
    while difference > std::f64::consts::PI {
        difference -= std::f64::consts::TAU;
    }
    while difference < -std::f64::consts::PI {
        difference += std::f64::consts::TAU;
    }
    difference
}

/// The **distance** between two angles, the short way round.
fn angle_distance(from: f64, to: f64) -> f64 {
    shortest(from, to).abs()
}

/// Where two lines **meet**: the first through `from` along `u`, the
/// second through `to` along `v`.
fn meet(from: (f64, f64), ux: f64, uy: f64, to: (f64, f64), vx: f64, vy: f64) -> Option<(f64, f64)> {
    let determinant = ux * vy - uy * vx;
    if determinant.abs() <= f64::EPSILON {
        return None;
    }
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    let along = (dx * vy - dy * vx) / determinant;
    Some((from.0 + ux * along, from.1 + uy * along))
}

/// A point on a cubic curve.
fn cubic(from: Point, c1: Point, c2: Point, to: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let x = u * u * u * from.x + 3.0 * u * u * t * c1.x + 3.0 * u * t * t * c2.x + t * t * t * to.x;
    let y = u * u * u * from.y + 3.0 * u * u * t * c1.y + 3.0 * u * t * t * c2.y + t * t * t * to.y;
    Point::new(x, y)
}
