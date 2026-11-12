//! The **axis**: a line, with a point on it and the way it runs.
//!
//! The tree's own axis: it knows which **side** of it a point is on,
//! and it can cut a **box** into the two **half-planes** it makes.
//!
//! The rules are the tree's own:
//!
//! - an axis through **two points** needs them to differ;
//! - an axis may be **vertical** or **horizontal**, and it runs
//!   through the point it is given;
//! - the **side** of a point is the turn of the axis at it;
//! - a half-plane is the box, cut by the axis.

use craftkit_sdk::{Path, Point, Segment, SubPath};

/// A line, with a point on it and the way it runs.
pub struct Axis {
    /// A point on the axis.
    pub point: Point,
    /// The way the axis runs.
    pub direction: Point,
}

impl Axis {
    /// The axis **through two points**, when they differ.
    pub fn through(from: Point, to: Point) -> Option<Axis> {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        if dx.abs() < 1e-12 && dy.abs() < 1e-12 {
            return None;
        }
        Some(Axis { point: from, direction: Point::new(dx, dy) })
    }

    /// A **vertical** axis through a point.
    pub fn vertical(point: Point) -> Axis {
        Axis { point, direction: Point::new(0.0, 1.0) }
    }

    /// A **horizontal** axis through a point.
    pub fn horizontal(point: Point) -> Axis {
        Axis { point, direction: Point::new(1.0, 0.0) }
    }

    /// Which **side** of the axis a point is on.
    ///
    /// The tree's own: the turn of the axis's way at the point. A point
    /// to the left of a vertical axis has a positive side, and a point
    /// above a horizontal one has a negative side.
    pub fn side(&self, point: Point) -> f64 {
        self.direction.x * (point.y - self.point.y) - self.direction.y * (point.x - self.point.x)
    }

    /// One **half** of a box, cut by the axis: the part of the box on
    /// the side the sign names.
    ///
    /// The box is a rectangle and the axis is straight, so the half is
    /// the rectangle's corners on that side, with the axis's crossings
    /// put in between them.
    pub fn half_plane(&self, sign: f64, box_of: (f64, f64, f64, f64)) -> Option<Path> {
        let (x0, y0, x1, y1) = box_of;
        let corners = [Point::new(x0, y0), Point::new(x1, y0), Point::new(x1, y1), Point::new(x0, y1)];
        let mut kept: Vec<Point> = Vec::with_capacity(corners.len() + 4);
        for index in 0..corners.len() {
            let a = corners[index];
            let b = corners[(index + 1) % corners.len()];
            let ta = sign * self.side(a);
            let tb = sign * self.side(b);
            if ta >= 0.0 {
                kept.push(a);
            }
            if (ta >= 0.0) != (tb >= 0.0) {
                let ratio = ta / (ta - tb);
                kept.push(Point::new(a.x + ratio * (b.x - a.x), a.y + ratio * (b.y - a.y)));
            }
        }
        if kept.len() < 3 {
            return None;
        }
        let segments: Vec<Segment> = kept
            .windows(2)
            .map(|pair| Segment::Line { to: pair[1] })
            .chain(std::iter::once(Segment::Line { to: kept[0] }))
            .collect();
        Some(Path::single(SubPath { start: kept[0], segments, closed: true }))
    }
}
