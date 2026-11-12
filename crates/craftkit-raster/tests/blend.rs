//! The **blend mode**: how a thing mixes with what is under it.
//!
//! The modes are the PDF's and the screen's, and the kit says how each
//! of them mixes — so that a shape, a letter and a picture all mix the
//! same way.

use craftkit_raster::{Canvas, Image, Raster};
use craftkit_sdk::camera::Camera;
use craftkit_sdk::document::{BlendMode, Colour, Fill, Point, Rect};

/// A canvas of that size, with a camera at the origin.
fn a_canvas(width: usize, height: usize) -> Raster {
    Raster::new(width, height, Camera::new()).expect("that canvas fits")
}

/// A rectangle of that size, at the origin.
fn a_rect(width: f64, height: f64) -> Rect {
    Rect { x: 0.0, y: 0.0, w: width, h: height }
}

/// The colour of a pixel, or `None` if it is not there.
fn pixel(image: &Image, x: usize, y: usize) -> Option<[u8; 4]> {
    image.pixel(x, y)
}

/// The modes mix the two colours the way the **PDF** says they do.
///
/// The numbers are the tree's own: the tree's colour crate has the
/// same formulas, and the same numbers.
#[test]
fn the_modes_mix_the_two_colours() {
    let backdrop = Colour([0.2, 0.5, 0.9, 1.0]);
    let source = Colour([0.6, 0.25, 0.5, 1.0]);
    let multiply = BlendMode::Multiply.mix(backdrop, source);
    assert!((multiply.0[0] - 0.12).abs() < 1e-5, "multiply is the product: {multiply:?}");
    assert!((multiply.0[1] - 0.125).abs() < 1e-5, "on each channel: {multiply:?}");
    let screen = BlendMode::Screen.mix(backdrop, source);
    assert!((screen.0[0] - 0.68).abs() < 1e-5, "screen is the inverse of the product: {screen:?}");
    let darken = BlendMode::Darken.mix(backdrop, source);
    assert!((darken.0[0] - 0.2).abs() < 1e-5, "darken is the darker: {darken:?}");
    let lighten = BlendMode::Lighten.mix(backdrop, source);
    assert!((lighten.0[0] - 0.6).abs() < 1e-5, "lighten is the lighter: {lighten:?}");
    let difference = BlendMode::Difference.mix(backdrop, source);
    assert!((difference.0[0] - 0.4).abs() < 1e-5, "difference is the gap: {difference:?}");
    // The plain mode is the source itself.
    let plain = BlendMode::Normal.mix(backdrop, source);
    assert!((plain.0[0] - 0.6).abs() < 1e-5, "the plain mode lays the source over: {plain:?}");
}

/// A mode that is **named** is found, however it is written.
#[test]
fn a_mode_is_named() {
    assert_eq!(BlendMode::parse("Multiply"), Some(BlendMode::Multiply));
    assert_eq!(BlendMode::parse("multiply"), Some(BlendMode::Multiply));
    assert_eq!(BlendMode::parse("color burn"), Some(BlendMode::ColorBurn));
    assert_eq!(BlendMode::parse("ColorBurn"), Some(BlendMode::ColorBurn));
    assert_eq!(BlendMode::parse("soft_light"), Some(BlendMode::SoftLight));
    assert_eq!(BlendMode::parse("shade"), None);
}

/// A shape drawn with a mode **mixes** with what is under it.
#[test]
fn a_shape_mixes_with_what_is_under_it() {
    // A **red** shape over a **blue** one: the plain mode leaves the
    // red, and the multiply mode leaves something darker than either.
    let mut plain = a_canvas(100, 100);
    plain.rect(a_rect(80.0, 80.0), Fill::Solid(Colour([0.0, 0.0, 1.0, 1.0])), BlendMode::Normal);
    plain.rect(a_rect(80.0, 80.0), Fill::Solid(Colour([1.0, 0.0, 0.0, 1.0])), BlendMode::Normal);
    let plain_pixel = pixel(plain.image(), 60, 20).expect("the pixel is there");

    let mut multiplied = a_canvas(100, 100);
    multiplied.rect(a_rect(80.0, 80.0), Fill::Solid(Colour([0.0, 0.0, 1.0, 1.0])), BlendMode::Normal);
    multiplied.rect(a_rect(80.0, 80.0), Fill::Solid(Colour([1.0, 0.0, 0.0, 1.0])), BlendMode::Multiply);
    let multiplied_pixel = pixel(multiplied.image(), 60, 20).expect("the pixel is there");

    assert_eq!(plain_pixel, [255, 0, 0, 255], "the plain mode lays the red over");
    assert!(
        multiplied_pixel[0] < plain_pixel[0] && multiplied_pixel[2] < 128,
        "the multiply mode mixes the two: {multiplied_pixel:?}"
    );
}

/// A shape that is **transparent** mixes with what is under it, and the
/// mix is with the backdrop's own opacity.
#[test]
fn a_transparent_shape_lets_the_backdrop_through() {
    let mut canvas = a_canvas(100, 100);
    canvas.rect(a_rect(80.0, 80.0), Fill::Solid(Colour([0.0, 0.0, 1.0, 1.0])), BlendMode::Normal);
    canvas.rect(a_rect(80.0, 80.0), Fill::Solid(Colour([1.0, 0.0, 0.0, 0.5])), BlendMode::Normal);
    let mixed = pixel(canvas.image(), 60, 20).expect("the pixel is there");
    assert!(mixed[0] > 100 && mixed[0] < 160, "the red is half over the blue: {mixed:?}");
    assert!(mixed[2] > 100 && mixed[2] < 160, "and the blue is half there: {mixed:?}");
}

/// The mode is the kit's own, and it is the same wherever it is used.
#[test]
fn the_mode_is_the_same_everywhere() {
    let backdrop = Colour([0.5, 0.5, 0.5, 1.0]);
    let source = Colour([0.5, 0.5, 0.5, 1.0]);
    assert_eq!(
        BlendMode::Normal.mix(backdrop, source),
        BlendMode::Normal.mix(backdrop, source),
        "the mode is a value, and a value is the same twice"
    );
    assert_ne!(
        BlendMode::Multiply.mix(backdrop, source),
        BlendMode::Normal.mix(backdrop, source),
        "and the modes are not the same"
    );
    // A point is where a shape is; the mode does not care.
    let _ = Point::new(0.0, 0.0);
}
