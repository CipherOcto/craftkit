//! The safe area: the part of the screen nothing covers.
//!
//! A phone's status bar and navigation bar are drawn **over** the
//! application, and the clock lands on the toolbar's first row, where a tap
//! is swallowed (§239). The platform says where the application may draw —
//! on Android, the window is laid out in the screen and the platform reports
//! the rectangle that its bars do not cover (§247).
//!
//! `egui` has the mechanism: the **safe area**, which every panel is laid out
//! inside. This module turns the platform's answer into that mechanism, and
//! it does the arithmetic in one place so that it can be tested.
//!
//! **The units are the whole difficulty.** The platform reports pixels; `egui`
//! works in points. The two are the same length divided by the scale factor,
//! and getting that division wrong puts the toolbar a whole screen-height out
//! of place.

use egui::{Rect, SafeAreaInsets, epaint::MarginF32};
/// The platform's rectangle, in pixels.
///
/// The rectangle the platform reports: the area the application may draw in,
/// relative to the window's own top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformRect {
    /// The left edge, in pixels.
    pub left: i32,
    /// The top edge, in pixels.
    pub top: i32,
    /// The right edge, in pixels.
    pub right: i32,
    /// The bottom edge, in pixels.
    pub bottom: i32,
}

impl PlatformRect {
    /// The rectangle, if it says anything at all.
    ///
    /// An empty rectangle is what the platform reports before it has laid the
    /// window out, and it is **not** an inset: a rectangle that covers
    /// nothing would reserve the whole screen.
    pub fn content(self) -> Option<Self> {
        if self.right > self.left && self.bottom > self.top { Some(self) } else { None }
    }
}

/// The insets the platform's rectangle implies, in the points `egui` uses.
///
/// `None` when the platform has not said anything: an empty rectangle, a
/// viewport that is not known yet, or a scale factor that is not a number.
pub fn insets(content: PlatformRect, viewport: Rect, pixels_per_point: f32) -> Option<SafeAreaInsets> {
    let content = content.content()?;
    if !pixels_per_point.is_finite() || pixels_per_point <= 0.0 {
        return None;
    }
    if !viewport.is_finite() {
        return None;
    }
    let points = |pixels: i32| pixels as f32 / pixels_per_point;
    let margin = MarginF32 {
        left: (points(content.left) - viewport.left()).max(0.0),
        top: (points(content.top) - viewport.top()).max(0.0),
        right: (viewport.right() - points(content.right)).max(0.0),
        bottom: (viewport.bottom() - points(content.bottom)).max(0.0),
    };
    Some(SafeAreaInsets(margin))
}

/// The rectangle the insets leave, in the points `egui` uses.
///
/// The safe area's own meaning, spelled out: the viewport with the insets
/// taken off it.
pub fn content_rect(viewport: Rect, insets: SafeAreaInsets) -> Rect {
    viewport - insets.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Pos2;

    /// The insets are the platform's answer, in `egui`'s units.
    ///
    /// §247: the platform says where the application may draw, and the tree
    /// lays out inside it. The arithmetic is the whole of it, and the
    /// arithmetic has a scale factor in it.
    #[test]
    fn the_insets_are_the_platforms_answer_in_egui_units() {
        let viewport = Rect::from_min_max(Pos2::ZERO, Pos2::new(360.0, 800.0));
        // A phone at two physical pixels per point: a 720 x 1600 screen.
        let content = PlatformRect { left: 0, top: 167, right: 720, bottom: 1504 };
        let insets = insets(content, viewport, 2.0).expect("the platform said something");
        assert_eq!(insets.0.left, 0.0);
        assert_eq!(insets.0.top, 83.5);
        assert_eq!(insets.0.right, 0.0);
        assert_eq!(insets.0.bottom, 48.0);
        // And the rectangle that is left is the platform's, in points.
        let inside = content_rect(viewport, insets);
        assert_eq!(inside, Rect::from_min_max(Pos2::new(0.0, 83.5), Pos2::new(360.0, 752.0)));
    }

    /// A rectangle that covers nothing is not an inset.
    ///
    /// The platform reports an empty rectangle before it has laid the window
    /// out, and an empty rectangle reserved as an inset would take the whole
    /// screen away from the application.
    #[test]
    fn an_empty_rectangle_is_not_an_inset() {
        let viewport = Rect::from_min_max(Pos2::ZERO, Pos2::new(360.0, 800.0));
        let empty = PlatformRect { left: 0, top: 0, right: 0, bottom: 0 };
        assert!(insets(empty, viewport, 2.0).is_none());
    }

    /// A scale factor that is not a number is not an answer.
    ///
    /// A zero would divide by zero, and a negative one would turn the inset
    /// inside out.
    #[test]
    fn a_scale_factor_that_is_not_a_number_is_not_an_answer() {
        let viewport = Rect::from_min_max(Pos2::ZERO, Pos2::new(360.0, 800.0));
        let content = PlatformRect { left: 0, top: 167, right: 720, bottom: 1504 };
        assert!(insets(content, viewport, 0.0).is_none());
        assert!(insets(content, viewport, -2.0).is_none());
        assert!(insets(content, viewport, f32::NAN).is_none());
    }

    /// An inset is never negative.
    ///
    /// A platform that reports a rectangle *larger* than the window is
    /// saying something odd, and the tree's answer is to reserve nothing on
    /// that side rather than to push the panels off the screen.
    #[test]
    fn an_inset_is_never_negative() {
        let viewport = Rect::from_min_max(Pos2::ZERO, Pos2::new(360.0, 800.0));
        // Every edge outside the viewport: 900 x 1700 pixels at two pixels
        // per point is 450 x 850 points, and the viewport is 360 x 800.
        let content = PlatformRect { left: -100, top: -50, right: 900, bottom: 1700 };
        let insets = insets(content, viewport, 2.0).expect("the platform said something");
        assert_eq!(insets.0.left, 0.0);
        assert_eq!(insets.0.top, 0.0);
        assert_eq!(insets.0.right, 0.0);
        assert_eq!(insets.0.bottom, 0.0);
    }
}
