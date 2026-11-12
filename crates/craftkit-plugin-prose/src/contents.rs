//! The **table of contents** of a document.
//!
//! A table of contents is the list of the **headings** of a document,
//! with the level each of them is at. The tree's table of contents is
//! a field in the document and it is rebuilt when the document
//! changes; here it is **derived**: it is read out of the headings,
//! and it cannot be out of date.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A **heading** is a block on a style with a level, and the level
//!   is the level of the style with the level of its base under it.
//! - The **levels** that are in the table are the tree's: three of
//!   them, one to three, unless the reader asks for others. A level
//!   that is not a whole number from one to nine is refused.
//! - The **text** of an entry is the text the heading shows, so a
//!   heading that is a cross-reference shows what it references.
//! - The **number** of an entry is the number its heading shows, and
//!   a heading that is in a list of bullets has no number in the
//!   table: the tree's own rule.
//!
//! # What it does not do
//!
//! - **The table is not laid out.** The tree's table of contents ends
//!   every entry with the **page** the heading is on, and it has a
//!   tab leader and a right-aligned page number. There is no layout
//!   here, and so there is no page to show.
//! - **The table is not in the document.** The tree's is a field with
//!   the entries under it, and here it is read and not stored.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The **levels** that are in the table of contents when the reader
/// does not say: the tree's own, one to three.
pub const DEFAULT_LEVELS: u32 = 3;

/// The **least** a level may be.
pub const MIN_LEVEL: u32 = 1;

/// The **most** a level may be.
pub const MAX_LEVEL: u32 = 9;

/// The **entries** of a table of contents: the headings of a document,
/// in the order they stand.
///
/// The entries are derived from the blocks and their styles, and the
/// number of an entry is the number its heading shows.
pub fn entries_of(document: &craftkit_sdk::Document, document_node: u64, levels: u32) -> Vec<Json> {
    let Some(node) = document.node(document_node) else { return Vec::new() };
    let mut entries = Vec::new();
    for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
        let Some(style) = crate::styles::style_of_block(document, block.id) else { continue };
        let Some(level) = crate::outline::effective_level(document, style.id) else { continue };
        if !(MIN_LEVEL..=MAX_LEVEL).contains(&level) || level > levels {
            continue;
        }
        let text = crate::references::text_of(document, block);
        let text = match text {
            Json::String(text) => text.trim().to_string(),
            _ => String::new(),
        };
        if text.is_empty() {
            continue;
        }
        entries.push(json!({
            "block": block.id,
            "level": level,
            "text": text,
            "number": crate::numbering::number_of(document, block.id),
        }));
    }
    entries
}

/// **Read** the **table of contents** of a document: its headings, with
/// the level each of them is at.
#[derive(Command)]
#[command(id = "prose.document.contents", area = "prose", layer = "prose", icon = "list")]
pub struct ReadContents {
    /// The document.
    pub document: NodeRef,
    /// The levels that are in the table: three of them, one to three,
    /// unless the reader asks for others.
    pub levels: Option<Number>,
}

impl ReadContents {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.contents".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        if node.kind != "prose.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{document}` is a `{}` and not a document", node.kind),
            });
        }
        // The **levels**: the tree's own, and a level that is not a
        // whole number from one to nine is refused.
        let levels = match &self.levels {
            Some(levels) => {
                let value = levels.0;
                if !value.is_finite() || value.fract() != 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("a level is a whole number: {value} is not one"),
                    });
                }
                if value < f64::from(MIN_LEVEL) || value > f64::from(MAX_LEVEL) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "a level is a whole number from {MIN_LEVEL} to {MAX_LEVEL}: {value} is not one"
                        ),
                    });
                }
                value as u32
            }
            None => DEFAULT_LEVELS,
        };
        let entries = entries_of(session.document(), document, levels);
        Ok(Outcome::new(format!(
            "the table of contents of the document `{document}` has {} entries",
            entries.len()
        ))
        .with_data(json!({
            "document": document,
            "levels": levels,
            "entries": entries,
            "count": entries.len(),
        })))
    }
}
