//! The **picture**: a buffer of pixels, and the PNG that carries it.

use craftkit_sdk::document::Colour;

/// A picture: RGBA pixels, four bytes each, in a row.
///
/// It is **not** a document and it does not know what it shows. It is a
/// buffer, and everything that draws into it is somebody else's business.
#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Image {
    /// A picture of that size, filled with one colour.
    ///
    /// The size is the caller's, and the caller's size is **bounded by
    /// the caller**: [`crate::MAX_IMAGE_PIXELS`] is this crate's number
    /// and a program that wants a different one says so.
    pub fn new(width: usize, height: usize, background: Colour) -> Self {
        let mut pixels = Vec::with_capacity(width.saturating_mul(height).saturating_mul(4));
        let [r, g, b, a] = bytes_of(background);
        for _ in 0..width.saturating_mul(height) {
            pixels.extend_from_slice(&[r, g, b, a]);
        }
        Image { width, height, pixels }
    }

    /// How wide the picture is.
    pub fn width(&self) -> usize {
        self.width
    }

    /// How tall the picture is.
    pub fn height(&self) -> usize {
        self.height
    }

    /// The colour of a pixel, and `None` if it is not there.
    pub fn pixel(&self, x: usize, y: usize) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = (y * self.width + x) * 4;
        let slice = self.pixels.get(at..at + 4)?;
        Some([slice[0], slice[1], slice[2], slice[3]])
    }

    /// Paint a pixel, **over** what is there.
    ///
    /// The colour is mixed with the pixel under it: a colour with an
    /// alpha of half a pixel is half a pixel.
    pub fn blend(&mut self, x: usize, y: usize, colour: Colour) {
        if x >= self.width || y >= self.height {
            return;
        }
        let [r, g, b, a] = bytes_of(colour);
        let alpha = f32::from(a) / 255.0;
        if alpha <= 0.0 {
            return;
        }
        let at = (y * self.width + x) * 4;
        let Some(slice) = self.pixels.get_mut(at..at + 4) else { return };
        if alpha >= 1.0 {
            slice[0] = r;
            slice[1] = g;
            slice[2] = b;
            slice[3] = 255;
            return;
        }
        for (channel, value) in [r, g, b].into_iter().enumerate() {
            let under = f32::from(slice[channel]);
            let mixed = f32::from(value) * alpha + under * (1.0 - alpha);
            slice[channel] = mixed.round().clamp(0.0, 255.0) as u8;
        }
        let under = f32::from(slice[3]);
        slice[3] = (under + (255.0 - under) * alpha).round().clamp(0.0, 255.0) as u8;
    }

    /// The picture, as a PNG file.
    ///
    /// The stream is **stored** and not compressed: a picture that is a
    /// little bigger is a picture that any reader can open, and the
    /// writer has no dependency to break.
    pub fn to_png(&self) -> Vec<u8> {
        crate::png::write(self)
    }

    /// The raw rows: a filter byte and the pixels, row by row.
    pub(crate) fn rows(&self) -> Vec<u8> {
        let mut rows = Vec::with_capacity(self.height * (1 + self.width * 4));
        for y in 0..self.height {
            rows.push(0);
            let start = y * self.width * 4;
            if let Some(row) = self.pixels.get(start..start + self.width * 4) {
                rows.extend_from_slice(row);
            }
        }
        rows
    }
}

/// The colour as four bytes.
fn bytes_of(colour: Colour) -> [u8; 4] {
    let [r, g, b, a] = colour.0;
    [
        (r.clamp(0.0, 1.0) * 255.0).round() as u8,
        (g.clamp(0.0, 1.0) * 255.0).round() as u8,
        (b.clamp(0.0, 1.0) * 255.0).round() as u8,
        (a.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}
