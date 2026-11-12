//! The **fonts** of a PDF file.
//!
//! A file names its fonts in objects of the type `Font`, and each one
//! says what it is: the **base font** it draws with, the **subtype**
//! it is written in, and where its **descriptor** is. The descriptor
//! says whether the font's own program is in the file: a font whose
//! `FontFile`, `FontFile2` or `FontFile3` is there is **embedded**, and
//! a font without one is **not**.
//!
//! A PDF/A file must have every font embedded, and the preflight says
//! so when one is not (§511).
//!
//! - a **Type 3** font is its own program, and it is embedded in the
//!   very object that names it: it needs no descriptor, and the rule
//!   does not touch it;
//! - a **Type 0** font is a composite: its programs are in the fonts
//!   it descends from, and it is embedded when the first of them is.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, NodeRef, Outcome, Session};

/// A **font** of a document.
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// The **base font** the font draws with: `Helvetica`, and the
    /// like.
    pub base: String,
    /// The **subtype** the font is written in: `Type1`, `TrueType`,
    /// `Type0`, `Type3`.
    pub subtype: String,
    /// Whether the font's own program is **in the file**.
    pub embedded: bool,
}

impl Font {
    /// The font, as the caller gets it.
    pub fn to_json(&self) -> Json {
        json!({
            "base": self.base.clone(),
            "subtype": self.subtype.clone(),
            "embedded": self.embedded,
        })
    }
}

/// The **entries** of a node's dictionary: the key and the value, in
/// the order the file had them.
fn entries_of(node: &craftkit_sdk::Node) -> Vec<(String, Json)> {
    crate::entries_of(&node.data)
}

/// The **name** an entry says: the value of a name entry is a text.
fn name_of(entries: &[(String, Json)], key: &str) -> Option<String> {
    entries
        .iter()
        .find(|(name, _)| name == key)
        .and_then(|(_, value)| value.as_str())
        .map(|text| text.to_string())
}

/// The **entry** of a dictionary, by its key.
fn entry<'a>(entries: &'a [(String, Json)], key: &str) -> Option<&'a Json> {
    entries.iter().find(|(name, _)| name == key).map(|(_, value)| value)
}

/// The **object number** a node holds.
fn number_of(node: &craftkit_sdk::Node) -> Option<u64> {
    node.data.get("object").and_then(|object| object.as_u64())
}

/// The **node** a reference names: the node with that object number.
fn node_of<'a>(document: &'a Document, value: &Json) -> Option<&'a craftkit_sdk::Node> {
    let number = crate::number_of(value)?;
    document.walk().find(|node| number_of(node) == Some(number))
}

/// Whether a dictionary is a **font**: the tree's own rule.
fn is_a_font(entries: &[(String, Json)]) -> bool {
    name_of(entries, "Type").as_deref() == Some("Font")
        || (entry(entries, "BaseFont").is_some() && entry(entries, "Subtype").is_some())
}

/// Whether the **descriptor** of a font holds the font's program.
fn has_a_program(entries: &[(String, Json)]) -> bool {
    entry(entries, "FontFile").is_some()
        || entry(entries, "FontFile2").is_some()
        || entry(entries, "FontFile3").is_some()
}

/// The **descriptor** of a font: the object that says how the font is
/// drawn, and whether its program is in the file.
///
/// A composite font (a `Type0`) has no descriptor of its own: its
/// programs are in the fonts it descends from, and the first of them
/// is the one that answers.
fn descriptor_of<'a>(
    document: &'a Document,
    entries: &'a [(String, Json)],
) -> Option<&'a craftkit_sdk::Node> {
    if name_of(entries, "Subtype").as_deref() == Some("Type0") {
        let descendants = entry(entries, "DescendantFonts")?.as_array()?;
        let first = descendants.first()?;
        let font = node_of(document, first)?;
        let entries = entries_of(font);
        let descriptor = entry(&entries, "FontDescriptor")?;
        return node_of(document, descriptor);
    }
    let descriptor = entry(entries, "FontDescriptor")?;
    node_of(document, descriptor)
}

/// The **font** a node is: what the node says about itself.
pub(crate) fn font_of(document: &Document, node: &craftkit_sdk::Node) -> Option<Font> {
    let entries = entries_of(node);
    if !is_a_font(&entries) {
        return None;
    }
    let subtype = name_of(&entries, "Subtype").unwrap_or_default();
    let base = name_of(&entries, "BaseFont").unwrap_or_else(|| "a font".to_string());
    let embedded = if subtype == "Type3" {
        true
    } else {
        descriptor_of(document, &entries).is_some_and(|descriptor| has_a_program(&entries_of(descriptor)))
    };
    Some(Font { base, subtype, embedded })
}

/// The **fonts** of a document, in the order the document has them.
///
/// A font that is a `Type3` is embedded in the very object that names
/// it, and it needs no descriptor: it is embedded.
pub fn fonts(document: &Document) -> Vec<Font> {
    let mut fonts: Vec<Font> = Vec::new();
    for node in document.walk() {
        if node.kind != "pdf.object" {
            continue;
        }
        let entries = entries_of(node);
        if !is_a_font(&entries) {
            continue;
        }
        let Some(font) = font_of(document, node) else { continue };
        fonts.push(font);
    }
    fonts
}

/// **Read** the fonts of a PDF file.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "pdf.fonts", area = "pdf", layer = "pdf", icon = "list")]
pub struct ReadFonts {
    /// The font to read.
    ///
    /// Leave it out and every font of the document is read.
    pub font: Option<NodeRef>,
}

impl ReadFonts {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let document = session.document();
        let mut fonts = fonts(document);
        if let Some(font) = self.font {
            let node = document.node(font.0).ok_or_else(|| EngineError::Failed {
                id: "pdf.fonts".to_string(),
                message: format!("there is no node `{}`", font.0),
            })?;
            let one = font_of(document, node).ok_or_else(|| EngineError::Failed {
                id: "pdf.fonts".to_string(),
                message: format!("the node `{}` is not a font", font.0),
            })?;
            fonts = vec![one];
        }
        let count = fonts.len();
        Ok(Outcome::new(format!("the file has {count} font(s)"))
            .with_data(json!({ "fonts": fonts.iter().map(Font::to_json).collect::<Vec<_>>() })))
    }
}
