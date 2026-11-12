//! The **place** of a thing, and the handle that resizes it.
//!
//! A rectangle is where a node is and how big it is. The handle that
//! resizes it is in the **kit** and not in a front end: every front end
//! that resizes a node puts its handle in the same corner, and a handle
//! that moved would resize the other way round.

use craftkit_sdk::document::{Point, Rect};

/// The handle is the corner opposite the anchor.
///
/// The anchor is the corner the rectangle is named by — the one at
/// `(x, y)` — and the handle is the one across from it.
#[test]
fn the_handle_is_the_corner_across_from_the_anchor() {
    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
    let handle = rect.resize_handle();
    assert_eq!(handle.x, 40.0, "the handle is at the far side");
    assert_eq!(handle.y, 20.0, "and at the bottom");
}

/// A resize keeps the anchor where it is.
///
/// The rectangle grows and shrinks from its corner and does not walk
/// across the world.
#[test]
fn a_resize_keeps_the_anchor_where_it_is() {
    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
    let resized = rect.resized_to(Point::new(50.0, 25.0));
    assert_eq!(resized.x, 10.0, "the anchor did not move across");
    assert_eq!(resized.y, 20.0, "and it did not move up");
    assert_eq!(resized.w, 40.0, "the width followed the handle");
    assert_eq!(resized.h, 5.0, "and so did the height");
}

/// A handle on the wrong side gives an empty rectangle, not a negative
/// one.
///
/// A size is not a direction: a rectangle with a negative width is a
/// rectangle that cannot be drawn and cannot be picked.
#[test]
fn a_handle_on_the_wrong_side_gives_an_empty_rectangle() {
    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
    let resized = rect.resized_to(Point::new(0.0, 0.0));
    assert_eq!(resized.w, 0.0, "the width is not negative");
    assert_eq!(resized.h, 0.0, "and the height is not either");
    assert_eq!(resized.x, 10.0, "the anchor stayed");
}
