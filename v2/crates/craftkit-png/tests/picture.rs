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

/// A PNG of **four-bit greyscale**, written by hand.
///
/// The samples of a four-bit picture are packed two to a byte, the
/// most significant bits first, and the rows are stored the way PNG
/// stores them: a filter byte and then the row.
fn a_four_bit_grey_png() -> Vec<u8> {
    let mut rows: Vec<u8> = Vec::new();
    // The first row: eight greys, two to a byte, the most significant
    // bits first.
    rows.push(0);
    rows.extend_from_slice(&[0x01, 0x23, 0x45, 0x67]);
    // The second row: the same, the other way round.
    rows.push(0);
    rows.extend_from_slice(&[0xBA, 0xDC, 0xFE, 0x10]);
    let mut out = Vec::new();
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    let mut header = Vec::new();
    header.extend_from_slice(&8u32.to_be_bytes());
    header.extend_from_slice(&2u32.to_be_bytes());
    header.extend_from_slice(&[4, 0, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    chunk(&mut out, b"IDAT", &a_zlib(&rows));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// A zlib stream of that data, stored and not compressed.
fn a_zlib(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(data).expect("the data is written");
    encoder.finish().expect("the stream is finished")
}

/// One chunk: its length, its name, its bytes and its checksum.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut checked = kind.to_vec();
    checked.extend_from_slice(data);
    out.extend_from_slice(&craftkit_png::crc32(kind, data).to_be_bytes());
}

/// A picture of **four-bit greyscale** is read, and its greys are
/// scaled up.
///
/// A four-bit grey is one of sixteen greys, and the picture that comes
/// out is eight-bit: the values are multiplied by seventeen, so that
/// the last one is white.
#[test]
fn a_picture_of_four_bit_greyscale_is_read() {
    let bytes = a_four_bit_grey_png();
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.width, 8);
    assert_eq!(picture.height, 2);
    assert_eq!(picture.channels, 1);
    assert_eq!(picture.samples.len(), 16, "there is a sample for every pixel");
    assert_eq!(picture.samples[0], 0, "black is black");
    assert_eq!(picture.samples[1], 17, "one is the first grey");
    assert_eq!(picture.samples[7], 119, "seven is the seventh grey");
    assert_eq!(picture.samples[8], 187, "the second row starts with eleven");
    assert_eq!(picture.samples[15], 0, "and ends with zero");
}

/// A PNG with a **palette**, written by hand.
///
/// The picture's samples are the palette's indices, and the colours are
/// what the picture is made of: the picture that comes out has three
/// samples a pixel, not one.
fn a_palette_png() -> Vec<u8> {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    // Four pixels: black, white, black, white.
    rows.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);
    let mut out = Vec::new();
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    let mut header = Vec::new();
    header.extend_from_slice(&4u32.to_be_bytes());
    header.extend_from_slice(&1u32.to_be_bytes());
    header.extend_from_slice(&[8, 3, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    // Two colours: black and white.
    chunk(&mut out, b"PLTE", &[0, 0, 0, 255, 255, 255]);
    chunk(&mut out, b"IDAT", &a_zlib(&rows));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// A picture with a palette is read, and its colours are the palette's.
#[test]
fn a_picture_with_a_palette_is_read() {
    let bytes = a_palette_png();
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.width, 4);
    assert_eq!(picture.height, 1);
    assert_eq!(picture.channels, 3, "a palette picture has three samples a pixel");
    assert_eq!(&picture.samples[0..3], &[0, 0, 0], "the first pixel is black");
    assert_eq!(&picture.samples[9..12], &[255, 255, 255], "the fourth is white");
}
