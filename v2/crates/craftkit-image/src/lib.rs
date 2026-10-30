//! The **size of a picture**, read from its header.
//!
//! # Why it exists
//!
//! The kit had no idea what a picture is. An image was "a plugin's
//! payload" — a thing an application knew about and the kit did not —
//! and a document that held a picture had to hold its **pixels**, because
//! there was nowhere else to put them.
//!
//! That is the wrong shape, and this crate is where the shape is
//! written down: a picture is a **size** and a **file**. The pixels stay
//! in the file, and the file travels beside the document in a bundle
//! (§25). What the document holds is the two numbers a layout needs.
//!
//! # What it reads
//!
//! - **JPEG** — the frame header, walked the way ITU-T T.81 says the
//!   markers are laid out. The walk is ported from the family's own
//!   clean-room decoder (`craft-jpeg`), which is the tree that knows
//!   about the twelve-bit and arithmetic-coded files that the common
//!   decoders refuse.
//! - **PNG** — the `IHDR` chunk, which is the first thing after the
//!   signature and says the size in four bytes each way.
//!
//! # What it does not do
//!
//! It does not decode a picture. There are **no pixels here**: a
//! decoder is a library of its own, and a document that wanted one would
//! ask for it by name.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

/// How many bytes of a picture's header are enough.
///
/// The header is at the front of the file, and a megabyte of it is more
/// than any picture needs: the JPEG's frame header comes after the
/// application's own segments, and the largest of those that anybody
/// writes is a few kilobytes.
pub const MAX_HEADER_BYTES: usize = 1024 * 1024;

/// The formats this crate reads the size of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// A JPEG file: ITU-T T.81.
    Jpeg,
    /// A PNG file.
    Png,
}

impl Format {
    /// The format's name, as it is written.
    pub fn name(&self) -> &'static str {
        match self {
            Format::Jpeg => "jpeg",
            Format::Png => "png",
        }
    }

    /// The format with that name, if there is one.
    pub fn of(name: &str) -> Option<Format> {
        match name {
            "jpeg" | "jpg" => Some(Format::Jpeg),
            "png" => Some(Format::Png),
            _ => None,
        }
    }
}

/// How big a picture is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Size {
    /// How wide it is, in pixels.
    pub width: u32,
    /// How tall it is, in pixels.
    pub height: u32,
    /// What kind of picture it is.
    pub format: Format,
}

impl Size {
    /// How many pixels it has.
    pub fn pixels(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// Its width divided by its height, or `None` when it has no height.
    pub fn aspect(&self) -> Option<f64> {
        if self.height == 0 {
            return None;
        }
        Some(f64::from(self.width) / f64::from(self.height))
    }
}

/// The size of the picture in these bytes.
///
/// The bytes are the **head** of the file: the caller reads as much as
/// it is willing to read and stops. A picture whose header is not in the
/// bytes is a picture this call cannot measure — not a picture that is
/// broken.
pub fn size(bytes: &[u8]) -> Result<Size, String> {
    if bytes.len() > MAX_HEADER_BYTES {
        return Err(format!("the header is {} bytes, and the limit is {MAX_HEADER_BYTES}", bytes.len()));
    }
    if is_png(bytes) {
        return png_size(bytes);
    }
    if is_jpeg(bytes) {
        return jpeg_size(bytes);
    }
    Err("these bytes are not a picture this crate can measure".to_string())
}

/// Whether these bytes are the head of a PNG file.
pub fn is_png(bytes: &[u8]) -> bool {
    bytes.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10])
}

/// Whether these bytes are the head of a JPEG file.
pub fn is_jpeg(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xFF, 0xD8])
}

/// The size of a PNG.
///
/// The `IHDR` chunk is the first one after the signature, and it says the
/// size in the first four bytes it carries.
fn png_size(bytes: &[u8]) -> Result<Size, String> {
    // The signature is eight bytes, the chunk's length is four, its name
    // is four, and then the size: four bytes each way.
    let width = read_be32(bytes, 16)?;
    let height = read_be32(bytes, 20)?;
    if width == 0 || height == 0 {
        return Err("the picture has no size".to_string());
    }
    Ok(Size { width, height, format: Format::Png })
}

/// The size of a JPEG.
///
/// The markers are walked the way T.81 lays them out, and the frame
/// header — the `SOFn` segment — says the size. The walk is the
/// family's: it reads the headers only, so it is cheap enough to route
/// every picture an application opens.
fn jpeg_size(bytes: &[u8]) -> Result<Size, String> {
    let mut at = 2;
    loop {
        if bytes.get(at) != Some(&0xFF) {
            return Err("the picture's markers are broken".to_string());
        }
        let marker = *bytes.get(at + 1).ok_or("the picture's markers are broken")?;
        match marker {
            // A fill byte: the marker is the next one.
            0xFF => at += 1,
            // A marker with no payload.
            0x01 | 0xD0..=0xD8 => at += 2,
            // The end of the picture, or the start of the scan: there is
            // no frame header before either of them.
            0xD9 | 0xDA => return Err("the picture has no frame header".to_string()),
            _ => {
                let length = usize::from(u16::from_be_bytes([
                    *bytes.get(at + 2).ok_or("the picture's markers are broken")?,
                    *bytes.get(at + 3).ok_or("the picture's markers are broken")?,
                ]));
                if is_frame(marker) {
                    // The segment's own bytes: the precision, the height
                    // and the width, and then the components.
                    let segment =
                        bytes.get(at + 4..at + 2 + length).ok_or("the picture's frame header is broken")?;
                    let height = u32::from(be16(segment, 1)?);
                    let width = u32::from(be16(segment, 3)?);
                    if width == 0 || height == 0 {
                        return Err("the picture has no size".to_string());
                    }
                    return Ok(Size { width, height, format: Format::Jpeg });
                }
                at = at.checked_add(2 + length).ok_or("the picture's markers are broken")?;
            }
        }
    }
}

/// Whether a marker is a frame header.
///
/// The `SOFn` markers are `0xC0` to `0xCF`, and three of them are
/// something else: `0xC4` is a table of Huffman codes, `0xC8` is a
/// reserved marker, and `0xCC` is a reserved marker.
fn is_frame(marker: u8) -> bool {
    matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC)
}

/// Two bytes, big-endian.
fn be16(bytes: &[u8], at: usize) -> Result<u16, String> {
    let slice = bytes.get(at..at + 2).ok_or("the picture's frame header is broken")?;
    Ok(u16::from_be_bytes([slice[0], slice[1]]))
}

/// Four bytes, big-endian.
fn read_be32(bytes: &[u8], at: usize) -> Result<u32, String> {
    let slice = bytes.get(at..at + 4).ok_or("the picture's header is broken")?;
    Ok(u32::from_be_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A PNG header: the signature, the `IHDR` chunk and its size.
    fn a_png(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = vec![137, 80, 78, 71, 13, 10, 26, 10];
        bytes.extend_from_slice(&13u32.to_be_bytes());
        bytes.extend_from_slice(b"IHDR");
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&[8, 6, 0, 0, 0]);
        bytes
    }

    /// A JPEG header: the start of the picture, an application segment
    /// and then the frame.
    fn a_jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut bytes = vec![0xFF, 0xD8];
        // An application segment with four bytes of payload.
        bytes.extend_from_slice(&[0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00]);
        // The frame header: the precision, the height, the width, and
        // one component.
        bytes.extend_from_slice(&[0xFF, 0xC0, 0x00, 0x0B, 0x08]);
        bytes.extend_from_slice(&height.to_be_bytes());
        bytes.extend_from_slice(&width.to_be_bytes());
        bytes.extend_from_slice(&[0x01, 0x01, 0x11, 0x00]);
        bytes
    }

    #[test]
    fn the_size_of_a_png_is_read() {
        let size = size(&a_png(640, 480)).expect("that picture has a size");
        assert_eq!(size.width, 640);
        assert_eq!(size.height, 480);
        assert_eq!(size.format, Format::Png);
        assert_eq!(size.pixels(), 640 * 480);
        assert_eq!(size.aspect(), Some(640.0 / 480.0));
    }

    #[test]
    fn the_size_of_a_jpeg_is_read() {
        let size = size(&a_jpeg(800, 600)).expect("that picture has a size");
        assert_eq!(size.width, 800);
        assert_eq!(size.height, 600);
        assert_eq!(size.format, Format::Jpeg);
    }

    #[test]
    fn bytes_that_are_not_a_picture_are_refused() {
        let error = size(b"not a picture at all").expect_err("that is not a picture");
        assert!(error.contains("not a picture"), "{error}");
    }

    #[test]
    fn a_picture_with_a_broken_header_is_refused() {
        let mut bytes = a_png(640, 480);
        bytes.truncate(18);
        let error = size(&bytes).expect_err("that header is broken");
        assert!(error.contains("broken"), "{error}");
    }

    #[test]
    fn a_picture_with_no_size_is_refused() {
        let error = size(&a_png(0, 480)).expect_err("that picture has no size");
        assert!(error.contains("no size"), "{error}");
    }

    #[test]
    fn a_jpeg_with_no_frame_is_refused() {
        let error = size(&[0xFF, 0xD8, 0xFF, 0xD9]).expect_err("that picture has no frame");
        assert!(error.contains("no frame"), "{error}");
    }
}
