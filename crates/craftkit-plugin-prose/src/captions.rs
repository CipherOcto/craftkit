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

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Text};

/// The longest **label** a caption may have.
pub const MAX_LABEL: usize = 40;

/// The longest **text** a caption may have.
pub const MAX_TEXT: usize = 1024;

/// The **formats** a number may be written in: the tree's own.
pub const FORMATS: [&str; 4] = ["arabic", "roman", "alphabetic", "ALPHABETIC"];

/// The label a caption has when it is not given one: the tree's.
pub const DEFAULT_LABEL: &str = "Figure";

/// The **separators** the tree offers between a chapter number and a
/// caption number.
pub const SEPARATORS: [&str; 5] = ["-", ".", ":", "\u{2014}", "\u{2013}"];

/// The separator a caption has when it is not given one: the tree's.
pub const DEFAULT_SEPARATOR: &str = "-";

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
pub fn label_of(name: &str, id: &str) -> Result<String, EngineError> {
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

/// The **separator** a caption has, and whether it is one of the
/// tree's.
fn separator_of(name: &str, id: &str) -> Result<String, EngineError> {
    let separator = name.trim();
    if SEPARATORS.contains(&separator) {
        return Ok(separator.to_string());
    }
    Err(EngineError::Failed {
        id: id.to_string(),
        message: format!("the separator `{separator}` is not one of the tree's"),
    })
}

/// The **captions** of a document, in order, each with the **number**
/// it has — derived.
pub fn captions_of(document: &craftkit_sdk::Document, node: &Node) -> Json {
    // The **counts**: the label, the number and the chapter the number
    // was counted in.
    let mut counts: Vec<(String, u64, u64)> = Vec::new();
    // The **headings** of the document: how many there are of each
    // level.
    let mut headings: [u64; 9] = [0; 9];
    let mut captions = Vec::new();
    for block in node.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
        // The **headings**: a paragraph whose style has a level.
        if let Some(style) = crate::styles::style_of_block(document, block.id)
            && let Some(level) = crate::outline::effective_level(document, style.id)
            && (1..=9).contains(&level)
        {
            headings[(level - 1) as usize] += 1;
        }
        let Some(caption) = caption_of(block) else { continue };
        let label =
            caption.get("label").and_then(|label| label.as_str()).unwrap_or(DEFAULT_LABEL).to_string();
        let format = caption.get("format").and_then(|format| format.as_str()).unwrap_or("arabic").to_string();
        // The **chapter** the caption names, and the number of that
        // chapter: the count of the headings of that level.
        let chapter = caption.get("chapter").and_then(|chapter| chapter.as_u64());
        let epoch = chapter.map(|level| headings[(level - 1) as usize]);
        let at = match counts.iter_mut().find(|(held, _, _)| *held == label) {
            Some((_, count, held)) => {
                // The **restart**: the number is counted again when
                // the chapter changes — the tree's own rule, and only
                // for a caption that names a chapter.
                if let Some(epoch) = epoch
                    && *held != epoch
                {
                    *count = 0;
                    *held = epoch;
                }
                *count += 1;
                *count
            }
            None => {
                counts.push((label.clone(), 1, epoch.unwrap_or(0)));
                1
            }
        };
        let mut caption = caption;
        let number = crate::numbers::of(at, &format);
        caption["number"] = json!(number.clone());
        match (chapter, epoch) {
            (Some(_), Some(epoch)) => {
                let separator = caption
                    .get("separator")
                    .and_then(|separator| separator.as_str())
                    .unwrap_or(DEFAULT_SEPARATOR)
                    .to_string();
                caption["chapter"] = json!(epoch);
                caption["full"] = json!(format!("{epoch}{separator}{number}"));
            }
            _ => {
                caption["chapter"] = Json::Null;
                caption["full"] = Json::Null;
            }
        }
        caption["block"] = json!(block.id);
        captions.push(caption);
    }
    Json::Array(captions)
}

commands! {
    /// **Say** what a **caption** captions.
    ///
    /// A paragraph that is a caption is changed, and a paragraph that is
    /// not one becomes one.
    prose.block.caption {
        area: "prose",
        layer: "prose",
        icon: "tag",
        mutates,
        undoable,
        params: {
            /// The paragraph.

            block: NodeRef,
            /// The label: what the thing is called.
            ///
            /// It is `Figure` when it is not given.

            label: Option<Text>,
            /// What the caption says.

            text: Option<Text>,
            /// The format of the number: `arabic`, `roman`, `alphabetic` or
            /// `ALPHABETIC`.

            format: Option<Text>,
            /// The **chapter** the caption is in: the level of a heading, one
            /// to nine.
            ///
            /// The number of a caption that names a chapter is counted again
            /// at each chapter, and the caption says the chapter it is in.

            chapter: Option<Number>,
            /// The **separator** between the chapter number and the number of
            /// the caption: `-`, `.`, `:`, an em dash or an en dash.

            separator: Option<Text>,
        },
        run: {

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
            if let Some(chapter) = &self.chapter {
                let level = chapter.0;
                if !level.is_finite() || !(1.0..=9.0).contains(&level) || level.fract() != 0.0 {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the chapter `{level}` is not one of the tree's nine levels"),
                    });
                }
                caption["chapter"] = json!(level as u64);
            }
            if let Some(separator) = &self.separator {
                caption["separator"] = json!(separator_of(&separator.0, &id)?);
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
    prose.document.captions {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The document.

            document: NodeRef,
        },
        run: {

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
            let captions = captions_of(session.document(), node);
            Ok(Outcome::new(format!(
                "the document `{document}` has {} captions",
                captions.as_array().map(|c| c.len()).unwrap_or(0)
            ))
            .with_data(json!({ "document": document, "captions": captions })))

        }
    }
}
