//! The **captions** of a document.
//!
//! A caption is a paragraph that **names** a thing: "Table 3 — the
//! sales of the year", "Figure 1 — the bridge". The **label** is what
//! the thing is called — `Table`, `Figure`, `Equation` — and the
//! **number** is the count of the captions that carry that label.
//!
//! # The rules
//!
//! - A caption is a **paragraph** that says what it captions, and the
//!   caption is a fact of the block: it is in its payload, beside its
//!   text.
//! - The **label** is at most **40** characters long, and it holds no
//!   quote, no backslash and no control character. An empty label is
//!   the tree's label, `Figure`.
//! - The **text** of the caption is what it says, and it may be
//!   empty: a caption with no text names the thing and says nothing
//!   else.
//! - The **format** of the number is one of the tree's own: `arabic`,
//!   `roman`, `alphabetic` and `ALPHABETIC`. A format that is not one
//!   of them is refused by name.
//! - The **number** of a caption is **derived**: it is the count of
//!   the captions that carry the same label, in the order of the
//!   document. It is worked out every time, so it cannot be out of
//!   date, and a caption that is taken away renumbers the ones after
//!   it.
//! - A block that is **not a paragraph** may not be a caption.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The longest **label** a caption may have.
pub const MAX_LABEL: usize = 40;

/// The longest **text** a caption may have.
pub const MAX_TEXT: usize = 1024;

/// The **formats** a number may be written in: the tree's own.
pub const FORMATS: [&str; 4] = ["arabic", "roman", "alphabetic", "ALPHABETIC"];

/// The label a caption has when it is not given one: the tree's.
pub const DEFAULT_LABEL: &str = "Figure";

/// The **caption** of a block, as it is kept.
pub fn caption_of(node: &Node) -> Option<Json> {
    node.data.get("caption").and_then(|caption| caption.as_object()).cloned().map(Json::Object)
}

/// The **format** a name says, if it is one.
fn format_of(name: &str, id: &str) -> Result<&'static str, EngineError> {
    // The **exact** name first: `alphabetic` and `ALPHABETIC` are two
    // formats and not one, so the spelling decides.
    FORMATS
        .iter()
        .find(|format| **format == name.trim())
        .copied()
        .or_else(|| FORMATS.iter().find(|format| format.eq_ignore_ascii_case(name.trim())).copied())
        .ok_or_else(|| EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no format `{name}`: the formats are {}", FORMATS.join(", ")),
        })
}

/// The **label** a name says: no quote, no backslash and no control
/// character, as the tree's.
fn label_of(name: &str, id: &str) -> Result<String, EngineError> {
    let label = name.trim();
    if label.chars().count() > MAX_LABEL {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a label is at most {MAX_LABEL} characters long"),
        });
    }
    if label.chars().any(|character| character == '"' || character == '\\' || character.is_control()) {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a label holds no quote, no backslash and no control character".to_string(),
        });
    }
    if label.is_empty() {
        return Ok(DEFAULT_LABEL.to_string());
    }
    Ok(label.to_string())
}

/// The **text** of a caption.
fn text_of(name: &str, id: &str) -> Result<String, EngineError> {
    if name.chars().count() > MAX_TEXT {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the text of a caption is at most {MAX_TEXT} characters long"),
        });
    }
    Ok(name.to_string())
}

/// The **captions** of a document, in order, each with the **number**
/// it has — derived.
pub fn captions_of(document: &Node) -> Json {
    let mut counts: Vec<(String, u64)> = Vec::new();
    let mut captions = Vec::new();
    for block in document.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
        let Some(caption) = caption_of(block) else { continue };
        let label =
            caption.get("label").and_then(|label| label.as_str()).unwrap_or(DEFAULT_LABEL).to_string();
        let format = caption.get("format").and_then(|format| format.as_str()).unwrap_or("arabic").to_string();
        let at = match counts.iter_mut().find(|(held, _)| *held == label) {
            Some((_, count)) => {
                *count += 1;
                *count
            }
            None => {
                counts.push((label.clone(), 1));
                1
            }
        };
        let mut caption = caption;
        caption["number"] = json!(crate::numbers::of(at, &format));
        caption["block"] = json!(block.id);
        captions.push(caption);
    }
    Json::Array(captions)
}

/// **Say** what a **caption** captions.
///
/// A paragraph that is a caption is changed, and a paragraph that is
/// not one becomes one.
#[derive(Command)]
#[command(id = "prose.block.caption", area = "prose", layer = "prose", icon = "tag", mutates, undoable)]
pub struct SetCaption {
    /// The paragraph.
    pub block: NodeRef,
    /// The label: what the thing is called.
    ///
    /// It is `Figure` when it is not given.
    pub label: Option<Text>,
    /// What the caption says.
    pub text: Option<Text>,
    /// The format of the number: `arabic`, `roman`, `alphabetic` or
    /// `ALPHABETIC`.
    pub format: Option<Text>,
}

impl SetCaption {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.caption".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        if node.kind != "prose.block.paragraph" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not a paragraph", node.kind),
            });
        }
        let mut caption = caption_of(&node)
            .unwrap_or_else(|| json!({ "label": DEFAULT_LABEL, "text": "", "format": "arabic" }));
        if let Some(label) = &self.label {
            caption["label"] = json!(label_of(&label.0, &id)?);
        }
        if let Some(text) = &self.text {
            caption["text"] = json!(text_of(&text.0, &id)?);
        }
        if let Some(format) = &self.format {
            caption["format"] = json!(format_of(&format.0, &id)?);
        }
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        node.data["caption"] = caption.clone();
        Ok(Outcome::changed(format!("the paragraph `{block}` is a caption"))
            .with_data(json!({ "block": block, "caption": caption })))
    }
}

/// **Read** the **captions** of a document, each with the **number**
/// it has.
#[derive(Command)]
#[command(id = "prose.document.captions", area = "prose", layer = "prose", icon = "list")]
pub struct ReadCaptions {
    /// The document.
    pub document: NodeRef,
}

impl ReadCaptions {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.captions".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document) else {
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
        let captions = captions_of(node);
        Ok(Outcome::new(format!(
            "the document `{document}` has {} captions",
            captions.as_array().map(|c| c.len()).unwrap_or(0)
        ))
        .with_data(json!({ "document": document, "captions": captions })))
    }
}
