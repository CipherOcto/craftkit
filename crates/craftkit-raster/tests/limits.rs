//! The **limits of the canvas**: the pictures it will not paint.
//!
//! A picture's size comes from outside — a command parameter, a file —
//! so it is checked before the memory is taken, and a limit that no test
//! names is a limit nothing proves holds.

use craftkit_raster::{MAX_IMAGE_PIXELS, MAX_IMAGE_SIDE, Raster};
use craftkit_sdk::camera::Camera;

/// A picture of **more pixels** than `MAX_IMAGE_PIXELS` is refused.
///
/// The size here is four thousand and ninety-six by four thousand and
/// ninety-seven: both sides are within [`MAX_IMAGE_SIDE`], so the bound
/// that trips is the number of pixels and not the shape.
#[test]
fn a_picture_over_the_pixel_limit_is_refused() {
    let width = 4096;
    let height = 4097;
    assert!(width <= MAX_IMAGE_SIDE && height <= MAX_IMAGE_SIDE, "both sides are within the shape bound");
    assert!(width * height > MAX_IMAGE_PIXELS, "and the picture holds more than the pixel bound");
    let error = Raster::new(width, height, Camera::new()).expect_err("that picture is too big");
    assert!(error.contains(&MAX_IMAGE_PIXELS.to_string()), "the error names the bound: {error}");
}

/// A picture **within** `MAX_IMAGE_PIXELS` is painted.
///
/// This is the other half of the test above: a cap that refuses
/// everything is not a cap, it is a wall.
#[test]
fn a_picture_within_the_limit_is_made() {
    let raster = Raster::new(64, 64, Camera::new()).expect("that picture fits");
    assert_eq!(raster.image().width(), 64, "the picture is as wide as it was asked to be");
    assert_eq!(raster.image().height(), 64, "and as tall");
}
