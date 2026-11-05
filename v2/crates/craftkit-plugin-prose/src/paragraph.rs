//! The **paragraph** of a style.
//!
//! A style carries the **character** of the text it applies to, and it
//! carries the **paragraph** it applies to: how the text stands on the
//! page — how it is aligned, how much room is above it and below it,
//! how its lines are spaced and how far it is indented.
//!
//! The rules:
//!
//! - an **alignment** is one of the tree's own: `left`, `centre`,
//!   `right`, `justify` or `distribute`;
//! - a **space** above or below a paragraph is a number of points that
//!   is not negative;
//! - a **line spacing** is a multiple of the line — one is a single
//!   line and two is a double one;
//! - an **indent** is a number of points, and the indent of the first
//!   line may be **negative**: a negative one hangs the line.
//!
//! A style that is **based on** another has the paragraph of its base
//! **under it**, as it has the character of its base under it: a style
//! that says nothing about the indentation has the indentation of its
//! base, and a style that says one has its own.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::styles::{chain_of, style_of_block};

/// The **alignments** a paragraph may have: the tree's own.
pub const ALIGNMENTS: [&str; 5] = ["left", "centre", "right", "justify", "distribute"];

/// The **most** room a paragraph may have above it or below it, in
/// points.
pub const MAX_SPACE: f64 = 10_000.0;

/// The **most** a line spacing may be, as a multiple.
pub const MAX_LINE_SPACING: f64 = 100.0;

/// The **most** an indent may be, in points.
pub const MAX_INDENT: f64 = 10_000.0;

/// The **paragraph** a style carries.
pub fn paragraph_of(style: &Node) -> Json {
    style.data.get("paragraph").cloned().unwrap_or(Json::Object(craftkit_sdk::serde_json::Map::new()))
}

/// The **paragraph** of a style, with the paragraph of its base under
/// it.
///
/// The chain is walked from the furthest base to the style itself, so
/// that the nearer style wins.
pub fn effective_paragraph(document: &craftkit_sdk::Document, style: u64) -> Json {
    let mut paragraph = craftkit_sdk::serde_json::Map::new();
    for id in chain_of(document, style) {
        let Some(node) = document.node(id) else { continue };
        let Some(map) = paragraph_of(node).as_object().cloned() else { continue };
        for (key, value) in map {
            paragraph.insert(key, value);
        }
    }
    Json::Object(paragraph)
}

/// The **alignment** a name says, if it is one of the tree's.
pub fn alignment_of(text: &str) -> Option<&'static str> {
    match text.trim().to_ascii_lowercase().as_str() {
        "left" => Some("left"),
        "centre" | "center" => Some("centre"),
        "right" => Some("right"),
        "justify" => Some("justify"),
        "distribute" => Some("distribute"),
        _ => None,
    }
}

/// **Set** the paragraph of a style.
///
/// The property that is said is set, and the property that is not said
/// stays as it was.
#[derive(Command)]
#[command(id = "prose.style.paragraph", area = "prose", layer = "prose", icon = "pencil", mutates, undoable)]
pub struct SetStyleParagraph {
    /// The style.
    pub style: NodeRef,
    /// The **alignment** of the paragraph.
    pub alignment: Option<Text>,
    /// The room **above** the paragraph, in points.
    pub space_before: Option<Number>,
    /// The room **below** the paragraph, in points.
    pub space_after: Option<Number>,
    /// The **line spacing**, as a multiple of the line.
    pub line_spacing: Option<Number>,
    /// The **indent** of the paragraph, in points.
    pub indent_left: Option<Number>,
    /// The **indent** at the right of the paragraph, in points.
    pub indent_right: Option<Number>,
    /// The **indent of the first line**, in points. A negative one
    /// hangs the line.
    pub indent_first: Option<Number>,
}

impl SetStyleParagraph {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.style.paragraph".to_string();
        let style = self.style.0;
        let Some(node) = session.document().node(style) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no style `{style}`"),
            });
        };
        if node.kind != "prose.style" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{style}` is a `{}` and not a style", node.kind),
            });
        }
        let mut paragraph = match paragraph_of(node).as_object().cloned() {
            Some(map) => map,
            None => craftkit_sdk::serde_json::Map::new(),
        };
        if let Some(alignment) = &self.alignment {
            let Some(alignment) = alignment_of(&alignment.0) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "`{}` is not an alignment: it is one of {}",
                        alignment.0.trim(),
                        ALIGNMENTS.join(", ")
                    ),
                });
            };
            paragraph.insert("alignment".to_string(), Json::String(alignment.to_string()));
        }
        let room = |value: Number, what: &str, most: f64| -> Result<f64, EngineError> {
            let value = value.0;
            if !value.is_finite() || value < 0.0 || value > most {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{value} is not {what}: it is a number from zero to {most}"),
                });
            }
            Ok(value)
        };
        if let Some(space) = &self.space_before {
            let space = room(*space, "a space above a paragraph", MAX_SPACE)?;
            paragraph.insert("space_before".to_string(), Json::from(space));
        }
        if let Some(space) = &self.space_after {
            let space = room(*space, "a space below a paragraph", MAX_SPACE)?;
            paragraph.insert("space_after".to_string(), Json::from(space));
        }
        if let Some(spacing) = &self.line_spacing {
            let spacing = spacing.0;
            if !spacing.is_finite() || spacing <= 0.0 || spacing > MAX_LINE_SPACING {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "{spacing} is not a line spacing: it is a multiple of the line from zero to {MAX_LINE_SPACING}"
                    ),
                });
            }
            paragraph.insert("line_spacing".to_string(), Json::from(spacing));
        }
        let indent = |value: Number, what: &str| -> Result<f64, EngineError> {
            let value = value.0;
            if !value.is_finite() || value < 0.0 || value > MAX_INDENT {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("{value} is not {what}: it is a number from zero to {MAX_INDENT}"),
                });
            }
            Ok(value)
        };
        if let Some(value) = &self.indent_left {
            let value = indent(*value, "an indent")?;
            paragraph.insert("indent_left".to_string(), Json::from(value));
        }
        if let Some(value) = &self.indent_right {
            let value = indent(*value, "an indent at the right")?;
            paragraph.insert("indent_right".to_string(), Json::from(value));
        }
        if let Some(value) = &self.indent_first {
            let value = value.0;
            if !value.is_finite() || value < -MAX_INDENT || value > MAX_INDENT {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "{value} is not an indent of the first line: it is a number from minus {MAX_INDENT} to {MAX_INDENT}"
                    ),
                });
            }
            paragraph.insert("indent_first".to_string(), Json::from(value));
        }
        if paragraph.is_empty() {
            return Err(EngineError::Failed {
                id,
                message: "the command says nothing to change".to_string(),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(style) {
            node.data["paragraph"] = Json::Object(paragraph);
        }
        Ok(Outcome::changed(format!("the paragraph of the style `{style}` is set"))
            .with_data(json!({ "style": style })))
    }
}

/// **Read** the paragraph of a style, with the paragraph of its base
/// under it.
#[derive(Command)]
#[command(id = "prose.style.paragraph.read", area = "prose", layer = "prose", icon = "list")]
pub struct ReadStyleParagraph {
    /// The style.
    pub style: NodeRef,
}

impl ReadStyleParagraph {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.style.paragraph.read".to_string();
        let style = self.style.0;
        let Some(node) = session.document().node(style) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no style `{style}`"),
            });
        };
        if node.kind != "prose.style" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{style}` is a `{}` and not a style", node.kind),
            });
        }
        let paragraph = effective_paragraph(session.document(), style);
        Ok(Outcome::new(format!("the style `{style}` has a paragraph")).with_data(json!({
            "style": style,
            "paragraph": paragraph,
            "chain": chain_of(session.document(), style),
        })))
    }
}

/// **Read** the paragraph of a block: the paragraph of the style it is
/// on.
#[derive(Command)]
#[command(id = "prose.block.paragraph", area = "prose", layer = "prose", icon = "list")]
pub struct ReadBlockParagraph {
    /// The block.
    pub block: NodeRef,
}

impl ReadBlockParagraph {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.paragraph".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        if !node.kind.starts_with("prose.block.") {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not a block", node.kind),
            });
        }
        let Some(style) = style_of_block(session.document(), block) else {
            return Ok(Outcome::new(format!("the block `{block}` is on no style")).with_data(json!({
                "block": block,
                "style": Json::Null,
                "paragraph": Json::Object(craftkit_sdk::serde_json::Map::new()),
            })));
        };
        let paragraph = effective_paragraph(session.document(), style.id);
        Ok(Outcome::new(format!("the block `{block}` is on the style `{}`", style.name)).with_data(json!({
            "block": block,
            "style": style.id,
            "paragraph": paragraph,
        })))
    }
}
