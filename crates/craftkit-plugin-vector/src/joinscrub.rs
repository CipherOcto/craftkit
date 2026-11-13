//! The **join scrub**: the ends near a drag joined together.
//!
//! The tree's Join Scrub drags along the ends of open paths, and the
//! two ends that are nearest the drag are joined: the ends of two
//! paths, or the two ends of one path, which closes it.
//!
//! The rules are the tree's own:
//!
//! - the ends that are **near the drag** are the ones that are joined,
//!   and so are the ends near the drag's **first** and **last** point;
//! - the ends are **ordered along the drag**, and the first two are
//!   the ones that are joined;
//! - two ends of **one** contour close it, and the curve that came
//!   into the last point comes into the first point;
//! - two ends of **two** contours are joined into one contour.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Point, Segment, SubPath};

use crate::erasepath::{distance_to_drag, read_drag};
use crate::joinends::join_contours;
use crate::{vector_of, write_back};

/// The **tolerance** of the scrub when the caller names none.
const DEFAULT_TOLERANCE: f64 = 12.0;

/// The order of an end that is **after** everything else.
const LAST: f64 = 1e30;

commands! {
    /// Join the ends that are near a drag.
    vector.path.join.scrub {
        area: "vector",
        layer: "vector",
        icon: "path",
        mutates,
        undoable,
        menu: ("Path", "Points"),
        params: {
            /// The path.
            path: NodeRef,
            /// The scrub: a run of points, `[[x, y], …]`.
            points: Json,
            /// How near the ends have to be. 12 if it is not given.
            tolerance: Option<Number>,
        },
        run: {
            let id = "vector.path.join.scrub".to_string();
            let drag =
                read_drag(&self.points).map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            let tolerance = self.tolerance.as_ref().map(|tolerance| tolerance.0).unwrap_or(DEFAULT_TOLERANCE);
            if !tolerance.is_finite() || tolerance < 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: "the tolerance of the scrub is a number and not a negative one".to_string(),
                });
            }
            let mut vector = vector_of(session, self.path.0)?;
            // The **ends** that are near the drag, and where they are
            // along it.
            let mut ends: Vec<(f64, usize, bool)> = Vec::new();
            for (index, contour) in vector.path.subpaths.iter().enumerate() {
                if contour.closed || contour.segments.is_empty() {
                    continue;
                }
                for (at_start, point) in [(true, contour.start), (false, contour.end())] {
                    let Some(order) = order_along(&drag, point, tolerance) else {
                        continue;
                    };
                    ends.push((order, index, at_start));
                }
            }
            ends.sort_by(|one, other| one.0.total_cmp(&other.0));
            if ends.len() < 2 {
                return Ok(Outcome::new("fewer than two ends were near the scrub")
                    .with_data(json!({ "id": self.path.0 })));
            }
            let (_, first_contour, first_at_start) = ends[0];
            let (_, second_contour, second_at_start) = ends[1];
            if first_contour == second_contour {
                // The two ends of **one** contour: it closes.
                let contour = vector.path.subpaths[first_contour].clone();
                let closed = close_contour(contour, tolerance);
                vector.path.subpaths[first_contour] = closed;
                write_back(session, self.path.0, &vector)?;
                return Ok(Outcome::changed("the path is closed")
                    .with_data(json!({ "id": self.path.0, "contour": first_contour })));
            }
            // The two ends of **two** contours: they are joined.
            let one = vector.path.subpaths[first_contour].clone();
            let two = vector.path.subpaths[second_contour].clone();
            // The end of the first contour comes last, and the end of the
            // second comes first.
            let joined = join_contours(&one, &two, first_at_start, !second_at_start, None);
            let mut subpaths = vector.path.subpaths.clone();
            if first_contour < second_contour {
                subpaths[first_contour] = joined;
                subpaths.remove(second_contour);
            } else {
                subpaths.remove(second_contour);
                subpaths[first_contour - 1] = joined;
            }
            vector.path.subpaths = subpaths;
            write_back(session, self.path.0, &vector)?;
            Ok(Outcome::changed("the ends are joined")
                .with_data(json!({ "id": self.path.0, "contour": first_contour.min(second_contour) })))

        }
    }
}

/// Where an end is **along the drag**, when it is near it.
///
/// The tree's own rule: the ends on the drag are ordered by where they
/// are on it, and the ends near the drag's **first** point come before
/// everything and the ends near its **last** point come after.
fn order_along(drag: &[Point], point: Point, tolerance: f64) -> Option<f64> {
    let on_drag = distance_to_drag(drag, point) <= tolerance;
    let first = drag.first()?;
    let last = drag.last()?;
    let near_first = first_distance(point, *first) <= tolerance;
    let near_last = first_distance(point, *last) <= tolerance;
    if on_drag {
        return Some(distance_along(drag, point));
    }
    if near_first && (!near_last || first_distance(point, *first) <= first_distance(point, *last)) {
        return Some(-1.0);
    }
    if near_last {
        return Some(LAST);
    }
    None
}

/// How far along the drag a point lies.
fn distance_along(drag: &[Point], point: Point) -> f64 {
    let mut walked = 0.0;
    let mut best = (f64::INFINITY, 0.0);
    for window in drag.windows(2) {
        let length = (window[1].x - window[0].x).hypot(window[1].y - window[0].y);
        let along = projection(window[0], window[1], point);
        let distance = distance_to_line(window[0], window[1], point);
        if distance < best.0 {
            best = (distance, walked + along * length);
        }
        walked += length;
    }
    best.1
}

/// How far a point is from **two** points.
fn first_distance(point: Point, other: Point) -> f64 {
    (point.x - other.x).hypot(point.y - other.y)
}

/// How far along the side two points a point falls.
fn projection(one: Point, two: Point, at: Point) -> f64 {
    let (dx, dy) = (two.x - one.x, two.y - one.y);
    let length = dx * dx + dy * dy;
    if length <= 1e-12 {
        return 0.0;
    }
    (((at.x - one.x) * dx + (at.y - one.y) * dy) / length).clamp(0.0, 1.0)
}

/// How far a point is from the **side** through two points.
fn distance_to_line(one: Point, two: Point, at: Point) -> f64 {
    let (dx, dy) = (two.x - one.x, two.y - one.y);
    let length = dx * dx + dy * dy;
    if length <= 1e-12 {
        return (at.x - one.x).hypot(at.y - one.y);
    }
    let along = (((at.x - one.x) * dx + (at.y - one.y) * dy) / length).clamp(0.0, 1.0);
    let closest = Point::new(one.x + dx * along, one.y + dy * along);
    (at.x - closest.x).hypot(at.y - closest.y)
}

/// A contour **closed**: its two ends meet.
///
/// The tree's own rule: the curve that came into the last point comes
/// into the first point, and the first point moves to the **middle** of
/// the two ends.
fn close_contour(contour: SubPath, tolerance: f64) -> SubPath {
    let start = contour.start;
    let end = contour.end();
    if (start.x - end.x).hypot(start.y - end.y) > tolerance {
        // The ends are two places: a **straight side** closes them.
        let mut segments = contour.segments.clone();
        segments.push(Segment::Line { to: start });
        return SubPath { start, segments, closed: true };
    }
    // The ends are **one place**: the last side is taken away and the
    // curve that came into it comes into the first point instead.
    let mut segments = contour.segments.clone();
    let last = segments.pop();
    let middle = Point::new((start.x + end.x) / 2.0, (start.y + end.y) / 2.0);
    let before = segments.last().map(|segment| segment.end()).unwrap_or(start);
    let c2 = match &last {
        Some(Segment::Cubic { c2, .. }) => Point::new(c2.x + (middle.x - end.x), c2.y + (middle.y - end.y)),
        _ => before,
    };
    segments.push(Segment::Cubic { c1: before, c2, to: middle });
    SubPath { start: middle, segments, closed: true }
}
