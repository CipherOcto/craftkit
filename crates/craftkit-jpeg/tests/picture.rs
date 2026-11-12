//! The decoder, on a picture made by **hand**.
//!
//! A decoder is tested against files that were written, and the way to
//! have one without an encoder is to write the bytes by hand: a baseline
//! picture of one grey block, with the smallest Huffman tables a picture
//! can have.
//!
//! There is no encoder here, and no dependency: the bytes are written
//! out, segment by segment, the way ITU-T T.81 says they are laid out.

use craftkit_jpeg::{Decoded, Limits, Samples, decode, frame};

/// A baseline JPEG of one grey block.
///
/// The quantisation table says every coefficient is sixteen, the two
/// Huffman tables have one code each — the code for "the difference is
/// zero" and the code for "no more coefficients" — and the entropy
/// coded data is those two codes and then the padding bits.
fn a_grey_jpeg() -> Vec<u8> {
    let mut bytes = Vec::new();
    // The start of the picture.
    bytes.extend_from_slice(&[0xFF, 0xD8]);
    // The quantisation table: one table, eight bits, every value sixteen.
    bytes.extend_from_slice(&[0xFF, 0xDB]);
    bytes.extend_from_slice(&67u16.to_be_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(&[16u8; 64]);
    // The frame: eight bits, eight by eight, one component.
    bytes.extend_from_slice(&[0xFF, 0xC0]);
    bytes.extend_from_slice(&11u16.to_be_bytes());
    bytes.push(8);
    bytes.extend_from_slice(&8u16.to_be_bytes());
    bytes.extend_from_slice(&8u16.to_be_bytes());
    bytes.push(1);
    bytes.push(1);
    bytes.push(0x11);
    bytes.push(0);
    // The table of the differences: one code of two bits, for the value
    // zero. The canonical code of the first code of a length is that
    // many zero bits.
    bytes.extend_from_slice(&[0xFF, 0xC4]);
    bytes.extend_from_slice(&20u16.to_be_bytes());
    bytes.push(0x00);
    bytes.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    bytes.push(0);
    // The table of the coefficients: one code of two bits, for the end
    // of the block.
    bytes.extend_from_slice(&[0xFF, 0xC4]);
    bytes.extend_from_slice(&20u16.to_be_bytes());
    bytes.push(0x10);
    bytes.extend_from_slice(&[0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    bytes.push(0x00);
    // The scan: one component, the two tables above, every coefficient.
    bytes.extend_from_slice(&[0xFF, 0xDA]);
    bytes.extend_from_slice(&8u16.to_be_bytes());
    bytes.push(1);
    bytes.push(1);
    bytes.push(0x00);
    bytes.push(0);
    bytes.push(63);
    bytes.push(0);
    // The entropy coded data: "the difference is zero" and "no more
    // coefficients", and then the padding bits.
    bytes.push(0x0F);
    // The end of the picture.
    bytes.extend_from_slice(&[0xFF, 0xD9]);
    bytes
}

/// The samples of a decoded picture, as eight-bit numbers.
fn eight_bit(decoded: &Decoded) -> &[u8] {
    match &decoded.samples {
        Samples::U8(samples) => samples,
        Samples::U16(_) => panic!("the picture is not eight-bit"),
    }
}

/// A picture of one grey block is decoded, and it is mid grey.
///
/// The block has one coefficient — the average, which is zero — so every
/// pixel is the middle of the range.
#[test]
fn a_picture_of_one_grey_block_is_decoded() {
    let bytes = a_grey_jpeg();
    let decoded = decode(&bytes, &Limits::default(), None).expect("the picture is decoded");
    assert_eq!(decoded.width, 8);
    assert_eq!(decoded.height, 8);
    assert_eq!(decoded.components, 1);
    assert!(!decoded.damaged, "the picture was damaged");
    let samples = eight_bit(&decoded);
    assert_eq!(samples.len(), 64, "there is a sample for every pixel");
    assert!(samples.iter().all(|sample| *sample == 128), "the block is mid grey");
}

/// The frame is found without decoding it.
///
/// It is the header, and a header is cheap: an application can route
/// every picture it opens on the frame's marker alone.
#[test]
fn the_frame_is_found_without_decoding_it() {
    let bytes = a_grey_jpeg();
    let kind = frame(&bytes).expect("the frame is there");
    assert_eq!(kind.marker, 0xC0);
    assert_eq!(kind.precision, 8);
}

/// Bytes that are not a picture are refused, and not a crash.
#[test]
fn bytes_that_are_not_a_picture_are_refused() {
    let error = decode(b"not a picture at all", &Limits::default(), None).expect_err("that is not a picture");
    assert!(matches!(error, craftkit_jpeg::Error::Malformed(_)), "{error:?}");
}

/// The **limits are the caller's**.
///
/// A picture that is larger than the caller allows is refused before any
/// buffer is made: the decoder does not decide how much memory a program
/// may use.
#[test]
fn a_picture_over_the_limits_is_refused() {
    let mut limits = Limits::default();
    limits.max_pixels = 4;
    let error = decode(&a_grey_jpeg(), &limits, None).expect_err("that picture is too big");
    assert!(matches!(error, craftkit_jpeg::Error::LimitExceeded(_)), "{error:?}");
}

/// A picture that is cut short is **damaged**, not refused.
///
/// What was decoded is kept: a broken picture is worth showing, and the
/// caller is told that it is broken.
#[test]
fn a_picture_that_is_cut_short_is_damaged() {
    let mut bytes = a_grey_jpeg();
    bytes.truncate(bytes.len() - 3);
    match decode(&bytes, &Limits::default(), None) {
        Ok(decoded) => assert!(decoded.damaged, "the picture was not marked as damaged"),
        Err(craftkit_jpeg::Error::Malformed(_)) => {}
        Err(error) => panic!("unexpected: {error:?}"),
    }
}
