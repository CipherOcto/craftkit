//! The **reading** of SVG path data.
//!
//! The reader lives in the SDK now, so that the vector plugin and the a2ui
//! layer share **one** reader — and it reads **arcs**, which it did not
//! before.

use craftkit_sdk::svgpath::read_path;
use craftkit_sdk::{FillRule, Point, Segment};

/// The **subpaths** of a path.
fn subpaths(data: &str) -> Vec<craftkit_sdk::SubPath> {
    let path = read_path(data).expect("the path data is read");
    assert_eq!(path.rule, FillRule::NonZero, "the rule is the tree's own");
    path.subpaths
}

/// The **segments** of the first subpath.
fn segments(data: &str) -> Vec<Segment> {
    let paths = subpaths(data);
    assert_eq!(paths.len(), 1, "the path holds one subpath");
    paths[0].segments.clone()
}

/// The kinds of the segments, so that a test says what shape it is.
fn kinds(data: &str) -> Vec<&'static str> {
    segments(data)
        .iter()
        .map(|segment| match segment {
            Segment::Line { .. } => "line",
            Segment::Cubic { .. } => "cubic",
        })
        .collect()
}

/// A **move** and a **line**.
#[test]
fn a_move_and_a_line_are_read() {
    let paths = subpaths("M 10 20 L 30 40");
    assert_eq!(paths.len(), 1, "one subpath");
    assert_eq!(paths[0].start, Point::new(10.0, 20.0), "it starts where the move says");
    assert_eq!(paths[0].segments.len(), 1, "and holds one segment");
    assert!(matches!(paths[0].segments[0], Segment::Line { to } if to == Point::new(30.0, 40.0)));
}

/// Every command SVG writes is read.
#[test]
fn every_command_svg_writes_is_read() {
    assert_eq!(kinds("M 0 0 L 10 0"), vec!["line"]);
    assert_eq!(kinds("M 0 0 H 10"), vec!["line"], "a horizontal line");
    assert_eq!(kinds("M 0 0 V 10"), vec!["line"], "a vertical line");
    assert_eq!(kinds("M 0 0 C 1 1 2 2 3 3"), vec!["cubic"], "a cubic curve");
    assert_eq!(kinds("M 0 0 S 2 2 3 3"), vec!["cubic"], "a smooth cubic curve");
    assert_eq!(kinds("M 0 0 Q 1 1 2 2"), vec!["cubic"], "a quadratic curve, as the cubic it is");
    assert_eq!(kinds("M 0 0 T 2 2"), vec!["cubic"], "a smooth quadratic curve, as a cubic");
}

/// The **relative** spelling is relative to the pen.
#[test]
fn the_relative_spelling_is_relative_to_the_pen() {
    let paths = subpaths("M 10 10 l 5 5");
    assert!(matches!(paths[0].segments[0], Segment::Line { to } if to == Point::new(15.0, 15.0)));
}

/// A **close** ends the subpath.
#[test]
fn a_close_ends_the_subpath() {
    let paths = subpaths("M 0 0 L 10 0 L 10 10 Z");
    assert!(paths[0].closed, "the subpath is closed");
}

/// Path data that holds **no path** is refused.
#[test]
fn path_data_with_no_path_is_refused() {
    let error = read_path("").expect_err("the empty path data is refused");
    assert!(error.contains("no path"), "{error}");
}

/// A command this tree does not read is refused.
#[test]
fn an_unknown_command_is_refused() {
    let error = read_path("M 0 0 X 10 10").expect_err("the command is refused");
    assert!(error.contains("does not read it"), "{error}");
}

/// A number that is not a number is refused.
#[test]
fn a_number_that_is_not_a_number_is_refused() {
    let error = read_path("M 0 0 L 1e 0").expect_err("the number is refused");
    assert!(error.contains("not a number"), "{error}");
}

/// A number that is not finite is refused.
#[test]
fn a_number_that_is_not_finite_is_refused() {
    let error = read_path("M 0 0 L 1e400 0").expect_err("the number is refused");
    assert!(error.contains("not finite"), "{error}");
}

/// **An arc is read**, and it is read as the cubics it is.
#[test]
fn an_arc_is_read() {
    let kinds = kinds("M 0 0 A 10 10 0 0 1 20 20");
    assert!(!kinds.is_empty(), "the arc holds segments");
    assert!(kinds.iter().all(|kind| *kind == "cubic"), "and they are cubics: {kinds:?}");
}

/// The **end** of the last segment — where the arc lands.
fn end_of(data: &str) -> Point {
    let segments = segments(data);
    let last = segments.last().expect("the arc holds a segment");
    match last {
        Segment::Line { to } => *to,
        Segment::Cubic { to, .. } => *to,
    }
}

/// Whether two points are the same, to within a hair.
fn near(a: Point, b: Point) -> bool {
    (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
}

/// An arc of a quarter turn is **one** cubic.
#[test]
fn an_arc_of_a_quarter_turn_is_one_cubic() {
    let segments = segments("M 10 0 A 10 10 0 0 1 0 10");
    assert_eq!(segments.len(), 1, "a quarter turn is one cubic");
    assert!(
        near(end_of("M 10 0 A 10 10 0 0 1 0 10"), Point::new(0.0, 10.0)),
        "and it lands where the arc ends"
    );
}

/// An arc of a **half** turn is two cubics, and of a three-quarter turn
/// three.
#[test]
fn a_longer_arc_is_split() {
    assert_eq!(segments("M 10 0 A 10 10 0 0 1 -10 0").len(), 2, "a half turn is two cubics");
    assert_eq!(segments("M 10 0 A 10 10 0 1 1 0 10").len(), 3, "a three-quarter turn is three cubics");
}

/// Every arc **lands where it was told to land**.
#[test]
fn an_arc_lands_where_it_was_told_to_land() {
    let cases: [(&str, Point); 5] = [
        ("M 10 0 A 10 10 0 0 1 0 10", Point::new(0.0, 10.0)),
        ("M 10 0 A 10 10 0 0 1 -10 0", Point::new(-10.0, 0.0)),
        ("M 10 0 A 10 10 0 1 1 0 10", Point::new(0.0, 10.0)),
        ("M 0 0 A 5 5 45 1 0 20 20", Point::new(20.0, 20.0)),
        ("M 0 0 A 5 5 45 0 1 20 20", Point::new(20.0, 20.0)),
    ];
    for (data, expected) in cases {
        assert!(near(end_of(data), expected), "{data} lands at {:?}", end_of(data));
    }
}

/// A **relative** arc is relative to the pen.
#[test]
fn a_relative_arc_is_relative_to_the_pen() {
    let absolute = segments("M 10 0 A 10 10 0 0 1 0 10");
    let relative = segments("M 10 0 a 10 10 0 0 1 -10 10");
    match (&absolute[0], &relative[0]) {
        (Segment::Cubic { to: a, .. }, Segment::Cubic { to: b, .. }) => {
            assert_eq!(*a, *b, "the relative arc ends in the same place");
        }
        _ => panic!("both are cubics"),
    }
}

/// An arc whose **radius is nothing** is a straight line.
#[test]
fn an_arc_with_no_radius_is_a_line() {
    let kinds = kinds("M 0 0 A 0 0 0 0 1 20 20");
    assert_eq!(kinds, vec!["line"], "the arc is a line");
    let segments = segments("M 0 0 A 0 0 0 0 1 20 20");
    match &segments[0] {
        Segment::Line { to } => assert_eq!(*to, Point::new(20.0, 20.0), "and it ends where the arc ends"),
        other => panic!("the segment is a line: {other:?}"),
    }
}

/// An arc whose radii are **too small** is scaled up, which is what a
/// file's arc means.
#[test]
fn an_arc_with_too_small_radii_is_scaled_up() {
    // The radii are far too small to reach from (0,0) to (20,0), so the
    // ellipse is scaled up until they do.
    let segments = segments("M 0 0 A 1 1 0 0 1 20 0");
    assert!(!segments.is_empty(), "the arc holds segments");
    assert!(near(end_of("M 0 0 A 1 1 0 0 1 20 0"), Point::new(20.0, 0.0)), "and it lands where the arc ends");
}

/// An arc that starts and ends in the same place is **nothing**, and the
/// pen does not move.
#[test]
fn an_arc_that_does_not_move_is_nothing() {
    let segments = segments("M 5 5 A 10 10 0 0 1 5 5");
    assert!(segments.is_empty(), "the arc holds no segment");
}

/// An arc resets the **smooth** curve's memory, so a smooth curve after
/// an arc does not lean on it.
#[test]
fn an_arc_resets_the_smooth_curves_memory() {
    let kinds = kinds("M 0 0 A 10 10 0 0 1 20 0 S 30 0 40 0");
    assert!(kinds.iter().all(|kind| *kind == "cubic"), "everything is a cubic: {kinds:?}");
}

/// Path data that is **cut short** is refused, and not read as far as it
/// goes.
///
/// A path that ends in the middle of a command is a mistake, and a
/// mistake is refused: the tree does not draw the half a path it can
/// make sense of.
#[test]
fn path_data_that_is_cut_short_is_refused() {
    let error = read_path("M 10 20 L 30").expect_err("the path data is refused");
    assert!(error.contains("not enough numbers"), "{error}");
    let error = read_path("M 10 20 L").expect_err("the path data is refused");
    assert!(error.contains("not enough numbers"), "{error}");
}

/// A path that does **not begin** with a move is refused: the file says
/// so, and a line with no place to begin from is not a line.
#[test]
fn a_path_that_does_not_begin_with_a_move_is_refused() {
    let error = read_path("L 10 10").expect_err("the path data is refused");
    assert!(error.contains("begins with a move"), "{error}");
}

/// A point that **overflows** is refused: the numbers are finite, and
/// their sum need not be.
#[test]
fn a_point_that_overflows_is_refused() {
    let error = read_path("M 1e308 0 l 1e308 0").expect_err("the point is refused");
    assert!(error.contains("does not stay in numbers"), "{error}");
}

/// An arc's **flags** are single characters, and not numbers.
#[test]
fn an_arcs_flags_are_single_characters() {
    // The flags are written without a separator: `0110` is a flag, a
    // flag and a number.
    let segments = segments("M 0 0 A 10 10 0 0110 0");
    assert!(!segments.is_empty(), "the arc holds segments");
    // And a flag that is not a `0` or a `1` is refused.
    let error = read_path("M 0 0 A 10 10 0 2 1 10 10").expect_err("the flag is refused");
    assert!(error.contains("flag"), "{error}");
}
