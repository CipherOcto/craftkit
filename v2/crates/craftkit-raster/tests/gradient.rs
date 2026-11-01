//! The **gradient**: a fill whose colours change.
//!
//! The gradient is the kit's own — the colours, the stops they are at,
//! and where the gradient is put — and the raster draws it: every pixel
//! is asked where it is along the gradient, and the answer is its
//! colour.

use craftkit_raster::{Canvas, Image, Raster};
use craftkit_sdk::camera::Camera;
use craftkit_sdk::document::{
    BlendMode, Colour, Fill, Gradient, GradientGeometry, GradientKind, GradientStop, Path, Point, Rect,
};

/// A canvas of that size, with a camera at the origin.
fn a_canvas(width: usize, height: usize) -> Raster {
    Raster::new(width, height, Camera::new()).expect("that canvas fits")
}

/// A rectangle of that size, at the origin.
fn a_rect(width: f64, height: f64) -> Rect {
    Rect { x: 0.0, y: 0.0, w: width, h: height }
}

/// A square path of that size, at the origin.
fn a_square(size: f64) -> Path {
    Path::polygon(&[
        Point::new(0.0, 0.0),
        Point::new(size, 0.0),
        Point::new(size, size),
        Point::new(0.0, size),
    ])
}

/// The colour of a pixel, or `None` if it is not there.
fn pixel(image: &Image, x: usize, y: usize) -> Option<[u8; 4]> {
    image.pixel(x, y)
}

/// A **linear** gradient from black to white, across the world.
fn black_to_white() -> Gradient {
    Gradient {
        kind: GradientKind::Linear,
        stops: vec![
            GradientStop { offset: 0.0, colour: Colour::BLACK, opacity: 1.0 },
            GradientStop { offset: 1.0, colour: Colour::WHITE, opacity: 1.0 },
        ],
        geometry: GradientGeometry::Linear { from: Point::new(0.0, 40.0), to: Point::new(50.0, 40.0) },
    }
}

/// The rectangle is painted with the **gradient**: the left is dark and
/// the right is light.
#[test]
fn a_linear_gradient_changes_across_the_shape() {
    let mut canvas = a_canvas(100, 100);
    canvas.rect(a_rect(80.0, 80.0), Fill::Gradient(black_to_white()), BlendMode::Normal);
    let image = canvas.image();
    // The camera is at the origin and the picture is square, so the
    // world's origin is the picture's middle, and the world's up is the
    // picture's down. The rectangle reaches up and to the right from
    // the origin, so it is the upper right quarter of the picture.
    let left = pixel(image, 51, 10).expect("the pixel is there");
    let right = pixel(image, 95, 10).expect("the pixel is there");
    assert!(left[0] < right[0], "the left is darker than the right: {left:?} and {right:?}");
    assert!(left[0] < 40, "the left is nearly black: {left:?}");
    assert!(right[0] > 200, "the right is nearly white: {right:?}");
}

/// A gradient on a **path** is drawn like a gradient on a rectangle.
#[test]
fn a_gradient_is_drawn_on_a_path() {
    let mut canvas = a_canvas(100, 100);
    canvas.path(&a_square(80.0), Fill::Gradient(black_to_white()), BlendMode::Normal, None);
    let image = canvas.image();
    let left = pixel(image, 51, 10).expect("the pixel is there");
    let right = pixel(image, 95, 10).expect("the pixel is there");
    assert!(left[0] < right[0], "the path is painted with the gradient: {left:?} and {right:?}");
}

/// A **radial** gradient is dark in the middle and light at the edge.
#[test]
fn a_radial_gradient_changes_outwards() {
    let mut canvas = a_canvas(100, 100);
    let gradient = Gradient {
        kind: GradientKind::Radial,
        stops: vec![
            GradientStop { offset: 0.0, colour: Colour::BLACK, opacity: 1.0 },
            GradientStop { offset: 1.0, colour: Colour::WHITE, opacity: 1.0 },
        ],
        geometry: GradientGeometry::Radial { centre: Point::new(25.0, 25.0), radius: 25.0 },
    };
    canvas.rect(a_rect(80.0, 80.0), Fill::Gradient(gradient), BlendMode::Normal);
    let image = canvas.image();
    let middle = pixel(image, 75, 25).expect("the pixel is there");
    let edge = pixel(image, 95, 25).expect("the pixel is there");
    assert!(middle[0] < edge[0], "the middle is darker than the edge: {middle:?} and {edge:?}");
}

/// The colours between the stops are **mixed**.
#[test]
fn the_colours_are_mixed_between_the_stops() {
    let gradient = black_to_white();
    let middle = gradient.colour_at_parameter(0.5);
    assert!(middle.0[0] > 0.4 && middle.0[0] < 0.6, "the middle is between the two: {middle:?}");
    // Before the first stop is the first stop's colour, and after the
    // last is the last one's.
    assert_eq!(gradient.colour_at_parameter(-1.0), Colour::BLACK, "before the first stop");
    assert_eq!(gradient.colour_at_parameter(2.0), Colour::WHITE, "after the last stop");
}
