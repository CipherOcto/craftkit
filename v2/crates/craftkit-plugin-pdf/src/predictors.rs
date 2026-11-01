//! The **predictors**: the second step that sits on top of a filter.
//!
//! A filter undoes the **encoding** of a stream's bytes. A predictor
//! undoes a step that was made **after** the encoding: the bytes of an
//! image are not stored as they are, but as the **difference** from the
//! byte before them, so that the bytes of a picture compress far
//! better than the picture does.
//!
//! The PDF names the predictor in the `/DecodeParms` entry of the
//! stream's dictionary, and there are two of them (ISO 32000-2 §7.4.4.4):
//!
//! - **TIFF** — the predictor **2**: each byte is the difference from
//!   the byte before it, and the difference is undone from the left;
//! - **PNG** — the predictors **10 to 15**: each **row** of the image
//!   begins with a byte that says how the row was made, and there are
//!   five ways: the row as it is, the difference from the byte to the
//!   left, the difference from the byte above, the difference from the
//!   average of the two, and the difference from the **Paeth** guess.
//!
//! # What it does not do
//!
//! - **There is no encoder.** A predictor that undoes a step is worth
//!   having on its own, and the tree's PDF reads and does not write.
//! - **The rows are the image's rows.** The predictor needs to know how
//!   wide a row is, and that is the **caller's** to say: the PDF says
//!   it in `/Columns`, and the defaults are the PDF's own — one column,
//!   one colour, eight bits.

use craftkit_sdk::serde_json::Value as Json;

use crate::filters::Error;

/// The **predictor**: the step that was made after the encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predictor {
    /// No predictor: the bytes are the bytes.
    None,
    /// The TIFF predictor 2: the difference from the byte to the left.
    Tiff,
    /// The PNG predictors: each row says how it was made.
    Png,
}

impl Predictor {
    /// The predictor by the number the PDF gives it.
    ///
    /// The PDF says the number: **1** is no predictor, **2** is the TIFF
    /// one, and **10 to 15** are the PNG ones. A number that is not one
    /// of those is a predictor the kit does not know.
    pub fn by_number(number: i64) -> Option<Predictor> {
        match number {
            1 => Some(Predictor::None),
            2 => Some(Predictor::Tiff),
            10..=15 => Some(Predictor::Png),
            _ => None,
        }
    }
}

/// The **parameters** of a predictor.
///
/// The PDF says them in `/DecodeParms`, and the defaults are the PDF's
/// own: one column, one colour and eight bits to a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spec {
    /// The step that was made.
    pub predictor: Predictor,
    /// How many colours a pixel has.
    pub colors: usize,
    /// How many bits a component has.
    pub bits_per_component: usize,
    /// How many pixels wide a row is.
    pub columns: usize,
}

impl Default for Spec {
    fn default() -> Self {
        Spec { predictor: Predictor::None, colors: 1, bits_per_component: 8, columns: 1 }
    }
}

impl Spec {
    /// The bytes a **pixel** takes.
    ///
    /// A pixel is the colours together, and the bits may be fewer than
    /// eight, so the number of bytes is rounded up: a pixel is at least
    /// one byte, because a row that is a fraction of a byte is not a
    /// thing a PDF holds.
    pub fn bytes_per_pixel(&self) -> usize {
        let bits = self.colors.saturating_mul(self.bits_per_component);
        bits.div_ceil(8).max(1)
    }

    /// The bytes a **row** takes.
    pub fn row_bytes(&self) -> usize {
        self.columns.saturating_mul(self.colors).saturating_mul(self.bits_per_component).div_ceil(8)
    }
}

/// The predictor a **dictionary** names.
///
/// The `/DecodeParms` entry holds the parameters, and it may be a
/// dictionary or a **list** of them — one for each filter, in the same
/// order. The first one that is a dictionary is the one that is read.
///
/// The entry is returned even when it names no predictor: the
/// **parameters** are worth having on their own, because a caller may
/// say which predictor a stream is in and take the width from here.
pub fn spec_of(entries: &[(String, Json)]) -> Result<Option<Spec>, Error> {
    let Some((_, value)) = entries.iter().find(|(key, _)| key == "DecodeParms") else {
        return Ok(None);
    };
    let Some(pairs) = pairs_of(value) else {
        return Ok(None);
    };
    let mut spec = Spec::default();
    for (key, value) in pairs {
        let Some(number) = value.as_i64() else { continue };
        match key.as_str() {
            "Predictor" => {
                let Some(predictor) = Predictor::by_number(number) else {
                    return Err(Error::Unsupported(format!(
                        "the predictor {number} is not one of the kit's"
                    )));
                };
                spec.predictor = predictor;
            }
            "Colors" => spec.colors = number.max(1) as usize,
            "BitsPerComponent" => spec.bits_per_component = number.max(1) as usize,
            "Columns" => spec.columns = number.max(1) as usize,
            _ => {}
        }
    }
    Ok(Some(spec))
}

/// The pairs of a **dictionary** that is a list of pairs.
///
/// The value may be a dictionary itself or a **list** of them, and the
/// first one that is a dictionary is the one that is read.
fn pairs_of(value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(pairs) = dictionary_of(value) {
        return Some(pairs);
    }
    let list = value.as_array()?;
    for item in list {
        if let Some(pairs) = dictionary_of(item) {
            return Some(pairs);
        }
    }
    None
}

/// The pairs of a **dictionary**: a list of two-element lists.
fn dictionary_of(value: &Json) -> Option<Vec<(String, Json)>> {
    let list = value.as_array()?;
    let mut pairs = Vec::with_capacity(list.len());
    for item in list {
        let pair = item.as_array()?;
        let key = pair.first()?.as_str()?.to_string();
        let value = pair.get(1)?.clone();
        pairs.push((key, value));
    }
    Some(pairs)
}

/// Undo a predictor's step.
pub fn undo(data: &[u8], spec: &Spec) -> Result<Vec<u8>, Error> {
    match spec.predictor {
        Predictor::None => Ok(data.to_vec()),
        Predictor::Tiff => tiff(data, spec),
        Predictor::Png => png(data, spec),
    }
}

/// The TIFF predictor, undone.
fn tiff(data: &[u8], spec: &Spec) -> Result<Vec<u8>, Error> {
    let row_bytes = spec.row_bytes();
    let pixel_bytes = spec.bytes_per_pixel();
    let mut out = data.to_vec();
    if row_bytes == 0 {
        return Ok(out);
    }
    for row in out.chunks_mut(row_bytes) {
        for index in pixel_bytes..row.len() {
            row[index] = row[index].wrapping_add(row[index - pixel_bytes]);
        }
    }
    Ok(out)
}

/// The PNG predictors, undone.
fn png(data: &[u8], spec: &Spec) -> Result<Vec<u8>, Error> {
    let row_bytes = spec.row_bytes();
    let pixel_bytes = spec.bytes_per_pixel();
    let mut out: Vec<u8> = Vec::with_capacity(data.len());
    // The row above: the first row has none, and the difference from
    // nothing is the byte itself.
    let mut above: Vec<u8> = vec![0; row_bytes];
    let mut at = 0_usize;
    while at < data.len() {
        let kind = data[at];
        at += 1;
        let end = at + row_bytes;
        if end > data.len() {
            return Err(Error::Malformed("the last row is shorter than it says".to_string()));
        }
        let row = &data[at..end];
        at = end;
        let mut current = vec![0_u8; row_bytes];
        for index in 0..row_bytes {
            let left = if index >= pixel_bytes { current[index - pixel_bytes] } else { 0 };
            let up = above[index];
            let up_left = if index >= pixel_bytes { above[index - pixel_bytes] } else { 0 };
            current[index] = match kind {
                0 => row[index],
                1 => row[index].wrapping_add(left),
                2 => row[index].wrapping_add(up),
                3 => row[index].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => row[index].wrapping_add(paeth(left, up, up_left)),
                other => {
                    return Err(Error::Malformed(format!("the row's kind {other} is not one of the five")));
                }
            };
        }
        out.extend_from_slice(&current);
        above = current;
    }
    Ok(out)
}

/// The **Paeth** guess: the byte that is nearest to the three.
///
/// It is the one of the three that is closest to the sum of the other
/// two, and it is the guess the PNG made when it wrote the row.
fn paeth(left: u8, up: u8, up_left: u8) -> u8 {
    let left = i16::from(left);
    let up = i16::from(up);
    let up_left = i16::from(up_left);
    let estimate = left + up - up_left;
    let distance_left = (estimate - left).abs();
    let distance_up = (estimate - up).abs();
    let distance_up_left = (estimate - up_left).abs();
    if distance_left <= distance_up && distance_left <= distance_up_left {
        left as u8
    } else if distance_up <= distance_up_left {
        up as u8
    } else {
        up_left as u8
    }
}
