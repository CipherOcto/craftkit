//! The **widths** of a font: how wide each of its codes is.
//!
//! A font says how wide its characters are, in **thousandths of an
//! em**: a width of 500 is half the size of the text. A simple font
//! says it in `/Widths`, an array that begins at the font's
//! `/FirstChar`; a font of many codes says it in `/W`, and the codes
//! of such a font are two bytes long.
//!
//! # The rules
//!
//! - A font with no widths has **no widths**: the width of its text
//!   is not known, and the answer says so rather than guessing.
//! - A code the font has no width for takes the font's **default**
//!   width: `/MissingWidth` for a simple font and `/DW` for a font of
//!   many codes, which is a thousand when the font says nothing.
//! - The **codes** of a font of many codes are **two bytes** long, and
//!   the tree reads them as the codes they are: the mapping of a code
//!   to a character is the map of the font, and that is not here.
//!
//! # What it does not do
//!
//! - The **standard fourteen fonts** are not measured: a font that
//!   names one and gives no widths has no widths here.
//! - The **width of a code** is not the width of the character: a
//!   font may draw one character with another code.

use craftkit_sdk::Document;
use craftkit_sdk::serde_json::Value as Json;

/// The **widths** of a font.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Widths {
    /// The first code the widths are for.
    first: u32,
    /// The width of each code, in thousandths of an em.
    widths: Vec<f64>,
    /// The widths of a font of many codes: a range of codes and the
    /// width of each of them.
    ranges: Vec<(u32, u32, Vec<f64>)>,
    /// The width a code has when the font says nothing of it.
    default: f64,
    /// Whether the font is a font of **many codes**.
    many: bool,
}

impl Widths {
    /// The **width** of a code, in thousandths of an em.
    ///
    /// The answer is `None` when the font does not say: a width that
    /// is guessed is a width that is wrong.
    pub fn width(&self, code: u32) -> Option<f64> {
        if self.many {
            for (first, last, widths) in &self.ranges {
                if code >= *first && code <= *last {
                    let index = (code - first) as usize;
                    return widths.get(index).copied().or(Some(self.default));
                }
            }
            return Some(self.default);
        }
        match code.checked_sub(self.first) {
            Some(index) => {
                let index = index as usize;
                self.widths.get(index).copied().or(Some(self.default))
            }
            None => Some(self.default),
        }
    }

    /// The **codes** of the bytes of a string.
    ///
    /// A font of many codes has two bytes to a code, and a simple
    /// font has one.
    pub fn codes(&self, bytes: &[u8]) -> Vec<u32> {
        if self.many {
            return bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u32::from(u16::from_be_bytes([pair[0], pair[1]])))
                .collect();
        }
        bytes.iter().map(|byte| u32::from(*byte)).collect()
    }
}

/// The **array** a value holds, whether it is written where it is used
/// or named by a reference.
fn array_of(document: &Document, value: &Json) -> Option<Vec<Json>> {
    if let Json::Array(list) = value {
        return Some(list.clone());
    }
    let node = crate::preflight::node_of(document, value)?;
    if let Some(list) = node.data.as_array() {
        return Some(list.clone());
    }
    for key in ["array", "values", "Array"] {
        if let Some(list) = node.data.get(key).and_then(|value| value.as_array()) {
            return Some(list.clone());
        }
    }
    None
}

/// The **numbers** an array holds.
fn numbers_of(list: &[Json]) -> Vec<f64> {
    list.iter().filter_map(|value| value.as_f64()).collect()
}

/// The **widths** of a font of many codes: the ranges of `/W`.
fn many_of(document: &Document, entry: Option<&Json>, default: f64) -> Option<Vec<(u32, u32, Vec<f64>)>> {
    let list = array_of(document, entry?)?;
    let mut ranges: Vec<(u32, u32, Vec<f64>)> = Vec::new();
    let mut at = 0usize;
    while at < list.len() {
        let Some(first) = list[at].as_f64() else { break };
        at += 1;
        match list.get(at) {
            // A list of widths: `first [w1 w2 ...]`.
            Some(Json::Array(widths)) => {
                at += 1;
                let widths: Vec<f64> = widths.iter().filter_map(|value| value.as_f64()).collect();
                if widths.is_empty() {
                    continue;
                }
                let last = first as u32 + widths.len() as u32 - 1;
                ranges.push((first as u32, last, widths));
            }
            // A range: `first last width`.
            Some(second) => {
                let Some(second) = second.as_f64() else { break };
                at += 1;
                let width = list.get(at).and_then(|value| value.as_f64()).unwrap_or(default);
                at += 1;
                ranges.push((first as u32, second as u32, vec![width]));
            }
            None => break,
        }
    }
    if ranges.is_empty() { None } else { Some(ranges) }
}

/// The **widths** of a font, when it says them.
///
/// The answer is `None` when the font says nothing: a font with no
/// widths has no widths.
pub fn widths_of(document: &Document, font: u64) -> Option<Widths> {
    let node = document.node(font)?;
    let entries = crate::entries_of(&node.data);
    let entry = |key: &str| -> Option<Json> {
        entries.iter().find(|(name, _)| name == key).map(|(_, value)| value.clone())
    };
    let subtype =
        entry("Subtype").and_then(|value| value.as_str().map(|text| text.to_string())).unwrap_or_default();

    if subtype == "Type0" {
        // A font of many codes: `/W` holds the widths of the codes
        // and `/DW` the width of the rest.
        let default = entry("DW").and_then(|value| value.as_f64()).unwrap_or(1000.0);
        let ranges = many_of(document, entry("W").as_ref(), default)?;
        return Some(Widths { ranges, default, many: true, ..Widths::default() });
    }
    // A simple font: `/Widths` holds the widths and `/FirstChar` the
    // code they begin at.
    let first = entry("FirstChar").and_then(|value| value.as_f64()).unwrap_or(0.0) as u32;
    let list = entry("Widths").and_then(|value| array_of(document, &value))?;
    let widths = numbers_of(&list);
    if widths.is_empty() {
        return None;
    }
    let default = entry("MissingWidth").and_then(|value| value.as_f64()).unwrap_or(0.0);
    Some(Widths { first, widths, default, many: false, ranges: Vec::new() })
}
