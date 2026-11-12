//! The **contrast** of the text.
//!
//! Text that is hard to read is hard to read for everyone, and it is
//! impossible for some. The measure is the one the WCAG 2 gives: the
//! **relative luminance** of the two colours, and the **ratio**
//! between them.
//!
//! - the **luminance** of a colour is the light it gives off, from
//!   `0` for black to `1` for white;
//! - the **ratio** of two colours is `(lighter + 0.05) / (darker +
//!   0.05)`, and it runs from `1` (the same colour) to `21` (black
//!   on white).
//!
//! The ratio that is needed depends on the text: **four and a half**
//! for normal text, and **three** for text that is large — eighteen
//! points, or fourteen when it is bold. And a ratio under **three** is
//! a warning, not a tip: the text is not merely hard to read.

use craftkit_sdk::Colour;

/// The relative luminance of a colour, as the WCAG 2 gives it.
pub fn luminance(colour: &Colour) -> f64 {
    let channel = |value: f64| {
        if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
    };
    let red = channel(colour.0[0] as f64);
    let green = channel(colour.0[1] as f64);
    let blue = channel(colour.0[2] as f64);
    0.2126 * red + 0.7152 * green + 0.0722 * blue
}

/// The contrast ratio of two colours: from `1` to `21`.
pub fn contrast_ratio(one: &Colour, other: &Colour) -> f64 {
    let (first, second) = (luminance(one), luminance(other));
    let (high, low) = if first >= second { (first, second) } else { (second, first) };
    (high + 0.05) / (low + 0.05)
}

/// Whether the text is **large**: eighteen points, or fourteen when
/// it is bold.
pub fn is_large(size: Option<f64>, bold: bool) -> bool {
    let Some(size) = size else { return false };
    size >= 18.0 || (bold && size >= 14.0)
}

/// The ratio the text needs: three for large text, four and a half
/// for the rest.
pub fn needed(size: Option<f64>, bold: bool) -> f64 {
    if is_large(size, bold) { 3.0 } else { 4.5 }
}

/// Whether the ratio is **poor**: under three, and the text is not
/// merely hard to read.
pub fn is_poor(ratio: f64) -> bool {
    ratio < 3.0
}
