//! `craftkit-vector-geom` — path **operations**.
//!
//! # Why this crate exists
//!
//! The SDK owns the **shape** of a path: [`Path`], [`SubPath`],
//! [`Segment`], [`Point`]. That is the shape a file carries and a canvas
//! draws, and it belongs to the core, because everything has to agree on
//! it.
//!
//! The **operations** on a shape — union, difference, intersection,
//! offset, simplification, area — are a different thing. They are not a
//! seam: a program without them is not smaller, it is the same program
//! without a feature. And they are not small: a boolean operation over
//! bezier paths is a sweep line and an arrangement and a curve refit.
//!
//! So they live here: a **library**, not a plugin. It contributes nothing
//! to the registry, it is not wired in by the linker, and a plugin that
//! wants it depends on it the way it depends on the SDK.
//!
//! # The rule this changes
//!
//! The rule was: *a plugin depends on the SDK and the macro, and on
//! nothing else.* That rule is about **plugins** — the plugins are the
//! application's, and the application names no symbol of them. A **library**
//! is not a plugin: it adds no command, no kind, no tool and no panel, and
//! the linker does not know about it. The rule now reads:
//!
//! > A plugin depends on the SDK, the macro, and any library it needs. A
//! > library is not a plugin: it contributes nothing, and the linker does
//! > not wire it in.
//!
//! # The types
//!
//! Everything here speaks the SDK's types. A file never carries a
//! `kurbo` type, and the SDK never depends on `kurbo`: the conversion
//! happens here, at the edge, and both directions are tested.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

use craftkit_sdk::{Cap, FillRule, FormatValue, Join, Path, Point, Rect, Segment, SubPath};
use kurbo::{BezPath, PathEl, Shape};
use linesweeper::BinaryOp as SweepOp;

/// A **transform**: the six numbers of an affine matrix.
///
/// The matrix is the one a PDF and a PostScript file use, and the six
/// numbers stand for
///
/// ```text
/// | a  b  0 |
/// | c  d  0 |
/// | e  f  1 |
/// ```
///
/// so a point `(x, y)` moves to `(a·x + c·y + e, b·x + d·y + f)`. A
/// matrix of `1 0 0 1 0 0` is the one that moves nothing, and it is
/// what a path has when nothing says otherwise.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Matrix {
    /// The first row's first number.
    pub a: f64,
    /// The first row's second number.
    pub b: f64,
    /// The second row's first number.
    pub c: f64,
    /// The second row's second number.
    pub d: f64,
    /// The third row's first number: the move across.
    pub e: f64,
    /// The third row's second number: the move up.
    pub f: f64,
}

impl Matrix {
    /// The matrix that **moves nothing**.
    pub const fn identity() -> Matrix {
        Matrix { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 }
    }

    /// Whether the matrix is the one that **moves nothing**.
    pub fn is_identity(&self) -> bool {
        *self == Matrix::identity()
    }

    /// Where the matrix puts a **point**.
    pub fn apply(&self, point: Point) -> Point {
        Point::new(self.a * point.x + self.c * point.y + self.e, self.b * point.x + self.d * point.y + self.f)
    }

    /// The **determinant** of the matrix: how much it scales the area.
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// The matrix that **moves** a point.
    pub const fn translate(dx: f64, dy: f64) -> Matrix {
        Matrix { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: dx, f: dy }
    }

    /// The matrix that **turns** a point about a centre.
    ///
    /// The angle is in radians, and it turns the way the world's up is
    /// the picture's down: counter-clockwise on the page.
    pub fn rotate_about(centre: Point, angle: f64) -> Matrix {
        let (sin, cos) = angle.sin_cos();
        Matrix {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            e: centre.x - centre.x * cos + centre.y * sin,
            f: centre.y - centre.x * sin - centre.y * cos,
        }
    }

    /// The matrix that **mirrors** a point about a line.
    ///
    /// A line with no length mirrors nothing, and this is `None`.
    pub fn mirror(from: Point, to: Point) -> Option<Matrix> {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let length = dx * dx + dy * dy;
        if !length.is_finite() || length <= f64::EPSILON {
            return None;
        }
        let (ux, uy) = (dx / length.sqrt(), dy / length.sqrt());
        // The reflection about the line through `from` with the
        // direction (ux, uy): the point is turned about the line, and
        // the line's own place is kept.
        let reflect = Matrix {
            a: 2.0 * ux * ux - 1.0,
            b: 2.0 * ux * uy,
            c: 2.0 * ux * uy,
            d: 2.0 * uy * uy - 1.0,
            e: 0.0,
            f: 0.0,
        };
        // The point is taken **back** to the line, turned about it, and
        // put **back** where it was: the order matters, and the other
        // order mirrors about the line through the *negated* point.
        Some(Matrix::translate(-from.x, -from.y).then(&reflect).then(&Matrix::translate(from.x, from.y)))
    }

    /// The matrix that does this one and **then** the other.
    ///
    /// The two are done in the order they are written: this one first,
    /// and the other one after it.
    pub fn then(&self, other: &Matrix) -> Matrix {
        Matrix {
            a: other.a * self.a + other.c * self.b,
            b: other.b * self.a + other.d * self.b,
            c: other.a * self.c + other.c * self.d,
            d: other.b * self.c + other.d * self.d,
            e: other.a * self.e + other.c * self.f + other.e,
            f: other.b * self.e + other.d * self.f + other.f,
        }
    }

    /// The matrix that **undoes** this one.
    ///
    /// A matrix that collapses the plane onto a line has no undoing: it
    /// is `None`, and a path that is placed with such a matrix cannot
    /// be hit and cannot be picked out.
    pub fn inverse(&self) -> Option<Matrix> {
        let determinant = self.determinant();
        if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
            return None;
        }
        Some(Matrix {
            a: self.d / determinant,
            b: -self.b / determinant,
            c: -self.c / determinant,
            d: self.a / determinant,
            e: (self.c * self.f - self.d * self.e) / determinant,
            f: (self.b * self.e - self.a * self.f) / determinant,
        })
    }

    /// The **path** the matrix puts somewhere else.
    pub fn transform_path(&self, path: &Path) -> Path {
        let subpaths = path
            .subpaths
            .iter()
            .map(|subpath| SubPath {
                start: self.apply(subpath.start),
                segments: subpath
                    .segments
                    .iter()
                    .map(|segment| match segment {
                        Segment::Line { to } => Segment::Line { to: self.apply(*to) },
                        Segment::Cubic { c1, c2, to } => {
                            Segment::Cubic { c1: self.apply(*c1), c2: self.apply(*c2), to: self.apply(*to) }
                        }
                    })
                    .collect(),
                closed: subpath.closed,
            })
            .collect();
        Path { subpaths, rule: path.rule }
    }
}

impl Default for Matrix {
    fn default() -> Self {
        Matrix::identity()
    }
}

impl FormatValue for Matrix {
    fn to_json(&self) -> craftkit_sdk::serde_json::Value {
        craftkit_sdk::serde_json::json!([self.a, self.b, self.c, self.d, self.e, self.f])
    }

    fn from_json(value: &craftkit_sdk::serde_json::Value) -> Result<Self, String> {
        let list = value.as_array().ok_or_else(|| "a transform is six numbers".to_string())?;
        if list.len() != 6 {
            return Err(format!("a transform is six numbers, not {}", list.len()));
        }
        let mut numbers = [0.0_f64; 6];
        for (index, item) in list.iter().enumerate() {
            let number = item.as_f64().ok_or_else(|| "a transform is six numbers".to_string())?;
            if !number.is_finite() {
                return Err("a transform's numbers are finite".to_string());
            }
            numbers[index] = number;
        }
        Ok(Matrix {
            a: numbers[0],
            b: numbers[1],
            c: numbers[2],
            d: numbers[3],
            e: numbers[4],
            f: numbers[5],
        })
    }
}

/// The **outline** of a line: the shape a stroke covers.
///
/// A stroke is a line with a width, and the area it covers is a shape
/// in its own right: the path of a line, widened to the width and
/// closed at its ends. This is that shape, as a path — so that it may
/// be filled, and measured, and cut like any other.
///
/// The **width** is the width of the line and not its radius: a line
/// of width four covers two on each side of the path.
pub fn outline_stroke(path: &Path, width: f64, cap: Cap, join: Join, miter_limit: f64) -> Path {
    if !width.is_finite() || width <= 0.0 {
        return Path { subpaths: Vec::new(), rule: FillRule::NonZero };
    }
    let style = kurbo::Stroke {
        width,
        join: match join {
            Join::Miter => kurbo::Join::Miter,
            Join::Round => kurbo::Join::Round,
            Join::Bevel => kurbo::Join::Bevel,
        },
        miter_limit,
        start_cap: cap_of(cap),
        end_cap: cap_of(cap),
        dash_pattern: kurbo::Dashes::new(),
        dash_offset: 0.0,
    };
    let opts = kurbo::StrokeOpts::default();
    let outline = kurbo::stroke(to_bezpath(path), &style, &opts, STROKE_TOLERANCE);
    from_bezpath(&outline)
}

/// The **cap** of the kit, as the library's own.
fn cap_of(cap: Cap) -> kurbo::Cap {
    match cap {
        Cap::Butt => kurbo::Cap::Butt,
        Cap::Round => kurbo::Cap::Round,
        Cap::Square => kurbo::Cap::Square,
    }
}

/// How close the outline's curves are to the true ones.
const STROKE_TOLERANCE: f64 = 0.01;

/// Which boolean operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoolOp {
    /// Everything in either shape.
    Union,
    /// What is in both.
    Intersect,
    /// What is in the first and not in the second.
    Difference,
    /// What is in one or the other, but not both.
    Xor,
}

/// Why an operation could not be done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GeomError {
    /// The path has nothing in it.
    Empty,
    /// The path holds a number that is not finite.
    NotFinite,
    /// The operation itself failed.
    Operation(String),
}

impl std::fmt::Display for GeomError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GeomError::Empty => write!(formatter, "the path is empty"),
            GeomError::NotFinite => write!(formatter, "the path has a number that is not finite"),
            GeomError::Operation(reason) => write!(formatter, "the operation failed: {reason}"),
        }
    }
}

impl std::error::Error for GeomError {}

/// A path as the geometry library's own type.
///
/// The conversion is at the edge: everything inside this crate speaks
/// `kurbo`, and everything outside speaks the SDK.
pub fn to_bezpath(path: &Path) -> BezPath {
    let mut bez = BezPath::new();
    for subpath in &path.subpaths {
        if subpath.segments.is_empty() {
            continue;
        }
        bez.move_to((subpath.start.x, subpath.start.y));
        for segment in &subpath.segments {
            match segment {
                Segment::Line { to } => bez.line_to((to.x, to.y)),
                Segment::Cubic { c1, c2, to } => bez.curve_to((c1.x, c1.y), (c2.x, c2.y), (to.x, to.y)),
            }
        }
        if subpath.closed {
            bez.close_path();
        }
    }
    bez
}

/// The geometry library's path as the SDK's.
pub fn from_bezpath(bez: &BezPath) -> Path {
    let mut subpaths: Vec<SubPath> = Vec::new();
    let mut current: Option<SubPath> = None;
    for element in bez.elements() {
        match element {
            PathEl::MoveTo(point) => {
                if let Some(subpath) = current.take() {
                    subpaths.push(subpath);
                }
                current = Some(SubPath {
                    start: Point::new(point.x, point.y),
                    segments: Vec::new(),
                    closed: false,
                });
            }
            PathEl::LineTo(point) => {
                if let Some(subpath) = current.as_mut() {
                    subpath.segments.push(Segment::Line { to: Point::new(point.x, point.y) });
                }
            }
            PathEl::CurveTo(c1, c2, to) => {
                if let Some(subpath) = current.as_mut() {
                    subpath.segments.push(Segment::Cubic {
                        c1: Point::new(c1.x, c1.y),
                        c2: Point::new(c2.x, c2.y),
                        to: Point::new(to.x, to.y),
                    });
                }
            }
            PathEl::QuadTo(control, to) => {
                if let Some(subpath) = current.as_mut() {
                    // A quadratic becomes a cubic: the control point is put
                    // at two thirds of the way, and the curve is the same.
                    let from = subpath.end();
                    let c1 = Point::new(
                        from.x + (control.x - from.x) * (2.0 / 3.0),
                        from.y + (control.y - from.y) * (2.0 / 3.0),
                    );
                    let c2 = Point::new(
                        to.x + (control.x - to.x) * (2.0 / 3.0),
                        to.y + (control.y - to.y) * (2.0 / 3.0),
                    );
                    subpath.segments.push(Segment::Cubic { c1, c2, to: Point::new(to.x, to.y) });
                }
            }
            PathEl::ClosePath => {
                if let Some(subpath) = current.as_mut() {
                    subpath.closed = true;
                }
            }
        }
    }
    if let Some(subpath) = current.take() {
        subpaths.push(subpath);
    }
    Path { subpaths, rule: FillRule::default() }
}

/// A contour with **one more anchor** in it: a segment split in two.
///
/// The split is at `t` along the segment — zero at its beginning and
/// one at its end — and the two halves are the **same curve**: the
/// point at `t` is the point the original curve had at `t`, and the
/// shape of the curve does not change.
///
/// The segment that was split is the one at `index`, and the new
/// anchor is the point where the two halves meet.
pub fn split_segment(subpath: &SubPath, index: usize, t: f64) -> Result<SubPath, GeomError> {
    let Some(segment) = subpath.segments.get(index) else {
        return Err(GeomError::Operation(format!("the contour has no segment {index}")));
    };
    if !t.is_finite() {
        return Err(GeomError::NotFinite);
    }
    let t = t.clamp(0.0, 1.0);
    let from = if index == 0 { subpath.start } else { subpath.segments[index - 1].end() };
    let (left, right) = match segment {
        Segment::Line { to } => {
            let mid = Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t);
            (Segment::Line { to: mid }, Segment::Line { to: *to })
        }
        Segment::Cubic { c1, c2, to } => {
            // **De Casteljau**: the curve is cut by halving it over
            // and over, and the halves are the same curve.
            let lerp = |a: Point, b: Point| Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
            let q0 = lerp(from, *c1);
            let q1 = lerp(*c1, *c2);
            let q2 = lerp(*c2, *to);
            let r0 = lerp(q0, q1);
            let r1 = lerp(q1, q2);
            let mid = lerp(r0, r1);
            (Segment::Cubic { c1: q0, c2: r0, to: mid }, Segment::Cubic { c1: r1, c2: q2, to: *to })
        }
    };
    let mut segments = subpath.segments.clone();
    segments.splice(index..=index, [left, right]);
    Ok(SubPath { start: subpath.start, segments, closed: subpath.closed })
}

/// A path with the **degenerate** sides taken away, and its contours
/// closed as they should be.
///
/// A side that ends where it begins is no side: it is a point, and a
/// boolean cannot take a point. The outline of a stroke has such a
/// side where its cap closes, and it is taken away here — the side
/// that follows it begins in the same place, so the shape does not
/// change.
///
/// The outline of a stroke also comes back **open**, although its last
/// side ends where it begins: a contour that closes on itself is a
/// closed contour, and it is closed here.
pub fn without_degenerate(path: &Path) -> Path {
    // The **tolerance**: two points that are the same place.
    let scale = path
        .subpaths
        .iter()
        .flat_map(|subpath| subpath.segments.iter().map(|segment| segment.end()))
        .chain(path.subpaths.iter().map(|subpath| subpath.start))
        .map(|point| point.x.abs().max(point.y.abs()))
        .fold(1.0_f64, |held, value| held.max(value));
    let same = 1e-9 * scale;
    let is_same = |one: Point, two: Point| (one.x - two.x).abs() <= same && (one.y - two.y).abs() <= same;
    let mut subpaths: Vec<SubPath> = Vec::with_capacity(path.subpaths.len());
    for subpath in &path.subpaths {
        // A contour whose **last side** ends where it begins is a
        // closed contour: the outline of a stroke is one, and the
        // conversion does not say so.
        let closes = !subpath.closed
            && !subpath.segments.is_empty()
            && is_same(subpath.segments[subpath.segments.len() - 1].end(), subpath.start);
        let closed = subpath.closed || closes;
        let mut segments: Vec<Segment> = Vec::with_capacity(subpath.segments.len());
        for (index, segment) in subpath.segments.iter().enumerate() {
            let from = if index == 0 { subpath.start } else { subpath.segments[index - 1].end() };
            let degenerate = is_same(from, segment.end());
            if degenerate {
                // The side that follows begins in the same place, so
                // the contour does not change; and the side before it
                // may already end where the contour starts.
                let follows = index + 1 < subpath.segments.len();
                let closes_already =
                    index > 0 && closed && is_same(subpath.segments[index - 1].end(), subpath.start);
                if follows || closes_already {
                    continue;
                }
            }
            segments.push(*segment);
        }
        subpaths.push(SubPath { start: subpath.start, segments, closed });
    }
    Path { subpaths, rule: path.rule }
}

/// The boolean operation of two paths.
///
/// The result may have more than one contour: subtracting a shape from the
/// middle of another leaves a shape with a hole, and the hole is a
/// contour. That is why a path holds contours and a rule (§ the SDK's
/// `Path`).
pub fn boolean(first: &Path, second: &Path, op: BoolOp) -> Result<Path, GeomError> {
    if first.is_empty() || second.is_empty() {
        return Err(GeomError::Empty);
    }
    let a = to_bezpath(first);
    let b = to_bezpath(second);
    if !a.is_finite() || !b.is_finite() {
        return Err(GeomError::NotFinite);
    }
    let rule = match first.rule {
        FillRule::NonZero => linesweeper::FillRule::NonZero,
        FillRule::EvenOdd => linesweeper::FillRule::EvenOdd,
    };
    let sweep = match op {
        BoolOp::Union => SweepOp::Union,
        BoolOp::Intersect => SweepOp::Intersection,
        BoolOp::Difference => SweepOp::Difference,
        BoolOp::Xor => SweepOp::Xor,
    };
    let contours = linesweeper::binary_op(&a, &b, rule, sweep)
        .map_err(|error| GeomError::Operation(format!("{error:?}")))?;
    let mut subpaths = Vec::new();
    for contour in contours.contours() {
        let subpath = contour_subpath(&contour.path);
        if !subpath.segments.is_empty() {
            subpaths.push(subpath);
        }
    }
    Ok(Path { subpaths, rule: first.rule })
}

/// One closed contour as the SDK's subpath.
fn contour_subpath(bez: &BezPath) -> SubPath {
    let converted = from_bezpath(bez);
    let mut subpath = converted.subpaths.into_iter().next().unwrap_or_default();
    subpath.closed = true;
    subpath
}

/// The area a path covers.
///
/// The area is computed from the curves themselves, not from a flattening
/// of them: the integrand is a polynomial, and three-point Gauss-Legendre
/// integrates it exactly.
pub fn area(path: &Path) -> f64 {
    let mut total = 0.0;
    for subpath in &path.subpaths {
        if !subpath.closed {
            continue;
        }
        total += contour_area(subpath);
    }
    total.abs()
}

/// The contour's segments, with the closing one when the contour is closed.
///
/// A closed contour's last segment joins back to its start, and that edge
/// is part of the shape: it is where the outline goes home.
fn contour_segments(subpath: &SubPath) -> Vec<(Point, Segment)> {
    let mut segments: Vec<(Point, Segment)> = Vec::with_capacity(subpath.segments.len() + 1);
    let mut from = subpath.start;
    for segment in &subpath.segments {
        segments.push((from, *segment));
        from = segment.end();
    }
    if subpath.closed && !subpath.segments.is_empty() {
        let start = subpath.start;
        if from != start {
            segments.push((from, Segment::Line { to: start }));
        }
    }
    segments
}

/// The signed area of one closed contour.
fn contour_area(subpath: &SubPath) -> f64 {
    // Three-point Gauss-Legendre on [0, 1].
    const NODES: [f64; 3] = [-0.7745966692414834, 0.0, 0.7745966692414834];
    const WEIGHTS: [f64; 3] = [0.5555555555555556, 0.8888888888888888, 0.5555555555555556];
    let mut total = 0.0;
    for (from, segment) in contour_segments(subpath) {
        let (c1, c2, to) = match segment {
            Segment::Line { to } => {
                let c1 = lerp(from, to, 1.0 / 3.0);
                let c2 = lerp(from, to, 2.0 / 3.0);
                (c1, c2, to)
            }
            Segment::Cubic { c1, c2, to } => (c1, c2, to),
        };
        let mut sum = 0.0;
        for (node, weight) in NODES.iter().zip(WEIGHTS.iter()) {
            let t = (node + 1.0) / 2.0;
            let point = cubic(from, c1, c2, to, t);
            let derivative = cubic_derivative(from, c1, c2, to, t);
            sum += weight * (point.x * derivative.y - point.y * derivative.x);
        }
        // The nodes are on [-1, 1] and the curve is on [0, 1]: the step is
        // half as long.
        total += sum / 2.0;
    }
    total / 2.0
}

/// The point a third of the way along a line.
fn lerp(from: Point, to: Point, t: f64) -> Point {
    Point::new(from.x + (to.x - from.x) * t, from.y + (to.y - from.y) * t)
}

/// A point on a cubic curve.
fn cubic(from: Point, c1: Point, c2: Point, to: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let a = u * u * u;
    let b = 3.0 * u * u * t;
    let c = 3.0 * u * t * t;
    let d = t * t * t;
    Point::new(a * from.x + b * c1.x + c * c2.x + d * to.x, a * from.y + b * c1.y + c * c2.y + d * to.y)
}

/// How a cubic curve is moving at a point on it.
fn cubic_derivative(from: Point, c1: Point, c2: Point, to: Point, t: f64) -> Point {
    let u = 1.0 - t;
    let a = 3.0 * u * u;
    let b = 6.0 * u * t;
    let c = 3.0 * t * t;
    Point::new(
        a * (c1.x - from.x) + b * (c2.x - c1.x) + c * (to.x - c2.x),
        a * (c1.y - from.y) + b * (c2.y - c1.y) + c * (to.y - c2.y),
    )
}

/// Where a path is.
pub fn bounds(path: &Path) -> Rect {
    let bez = to_bezpath(path);
    if bez.is_empty() {
        return Rect::default();
    }
    let box_ = bez.bounding_box();
    Rect::new(box_.min_x(), box_.min_y(), box_.width(), box_.height())
}

/// A path with fewer points, close enough.
///
/// The curve is kept where the curve is: the simplification removes the
/// points that a flat line would serve, and leaves the rest alone.
pub fn simplify(path: &Path, tolerance: f64) -> Path {
    if !tolerance.is_finite() || tolerance < 0.0 {
        return path.clone();
    }
    let bez = to_bezpath(path);
    let simplified =
        kurbo::simplify::simplify_bezpath(bez.elements().to_vec(), tolerance, &Default::default());
    from_bezpath(&simplified)
}

/// A path pushed out (or in) by a distance.
///
/// The offset is **curve-preserving**: the curves are offset as curves, by
/// the geometry library, and the corners are joined with a straight line.
/// So an offset circle is still a circle.
///
/// Every contour must be closed: an open contour has two sides and no
/// inside, and offsetting it is a different operation.
///
/// An offset of a shape with a narrow neck can meet itself, and the loops
/// it makes are not removed: the operation offsets, it does not clean up.
pub fn offset(path: &Path, distance: f64) -> Result<Path, GeomError> {
    if !distance.is_finite() {
        return Err(GeomError::NotFinite);
    }
    let mut subpaths = Vec::new();
    for subpath in &path.subpaths {
        if !subpath.closed {
            return Err(GeomError::Operation(
                "an offset needs closed contours: an open one has no inside".to_string(),
            ));
        }
        let offset = offset_contour(subpath, distance)?;
        if !offset.segments.is_empty() {
            subpaths.push(offset);
        }
    }
    if subpaths.is_empty() {
        return Err(GeomError::Empty);
    }
    Ok(Path { subpaths, rule: path.rule })
}

/// One closed contour, pushed out (or in).
fn offset_contour(subpath: &SubPath, distance: f64) -> Result<SubPath, GeomError> {
    let segments = contour_segments(subpath);
    if segments.is_empty() {
        return Err(GeomError::Empty);
    }
    // Which way is out? The signed area says whether the inside is to the
    // left of the way the contour runs, and the offset goes the other way.
    let points = flatten(subpath, 0.05);
    let winding = signed_area(&points);
    if winding == 0.0 {
        return Err(GeomError::Empty);
    }
    let delta = if winding > 0.0 { -distance } else { distance };

    // The direction of each segment at its start and at its end.
    let mut starts = Vec::with_capacity(segments.len());
    let mut ends = Vec::with_capacity(segments.len());
    for (from, segment) in &segments {
        let (in_tangent, out_tangent) = tangents(*from, *segment);
        starts.push(in_tangent);
        ends.push(out_tangent);
    }

    let mut out = BezPath::new();
    let count = segments.len();
    for index in 0..count {
        let (from, segment) = segments[index];
        let previous = (index + count - 1) % count;
        // The corner between the previous segment and this one: where the
        // two moved lines meet.
        let corner = Point::new(from.x, from.y);
        let into = left_normal(corner, corner_with(corner, ends[previous]));
        let out_of = left_normal(corner, corner_with(corner, starts[index]));
        let a = Point::new(corner.x + into.x * delta, corner.y + into.y * delta);
        let b = Point::new(corner.x + out_of.x * delta, corner.y + out_of.y * delta);
        let limit = delta.abs() * 4.0 + 1.0;
        let miter = intersect(a, ends[previous], b, starts[index]);
        match miter {
            Some(point) if distance_between(point, corner) <= limit => {
                if index == 0 {
                    out.move_to((point.x, point.y));
                } else {
                    out.line_to((point.x, point.y));
                }
            }
            _ => {
                // A sharp corner: the two ends are joined with a straight
                // line rather than a spike.
                if index == 0 {
                    out.move_to((a.x, a.y));
                } else {
                    out.line_to((a.x, a.y));
                }
                out.line_to((b.x, b.y));
            }
        }
        // The segment itself, moved.
        match segment {
            Segment::Line { to } => {
                let normal = left_normal(from, to);
                let end = Point::new(to.x + normal.x * delta, to.y + normal.y * delta);
                out.line_to((end.x, end.y));
            }
            Segment::Cubic { c1, c2, to } => {
                let cubic = kurbo::CubicBez::new((from.x, from.y), (c1.x, c1.y), (c2.x, c2.y), (to.x, to.y));
                let mut piece = BezPath::new();
                kurbo::offset::offset_cubic(cubic, delta, 0.01, &mut piece);
                for element in piece.elements() {
                    if let PathEl::MoveTo(_) = element {
                        continue;
                    }
                    out.push(*element);
                }
            }
        }
    }
    out.close_path();
    let converted = from_bezpath(&out);
    let mut contour = converted.subpaths.into_iter().next().unwrap_or_default();
    contour.closed = true;
    Ok(contour)
}

/// The direction of a segment at its start and at its end.
fn tangents(from: Point, segment: Segment) -> (Point, Point) {
    match segment {
        Segment::Line { to } => {
            let direction = Point::new(to.x - from.x, to.y - from.y);
            (direction, direction)
        }
        Segment::Cubic { c1, c2, to } => {
            let start = cubic_derivative(from, c1, c2, to, 0.0);
            let end = cubic_derivative(from, c1, c2, to, 1.0);
            (start, end)
        }
    }
}

/// A point a little way along a direction.
fn corner_with(corner: Point, direction: Point) -> Point {
    Point::new(corner.x + direction.x, corner.y + direction.y)
}

/// How far apart two points are.
fn distance_between(one: Point, other: Point) -> f64 {
    ((one.x - other.x).powi(2) + (one.y - other.y).powi(2)).sqrt()
}

/// Where two lines meet.
fn intersect(a: Point, a_direction: Point, b: Point, b_direction: Point) -> Option<Point> {
    let denominator = a_direction.x * b_direction.y - a_direction.y * b_direction.x;
    if denominator.abs() < 1e-12 {
        return None;
    }
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let t = (dx * b_direction.y - dy * b_direction.x) / denominator;
    Some(Point::new(a.x + a_direction.x * t, a.y + a_direction.y * t))
}

/// The normal of an edge, to the left of the way it runs.
fn left_normal(from: Point, to: Point) -> Point {
    let dx = to.x - from.x;
    let dy = to.y - from.y;
    let length = (dx * dx + dy * dy).sqrt();
    if length <= f64::EPSILON {
        return Point::new(0.0, 0.0);
    }
    Point::new(-dy / length, dx / length)
}

/// The signed area of a polygon.
fn signed_area(points: &[Point]) -> f64 {
    let mut total = 0.0;
    let count = points.len();
    for index in 0..count {
        let a = points[index];
        let b = points[(index + 1) % count];
        total += a.x * b.y - b.x * a.y;
    }
    total / 2.0
}

/// A contour as a polygon: its points, with every curve sampled.
pub fn flatten(subpath: &SubPath, tolerance: f64) -> Vec<Point> {
    let steps = curve_steps(tolerance);
    let mut points = Vec::new();
    let mut current = subpath.start;
    points.push(current);
    for segment in &subpath.segments {
        match segment {
            Segment::Line { to } => {
                current = *to;
                points.push(current);
            }
            Segment::Cubic { c1, c2, to } => {
                for step in 1..=steps {
                    let t = step as f64 / steps as f64;
                    points.push(cubic(current, *c1, *c2, *to, t));
                }
                current = *to;
            }
        }
    }
    points
}

/// How many samples a curve is flattened with.
///
/// The more precise the tolerance, the more samples. The number is capped,
/// because a path that arrives from a file can be as large as it likes.
fn curve_steps(tolerance: f64) -> usize {
    if !tolerance.is_finite() || tolerance <= 0.0 {
        return 16;
    }
    let steps = (1.0 / tolerance.sqrt()).ceil() as usize;
    steps.clamp(4, 256)
}

/// A path as polygons: one per contour.
pub fn polygons(path: &Path, tolerance: f64) -> Vec<Vec<Point>> {
    path.subpaths.iter().map(|subpath| flatten(subpath, tolerance)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A matrix mirrors a point about the **line** it is given, and not
    /// about the line through the negated point.
    #[test]
    fn a_mirror_is_about_the_line_it_is_given() {
        let mirror = Matrix::mirror(Point::new(1.0, 0.0), Point::new(1.0, 10.0)).expect("the line");
        let point = mirror.apply(Point::new(0.0, 0.0));
        assert!((point.x - 2.0).abs() < 1e-9, "the point is across the line: {point:?}");
        assert!((point.y - 0.0).abs() < 1e-9, "the point keeps its height: {point:?}");
    }

    /// A matrix mirrors a point about a **horizontal** line too.
    #[test]
    fn a_mirror_is_about_a_horizontal_line() {
        let mirror = Matrix::mirror(Point::new(0.0, 5.0), Point::new(10.0, 5.0)).expect("the line");
        let point = mirror.apply(Point::new(3.0, 1.0));
        assert!((point.x - 3.0).abs() < 1e-9, "the point keeps its place: {point:?}");
        assert!((point.y - 9.0).abs() < 1e-9, "the point is across the line: {point:?}");
    }

    /// A path with a **degenerate** side has it taken away.
    #[test]
    fn a_degenerate_side_is_taken_away() {
        let mut path = square(0.0, 0.0, 10.0);
        // A side that ends where it begins: a point, and not a side.
        let start = path.subpaths[0].start;
        path.subpaths[0].segments.insert(0, Segment::Line { to: start });
        let cleaned = without_degenerate(&path);
        assert_eq!(cleaned.subpaths[0].segments.len(), path.subpaths[0].segments.len() - 1);
    }

    /// A square, as a path.
    fn square(x: f64, y: f64, size: f64) -> Path {
        Path::polygon(&[
            Point::new(x, y),
            Point::new(x + size, y),
            Point::new(x + size, y + size),
            Point::new(x, y + size),
        ])
    }

    /// A circle, as four curves.
    fn circle(x: f64, y: f64, radius: f64) -> Path {
        const KAPPA: f64 = 0.5522847498307936;
        let start = Point::new(x + radius, y);
        let segments = vec![
            Segment::Cubic {
                c1: Point::new(x + radius, y + KAPPA * radius),
                c2: Point::new(x + KAPPA * radius, y + radius),
                to: Point::new(x, y + radius),
            },
            Segment::Cubic {
                c1: Point::new(x - KAPPA * radius, y + radius),
                c2: Point::new(x - radius, y + KAPPA * radius),
                to: Point::new(x - radius, y),
            },
            Segment::Cubic {
                c1: Point::new(x - radius, y - KAPPA * radius),
                c2: Point::new(x - KAPPA * radius, y - radius),
                to: Point::new(x, y - radius),
            },
            Segment::Cubic {
                c1: Point::new(x + KAPPA * radius, y - radius),
                c2: Point::new(x + radius, y - KAPPA * radius),
                to: Point::new(x + radius, y),
            },
        ];
        Path::single(SubPath { start, segments, closed: true })
    }

    /// The area of a path is the area it covers.
    #[test]
    fn the_area_of_a_square_is_its_area() {
        assert!((area(&square(0.0, 0.0, 10.0)) - 100.0).abs() < 1e-9);
    }

    /// The area of a curve is the area under the curve, not a sample of it.
    #[test]
    fn the_area_of_a_circle_is_the_area_of_a_circle() {
        let radius = 10.0;
        let measured = area(&circle(0.0, 0.0, radius));
        let exact = std::f64::consts::PI * radius * radius;
        assert!((measured - exact).abs() / exact < 0.001, "the area is {measured}, not {exact}");
    }

    /// A path survives the trip through the geometry library's own type.
    #[test]
    fn a_path_survives_the_conversion() {
        let path = square(1.0, 2.0, 3.0);
        let back = from_bezpath(&to_bezpath(&path));
        assert_eq!(back.subpaths.len(), 1);
        assert_eq!(back.subpaths[0].points(), path.subpaths[0].points());
        assert!(back.subpaths[0].closed);
    }

    /// The union of two overlapping squares is what both of them cover.
    #[test]
    fn the_union_of_two_squares_is_the_area_of_both() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(5.0, 0.0, 10.0);
        let union = boolean(&a, &b, BoolOp::Union).expect("the union is computed");
        assert!((area(&union) - 150.0).abs() < 0.01, "the area is {}", area(&union));
    }

    /// The difference of two overlapping squares is what only the first covers.
    #[test]
    fn the_difference_of_two_squares_is_what_only_the_first_covers() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(5.0, 0.0, 10.0);
        let difference = boolean(&a, &b, BoolOp::Difference).expect("the difference is computed");
        assert!((area(&difference) - 50.0).abs() < 0.01, "the area is {}", area(&difference));
    }

    /// The intersection of two overlapping squares is what both cover.
    #[test]
    fn the_intersection_of_two_squares_is_what_both_cover() {
        let a = square(0.0, 0.0, 10.0);
        let b = square(5.0, 0.0, 10.0);
        let intersection = boolean(&a, &b, BoolOp::Intersect).expect("the intersection is computed");
        assert!((area(&intersection) - 50.0).abs() < 0.01, "the area is {}", area(&intersection));
    }

    /// A shape with a hole is more than one contour.
    ///
    /// This is the thing the flat path could not hold, and the reason a
    /// path has contours and a rule.
    #[test]
    fn a_shape_with_a_hole_has_more_than_one_contour() {
        let outer = square(0.0, 0.0, 10.0);
        let inner = square(3.0, 3.0, 4.0);
        let difference = boolean(&outer, &inner, BoolOp::Difference).expect("the difference is computed");
        assert!(difference.subpaths.len() > 1, "the shape has {} contour(s)", difference.subpaths.len());
        let measured = area(&difference);
        assert!((measured - 84.0).abs() < 0.05, "the area is {measured}, not 84");
    }

    /// An empty path is refused, rather than producing an empty result.
    #[test]
    fn an_empty_path_is_refused() {
        let empty = Path::default();
        let error = boolean(&empty, &square(0.0, 0.0, 1.0), BoolOp::Union).expect_err("it is refused");
        assert_eq!(error, GeomError::Empty);
    }

    /// A path with a number that is not finite is refused.
    #[test]
    fn a_path_that_is_not_finite_is_refused() {
        let mut path = square(0.0, 0.0, 1.0);
        path.subpaths[0].start = Point::new(f64::NAN, 0.0);
        let error = boolean(&path, &square(0.0, 0.0, 1.0), BoolOp::Union).expect_err("it is refused");
        assert_eq!(error, GeomError::NotFinite);
    }

    /// Where a path is, is where its points are.
    #[test]
    fn the_bounds_of_a_path_are_its_points() {
        let bounds = bounds(&square(2.0, 3.0, 4.0));
        assert_eq!(bounds, Rect::new(2.0, 3.0, 4.0, 4.0));
    }

    /// A square pushed out by one is a bigger square.
    #[test]
    fn an_offset_square_is_a_bigger_square() {
        let offset = offset(&square(0.0, 0.0, 10.0), 1.0).expect("the offset is computed");
        let measured = area(&offset);
        assert!((measured - 144.0).abs() < 0.5, "the area is {measured}, not 144");
    }

    /// A circle pushed out is still a circle.
    #[test]
    fn an_offset_circle_is_still_a_circle() {
        let offset = offset(&circle(0.0, 0.0, 10.0), 2.0).expect("the offset is computed");
        let measured = area(&offset);
        let exact = std::f64::consts::PI * 12.0 * 12.0;
        assert!((measured - exact).abs() / exact < 0.02, "the area is {measured}, not {exact}");
    }

    /// An open contour has no inside, so it cannot be offset.
    #[test]
    fn an_open_contour_cannot_be_offset() {
        let open = Path::single(SubPath {
            start: Point::new(0.0, 0.0),
            segments: vec![Segment::Line { to: Point::new(10.0, 0.0) }],
            closed: false,
        });
        let error = offset(&open, 1.0).expect_err("it is refused");
        assert!(matches!(error, GeomError::Operation(_)), "{error:?}");
    }

    /// A path with a point that is not finite is refused.
    #[test]
    fn an_offset_that_is_not_finite_is_refused() {
        let error = offset(&square(0.0, 0.0, 1.0), f64::NAN).expect_err("it is refused");
        assert_eq!(error, GeomError::NotFinite);
    }

    /// Simplifying a path with a point in the middle of a line removes it.
    #[test]
    fn simplifying_removes_a_point_that_is_not_needed() {
        let path = Path::single(SubPath {
            start: Point::new(0.0, 0.0),
            segments: vec![
                Segment::Line { to: Point::new(5.0, 0.0) },
                Segment::Line { to: Point::new(10.0, 0.0) },
            ],
            closed: true,
        });
        let simplified = simplify(&path, 0.01);
        assert!(simplified.subpaths[0].segments.len() < path.subpaths[0].segments.len());
    }
}
