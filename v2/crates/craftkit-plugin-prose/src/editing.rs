//! The **editing** of a document.
//!
//! A document is a **flat list of blocks**, and a list that can only
//! grow is a list that cannot be a document: the tree's word processor
//! writes a paragraph, and it takes it away, and it moves it, and it
//! changes what it says.
//!
//! So there are three commands here:
//!
//! - `prose.block.paragraph.text` changes **what a paragraph says**;
//! - `prose.block.remove` takes a block out of the document;
//! - `prose.block.move` puts a block somewhere else in the document.
//!
//! And the **runs** of a paragraph are not touched by any of them: a
//! run is an **interval** over the text, and the characters it covers
//! are **derived** — so when the text changes, the runs still say how
//! many characters they cover and what is under them is worked out
//! again. A run that runs past the end of the text covers what is
//! there and stops, and that is the model's own rule and not a new one.
//!
//! # What the port found
//!
//! - **The tree's order is the tree's order, and so is this one's.**
//!   The tree's paragraphs are a list in the document, and moving one
//!   is moving it in the list. Here the blocks are the document's
//!   children and the order of the children is the order of the
//!   blocks, so a move is a move of the children — and the styles and
//!   the media that live beside the blocks do not change their places.
//! - **The tree's move is up or down, and this one's is a place.** The
//!   tree has `para.moveUp` and `para.moveDown`, and each of them
//!   refuses at the end of the list. Here the block is given the place
//!   it goes to, counting from one among the blocks, and a place that
//!   is not there is refused — which is one command and not two.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// Change **what a paragraph says**.
///
/// The runs stay: a run is an interval over the text, and the text it
/// covers is derived.
#[derive(Command)]
#[command(
    id = "prose.block.paragraph.text",
    area = "prose",
    layer = "prose",
    icon = "pencil",
    mutates,
    undoable
)]
pub struct SetParagraphText {
    /// The paragraph.
    pub block: NodeRef,
    /// What it says now.
    pub text: Text,
}

impl SetParagraphText {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.paragraph.text".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
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
        let text = self.text.0.clone();
        let characters = text.chars().count();
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        node.data["text"] = Json::String(text.clone());
        Ok(Outcome::changed(format!("the paragraph {block} now says {characters} characters"))
            .with_data(json!({ "block": block, "characters": characters })))
    }
}

/// Take a **block** out of a document.
///
/// The block's edges go with it: a block that was in a style is not in
/// it any more.
#[derive(Command)]
#[command(id = "prose.block.remove", area = "prose", layer = "prose", icon = "trash", mutates, undoable)]
pub struct RemoveBlock {
    /// The block.
    pub block: NodeRef,
}

impl RemoveBlock {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.remove".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        if !is_block(node) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not a block", node.kind),
            });
        }
        if parent_of(session, block).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the block `{block}` is in no document"),
            });
        }
        if !session.document_mut()?.remove_node(block) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        }
        Ok(Outcome::changed(format!("the block `{block}` is taken away")).with_data(json!({ "id": block })))
    }
}

/// **Move** a block to another place in its document.
///
/// The place is counted from one among the blocks of the document, and
/// the blocks that are not the block itself keep their order.
#[derive(Command)]
#[command(id = "prose.block.move", area = "prose", layer = "prose", icon = "arrow", mutates, undoable)]
pub struct MoveBlock {
    /// The block.
    pub block: NodeRef,
    /// Which place it goes to, counting from one among the blocks of
    /// the document.
    pub to: Number,
}

impl MoveBlock {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.move".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        if !is_block(node) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not a block", node.kind),
            });
        }
        let Some(document) = parent_of(session, block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the block `{block}` is in no document"),
            });
        };
        let to = self.to.0;
        if !to.is_finite() || to < 1.0 || to.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a place is counted from one: {to} is not one"),
            });
        }
        let to = to as usize;
        let Some(document_node) = session.document().node(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{document}`"),
            });
        };
        let blocks: Vec<u64> = document_node
            .children
            .iter()
            .filter(|child| child.kind.starts_with("prose.block."))
            .map(|child| child.id)
            .collect();
        let count = blocks.len();
        if to > count {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the document `{document}` has {count} blocks and the {to} is not one of them"
                ),
            });
        }
        let from = blocks.iter().position(|other| *other == block).unwrap_or(0);
        if to == from + 1 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the block `{block}` is already the {to}"),
            });
        }
        // The **children** of the document: the block comes out of them
        // and goes back in at the place it was told.
        let Some(node) = session.document_mut()?.node_mut(document) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{document}`"),
            });
        };
        let mut children = node.children.clone();
        let Some(at) = children.iter().position(|child| child.id == block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the block `{block}` is in no document"),
            });
        };
        let taken = children.remove(at);
        let blocks: Vec<usize> = children
            .iter()
            .enumerate()
            .filter(|(_, child)| child.kind.starts_with("prose.block."))
            .map(|(index, _)| index)
            .collect();
        let insert_at = if to - 1 < blocks.len() { blocks[to - 1] } else { children.len() };
        children.insert(insert_at, taken);
        node.children = children;
        Ok(Outcome::changed(format!("the block `{block}` is now the {to} of the document `{document}`"))
            .with_data(json!({ "block": block, "document": document, "to": to })))
    }
}

/// The **alignments** a paragraph may have, in the tree's own names.
///
/// The tree's `start` and `end` are the logical edges, and the tree
/// writes them down as `left` and `right` — and so does this one.
pub const ALIGNMENTS: [&str; 5] = ["left", "center", "right", "justify", "distribute"];

/// The **alignment** a name says, if it is one.
fn alignment_of(name: &str) -> Option<&'static str> {
    let wanted = name.trim();
    match wanted.to_lowercase().as_str() {
        "left" | "start" => Some("left"),
        "center" => Some("center"),
        "right" | "end" => Some("right"),
        "justify" | "both" => Some("justify"),
        "distribute" => Some("distribute"),
        _ => None,
    }
}

/// The **largest** an indent may be, in points.
const MAX_INDENT: f64 = 1584.0;

/// The **largest** a spacing may be, in points.
const MAX_SPACING: f64 = 1584.0;

/// Say how a paragraph **stands**.
///
/// The alignment is one of the tree's own: `left`, `center`, `right`,
/// `justify` or `distribute`. The tree's `start` and `end` are the
/// logical edges and it writes them down as `left` and `right`.
#[derive(Command)]
#[command(
    id = "prose.block.paragraph.align",
    area = "prose",
    layer = "prose",
    icon = "align",
    mutates,
    undoable
)]
pub struct SetAlignment {
    /// The paragraph.
    pub block: NodeRef,
    /// How it stands: `left`, `center`, `right`, `justify` or
    /// `distribute`.
    pub align: Text,
}

impl SetAlignment {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.paragraph.align".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
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
        let Some(align) = alignment_of(&self.align.0) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no alignment `{}`: the alignments are {}",
                    self.align.0.trim(),
                    ALIGNMENTS.join(", ")
                ),
            });
        };
        if let Some(node) = session.document_mut()?.node_mut(block) {
            node.data["align"] = Json::String(align.to_string());
        }
        Ok(Outcome::changed(format!("the paragraph {block} stands {align}"))
            .with_data(json!({ "block": block, "align": align })))
    }
}

/// Say how far a paragraph is **indented**, in points.
///
/// Only what is named is changed. An indent outside the tree's own
/// range is **refused**: the tree clamps it and says nothing.
#[derive(Command)]
#[command(
    id = "prose.block.paragraph.indent",
    area = "prose",
    layer = "prose",
    icon = "indent",
    mutates,
    undoable
)]
pub struct SetIndent {
    /// The paragraph.
    pub block: NodeRef,
    /// How far from the left edge, in points.
    pub left: Option<Number>,
    /// How far from the right edge, in points.
    pub right: Option<Number>,
    /// How far the first line is in from the left, in points. A
    /// negative number is a **hanging** indent.
    pub first_line: Option<Number>,
}

impl SetIndent {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.paragraph.indent".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
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
        let left = indent_of(self.left, &id)?;
        let right = indent_of(self.right, &id)?;
        let first_line = indent_of(self.first_line, &id)?;
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        if let Some(left) = left {
            node.data["indent_left"] = Json::from(left);
        }
        if let Some(right) = right {
            node.data["indent_right"] = Json::from(right);
        }
        if let Some(first_line) = first_line {
            node.data["indent_first"] = Json::from(first_line);
        }
        Ok(Outcome::changed(format!("the paragraph {block} is indented")).with_data(json!({
            "block": block,
            "left": left,
            "right": right,
            "first_line": first_line,
        })))
    }
}

/// An indent, if it is one.
fn indent_of(value: Option<Number>, id: &str) -> Result<Option<f64>, EngineError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let points = value.0;
    if !points.is_finite() || points < -MAX_INDENT || points > MAX_INDENT {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!(
                "an indent is between -{MAX_INDENT} and {MAX_INDENT} points, and {points} is not"
            ),
        });
    }
    Ok(Some(points))
}

/// Say how much **space** is around a paragraph, in points.
///
/// Only what is named is changed. A space outside the tree's own range
/// is **refused**: the tree clamps it and says nothing.
#[derive(Command)]
#[command(
    id = "prose.block.paragraph.spacing",
    area = "prose",
    layer = "prose",
    icon = "space",
    mutates,
    undoable
)]
pub struct SetSpacing {
    /// The paragraph.
    pub block: NodeRef,
    /// How much space is above it, in points.
    pub before: Option<Number>,
    /// How much space is below it, in points.
    pub after: Option<Number>,
}

impl SetSpacing {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.paragraph.spacing".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block) else {
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
        let before = spacing_of(self.before, &id)?;
        let after = spacing_of(self.after, &id)?;
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{block}`"),
            });
        };
        if let Some(before) = before {
            node.data["space_before"] = Json::from(before);
        }
        if let Some(after) = after {
            node.data["space_after"] = Json::from(after);
        }
        Ok(Outcome::changed(format!("the paragraph {block} has its space")).with_data(json!({
            "block": block,
            "before": before,
            "after": after,
        })))
    }
}

/// A space, if it is one.
fn spacing_of(value: Option<Number>, id: &str) -> Result<Option<f64>, EngineError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let points = value.0;
    if !points.is_finite() || points < 0.0 || points > MAX_SPACING {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a space is between 0 and {MAX_SPACING} points, and {points} is not"),
        });
    }
    Ok(Some(points))
}

/// Whether a node is a **block**: a paragraph or a table.
fn is_block(node: &Node) -> bool {
    node.kind.starts_with("prose.block.")
}

/// The **document** a block is in.
fn parent_of(session: &Session, block: u64) -> Option<u64> {
    let document = session.document();
    for node in document.walk() {
        if node.children.iter().any(|child| child.id == block) {
            return Some(node.id);
        }
    }
    None
}
