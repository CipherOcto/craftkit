//! The **PNG reader**: the pixels, and the inflate they come in.
//!
//! The pictures here are written by two writers: the kit's own, which
//! stores its data and does not compress it, and the `image` crate's,
//! which compresses it — and a compressed picture is what exercises the
//! Huffman paths of the inflate.

use craftkit_png::{Limits, read};

/// A PNG of that size, with a colour in every pixel.
///
/// The colours are a function of the pixel's place, so a picture that is
/// read back is not just the right *size* but the right *picture*.
fn a_png(width: u32, height: u32) -> Vec<u8> {
    let mut picture = image::RgbImage::new(width, height);
    for (x, y, pixel) in picture.enumerate_pixels_mut() {
        *pixel = image::Rgb([(x * 7 % 256) as u8, (y * 11 % 256) as u8, 128]);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(picture)
        .write_to(&mut out, image::ImageFormat::Png)
        .expect("the picture is written");
    out.into_inner()
}

/// A picture the kit wrote itself is read back.
///
/// The kit's writer **stores** its data and does not compress it, so
/// this is the stored path of the inflate.
#[test]
fn a_picture_the_kit_wrote_is_read_back() {
    let mut written = craftkit_raster::Image::new(4, 3, craftkit_sdk::Colour::WHITE);
    written.blend(1, 1, craftkit_sdk::Colour::BLACK);
    let bytes = written.to_png();
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.width, 4);
    assert_eq!(picture.height, 3);
    assert_eq!(picture.channels, 4);
    assert_eq!(picture.samples.len(), 4 * 3 * 4);
    // The black pixel is where it was put.
    // The black pixel is the second of the first row.
    let at = 5 * 4;
    assert_eq!(&picture.samples[at..at + 4], &[0, 0, 0, 255]);
}

/// A **compressed** picture is read back.
///
/// This is the Huffman path of the inflate, and the picture is the
/// picture: every pixel, in the place it was put.
#[test]
fn a_compressed_picture_is_read_back() {
    let bytes = a_png(9, 5);
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.width, 9);
    assert_eq!(picture.height, 5);
    assert_eq!(picture.channels, 3);
    assert_eq!(picture.samples.len(), 9 * 5 * 3);
    for y in 0..5u32 {
        for x in 0..9u32 {
            let at = ((y * 9 + x) * 3) as usize;
            assert_eq!(picture.samples[at], (x * 7 % 256) as u8, "the red of {x},{y}");
            assert_eq!(picture.samples[at + 1], (y * 11 % 256) as u8, "the green of {x},{y}");
            assert_eq!(picture.samples[at + 2], 128, "the blue of {x},{y}");
        }
    }
}

/// Bytes that are not a PNG are refused.
#[test]
fn bytes_that_are_not_a_png_are_refused() {
    let error = read(b"not a picture at all", &Limits::default()).expect_err("that is not a PNG");
    assert!(error.to_string().contains("not a PNG"), "{error}");
}

/// A picture with a broken chunk is refused.
///
/// The checksum is the picture's own word about itself, and a picture
/// whose word does not match is a picture that is broken.
#[test]
fn a_picture_with_a_broken_chunk_is_refused() {
    let mut bytes = a_png(4, 4);
    // The header's checksum is the fourth chunk's bytes: break one.
    let at = 8 + 4 + 4 + 13;
    bytes[at] = bytes[at].wrapping_add(1);
    let error = read(&bytes, &Limits::default()).expect_err("that picture is broken");
    assert!(error.to_string().contains("checksum"), "{error}");
}

/// A picture that is cut short is refused.
#[test]
fn a_picture_that_is_cut_short_is_refused() {
    let mut bytes = a_png(8, 8);
    bytes.truncate(bytes.len() / 2);
    assert!(read(&bytes, &Limits::default()).is_err(), "a broken picture was read");
}

/// An **interlaced** picture is refused, and it is refused by name.
///
/// Adam7 is a different shape of file, and the reader says so instead
/// of drawing half of it.
#[test]
fn an_interlaced_picture_is_refused() {
    let mut bytes = a_png(4, 4);
    // The header's twelfth byte is the interlace flag, and the chunk's
    // checksum is written again afterwards: a broken checksum is a
    // different refusal.
    let at = 8 + 4 + 4 + 12;
    bytes[at] = 1;
    let checksum = craftkit_png::crc32(&bytes[12..16], &bytes[16..29]);
    bytes[29..33].copy_from_slice(&checksum.to_be_bytes());
    let error = read(&bytes, &Limits::default()).expect_err("that picture is interlaced");
    assert!(error.to_string().contains("interlaced"), "{error}");
}
