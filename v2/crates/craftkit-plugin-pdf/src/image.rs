//! The **pixels of an image** under a redaction region.
//!
//! This is the tree's own rule for an image the region only hits: the
//! image is decoded, every pixel whose centre maps into the region is
//! set to zero — no ink, and for an image mask the other way round —
//! and the result is written as a **new** image. The object the file
//! had is untouched, because other pages may use it.
//!
//! # The rules
//!
//! - **An image the region covers is gone**, and an image the region
//!   only hits has its covered pixels cleared. That is the tree's own
//!   rule, and it is the rule of the drawing that is taken away.
//! - **An image the kit cannot decode is taken away whole.** The tree's
//!   own rule, and it is fail-closed: an image that cannot be read is
//!   not left on the page with the text under it still in it.
//! - **The pixels are bounded.** An image with more pixels than the
//!   bound has is not cleared, and it is taken away instead.
//!
//! # What it does not do
//!
//! - **The image is not re-encoded.** The tree writes the new image as
//!   a Flate stream; this tree has no encoder for one, and the new
//!   image is written without a filter, which a file may hold.

use craftkit_sdk::serde_json::Value as Json;
use craftkit_sdk::{Document, Node};

use crate::content::Matrix;

/// The most **pixels** an image may have to be cleared in place.
const MAX_PIXELS: u64 = 64 << 20;

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.redact.drawings";

/// The **components** of the colour space of an image.
///
/// The tree's own rule: the name of the space says how many
/// components a sample has, and the profile of an ICC-based space says
/// it in its own dictionary.
fn components(document: &Document, entries: &[(String, Json)]) -> Option<usize> {
    let (_, value) = entries.iter().find(|(key, _)| key == "ColorSpace")?;
    let name = match value {
        Json::String(name) => name.clone(),
        Json::Array(list) => list.first()?.as_str()?.to_string(),
        _ => return None,
    };
    Some(match name.as_str() {
        "DeviceGray" | "G" | "CalGray" | "Indexed" | "I" | "Separation" => 1,
        "DeviceRGB" | "RGB" | "CalRGB" | "Lab" => 3,
        "DeviceCMYK" | "CMYK" => 4,
        "ICCBased" => {
            // The number of components lives in the profile.
            let Json::Array(list) = value else { return None };
            let profile = list.get(1)?;
            let node = crate::preflight::node_of(document, profile)?;
            let held = crate::entries_of(&node.data);
            let (_, number) = held.iter().find(|(key, _)| key == "N")?;
            let number = number.as_f64()?;
            if !number.is_finite() || number.fract() != 0.0 || number <= 0.0 {
                return None;
            }
            number as usize
        }
        _ => return None,
    })
}

/// The **samples** of an image: the bytes of them, and the shape they
/// have.
///
/// `None` when the image cannot be read, and the tree's own rule for
/// an image it cannot read is to take it away.
fn samples_of(document: &Document, node: &Node) -> Option<(Vec<u8>, usize, usize, usize, usize, bool)> {
    let entries = crate::entries_of(&node.data);
    // The **codec** of the image: the ones the kit cannot undo are not
    // undone.
    if let Some(names) = crate::filters::filter_of(&entries) {
        for name in &names {
            crate::filters::Filter::by_name(name)?;
        }
    }
    // The numbers of the file are held as they were written, and a
    // number that is not a whole one is not a number of an image.
    let number = |key: &str| {
        entries
            .iter()
            .find(|(name, _)| name == key)
            .and_then(|(_, value)| value.as_f64())
            .filter(|value| value.is_finite() && value.fract() == 0.0)
            .map(|value| value as i64)
    };
    let width = number("Width")?;
    let height = number("Height")?;
    if width <= 0 || height <= 0 || (width as u64) * (height as u64) > MAX_PIXELS {
        return None;
    }
    let mask = entries.iter().any(|(key, value)| key == "ImageMask" && value.as_bool() == Some(true));
    let (components, bits) = if mask {
        (1usize, 1usize)
    } else {
        (components(document, &entries)?, number("BitsPerComponent")? as usize)
    };
    if !matches!(bits, 1 | 2 | 4 | 8 | 16) {
        return None;
    }
    // The **bytes** of the samples: the filter and the predictor are
    // undone.
    let decoded = crate::decoded_bytes(node, None, None, false, COMMAND).ok()?;
    let row = (width as usize * components * bits).div_ceil(8);
    if decoded.bytes.len() < row * height as usize {
        return None;
    }
    let mut bytes = decoded.bytes;
    bytes.truncate(row * height as usize);
    Some((bytes, width as usize, height as usize, row, components * bits, mask))
}

/// The **covered pixels** of an image are cleared.
///
/// - `None` when the image cannot be read: it is taken away.
/// - `Some(None)` when no pixel of it is covered: it is left as it is.
/// - `Some(Some(bytes))` when the samples are cleared.
pub fn clear(document: &Document, node: &Node, ctm: &Matrix, rects: &[[f64; 4]]) -> Option<Option<Vec<u8>>> {
    let (mut data, width, height, row, bits_per_pixel, mask) = samples_of(document, node)?;
    let entries = crate::entries_of(&node.data);
    // An image mask paints with the sample **zero**, unless the decode
    // says the other way round: a cleared sample must not paint.
    let inverted = entries
        .iter()
        .find(|(key, _)| key == "Decode")
        .and_then(|(_, value)| value.as_array())
        .and_then(|list| list.first())
        .and_then(|first| first.as_f64())
        == Some(1.0);
    let clear_bit = mask && !inverted;
    let mut cleared = 0usize;
    for line in 0..height {
        // The image space: the origin is the bottom left of the unit
        // square, and the first row of the samples is the top one.
        let v = 1.0 - (line as f64 + 0.5) / height as f64;
        let (ax, ay) = ctm.apply(0.0, v);
        let (bx, by) = ctm.apply(1.0, v);
        let bounds = [ax.min(bx), ay.min(by), ax.max(bx), ay.max(by)];
        // The **row** is not under the region: nothing of it is
        // cleared, and the row is not looked at again.
        let under = rects.iter().any(|rect| {
            crate::redact::overlaps(*rect, [bounds[0], bounds[1] - 0.01, bounds[2], bounds[3] + 0.01])
        });
        if !under {
            continue;
        }
        for column in 0..width {
            let u = (column as f64 + 0.5) / width as f64;
            let (x, y) = ctm.apply(u, v);
            let covered =
                rects.iter().any(|rect| x >= rect[0] && x <= rect[2] && y >= rect[1] && y <= rect[3]);
            if !covered {
                continue;
            }
            cleared += 1;
            let start = line * row * 8 + column * bits_per_pixel;
            for bit in start..start + bits_per_pixel {
                let (byte, shift) = (bit / 8, 7 - bit % 8);
                if clear_bit {
                    data[byte] |= 1 << shift;
                } else {
                    data[byte] &= !(1 << shift);
                }
            }
        }
    }
    if cleared == 0 {
        return Some(None);
    }
    Some(Some(data))
}

/// The **entries** of the dictionary of an image, without the ones that
/// say how its bytes are encoded.
///
/// The tree's own rule: the new image carries the dictionary of the old
/// one, and the length and the filter of the old bytes are not its own.
pub fn entries_of_an_image(node: &Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
        .into_iter()
        .filter(|(key, _)| key != "Filter" && key != "Length" && key != "DecodeParms")
        .collect()
}
