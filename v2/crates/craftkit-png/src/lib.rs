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
//! - the **pixels**: the samples, as they are;
//! - the **transparency**: the `tRNS` chunk, which says which of a
//!   picture's colours are see-through.
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
    let mut transparency: Option<Vec<u8>> = None;
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
            b"tRNS" => {
                // The chunk's **meaning** depends on the picture's
                // colour type, and the header may come after it, so
                // the bytes are kept as they are and read below.
                transparency = Some(chunk.to_vec());
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
    if !matches!(depth, 1 | 2 | 4 | 8 | 16) {
        return Err(Error::Unsupported("the picture's bit depth is not one of the five".to_string()));
    }
    if depth < 8 && !matches!(colour, 0 | 3) {
        return Err(Error::Unsupported(
            "only a grey picture and a palette may be fewer than eight bits".to_string(),
        ));
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
    let transparency = transparency_for(colour, depth, transparency, palette.len())?;
    let raw = inflate::zlib(&data, limits)?;
    let stored = unfilter(&raw, width as usize, height as usize, channels, depth)?;
    let samples =
        compose(stored, width as usize, height as usize, depth, colour, &palette, transparency.as_ref())?;
    // A palette picture's samples are the palette's **indices**, and the
    // colours are what the picture is made of: the picture that comes
    // out has three samples a pixel, not one. A picture with a
    // transparency has an alpha as well.
    let channels = match colour {
        0 => {
            if transparency.is_some() {
                2
            } else {
                1
            }
        }
        2 => {
            if transparency.is_some() {
                4
            } else {
                3
            }
        }
        3 => {
            if transparency.is_some() {
                4
            } else {
                3
            }
        }
        // A picture that already has an alpha channel keeps it, and
        // the colour type says how many samples a pixel has.
        _ => channels,
    };
    Ok(Picture { width, height, channels, samples })
}

/// The **transparency** a picture carries.
///
/// PNG keeps it in one chunk, `tRNS`, and what the chunk means depends
/// on the picture's colour type: for a grey picture it is one grey
/// value, for a colour picture one colour, and for a palette picture
/// it is an alpha for each of the palette's entries.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Transparency {
    /// One grey value that is see-through.
    Grey(u16),
    /// One colour that is see-through.
    Colour([u16; 3]),
    /// The palette's entries and their alphas.
    Palette(Vec<u8>),
}

/// Read the `tRNS` chunk, now that the picture's colour type is known.
///
/// The chunk is **only** allowed where the format allows it: a grey or
/// a colour picture of eight or sixteen bits, and a palette picture.
/// A picture that already has an alpha channel may not have one as
/// well, because it has no use for it.
fn transparency_for(
    colour: u8,
    depth: u8,
    bytes: Option<Vec<u8>>,
    palette_len: usize,
) -> Result<Option<Transparency>, Error> {
    let Some(bytes) = bytes else { return Ok(None) };
    match colour {
        0 => {
            if bytes.len() != 2 {
                return Err(Error::Malformed("a grey picture's transparency is two bytes".to_string()));
            }
            if !matches!(depth, 8 | 16) {
                return Err(Error::Malformed(
                    "a grey picture's transparency is only for eight and sixteen bits".to_string(),
                ));
            }
            Ok(Some(Transparency::Grey(u16::from_be_bytes([bytes[0], bytes[1]]))))
        }
        2 => {
            if bytes.len() != 6 {
                return Err(Error::Malformed("a colour picture's transparency is six bytes".to_string()));
            }
            if !matches!(depth, 8 | 16) {
                return Err(Error::Malformed(
                    "a colour picture's transparency is only for eight and sixteen bits".to_string(),
                ));
            }
            let value = |at: usize| u16::from_be_bytes([bytes[at], bytes[at + 1]]);
            Ok(Some(Transparency::Colour([value(0), value(2), value(4)])))
        }
        3 => {
            if bytes.len() > palette_len {
                return Err(Error::Malformed("the transparency is longer than the palette".to_string()));
            }
            Ok(Some(Transparency::Palette(bytes)))
        }
        _ => Err(Error::Malformed(
            "a picture with an alpha channel may not have a transparency as well".to_string(),
        )),
    }
}

/// The samples of a picture that is **fewer than eight bits**.
///
/// A grey picture of one, two or four bits has its samples packed into
/// the bytes, most significant bits first, and a picture of fewer bits
/// than eight has its **grey** scaled up: a one-bit grey is black or
/// white and a four-bit grey is one of sixteen greys.
///
/// A palette picture's samples are the palette's **indices**, and the
/// colours are what the picture is made of.
fn unpack(samples: Vec<u8>, pixels: usize, depth: u8) -> Vec<u8> {
    let per_byte = 8 / depth as usize;
    let mask = (1u16 << depth) - 1;
    let mut values: Vec<u8> = Vec::with_capacity(pixels);
    for byte in samples {
        for index in 0..per_byte {
            let shift = 8 - depth as usize * (index + 1);
            let value = ((byte as u16) >> shift) & mask;
            values.push(value as u8);
        }
    }
    values.truncate(pixels);
    values
}

/// The picture's samples, as they come out of the file.
///
/// This is where a picture becomes what it is: a palette's indices
/// become the palette's colours, a grey of fewer than eight bits is
/// scaled up, and a transparency becomes an alpha beside every pixel.
fn compose(
    stored: Vec<u8>,
    width: usize,
    height: usize,
    depth: u8,
    colour: u8,
    palette: &[[u8; 3]],
    transparency: Option<&Transparency>,
) -> Result<Vec<u8>, Error> {
    let pixels = width * height;
    if colour == 3 {
        return compose_palette(stored, pixels, depth, palette, transparency);
    }
    compose_colour(stored, pixels, depth, colour, transparency)
}

/// A **palette** picture: the samples are indices, and the colours are
/// what the palette says.
fn compose_palette(
    stored: Vec<u8>,
    pixels: usize,
    depth: u8,
    palette: &[[u8; 3]],
    transparency: Option<&Transparency>,
) -> Result<Vec<u8>, Error> {
    let indices: Vec<u8> = if depth < 8 { unpack(stored, pixels, depth) } else { stored };
    let alphas: Option<&[u8]> = match transparency {
        Some(Transparency::Palette(alphas)) => Some(alphas),
        _ => None,
    };
    let mut out: Vec<u8> = Vec::with_capacity(pixels * if alphas.is_some() { 4 } else { 3 });
    for index in indices.iter().take(pixels) {
        let entry = palette
            .get(usize::from(*index))
            .ok_or_else(|| Error::Malformed("a pixel is outside the palette".to_string()))?;
        out.extend_from_slice(entry);
        if let Some(alphas) = alphas {
            // A palette entry the transparency does not mention is
            // fully opaque: the chunk says how many entries are see-
            // through, and the rest are not.
            out.push(alphas.get(usize::from(*index)).copied().unwrap_or(255));
        }
    }
    Ok(out)
}

/// A **grey** or **colour** picture.
fn compose_colour(
    stored: Vec<u8>,
    pixels: usize,
    depth: u8,
    colour: u8,
    transparency: Option<&Transparency>,
) -> Result<Vec<u8>, Error> {
    if depth < 8 {
        // A grey picture of fewer than eight bits: the samples are
        // packed into the bytes, and each one is scaled up to eight
        // bits, so that the last of them is white.
        let values = unpack(stored, pixels, depth);
        let scale = match depth {
            1 => 255,
            2 => 85,
            _ => 17,
        };
        return Ok(values.into_iter().map(|value| value * scale).collect());
    }
    // The colour type says how many samples a pixel has, and whether
    // the last of them is an alpha that is already there.
    let (colour_channels, alpha_in_file) = match colour {
        0 => (1usize, false),
        2 => (3, false),
        4 => (1, true),
        _ => (3, true),
    };
    let stored_channels = colour_channels + usize::from(alpha_in_file);
    let bytes_per_sample = usize::from(depth / 8);
    let has_alpha = transparency.is_some();
    let out_channels = colour_channels + usize::from(alpha_in_file || has_alpha);
    let mut out: Vec<u8> = Vec::with_capacity(pixels * out_channels);
    for pixel in 0..pixels {
        let base = pixel * stored_channels * bytes_per_sample;
        let mut transparent = false;
        match transparency {
            Some(Transparency::Grey(value)) => {
                let sample = sample_value(&stored, base, bytes_per_sample);
                transparent = sample_matches(sample, *value, depth);
            }
            Some(Transparency::Colour(colours)) => {
                transparent = (0..colour_channels).all(|channel| {
                    let at = base + channel * bytes_per_sample;
                    let sample = sample_value(&stored, at, bytes_per_sample);
                    sample_matches(sample, colours[channel], depth)
                });
            }
            None => {}
            // A palette picture has its transparency in its palette,
            // and it is handled there.
            Some(Transparency::Palette(_)) => {}
        }
        for channel in 0..colour_channels {
            let at = base + channel * bytes_per_sample;
            // A sixteen-bit picture comes out as **eight-bit**: the high
            // byte of each sample, which is what a canvas wants.
            out.push(stored[at]);
        }
        if alpha_in_file {
            // The alpha the picture came with: the high byte of the
            // last sample, for the same reason as the others.
            out.push(stored[base + colour_channels * bytes_per_sample]);
        } else if has_alpha {
            out.push(if transparent { 0 } else { 255 });
        }
    }
    Ok(out)
}

/// The value of one sample, as the sixteen-bit number the format
/// compares against.
fn sample_value(stored: &[u8], at: usize, bytes_per_sample: usize) -> u16 {
    if bytes_per_sample == 2 {
        return (u16::from(stored[at]) << 8) | u16::from(stored[at + 1]);
    }
    u16::from(stored[at])
}

/// Whether a sample is the one the transparency names.
///
/// The format writes the transparent value as a sixteen-bit number, and
/// a picture of eight bits compares against its **low** byte: a writer
/// of an eight-bit picture says `0x00vv`, and the value it names is
/// `vv`.
fn sample_matches(sample: u16, named: u16, depth: u8) -> bool {
    if depth == 16 { sample == named } else { sample == (named & 0xff) }
}

/// Undo the filters of the rows.
///
/// Every row is stored as the difference from the row above it, or from
/// the pixel to the left, and the five filters say which. The bytes per
/// pixel are what the filters count: one for a picture of eight-bit
/// samples, two for a sixteen-bit one.
fn unfilter(raw: &[u8], width: usize, height: usize, channels: u8, depth: u8) -> Result<Vec<u8>, Error> {
    // The filters count the bytes of a **pixel**: one for a picture of
    // fewer than eight bits, however many pixels are packed into it.
    let bytes_per_pixel = ((channels as usize) * (depth as usize / 8)).max(1);
    // The row holds the samples of every pixel, and a picture of fewer
    // than eight bits packs them: the row is as many bytes as the
    // samples need.
    let row_bytes = (width * channels as usize * depth as usize).div_ceil(8);
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
