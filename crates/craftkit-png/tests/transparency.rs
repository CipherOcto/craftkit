//! The **transparency** of a picture: the `tRNS` chunk.
//!
//! PNG keeps a picture's see-through colours in one chunk, and what
//! the chunk means depends on the picture's colour type: one grey
//! value, one colour, or an alpha for each of a palette's entries.
//! Every picture here is written **by hand**, because no writer writes
//! a `tRNS` chunk on purpose — the format says a picture with a
//! transparency is a picture with an alpha channel, and the writers
//! write the alpha.

use craftkit_png::{Limits, read};

/// One chunk: its length, its name, its bytes and its checksum.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&craftkit_png::crc32(kind, data).to_be_bytes());
}

/// A zlib stream of that data, stored and not compressed.
fn a_zlib(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(data).expect("the data is written");
    encoder.finish().expect("the stream is finished")
}

/// A PNG of that size, colour type and rows, with the chunks that
/// come before the data.
fn a_png(
    width: u32,
    height: u32,
    depth: u8,
    colour: u8,
    before: &[(&[u8; 4], Vec<u8>)],
    rows: &[u8],
) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[depth, colour, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);
    for (kind, data) in before {
        chunk(&mut out, kind, data);
    }
    chunk(&mut out, b"IDAT", &a_zlib(rows));
    chunk(&mut out, b"IEND", &[]);
    out
}

/// A **palette** picture with a transparency.
///
/// The palette's entries are the colours, and the transparency says
/// how see-through each of them is: the first is not seen at all, and
/// the second is half seen.
#[test]
fn a_palette_picture_with_a_transparency_is_read() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    rows.extend_from_slice(&[0, 1]);
    let bytes = a_png(
        2,
        1,
        8,
        3,
        &[
            (b"PLTE", vec![0, 0, 0, 255, 255, 255]),
            // Two entries, two alphas: none, and half.
            (b"tRNS", vec![0, 128]),
        ],
        &rows,
    );
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.channels, 4, "the picture has an alpha now");
    assert_eq!(picture.samples.len(), 2 * 4, "a sample for every pixel and channel");
    assert_eq!(&picture.samples[0..4], &[0, 0, 0, 0], "the first entry is not seen at all");
    assert_eq!(&picture.samples[4..8], &[255, 255, 255, 128], "the second is half seen");
}

/// A transparency that says **less** than the palette.
///
/// The chunk names the entries that are see-through, and the rest are
/// fully opaque: a picture does not become see-through because the
/// file stopped talking about it.
#[test]
fn a_transparency_that_says_less_than_the_palette_is_read() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    rows.extend_from_slice(&[0, 1]);
    let bytes = a_png(2, 1, 8, 3, &[(b"PLTE", vec![0, 0, 0, 255, 255, 255]), (b"tRNS", vec![0])], &rows);
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.channels, 4, "the picture has an alpha now");
    assert_eq!(picture.samples[3], 0, "the entry the chunk names is not seen");
    assert_eq!(picture.samples[7], 255, "the entry it does not name is fully opaque");
}

/// A **grey** picture with a transparency.
///
/// The chunk names one grey, and that grey is not seen; the others
/// are.
#[test]
fn a_grey_picture_with_a_transparency_is_read() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    rows.extend_from_slice(&[64, 128]);
    // The grey that is not seen: sixty-four.
    let bytes = a_png(2, 1, 8, 0, &[(b"tRNS", vec![0, 64])], &rows);
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.channels, 2, "the grey has an alpha beside it");
    assert_eq!(picture.samples[0], 64, "the grey is the grey");
    assert_eq!(picture.samples[1], 0, "and it is not seen");
    assert_eq!(picture.samples[2], 128, "the other grey is the other grey");
    assert_eq!(picture.samples[3], 255, "and it is fully opaque");
}

/// A **colour** picture with a transparency.
///
/// The chunk names one colour, and that colour is not seen.
#[test]
fn a_colour_picture_with_a_transparency_is_read() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    rows.extend_from_slice(&[10, 20, 30, 40, 50, 60]);
    // The colour that is not seen: ten, twenty, thirty.
    let bytes = a_png(2, 1, 8, 2, &[(b"tRNS", vec![0, 10, 0, 20, 0, 30])], &rows);
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.channels, 4, "the colour has an alpha beside it");
    assert_eq!(&picture.samples[0..4], &[10, 20, 30, 0], "the named colour is not seen");
    assert_eq!(&picture.samples[4..8], &[40, 50, 60, 255], "the other colour is fully opaque");
}

/// A **sixteen-bit** grey picture with a transparency.
///
/// The chunk names a sixteen-bit value, and it is the **whole** value:
/// a grey that shares its high byte but not its low byte is a
/// different grey, and it is seen.
#[test]
fn a_sixteen_bit_grey_picture_with_a_transparency_is_read() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    // Two greys: 0x0100 and 0x01FF.
    rows.extend_from_slice(&[0x01, 0x00, 0x01, 0xFF]);
    let bytes = a_png(2, 1, 16, 0, &[(b"tRNS", vec![0x01, 0x00])], &rows);
    let picture = read(&bytes, &Limits::default()).expect("the picture is read");
    assert_eq!(picture.channels, 2, "the grey has an alpha beside it");
    assert_eq!(picture.samples[0], 0x01, "the grey comes out as eight bits");
    assert_eq!(picture.samples[1], 0, "the named grey is not seen");
    assert_eq!(picture.samples[2], 0x01, "the other grey has the same high byte");
    assert_eq!(picture.samples[3], 255, "and it is seen, because it is not the same grey");
}

/// A transparency in a picture that **already has** an alpha is
/// refused.
///
/// The picture has no use for it, and the format does not allow it.
#[test]
fn a_transparency_beside_an_alpha_is_refused() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    rows.extend_from_slice(&[10, 20, 30, 255]);
    let bytes = a_png(1, 1, 8, 6, &[(b"tRNS", vec![0, 10, 0, 20, 0, 30])], &rows);
    let error = read(&bytes, &Limits::default()).expect_err("that picture may not have one");
    assert!(error.to_string().contains("alpha channel"), "{error}");
}

/// A transparency that is **longer than the palette** is refused.
#[test]
fn a_transparency_longer_than_the_palette_is_refused() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    rows.extend_from_slice(&[0, 1]);
    let bytes =
        a_png(2, 1, 8, 3, &[(b"PLTE", vec![0, 0, 0, 255, 255, 255]), (b"tRNS", vec![0, 128, 64])], &rows);
    let error = read(&bytes, &Limits::default()).expect_err("that transparency is too long");
    assert!(error.to_string().contains("longer than the palette"), "{error}");
}

/// A grey transparency of the **wrong length** is refused.
#[test]
fn a_grey_transparency_of_the_wrong_length_is_refused() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    rows.extend_from_slice(&[64, 128]);
    let bytes = a_png(2, 1, 8, 0, &[(b"tRNS", vec![64])], &rows);
    let error = read(&bytes, &Limits::default()).expect_err("that transparency is the wrong size");
    assert!(error.to_string().contains("two bytes"), "{error}");
}

/// A transparency in a picture of **four bits** is refused.
///
/// The format does not allow it: a grey of four bits cannot name a
/// sixteen-bit value, and a palette of four bits says its alphas in
/// the palette's own chunk.
#[test]
fn a_transparency_in_a_four_bit_grey_is_refused() {
    let mut rows: Vec<u8> = Vec::new();
    rows.push(0);
    rows.extend_from_slice(&[0x01, 0x23]);
    let bytes = a_png(4, 1, 4, 0, &[(b"tRNS", vec![0, 64])], &rows);
    let error = read(&bytes, &Limits::default()).expect_err("that transparency may not be there");
    assert!(error.to_string().contains("eight and sixteen bits"), "{error}");
}
