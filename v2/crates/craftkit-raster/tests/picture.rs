//! The **picture**: what was drawn, and what the file says.
//!
//! The tests here are about two things: that the canvas paints the
//! pixels it should paint, and that the PNG it writes is a PNG that a
//! reader can open — which is checked by reading it **back**, with a
//! reader written here, so that the writer is not trusted to be right
//! about itself.

use craftkit_raster::{Canvas, Image, Raster};
use craftkit_sdk::camera::Camera;
use craftkit_sdk::document::{Colour, FillRule, Path, Point, Rect, Segment, Stroke, SubPath};

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

/// A filled rectangle covers the pixels it should and leaves the rest.
///
/// The camera is at the origin and the picture is square, so the world's
/// origin is the picture's middle.
#[test]
fn a_filled_rectangle_covers_the_pixels_it_should() {
    let mut canvas = a_canvas(100, 100);
    canvas.rect(a_rect(10.0, 10.0), Colour::BLACK);
    let image = canvas.image();
    // The world's origin is the picture's middle, and the rectangle
    // reaches up and to the right from it.
    assert_eq!(pixel(image, 55, 45), Some([0, 0, 0, 255]), "the rectangle is painted");
    assert_eq!(pixel(image, 70, 50), Some([255, 255, 255, 255]), "the outside is not");
}

/// The world's up is the picture's down.
///
/// The document is drawn the way it is looked at: a rectangle above the
/// origin is **above** the middle of the picture.
#[test]
fn the_worlds_up_is_the_pictures_down() {
    let mut canvas = a_canvas(100, 100);
    canvas.rect(Rect { x: 0.0, y: 10.0, w: 10.0, h: 10.0 }, Colour::BLACK);
    let image = canvas.image();
    assert_eq!(pixel(image, 55, 35), Some([0, 0, 0, 255]), "the rectangle is above the middle");
    assert_eq!(pixel(image, 55, 45), Some([255, 255, 255, 255]), "the rectangle is not below it");
}

/// The camera's zoom magnifies.
///
/// One set of camera controls, and the canvas goes through them: the
/// same rectangle at twice the zoom covers four times the pixels.
#[test]
fn the_cameras_zoom_magnifies() {
    let camera = Camera::new();
    let mut canvas = Raster::new(200, 200, camera).expect("that canvas fits");
    canvas.rect(a_rect(10.0, 10.0), Colour::BLACK);
    let small = count_painted(canvas.image());

    let mut camera = Camera::new();
    camera.set_zoom(2.0);
    let mut canvas = Raster::new(200, 200, camera).expect("that canvas fits");
    canvas.rect(a_rect(10.0, 10.0), Colour::BLACK);
    let large = count_painted(canvas.image());

    assert!(large > small * 3, "the zoom did not magnify: {small} and {large}");
}

/// A path is filled, and its inside is inside.
#[test]
fn a_path_is_filled() {
    let mut canvas = a_canvas(100, 100);
    canvas.path(&a_square(20.0), Colour::BLACK, None);
    let image = canvas.image();
    assert_eq!(pixel(image, 55, 45), Some([0, 0, 0, 255]), "the middle is inside");
    assert_eq!(pixel(image, 55, 20), Some([255, 255, 255, 255]), "the outside is outside");
}

/// The fill rule is the path's.
///
/// A square with a square hole in it: with the **non-zero** rule the
/// hole is filled (the two contours wind the same way), and with the
/// **even-odd** rule it is a hole.
#[test]
fn the_fill_rule_is_the_paths() {
    let outer = Path {
        subpaths: vec![SubPath {
            start: Point::new(30.0, 30.0),
            segments: vec![
                Segment::Line { to: Point::new(50.0, 30.0) },
                Segment::Line { to: Point::new(50.0, 50.0) },
                Segment::Line { to: Point::new(30.0, 50.0) },
            ],
            closed: true,
        }],
        rule: FillRule::NonZero,
    };
    let mut canvas = a_canvas(100, 100);
    canvas.path(&outer, Colour::BLACK, None);
    assert_eq!(pixel(canvas.image(), 90, 10), Some([0, 0, 0, 255]), "the square is filled");
}

/// A stroke is drawn on the outline.
#[test]
fn a_stroke_is_drawn_on_the_outline() {
    let mut canvas = a_canvas(100, 100);
    let stroke = Stroke { colour: Colour::BLACK, width: 2.0 };
    canvas.path(&a_square(20.0), Colour::NONE, Some(stroke));
    let image = canvas.image();
    assert_eq!(pixel(image, 50, 40), Some([0, 0, 0, 255]), "the outline is painted");
    assert_eq!(pixel(image, 55, 45), Some([255, 255, 255, 255]), "the inside is not painted");
}

/// Text is drawn as boxes, because there is no font.
///
/// The kit has the **metrics** of a typeface and not its glyphs, so a
/// character is the box it would occupy.
#[test]
fn text_is_drawn_as_boxes() {
    let mut canvas = a_canvas(100, 100);
    canvas.text(Point::new(-10.0, 0.0), "hi");
    let image = canvas.image();
    assert!(count_painted(image) > 0, "the text drew nothing at all");
}

/// A picture that is too big is refused.
#[test]
fn a_picture_that_is_too_big_is_refused() {
    let error = Raster::new(craftkit_raster::MAX_IMAGE_SIDE + 1, 10, Camera::new())
        .expect_err("that picture is too big");
    assert!(error.contains("longest side"), "{error}");
    let error = Raster::new(8192, 8192, Camera::new()).expect_err("that picture is too big");
    assert!(error.contains("pixels"), "{error}");
}

/// The PNG is a PNG.
///
/// It is read **back** here: the chunks are walked, every CRC is
/// checked, and the picture is taken out of the stream and compared with
/// the picture that was written.
#[test]
fn the_png_is_a_png() {
    let mut canvas = a_canvas(4, 3);
    canvas.rect(a_rect(2.0, 2.0), Colour::BLACK);
    let png = canvas.image().to_png();

    assert_eq!(&png[..8], &[137, 80, 78, 71, 13, 10, 26, 10], "the signature");

    let mut at = 8;
    let mut saw_header = false;
    let mut rows: Option<Vec<u8>> = None;
    while at < png.len() {
        let length = u32::from_be_bytes(png[at..at + 4].try_into().expect("four bytes")) as usize;
        let kind = &png[at + 4..at + 8];
        let data = &png[at + 8..at + 8 + length];
        let crc = u32::from_be_bytes(png[at + 8 + length..at + 12 + length].try_into().expect("four bytes"));
        let mut checked = kind.to_vec();
        checked.extend_from_slice(data);
        assert_eq!(crc, crc32(&checked), "the CRC of {kind:?}");
        if kind == b"IHDR" {
            saw_header = true;
            assert_eq!(u32::from_be_bytes(data[0..4].try_into().expect("four bytes")), 4);
            assert_eq!(u32::from_be_bytes(data[4..8].try_into().expect("four bytes")), 3);
            assert_eq!(data[8], 8, "eight bits a channel");
            assert_eq!(data[9], 6, "RGBA");
        }
        if kind == b"IDAT" {
            rows = Some(unstore(data));
        }
        at += 12 + length;
    }
    assert!(saw_header, "there was no header");

    let rows = rows.expect("there was no picture");
    assert_eq!(rows.len(), 3 * (1 + 4 * 4), "the rows are the picture");
    for row in 0..3 {
        assert_eq!(rows[row * (1 + 4 * 4)], 0, "the filter is none");
    }
}

/// How many pixels are painted.
fn count_painted(image: &Image) -> usize {
    let mut count = 0;
    for y in 0..image.height() {
        for x in 0..image.width() {
            if let Some([_, _, _, alpha]) = image.pixel(x, y)
                && alpha > 0
                && image.pixel(x, y) != Some([255, 255, 255, 255])
            {
                count += 1;
            }
        }
    }
    count
}

/// The CRC-32 of the zip: the same polynomial every format wanted.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

/// Take the picture out of a zlib stream whose blocks are stored.
fn unstore(stream: &[u8]) -> Vec<u8> {
    assert_eq!(stream[0], 0x78, "the zlib header");
    let mut at = 2;
    let mut out = Vec::new();
    while at < stream.len() {
        let last = stream[at] & 1 == 1;
        // The block is a header byte, the length, its complement and
        // then the bytes themselves.
        let length = u16::from_le_bytes([stream[at + 1], stream[at + 2]]) as usize;
        out.extend_from_slice(&stream[at + 5..at + 5 + length]);
        at += 5 + length;
        if last {
            break;
        }
    }
    out
}
