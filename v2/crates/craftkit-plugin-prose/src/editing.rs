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
