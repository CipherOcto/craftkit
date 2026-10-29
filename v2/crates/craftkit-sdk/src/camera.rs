//! The **camera**: what is on screen, and how much of it.
//!
//! # The problem this solves
//!
//! Every application with a canvas has a camera: a zoom, a scroll, and
//! the rules that keep them sane. Without one, each canvas invents its
//! own, and two canvases in the same program scroll differently, zoom
//! differently, and fight over the wheel.
//!
//! The tree this replaces has a rule about it — **one set of camera
//! controls**, one module that owns them, with its own caps, and every
//! canvas goes through it — and v2 had no camera at all, so the rule had
//! nothing to hold.
//!
//! # What a camera is
//!
//! A **zoom** (a factor) and an **offset** (the scroll, in the world's
//! own units). That is all: everything else — what is selected, what is
//! hovered, what the panels show — is the front end's business.
//!
//! # The rules
//!
//! - the zoom is **capped**: it is between [`Camera::MIN_ZOOM`] and
//!   [`Camera::MAX_ZOOM`], and a zoom outside that is **clamped** rather
//!   than refused. A wheel event can send anything, and a camera that
//!   refuses to move is a camera that feels broken;
//! - the offset is **not** capped: a document may be scrolled anywhere,
//!   and the cap that matters is the zoom's;
//! - the camera is **not** part of what the undo restores: scrolling is
//!   not an edit, and an undo that scrolls the view is a surprise.
//!
//! The camera is a **value**. A front end owns one, asks it where to
//! draw, and gives it the wheel. Two canvases that share a camera scroll
//! together, and two that do not, do not.

use crate::document::{Point, Rect};

/// A camera: a zoom and an offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    /// How much is magnified.
    zoom: f64,
    /// Where the middle of the screen is, in the world's units.
    offset: Point,
}

impl Default for Camera {
    fn default() -> Self {
        Camera::new()
    }
}

impl Camera {
    /// The smallest zoom: a hundredth of the world's unit.
    ///
    /// Below this a document is a smudge, and a program that lets the
    /// zoom go that far has lost the picture.
    pub const MIN_ZOOM: f64 = 0.01;

    /// The largest zoom: a hundred times the world's unit.
    ///
    /// Above this a single pixel of the world fills the screen, and the
    /// program is drawing a texture and not a document.
    pub const MAX_ZOOM: f64 = 100.0;

    /// A camera at the start: no zoom, and the world's origin in the
    /// middle.
    pub fn new() -> Self {
        Camera { zoom: 1.0, offset: Point::new(0.0, 0.0) }
    }

    /// How much is magnified.
    pub fn zoom(&self) -> f64 {
        self.zoom
    }

    /// Where the middle of the screen is, in the world's units.
    pub fn offset(&self) -> Point {
        self.offset
    }

    /// Set the zoom, within the caps.
    ///
    /// A zoom outside the caps is **clamped**: the wheel can send
    /// anything, and a camera that refuses to move feels broken.
    pub fn set_zoom(&mut self, zoom: f64) {
        self.zoom = clamp_zoom(zoom);
    }

    /// Multiply the zoom, within the caps.
    ///
    /// This is what the wheel and the pinch do.
    pub fn zoom_by(&mut self, factor: f64) {
        if !factor.is_finite() || factor <= 0.0 {
            return;
        }
        self.zoom = clamp_zoom(self.zoom * factor);
    }

    /// Move the camera, in the world's units.
    pub fn pan(&mut self, across: f64, down: f64) {
        if !across.is_finite() || !down.is_finite() {
            return;
        }
        self.offset = Point::new(self.offset.x + across, self.offset.y + down);
    }

    /// Put a point of the world at the middle of the screen.
    pub fn look_at(&mut self, point: Point) {
        self.offset = point;
    }

    /// Where a point of the world is on the screen.
    ///
    /// The screen's point is measured from the **middle**, so a front end
    /// that draws in its own coordinates adds its own half-width and
    /// half-height.
    pub fn to_screen(&self, point: Point) -> Point {
        Point::new((point.x - self.offset.x) * self.zoom, (point.y - self.offset.y) * self.zoom)
    }

    /// Where a point of the screen is in the world.
    ///
    /// It is the other way round, and it is what a click is: the screen
    /// point is turned back into the world.
    pub fn to_world(&self, point: Point) -> Point {
        Point::new(point.x / self.zoom + self.offset.x, point.y / self.zoom + self.offset.y)
    }

    /// What part of the world is on screen.
    ///
    /// The **half** sizes are in screen units: the rectangle is what the
    /// screen shows, and the caller says how big the screen is.
    pub fn visible(&self, half_width: f64, half_height: f64) -> Rect {
        let half_width = half_width.max(0.0) / self.zoom;
        let half_height = half_height.max(0.0) / self.zoom;
        Rect::new(
            self.offset.x - half_width,
            self.offset.y - half_height,
            half_width * 2.0,
            half_height * 2.0,
        )
    }

    /// Whether a rectangle of the world is on screen.
    ///
    /// It is a test and not a list: a front end that draws a thousand
    /// nodes asks once each, and the ones that are off screen are not
    /// drawn.
    pub fn shows(&self, rect: Rect, half_width: f64, half_height: f64) -> bool {
        let visible = self.visible(half_width, half_height);
        visible.intersects(rect)
    }
}

/// The zoom, within the caps.
fn clamp_zoom(zoom: f64) -> f64 {
    if zoom.is_nan() {
        // Not a number is not a zoom: the camera stays where it is.
        return 1.0;
    }
    zoom.clamp(Camera::MIN_ZOOM, Camera::MAX_ZOOM)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A point of the world.
    fn point(x: f64, y: f64) -> Point {
        Point::new(x, y)
    }

    /// A camera at the start.
    fn camera() -> Camera {
        Camera::new()
    }

    /// The zoom is capped at both ends.
    #[test]
    fn the_zoom_is_capped_at_both_ends() {
        let mut camera = camera();
        camera.set_zoom(0.0001);
        assert_eq!(camera.zoom(), Camera::MIN_ZOOM, "the zoom is as small as it goes");
        camera.set_zoom(10_000.0);
        assert_eq!(camera.zoom(), Camera::MAX_ZOOM, "the zoom is as big as it goes");
    }

    /// A zoom that is not a number does not move the camera.
    #[test]
    fn a_zoom_that_is_not_a_number_does_not_move_the_camera() {
        let mut camera = camera();
        camera.set_zoom(f64::NAN);
        assert_eq!(camera.zoom(), 1.0, "the camera is where it was");
        camera.set_zoom(f64::INFINITY);
        assert_eq!(camera.zoom(), Camera::MAX_ZOOM, "infinity is as big as it goes");
    }

    /// The wheel zooms, and the caps hold.
    #[test]
    fn the_wheel_zooms_and_the_caps_hold() {
        let mut camera = camera();
        camera.zoom_by(2.0);
        assert_eq!(camera.zoom(), 2.0);
        camera.zoom_by(0.5);
        assert_eq!(camera.zoom(), 1.0);
        for _ in 0..100 {
            camera.zoom_by(10.0);
        }
        assert_eq!(camera.zoom(), Camera::MAX_ZOOM, "the cap holds");
    }

    /// A zoom factor that is not a number is ignored.
    #[test]
    fn a_zoom_factor_that_is_not_a_number_is_ignored() {
        let mut camera = camera();
        camera.zoom_by(0.0);
        assert_eq!(camera.zoom(), 1.0, "a zoom of nothing is not a zoom");
        camera.zoom_by(f64::NAN);
        assert_eq!(camera.zoom(), 1.0, "not a number is not a zoom");
    }

    /// A point of the world and a point of the screen are two ways of
    /// saying where something is.
    #[test]
    fn a_point_of_the_world_and_a_point_of_the_screen_are_the_same_place() {
        let mut camera = camera();
        camera.look_at(point(10.0, 20.0));
        camera.set_zoom(4.0);
        let screen = camera.to_screen(point(12.0, 16.0));
        assert_eq!(screen.x, 8.0);
        assert_eq!(screen.y, -16.0);
        let world = camera.to_world(screen);
        assert_eq!(world.x, 12.0, "the point came back");
        assert_eq!(world.y, 16.0);
    }

    /// What is on screen.
    #[test]
    fn what_is_on_screen_is_the_rectangle_around_the_middle() {
        let mut camera = camera();
        camera.look_at(point(5.0, 5.0));
        camera.set_zoom(2.0);
        // The screen is two hundred units across and the zoom is two, so
        // the world is a hundred units across, and the middle of the
        // screen is at five.
        let visible = camera.visible(100.0, 50.0);
        assert_eq!(visible.x, -45.0, "the left edge");
        assert_eq!(visible.y, -20.0, "the top edge");
        assert_eq!(visible.w, 100.0, "the width is the screen's width in the world");
        assert_eq!(visible.h, 50.0, "the height is the screen's height in the world");
    }

    /// A rectangle that is off screen is not shown.
    #[test]
    fn a_rectangle_that_is_off_screen_is_not_shown() {
        let mut camera = camera();
        camera.look_at(point(0.0, 0.0));
        assert!(camera.shows(Rect::new(-1.0, -1.0, 2.0, 2.0), 100.0, 100.0), "it is on screen");
        assert!(!camera.shows(Rect::new(1000.0, 1000.0, 2.0, 2.0), 100.0, 100.0), "it is not");
    }

    /// Panning moves the camera.
    #[test]
    fn panning_moves_the_camera() {
        let mut camera = camera();
        camera.pan(10.0, -5.0);
        assert_eq!(camera.offset().x, 10.0);
        assert_eq!(camera.offset().y, -5.0);
        camera.pan(f64::NAN, 0.0);
        assert_eq!(camera.offset().x, 10.0, "a pan that is not a number does not move it");
    }
}
