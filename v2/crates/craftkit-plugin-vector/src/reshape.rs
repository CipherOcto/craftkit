//! The **reshape** of a path.
//!
//! The reshape tool grabs the path at a point and drags it, and the
//! anchors around the grab follow with a **smooth falloff** over a
//! radius, and the radius is measured **along** the path — not in a
//! straight line from the grab, but along the curve the path is.
//!
//! The falloff is the tree's: the weight of an anchor is
//! `(1 - q²)²`, where `q` is the anchor's distance along the path from
//! the grabbed one over the radius, and `q` is at most one. So the
//! grabbed anchor moves the whole way, the anchor at the radius does
//! not move at all, and the weight falls smoothly in between.
//!
//! The grab may be on a point that is **not** an anchor: the path is
//! grabbed where it is nearest to the point, and an anchor is added
//! there — unless an anchor is already within the tolerance.

use craftkit_sdk::{Path, Point, Segment, SubPath};

use craftkit_vector_geom as geom;

/// How many parts a curve is cut into when it is measured.
const SAMPLES: usize = 64;

/// The **nearest point** on a path: which contour, which segment, where
/// on the segment, and how far away it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nearest {
    /// The contour the point is on.
    pub contour: usize,
    /// The segment of the contour.
    pub segment: usize,
    /// Where on the segment: zero at its beginning and one at its end.
    pub t: f64,
    /// How far the point is from the grab.
    pub distance: f64,
}

/// The nearest point of a path to a point.
pub fn nearest(path: &Path, at: Point) -> Option<Nearest> {
    let mut best: Option<Nearest> = None;
    for (contour, subpath) in path.subpaths.iter().enumerate() {
        for (segment, segment_value) in subpath.segments.iter().enumerate() {
            let from = if segment == 0 { subpath.start } else { subpath.segments[segment - 1].end() };
            let (t, distance) = nearest_on_segment(from, segment_value, at);
            let better = match best {
                Some(current) => distance < current.distance,
                None => true,
            };
            if better {
                best = Some(Nearest { contour, segment, t, distance });
            }
        }
    }
    best
}

/// The nearest point of a **segment** to a point, and where on the
/// segment it is.
///
/// A straight line is measured exactly: the point is projected onto it.
/// A curve is sampled and then refined, because the nearest point of a
/// curve is not one of its samples.
fn nearest_on_segment(from: Point, segment: &Segment, at: Point) -> (f64, f64) {
    match segment {
        Segment::Line { to } => {
            let dx = to.x - from.x;
            let dy = to.y - from.y;
            let length = dx * dx + dy * dy;
            if length <= f64::EPSILON {
                return (0.0, distance(from, at));
            }
            let t = (((at.x - from.x) * dx + (at.y - from.y) * dy) / length).clamp(0.0, 1.0);
            let point = Point::new(from.x + dx * t, from.y + dy * t);
            (t, distance(point, at))
        }
        Segment::Cubic { c1, c2, to } => {
            // The curve is sampled, and the best sample is then
            // **refined**: the nearest point of a curve is between two
            // samples, and the distance there is smaller.
            let mut best_t = 0.0;
            let mut best = distance(from, at);
            for step in 1..=SAMPLES {
                let t = step as f64 / SAMPLES as f64;
                let here = distance(cubic_at(from, *c1, *c2, *to, t), at);
                if here < best {
                    best = here;
                    best_t = t;
                }
            }
            let step = 1.0 / SAMPLES as f64;
            let mut low = (best_t - step).max(0.0);
            let mut high = (best_t + step).min(1.0);
            for _ in 0..24 {
                let middle = (low + high) / 2.0;
                let left = (low + middle) / 2.0;
                let right = (middle + high) / 2.0;
                let here = distance(cubic_at(from, *c1, *c2, *to, left), at);
                let there = distance(cubic_at(from, *c1, *c2, *to, right), at);
                if here <= there {
                    high = middle;
                } else {
                    low = middle;
                }
            }
            let t = (low + high) / 2.0;
            (t, distance(cubic_at(from, *c1, *c2, *to, t), at))
        }
    }
}

/// The point of a **cubic** at `t`.
fn cubic_at(from: Point, c1: Point, c2: Point, to: Point, t: f64) -> Point {
    let rest = 1.0 - t;
    let a = rest * rest * rest;
    let b = 3.0 * rest * rest * t;
    let c = 3.0 * rest * t * t;
    let d = t * t * t;
    Point::new(a * from.x + b * c1.x + c * c2.x + d * to.x, a * from.y + b * c1.y + c * c2.y + d * to.y)
}

/// How far two points are.
fn distance(from: Point, to: Point) -> f64 {
    ((from.x - to.x).powi(2) + (from.y - to.y).powi(2)).sqrt()
}

/// The **anchors** of a contour: its points, without the last one when
/// it is the first one again.
pub fn anchors(subpath: &SubPath) -> Vec<Point> {
    let mut points = subpath.points();
    if subpath.closed
        && points.len() >= 2
        && let Some(last) = points.last()
        && (last.x - points[0].x).abs() < f64::EPSILON
        && (last.y - points[0].y).abs() < f64::EPSILON
    {
        points.pop();
    }
    points
}

/// The **length** of a segment, measured along it.
fn segment_length(from: Point, segment: &Segment) -> f64 {
    match segment {
        Segment::Line { to } => distance(from, *to),
        Segment::Cubic { c1, c2, to } => {
            let mut total = 0.0;
            let mut last = from;
            for step in 1..=SAMPLES {
                let here = cubic_at(from, *c1, *c2, *to, step as f64 / SAMPLES as f64);
                total += distance(last, here);
                last = here;
            }
            total
        }
    }
}

/// The **positions** of the anchors along the contour, and the length of
/// the contour.
///
/// The positions are the arc lengths: how far along the path each
/// anchor is. For a **closed** contour the closing edge is part of the
/// length and not of the positions — the last anchor is where the
/// closing edge begins.
pub fn positions(subpath: &SubPath) -> (Vec<f64>, f64) {
    let mut positions: Vec<f64> = vec![0.0];
    let mut total = 0.0;
    for (index, segment) in subpath.segments.iter().enumerate() {
        let from = if index == 0 { subpath.start } else { subpath.segments[index - 1].end() };
        total += segment_length(from, segment);
        positions.push(total);
    }
    // The **closing edge** of a closed contour that leaves it out: the
    // straight line from the last anchor back to the first.
    if subpath.closed {
        let anchors = anchors(subpath);
        if let (Some(first), Some(last)) = (anchors.first(), anchors.last())
            && anchors.len() > 1
        {
            total += distance(*last, *first);
        }
    }
    (positions, total)
}

/// The **size** of a contour: the length of its diagonal.
///
/// The tree's rule: the default reach of the reshape is half the size
/// of the contour, and it is never less than one.
pub fn size(subpath: &SubPath) -> f64 {
    let flat = geom::flatten(subpath, 0.01);
    if flat.is_empty() {
        return 1.0;
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in &flat {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    let diagonal = (max_x - min_x).hypot(max_y - min_y);
    (diagonal / 2.0).max(1.0)
}

/// The **weight** of an anchor that is `distance` along the path from
/// the grabbed one, when the radius is `radius`.
///
/// The weight is one at the grab and zero at the radius, and it falls
/// smoothly in between.
pub fn weight(distance: f64, radius: f64) -> f64 {
    let q = if radius > 0.0 { (distance / radius).min(1.0) } else { 1.0 };
    (1.0 - q * q).powi(2)
}
