//! The **PNG writer**: a picture, as a file anybody can open.
//!
//! It is written here and not taken from a crate because the whole point
//! of the kit is that it has **no dependency it did not choose**, and a
//! PNG writer is a hundred lines of arithmetic: a signature, three
//! chunks, a CRC and a stored deflate stream.
//!
//! The stream is **stored** and not compressed. A PNG file that is
//! bigger than it needs to be is a PNG file that any reader can open,
//! and the reader is the thing that matters.

use crate::image::Image;

/// The eight bytes every PNG begins with.
const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Write the picture as a PNG file.
pub fn write(image: &Image) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&SIGNATURE);

    // The header: the size, eight bits a channel, and **RGBA** — the
    // fourth channel is the alpha, and a picture without one cannot say
    // what is behind it.
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&(image.width() as u32).to_be_bytes());
    header.extend_from_slice(&(image.height() as u32).to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &header);

    // The picture itself: the rows, in a zlib stream.
    chunk(&mut out, b"IDAT", &zlib_stored(&image.rows()));

    chunk(&mut out, b"IEND", &[]);
    out
}

/// One chunk: its length, its name, its bytes and its CRC.
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// The rows, in a zlib stream whose deflate part is **stored**.
fn zlib_stored(rows: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(rows.len() + rows.len() / 65535 * 5 + 16);
    // The zlib header: deflate, a window of thirty-two kibibytes, and
    // the check bits that say the header is not a coincidence.
    out.extend_from_slice(&[0x78, 0x01]);
    let mut at = 0;
    while at < rows.len() {
        let take = (rows.len() - at).min(65535);
        let last = at + take >= rows.len();
        out.push(if last { 1 } else { 0 });
        out.extend_from_slice(&(take as u16).to_le_bytes());
        out.extend_from_slice(&(!(take as u16)).to_le_bytes());
        out.extend_from_slice(&rows[at..at + take]);
        at += take;
    }
    out.extend_from_slice(&adler32(rows).to_be_bytes());
    out
}

/// The CRC a PNG chunk ends with.
///
/// The CRC-32 of the zip: the same polynomial every format that wanted
/// a checksum ended up with.
pub fn crc32(bytes: &[u8]) -> u32 {
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

/// The checksum a zlib stream ends with.
fn adler32(bytes: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for byte in bytes {
        a = (a + u32::from(*byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}
