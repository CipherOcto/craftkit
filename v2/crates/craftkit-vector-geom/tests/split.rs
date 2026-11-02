//! The **split** of a segment.
//!
//! A segment split in two is the **same curve**: the point where the
//! halves meet is the point the curve had at `t`.

use craftkit_sdk::document::{Path, Point, Segment, SubPath};
use craftkit_vector_geom::split_segment;

/// A **curve**: one cubic segment, from here to there.
fn a_curve() -> SubPath {
    SubPath {
        start: Point::new(0.0, 0.0),
        segments: vec![Segment::Cubic {
            c1: Point::new(0.0, 10.0),
            c2: Point::new(10.0, 10.0),
            to: Point::new(10.0, 0.0),
        }],
        closed: false,
    }
}

/// The **end** of a segment.
fn end_of(segment: &Segment) -> Point {
    match segment {
        Segment::Line { to } => *to,
        Segment::Cubic { to, .. } => *to,
    }
}

/// The **point** of a curve at `t`, worked out the long way: the curve
/// is a polynomial and this is the value of it.
fn curve_at(from: Point, curve: &Segment, t: f64) -> Point {
    let Segment::Cubic { c1, c2, to } = curve else { return from };
    let u = 1.0 - t;
    let a = u * u * u;
    let b = 3.0 * u * u * t;
    let c = 3.0 * u * t * t;
    let d = t * t * t;
    Point::new(a * from.x + b * c1.x + c * c2.x + d * to.x, a * from.y + b * c1.y + c * c2.y + d * to.y)
}

/// A segment split in two is the **same curve**: the point where the
/// halves meet is the point the curve had at `t`.
#[test]
fn a_split_curve_is_the_same_curve() {
    let subpath = a_curve();
    let curve = subpath.segments.first().expect("the segment is there");
    let split = split_segment(&subpath, 0, 0.5).expect("the segment is split");
    assert_eq!(split.segments.len(), 2, "there are two halves");
    let middle = curve_at(subpath.start, curve, 0.5);
    let meeting = end_of(&split.segments[0]);
    assert!((meeting.x - middle.x).abs() < 1e-9, "at the point of the curve: {}", middle.x);
    assert!((meeting.y - middle.y).abs() < 1e-9, "at the point of the curve: {}", middle.y);
    // The **end** of the curve does not move: the second half ends
    // where the whole curve ended.
    let end = end_of(curve);
    let last = end_of(&split.segments[1]);
    assert!((last.x - end.x).abs() < 1e-12, "and the curve ends where it did");
    assert!((last.y - end.y).abs() < 1e-12, "and the curve ends where it did");
}

/// A **line** split in two is the same line.
#[test]
fn a_split_line_is_the_same_line() {
    let subpath = SubPath {
        start: Point::new(0.0, 0.0),
        segments: vec![Segment::Line { to: Point::new(10.0, 0.0) }],
        closed: false,
    };
    let split = split_segment(&subpath, 0, 0.25).expect("the segment is split");
    assert_eq!(split.segments.len(), 2, "there are two halves");
    let meeting = end_of(&split.segments[0]);
    assert!((meeting.x - 2.5).abs() < 1e-12, "a quarter of the way along: {}", meeting.x);
}

/// A segment that does **not** exist is refused.
#[test]
fn a_segment_that_is_not_there_is_refused() {
    let subpath = a_curve();
    let error = split_segment(&subpath, 7, 0.5).expect_err("the split is refused");
    assert!(error.to_string().contains("no segment 7"), "the error says why: {error}");
}

/// A split at the **end** of a segment is the same curve: the second
/// half is a point.
#[test]
fn a_split_at_the_end_is_the_same_curve() {
    let subpath = a_curve();
    let split = split_segment(&subpath, 0, 1.0).expect("the segment is split");
    assert_eq!(split.segments.len(), 2, "there are two halves");
    let first = end_of(&split.segments[0]);
    let second = end_of(&split.segments[1]);
    assert!((first.x - second.x).abs() < 1e-12, "the halves meet at the end");
}

/// The path of a **contour** is not changed by the split: the contour
/// still starts where it started.
#[test]
fn the_start_of_the_contour_does_not_move() {
    let path = Path::single(a_curve());
    let Some(subpath) = path.subpaths.first() else { panic!("the contour is there") };
    let split = split_segment(subpath, 0, 0.5).expect("the segment is split");
    assert_eq!(split.start, subpath.start, "the contour starts where it started");
    assert_eq!(split.closed, subpath.closed, "and it is closed as it was");
}
