//! The **PNG reader**: the pixels of a picture, from the file.
//!
//! # Why it exists
//!
//! The kit writes PNG (§67) and reads a PNG's *header* (§68), and the
//! pixels were out of reach: a PNG's pixels are in a **zlib** stream,
//! and the kit had no inflate. So there was no way to *look* at a
//! picture the kit had made, and no way to look at one anybody else
//! had made.
//!
//! # What it is
//!
//! - the **chunks**: the signature, the header, the palette, the data
//!   and the end, each with its CRC checked;
//! - the **inflate**: the zlib stream, from [`inflate`];
//! - the **filters**: the five ways a row may be stored, undone;
//! - the **pixels**: the samples, as they are.
//!
//! # What it does not do
//!
//! - **It does not do interlaced pictures.** Adam7 is a different shape
//!   of file, and it is a refusal with a name.
//! - **It does not do bit depths of one, two and four bits.** Eight and
//!   sixteen are done; the others are a refusal with a name.
//! - **It does not do the text and the time.** A picture may carry them,
//!   and they are left in the file.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod inflate;

pub use inflate::{Error, Limits, adler32};

/// The eight bytes every PNG begins with.
const SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

/// Inflate a zlib stream.
///
/// It is here so that a caller can inflate a stream that is not a
/// picture's: the reader below is the only user of it, and the tests
/// are its own.
pub fn inflate_bytes(bytes: &[u8], limits: &Limits) -> Result<Vec<u8>, Error> {
    inflate::zlib(bytes, limits)
}

/// A picture, read from a PNG file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Picture {
    /// How wide it is, in pixels.
    pub width: u32,
    /// How tall it is, in pixels.
    pub height: u32,
    /// How many samples a pixel has: one for grey, three for a colour
    /// picture, four when there is an alpha.
    pub channels: u8,
    /// The samples, interleaved by pixel, the rows top to bottom.
    ///
    /// A sixteen-bit picture comes out as **eight-bit**: the high byte of
    /// each sample, which is what a canvas wants and what a document
    /// holds.
    pub samples: Vec<u8>,
}

impl Picture {
    /// How many pixels it has.
    pub fn pixels(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }
}

/// The picture in those bytes.
///
/// The bytes are the whole file: the picture is in them, and the file
/// says how big it is.
pub fn read(bytes: &[u8], limits: &Limits) -> Result<Picture, Error> {
    if !bytes.starts_with(&SIGNATURE) {
        return Err(Error::Malformed("these bytes are not a PNG".to_string()));
    }
    let mut at = SIGNATURE.len();
    let mut width = 0u32;
    let mut height = 0u32;
    let mut depth = 0u8;
    let mut colour = 0u8;
    let mut interlaced = false;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut data: Vec<u8> = Vec::new();
    let mut saw_header = false;
    let mut saw_end = false;
    while at + 8 <= bytes.len() {
        let length = u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]) as usize;
        let kind = &bytes[at + 4..at + 8];
        let start = at + 8;
        let end = start + length;
        let Some(chunk) = bytes.get(start..end) else {
            return Err(Error::Malformed("a chunk is shorter than it says".to_string()));
        };
        let Some(crc_bytes) = bytes.get(end..end + 4) else {
            return Err(Error::Malformed("a chunk has no checksum".to_string()));
        };
        let crc = u32::from_be_bytes([crc_bytes[0], crc_bytes[1], crc_bytes[2], crc_bytes[3]]);
        if crc != crc32(kind, chunk) {
            return Err(Error::Malformed("a chunk's checksum does not match".to_string()));
        }
        match kind {
            b"IHDR" => {
                if chunk.len() < 13 {
                    return Err(Error::Malformed("the header is too short".to_string()));
                }
                width = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                height = u32::from_be_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
                depth = chunk[8];
                colour = chunk[9];
                interlaced = chunk[12] != 0;
                saw_header = true;
            }
            b"PLTE" => {
                let mut entries: Vec<[u8; 3]> = Vec::new();
                for entry in chunk.chunks(3) {
                    if entry.len() < 3 {
                        break;
                    }
                    entries.push([entry[0], entry[1], entry[2]]);
                }
                palette = entries;
            }
            b"IDAT" => {
                data.extend_from_slice(chunk);
            }
            b"IEND" => saw_end = true,
            _ => {}
        }
        at = end + 4;
        if saw_end {
            break;
        }
    }
    if !saw_header {
        return Err(Error::Malformed("the picture has no header".to_string()));
    }
    if !saw_end {
        return Err(Error::Malformed("the picture has no end".to_string()));
    }
    if width == 0 || height == 0 {
        return Err(Error::Malformed("the picture has no size".to_string()));
    }
    if interlaced {
        return Err(Error::Unsupported("the picture is interlaced".to_string()));
    }
    if !matches!(depth, 8 | 16) {
        return Err(Error::Unsupported("the picture is not eight or sixteen bits".to_string()));
    }
    let channels = match colour {
        0 => 1,
        2 => 3,
        3 => 1,
        4 => 2,
        6 => 4,
        _ => {
            return Err(Error::Unsupported(
                "the picture has a colour type this reader does not know".to_string(),
            ));
        }
    };
    if colour == 3 && palette.is_empty() {
        return Err(Error::Malformed("the picture has a palette and no palette".to_string()));
    }
    let raw = inflate::zlib(&data, limits)?;
    let samples = unfilter(&raw, width as usize, height as usize, channels, depth, &palette)?;
    Ok(Picture { width, height, channels, samples })
}

/// Undo the filters of the rows.
///
/// Every row is stored as the difference from the row above it, or from
/// the pixel to the left, and the five filters say which. The bytes per
/// pixel are what the filters count: one for a picture of eight-bit
/// samples, two for a sixteen-bit one.
fn unfilter(
    raw: &[u8],
    width: usize,
    height: usize,
    channels: u8,
    depth: u8,
    palette: &[[u8; 3]],
) -> Result<Vec<u8>, Error> {
    let bytes_per_pixel = (channels as usize) * (depth as usize / 8);
    let row_bytes = width * bytes_per_pixel;
    if raw.len() < height * (row_bytes + 1) {
        return Err(Error::Malformed("the picture's rows are shorter than they should be".to_string()));
    }
    let mut out: Vec<u8> = Vec::with_capacity(height * row_bytes);
    let mut previous = vec![0u8; row_bytes];
    let mut current = vec![0u8; row_bytes];
    for row in 0..height {
        let start = row * (row_bytes + 1);
        let filter = raw[start];
        let source = &raw[start + 1..start + 1 + row_bytes];
        current.copy_from_slice(source);
        for index in 0..row_bytes {
            let left = if index >= bytes_per_pixel { current[index - bytes_per_pixel] } else { 0 };
            let up = previous[index];
            let up_left = if index >= bytes_per_pixel { previous[index - bytes_per_pixel] } else { 0 };
            current[index] = match filter {
                0 => current[index],
                1 => current[index].wrapping_add(left),
                2 => current[index].wrapping_add(up),
                3 => current[index].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => current[index].wrapping_add(paeth(left, up, up_left)),
                _ => {
                    return Err(Error::Malformed(
                        "the row has a filter this reader does not know".to_string(),
                    ));
                }
            };
        }
        out.extend_from_slice(&current);
        previous.copy_from_slice(&current);
    }
    // A palette picture's samples are the palette's **indices**, and the
    // colours are what the picture is made of.
    if !palette.is_empty() && channels == 1 && depth == 8 {
        let mut colours = Vec::with_capacity(out.len() * 3);
        for index in out {
            let entry = palette
                .get(index as usize)
                .ok_or_else(|| Error::Malformed("a pixel is outside the palette".to_string()))?;
            colours.extend_from_slice(entry);
        }
        return Ok(colours);
    }
    if depth == 16 {
        // The high byte of each sample: what a canvas wants.
        let mut eight = Vec::with_capacity(out.len() / 2);
        for index in (0..out.len()).step_by(2) {
            eight.push(out[index]);
        }
        return Ok(eight);
    }
    Ok(out)
}

/// The Paeth predictor of PNG's fourth filter.
///
/// It guesses the pixel from the one to the left, the one above and the
/// one above and to the left, and picks the guess that is nearest.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let estimate = i16::from(left) + i16::from(up) - i16::from(up_left);
    let left_distance = (estimate - i16::from(left)).abs();
    let up_distance = (estimate - i16::from(up)).abs();
    let up_left_distance = (estimate - i16::from(up_left)).abs();
    if left_distance <= up_distance && left_distance <= up_left_distance {
        left
    } else if up_distance <= up_left_distance {
        up
    } else {
        up_left
    }
}

/// The CRC a chunk ends with.
///
/// It is the CRC-32 of the zip: the same polynomial every format that
/// wanted a checksum ended up with.
pub fn crc32(kind: &[u8], data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for byte in kind.iter().chain(data.iter()) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}
