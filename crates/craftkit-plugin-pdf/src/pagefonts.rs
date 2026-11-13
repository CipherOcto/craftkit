//! The **fonts** of a page: the fonts it names, and the maps they carry.
//!
//! A page draws its text with the fonts in its **resources**, and it
//! names each of them: `/F1`, `/F2`, and so on. That name is the page's
//! own — it is what a content stream writes when it says `/F1 12 Tf` —
//! and it is not the name of the font object itself.
//!
//! A font may carry a **map** (§575) that says what its codes stand
//! for. This module is where the page's fonts and their maps are put
//! together: the name the page uses, the font behind it, and the map
//! that makes its codes readable.
//!
//! # The rules
//!
//! - **A page with no resources has no fonts.** A page that names no
//!   font says nothing about its fonts, and the answer says so.
//! - **A font with no map has no Unicode.** The answer says which of
//!   the page's fonts carry a map and which do not.
//! - **A font the file does not hold is skipped.** A resource that
//!   names an object the file does not have is not a font this tree
//!   can read.
//!
//! # What it does not do
//!
//! - **The resources of the parent are not read.** A page may take its
//!   fonts from the `/Pages` node above it, and that is not read here.
//! - **The encoding of a font is not read.** A font with no map has no
//!   Unicode here, and the tree falls back to the encoding.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome};

/// The name of the command, in the messages.
const COMMAND: &str = "pdf.page.fonts";

/// The **entry** of a dictionary, by its key.
fn entry_of<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **pairs** of a dictionary that is a value.
fn dictionary_of(value: &Json) -> Option<Vec<(String, Json)>> {
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

/// The **entries** of a value: the pairs of a dictionary, whether it is
/// written where it is used or named by a reference.
fn entries_of_value(document: &Document, value: &Json) -> Option<Vec<(String, Json)>> {
    if let Some(node) = crate::preflight::node_of(document, value) {
        return Some(crate::entries_of(&node.data));
    }
    dictionary_of(value)
}

/// The **fonts** of a page: the name each has and the node of it.
///
/// The fonts of a page are in the page's resources, and the resources
/// may be written where they are used or named by a reference.
pub(crate) fn fonts_of(document: &Document, page: u64) -> Vec<(String, u64)> {
    let Some(node) = document.node(page) else { return Vec::new() };
    let entries = crate::entries_of(&node.data);
    let Some(resources) = entry_of(&entries, "Resources") else { return Vec::new() };
    let Some(resources) = entries_of_value(document, resources) else { return Vec::new() };
    let Some(fonts) = entry_of(&resources, "Font") else { return Vec::new() };
    let Some(fonts) = entries_of_value(document, fonts) else { return Vec::new() };
    let mut out: Vec<(String, u64)> = Vec::new();
    for (name, value) in fonts {
        let Some(number) = crate::number_of(&value) else { continue };
        let Some(node) = document
            .walk()
            .find(|node| node.data.get("object").and_then(|number| number.as_u64()) == Some(number))
        else {
            continue;
        };
        out.push((name, node.id));
    }
    out
}

/// The **maps** of the fonts of a page: the name each font has and the
/// map it carries.
///
/// A font with no map is not in the list: there is nothing to read
/// through it.
pub(crate) fn maps_of(document: &Document, page: u64) -> Vec<(String, crate::tounicode::Map)> {
    let mut out: Vec<(String, crate::tounicode::Map)> = Vec::new();
    for (name, font) in fonts_of(document, page) {
        if let Ok(Some(map)) = crate::tounicode::unicode_of(document, font) {
            out.push((name, map));
        }
    }
    out
}

commands! {
    /// **Read** the **fonts** of a page: the fonts it names, and the maps
    /// they carry.
    ///
    /// A query: it does not change the document. A page with no resources
    /// has no fonts, and the answer says so.
    pdf.page.fonts {
        area: "pdf",
        layer: "pdf",
        icon: "list",
        params: {
            /// The page whose fonts are read.
            page: NodeRef,
        },
        run: {
            let page = self.page.0;
            let document = session.document();
            let Some(node) = document.node(page) else {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("there is no node `{page}`"),
                });
            };
            if node.kind != "pdf.object" && node.kind != "pdf.stream" {
                return Err(EngineError::Failed {
                    id: COMMAND.to_string(),
                    message: format!("the node `{page}` is a `{}` and not an object", node.kind),
                });
            }
            let mut listed: Vec<Json> = Vec::new();
            for (name, font) in fonts_of(document, page) {
                let described = document.node(font).and_then(|node| crate::fonts::font_of(document, node));
                let map = crate::tounicode::unicode_of(document, font).ok().flatten();
                // How **tall** the font's glyphs are: the metrics of the
                // font, and the tree's own rule when the font says nothing.
                let metrics = crate::metrics::metrics_of(document, font);
                listed.push(json!({
                    "name": name,
                    "font": font,
                    "base": described.as_ref().map(|font| font.base.clone()).unwrap_or_default(),
                    "subtype": described.as_ref().map(|font| font.subtype.clone()).unwrap_or_default(),
                    "has_map": map.is_some(),
                    "codes": map.as_ref().map(|map| map.entries.len()).unwrap_or(0),
                    "ascent": metrics.ascent,
                    "descent": metrics.descent,
                }));
            }
            let count = listed.len();
            Ok(Outcome::new(format!("the page `{page}` names {count} font(s)")).with_data(json!({
                "page": page,
                "fonts": listed,
                "count": count,
            })))

        }
    }
}
