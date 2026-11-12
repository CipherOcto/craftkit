//! The **picture**: the pixels of an image.
//!
//! A picture in a document is a **size and a file** (§ the image
//! battery), and the pixels are in neither: they are in the file, and
//! they are read when somebody wants to see them.
//!
//! This is the type the pixels come back as: four bytes a pixel — red,
//! green, blue and alpha — and the rows top to bottom.
//!
//! It is in the SDK and not in a rasteriser because the **canvas** is
//! the seam that draws it: a backend that draws a document may be a
//! raster, a PDF writer or a recorder, and all of them need the pixels
//! in the same shape.

use crate::document::Rect;

/// A **picture**: the pixels of an image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Picture {
    /// How wide it is, in pixels.
    pub width: u32,
    /// How tall it is, in pixels.
    pub height: u32,
    /// The pixels: four bytes each — red, green, blue and alpha — and
    /// the rows top to bottom.
    pub pixels: Vec<u8>,
}

impl Picture {
    /// A picture of that size, with those pixels.
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Picture {
        Picture { width, height, pixels }
    }

    /// A picture from **samples**: one, three or four a pixel.
    ///
    /// A grey picture is one sample a pixel and the three colours are
    /// the same number; a colour picture is three and the fourth is
    /// opaque; a picture with an alpha is four. A sample count the
    /// picture does not have is a picture of **nothing**.
    pub fn from_samples(width: u32, height: u32, channels: u8, samples: Vec<u8>) -> Picture {
        let count = width as usize * height as usize;
        let mut pixels = Vec::with_capacity(count * 4);
        for index in 0..count {
            let at = index * channels as usize;
            let get = |offset: usize| samples.get(at + offset).copied().unwrap_or(0);
            match channels {
                1 => {
                    let grey = get(0);
                    pixels.extend_from_slice(&[grey, grey, grey, 255]);
                }
                3 => pixels.extend_from_slice(&[get(0), get(1), get(2), 255]),
                4 => pixels.extend_from_slice(&[get(0), get(1), get(2), get(3)]),
                _ => {}
            }
        }
        Picture { width, height, pixels }
    }

    /// How many pixels it has.
    pub fn pixels_count(&self) -> u64 {
        u64::from(self.width) * u64::from(self.height)
    }

    /// The pixel at a place, if it is there.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let at = (y as usize * self.width as usize + x as usize) * 4;
        let pixel = self.pixels.get(at..at + 4)?;
        Some([pixel[0], pixel[1], pixel[2], pixel[3]])
    }

    /// Where the picture is drawn: the rectangle it fills.
    ///
    /// A picture with no size has no place, and a picture with a place
    /// but no pixels is not drawn.
    pub fn is_empty(&self) -> bool {
        self.width == 0
            || self.height == 0
            || self.pixels.len() < (self.width as usize * self.height as usize * 4)
    }

    /// The picture's **place**: the rectangle it fills, at the origin.
    pub fn rect(&self) -> Rect {
        Rect::new(0.0, 0.0, f64::from(self.width), f64::from(self.height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A **grey** picture is one sample a pixel, and the three colours
    /// are the same number.
    #[test]
    fn a_grey_picture_is_grey() {
        let picture = Picture::from_samples(2, 1, 1, vec![0, 255]);
        assert_eq!(picture.pixel(0, 0), Some([0, 0, 0, 255]));
        assert_eq!(picture.pixel(1, 0), Some([255, 255, 255, 255]));
    }

    /// A **colour** picture is three samples a pixel, and the fourth is
    /// opaque: a picture without an alpha is not see-through.
    #[test]
    fn a_colour_picture_is_opaque() {
        let picture = Picture::from_samples(1, 1, 3, vec![10, 20, 30]);
        assert_eq!(picture.pixel(0, 0), Some([10, 20, 30, 255]));
    }

    /// A picture with an **alpha** keeps it.
    #[test]
    fn an_alpha_is_kept() {
        let picture = Picture::from_samples(1, 1, 4, vec![10, 20, 30, 128]);
        assert_eq!(picture.pixel(0, 0), Some([10, 20, 30, 128]));
    }

    /// A sample count the picture does not have is a picture of
    /// **nothing**: there is no colour to make up.
    #[test]
    fn an_unknown_channel_count_is_nothing() {
        let picture = Picture::from_samples(2, 2, 2, vec![0; 8]);
        assert!(picture.is_empty(), "the picture has no pixels");
        assert_eq!(picture.pixels_count(), 4, "but it has a size");
    }

    /// A pixel that is not there is **not there**: the picture does not
    /// make one up.
    #[test]
    fn a_pixel_outside_the_picture_is_none() {
        let picture = Picture::from_samples(2, 2, 3, vec![0; 12]);
        assert_eq!(picture.pixel(2, 0), None);
        assert_eq!(picture.pixel(0, 2), None);
        assert_eq!(picture.pixel(1, 1), Some([0, 0, 0, 255]));
    }
}
