//! The **icon** — a shape the agent draws.
//!
//! # What an icon is
//!
//! a2ui's `Icon` has three branches: a **string enum** of fifty-nine
//! built-in names, an **object** with an `svgPath`, and a **binding**.
//!
//! The fifty-nine names are **removed** — a declared narrowing, and the
//! reason is that craftkit has **no icon vocabulary**: drawing
//! fifty-nine glyphs is a large hand-written body, and a vocabulary of
//! names that the tree cannot draw is a vocabulary that lies.
//!
//! So an icon is a **path**: the agent sends the shape, and the tree
//! draws it. The path is read by **the SDK's reader**
//! (`craftkit_sdk::svgpath`), which is the same reader the vector plugin
//! uses — one reader, in one place.
//!
//! # The path is **fitted** to the icon's place
//!
//! The path is in the agent's own coordinates, and the node's rectangle
//! is where the icon is drawn. So the path is **scaled** to fit the
//! rectangle and **moved** into it, keeping its shape: an icon drawn in
//! raw coordinates would be a shape of unknown size in a known place,
//! and a shape that does not fit its place is a shape that is not drawn.

use craftkit_sdk::document::{Path, Point, Rect, Segment};

/// The path of an icon, if the name is one.
///
/// A name that is **not** a path is **refused** — the caller says so,
/// and the icon is not drawn. The tree does not guess and does not fall
/// back to a placeholder shape.
pub fn path_of(name: &str) -> Option<Path> {
    craftkit_sdk::svgpath::read_path(name).ok()
}

/// The path, **fitted** into a rectangle.
///
/// The shape is kept: the path is scaled by one number, so the icon
/// looks as the agent drew it. A path with **no extent** — a point, or
/// nothing at all — has nothing to fit, and there is no drawing of it.
pub fn fitted(path: &Path, rect: Rect) -> Option<Path> {
    let (min_x, min_y, max_x, max_y) = bounds_of(path)?;
    let width = max_x - min_x;
    let height = max_y - min_y;
    // A path with **no extent** has nothing to fit, and a rectangle
    // with no room has nowhere to fit it: either way there is no
    // drawing.
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return None;
    }
    if !rect.w.is_finite() || !rect.h.is_finite() || rect.w <= 0.0 || rect.h <= 0.0 {
        return None;
    }
    // The shape is kept: one number scales it, and it is the smaller of
    // the two, so the icon fits both ways.
    let scale = (rect.w / width).min(rect.h / height);
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    // And it is **centred** in the place it is drawn in.
    let drawn_width = width * scale;
    let drawn_height = height * scale;
    let at_x = rect.x + (rect.w - drawn_width) / 2.0;
    let at_y = rect.y + (rect.h - drawn_height) / 2.0;
    let move_point =
        |point: Point| Point::new(at_x + (point.x - min_x) * scale, at_y + (point.y - min_y) * scale);
    let mut subpaths = Vec::with_capacity(path.subpaths.len());
    for subpath in &path.subpaths {
        let mut segments = Vec::with_capacity(subpath.segments.len());
        for segment in &subpath.segments {
            segments.push(match segment {
                Segment::Line { to } => Segment::Line { to: move_point(*to) },
                Segment::Cubic { c1, c2, to } => {
                    Segment::Cubic { c1: move_point(*c1), c2: move_point(*c2), to: move_point(*to) }
                }
            });
        }
        subpaths.push(craftkit_sdk::document::SubPath {
            start: move_point(subpath.start),
            segments,
            closed: subpath.closed,
        });
    }
    Some(Path { subpaths, rule: path.rule })
}

/// The **bounds** of a path: the box its points lie in.
///
/// The control points are in the bounds as well: a curve may leave the
/// box its ends lie in, and the box that leaves out the curve's own
/// shape would cut the icon short.
fn bounds_of(path: &Path) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    let mut seen = false;
    let mut take = |point: Point| {
        if !point.x.is_finite() || !point.y.is_finite() {
            return;
        }
        seen = true;
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    };
    for subpath in &path.subpaths {
        take(subpath.start);
        for segment in &subpath.segments {
            match segment {
                Segment::Line { to } => take(*to),
                Segment::Cubic { c1, c2, to } => {
                    take(*c1);
                    take(*c2);
                    take(*to);
                }
            }
        }
    }
    if seen { Some((min_x, min_y, max_x, max_y)) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A path is read.
    #[test]
    fn a_path_is_read() {
        let path = path_of("M 0 0 L 10 0").expect("the path is read");
        assert_eq!(path.subpaths.len(), 1);
    }

    /// A name that is **not** a path is refused.
    #[test]
    fn a_name_that_is_not_a_path_is_refused() {
        assert!(path_of("check").is_none(), "a built-in name is not a path");
    }

    /// A path is **fitted** into the place it is drawn in.
    #[test]
    fn a_path_is_fitted_into_its_place() {
        let path = path_of("M 0 0 L 10 0 L 10 10 L 0 10 Z").expect("the path is read");
        let fitted = fitted(&path, Rect::new(0.0, 0.0, 20.0, 20.0)).expect("the path is fitted");
        let (min_x, min_y, max_x, max_y) = bounds_of(&fitted).expect("the path has bounds");
        assert!((min_x - 0.0).abs() < 0.001, "{min_x}");
        assert!((min_y - 0.0).abs() < 0.001, "{min_y}");
        assert!((max_x - 20.0).abs() < 0.001, "{max_x}");
        assert!((max_y - 20.0).abs() < 0.001, "{max_y}");
    }

    /// A path that is **not square** keeps its shape.
    #[test]
    fn a_path_that_is_not_square_keeps_its_shape() {
        let path = path_of("M 0 0 L 10 0 L 10 5 L 0 5 Z").expect("the path is read");
        let fitted = fitted(&path, Rect::new(0.0, 0.0, 20.0, 20.0)).expect("the path is fitted");
        let (min_x, min_y, max_x, max_y) = bounds_of(&fitted).expect("the path has bounds");
        assert!((max_x - min_x - 20.0).abs() < 0.001, "{}", max_x - min_x);
        assert!((max_y - min_y - 10.0).abs() < 0.001, "{}", max_y - min_y);
        assert!((min_y - 5.0).abs() < 0.001, "and it is centred: {min_y}");
    }

    /// A path with **no extent** is not fitted.
    #[test]
    fn a_path_with_no_extent_is_not_fitted() {
        let path = path_of("M 5 5").expect("the path is read");
        assert!(fitted(&path, Rect::new(0.0, 0.0, 20.0, 20.0)).is_none());
    }
}
