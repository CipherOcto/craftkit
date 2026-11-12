//! The **metrics** of a font: how tall its glyphs are.
//!
//! A font says how high its glyphs go above the baseline and how far
//! they go below it. A font of **type three** says it in its own box:
//! the `/FontBBox` of the font and the `/FontMatrix` that says how
//! that box is drawn. Any other font says it in its **descriptor**:
//! `/Ascent` and `/Descent`, both in thousandths of an em.
//!
//! The **metrics** here are in the **glyph space**: a fraction of an
//! em, so an ascent of `0.9` is nine tenths of the size of the text.
//!
//! # The rules
//!
//! - The tree's own rules are the rules:
//!   - an ascent that is not believable — one that is not above `0.3`
//!     — is the tree's own **0.9**, and a descent that is not
//!     believable — one that is not below `-0.05` — is the tree's own
//!     **-0.25**;
//!   - an ascent that is believable is kept, but never above **1.5**,
//!     and a descent that is believable is kept, but never below
//!     **-0.6**;
//!   - a font of type three says it in its box: the ascent is the top
//!     of the box and the descent the bottom, and the ascent is never
//!     below **0.5** and the descent never above **-0.1**.
//! - **A font that says nothing has the tree's own metrics**: the
//!   ascent is `0.9` and the descent `-0.25`. A font that says nothing
//!   is not a font with no metrics; the tree's own rule gives it some,
//!   and this tree does not make up others.
//! - **A composite font** says it in the **descendant** it descends
//!   from: a font of type zero has no descriptor of its own.
//!
//! # What it does not do
//!
//! - **The standard fourteen fonts** are not measured: a font that
//!   names one and says nothing has the tree's own metrics, which are
//!   the tree's own and not the font's.
//! - **The height of a glyph** is not the height of the font: a font
//!   says how high its glyphs go, and a particular glyph may be
//!   smaller.

use craftkit_sdk::Document;
use craftkit_sdk::serde_json::Value as Json;

/// The **ascent** a font must claim before its own claim is believed.
const ASCENT_BELIEVED: f64 = 0.3;

/// The **ascent** no font may claim beyond it.
const ASCENT_HIGHEST: f64 = 1.5;

/// The **descent** a font must claim below before its own claim is
/// believed.
const DESCENT_BELIEVED: f64 = -0.05;

/// The **descent** no font may claim beyond it.
const DESCENT_LOWEST: f64 = -0.6;

/// The **ascent** of a font of type three is never below this.
const TYPE3_ASCENT_LOWEST: f64 = 0.5;

/// The **descent** of a font of type three is never above this.
const TYPE3_DESCENT_HIGHEST: f64 = -0.1;

/// The **metrics** of a font: how tall its glyphs are.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Metrics {
    /// How high the glyphs go above the baseline, in glyphs of an em.
    pub ascent: f64,
    /// How far the glyphs go below the baseline, in glyphs of an em.
    pub descent: f64,
}

impl Metrics {
    /// The **metrics** of a font that says nothing: the tree's own.
    pub const FALLBACK: Metrics = Metrics { ascent: 0.9, descent: -0.25 };
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **entries** of a value: the pairs of a dictionary, whether it is
/// written where it is used or named by a reference.
fn entries_of_value(document: &Document, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(node) = crate::preflight::node_of(document, value) {
        return Some(crate::entries_of(&node.data));
    }
    let list = value.get("entries")?.as_array()?;
    Some(
        list.iter()
            .filter_map(|entry| {
                let pair = entry.as_array()?;
                let key = pair.first()?.as_str()?.to_string();
                Some((key, pair.get(1)?.clone()))
            })
            .collect(),
    )
}

/// The **numbers** an array holds, whether it is written where it is
/// used or named by a reference.
fn numbers_of(document: &Document, value: &Json) -> Vec<f64> {
    if let Some(list) = value.as_array() {
        return list.iter().filter_map(|number| number.as_f64()).collect();
    }
    let Some(node) = crate::preflight::node_of(document, value) else { return Vec::new() };
    for key in ["array", "values", "Array"] {
        if let Some(list) = node.data.get(key).and_then(|value| value.as_array()) {
            return list.iter().filter_map(|number| number.as_f64()).collect();
        }
    }
    Vec::new()
}

/// The **metrics** of a font of type three: its own box, drawn by its
/// own matrix.
///
/// The answer is `None` when the font says neither: a font with no box
/// has no metrics of its own here.
fn type3_of(document: &Document, entries: &[(String, Json)]) -> Option<Metrics> {
    let box_ = numbers_of(document, entry(entries, "FontBBox")?);
    let matrix = numbers_of(document, entry(entries, "FontMatrix")?);
    if box_.len() != 4 || matrix.len() != 6 {
        return None;
    }
    let scale = matrix[3];
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    Some(Metrics {
        ascent: (box_[3] * scale).max(TYPE3_ASCENT_LOWEST),
        descent: (box_[1] * scale).min(TYPE3_DESCENT_HIGHEST),
    })
}

/// The **descriptor** of a font: the object that says how the font is
/// drawn.
///
/// A composite font (a `Type0`) has no descriptor of its own: the
/// descriptor is in the descendant it descends from, and it is the
/// first one that answers.
fn descriptor_of(document: &Document, entries: &[(String, Json)]) -> Option<Vec<(String, Json)>> {
    if entry(entries, "Subtype").and_then(|value| value.as_str()) == Some("Type0") {
        let descendants = entry(entries, "DescendantFonts")?.as_array()?;
        let first = descendants.first()?;
        let entries = entries_of_value(document, first)?;
        let descriptor = entry(&entries, "FontDescriptor")?;
        return entries_of_value(document, descriptor);
    }
    let descriptor = entry(entries, "FontDescriptor")?;
    entries_of_value(document, descriptor)
}

/// The **metrics** of a font, from its **descriptor**.
///
/// The tree's own rule: the ascent and the descent are in thousandths
/// of an em, and a font that claims nothing believable has the tree's
/// own metrics.
fn metrics_of_descriptor(entries: &[(String, Json)]) -> Metrics {
    let ascent = entry(entries, "Ascent").and_then(|value| value.as_f64()).unwrap_or(0.0) / 1000.0;
    let descent = entry(entries, "Descent").and_then(|value| value.as_f64()).unwrap_or(0.0) / 1000.0;
    Metrics {
        ascent: if ascent > ASCENT_BELIEVED { ascent.min(ASCENT_HIGHEST) } else { Metrics::FALLBACK.ascent },
        descent: if descent < DESCENT_BELIEVED {
            descent.max(DESCENT_LOWEST)
        } else {
            Metrics::FALLBACK.descent
        },
    }
}

/// The **metrics** of a font: how tall its glyphs are.
///
/// The answer is the tree's own metrics when the font says nothing
/// believable: a font that says nothing is not a font with no metrics.
pub fn metrics_of(document: &Document, font: u64) -> Metrics {
    let Some(node) = document.node(font) else { return Metrics::FALLBACK };
    let entries = crate::entries_of(&node.data);
    let subtype = entry(&entries, "Subtype").and_then(|value| value.as_str()).unwrap_or_default();
    // A font of type three says it in its own box, and the descriptor
    // of one says it again: the descriptor is the last word.
    let own = if subtype == "Type3" { type3_of(document, &entries) } else { None };
    match descriptor_of(document, &entries) {
        Some(descriptor) => metrics_of_descriptor(&descriptor),
        None => own.unwrap_or(Metrics::FALLBACK),
    }
}
