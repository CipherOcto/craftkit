//! The **numbering** of the pages.
//!
//! The **format** the numbers are written in — `decimal`,
//! `lowerRoman`, `upperRoman`, `lowerLetter` or `upperLetter` — and
//! the number the **first** page has.
//!
//! # The rules
//!
//! - The format is one of the tree's own, and a format that is not
//!   one of them is refused by name.
//! - The **start** is from **one** to **a hundred thousand**. The
//!   tree clamps a start to that range and says nothing; here a start
//!   outside it is refused.
//! - A part that is **not named** keeps what it was.
//! - The **first** page's number is **derived**: it is the start,
//!   written in the format — `iv` for a start of four and a format of
//!   `lowerRoman`.
//! - There is no **section** here, so the tree's "the pages number
//!   on from the section before" is refused by name.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The most a **start** may be.
pub const MAX_START: u64 = 100_000;

/// The **numbering** of the pages, as it is kept.
pub fn numbering_of(node: &Node) -> Json {
    node.data
        .get("page_numbers")
        .and_then(|numbering| numbering.as_object())
        .cloned()
        .map(Json::Object)
        .unwrap_or_else(|| Json::Object(craftkit_sdk::serde_json::Map::new()))
}

/// The **numbering** of the pages, as it is read: the format, the
/// start, and the **first** page's number, which is derived.
pub fn read_of(node: &Node) -> Json {
    let numbering = numbering_of(node);
    let format = numbering.get("format").and_then(|format| format.as_str()).unwrap_or("decimal").to_string();
    let start = numbering.get("start").and_then(|start| start.as_u64()).unwrap_or(1);
    json!({
        "format": format,
        "start": start,
        "first": crate::numbers::of(start, &format),
    })
}

/// The **format** a name says, if it is one.
fn format_of(name: &str, id: &str) -> Result<&'static str, EngineError> {
    crate::numbers::PAGE_FORMATS
        .iter()
        .find(|format| **format == name.trim())
        .copied()
        .or_else(|| {
            crate::numbers::PAGE_FORMATS
                .iter()
                .find(|format| format.eq_ignore_ascii_case(name.trim()))
                .copied()
        })
        .ok_or_else(|| EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "there is no format `{name}`: the formats are {}",
                crate::numbers::PAGE_FORMATS.join(", ")
            ),
        })
}

/// **Set** the **numbering** of the pages.
///
/// The format the numbers are written in, and the number the first
/// page has.
#[derive(Command)]
#[command(id = "prose.page.numbers", area = "prose", layer = "prose", icon = "list", mutates, undoable)]
pub struct SetPageNumbers {
    /// The document.
    pub document: NodeRef,
    /// The format the numbers are written in.
    ///
    /// It is `decimal` when it is not given.
    pub format: Option<Text>,
    /// The number the first page has.
    ///
    /// It is from one to a hundred thousand.
    pub start: Option<Number>,
}

impl SetPageNumbers {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.page.numbers".to_string();
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
        let mut numbering = numbering_of(&node);
        if let Some(format) = &self.format {
            numbering["format"] = json!(format_of(&format.0, &id)?);
        }
        if let Some(start) = &self.start {
            let value = start.0;
            if !value.is_finite() || value.fract() != 0.0 || value < 1.0 || value > MAX_START as f64 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("a start is from one to {MAX_START}, and {value} is not one of them"),
                });
            }
            numbering["start"] = json!(value as u64);
        }
        let Some(node) = session.document_mut()?.node_mut(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        node.data["page_numbers"] = numbering.clone();
        Ok(Outcome::changed(format!("the pages of the document `{document}` are numbered"))
            .with_data(json!({ "document": document, "numbers": read_of(node) })))
    }
}
