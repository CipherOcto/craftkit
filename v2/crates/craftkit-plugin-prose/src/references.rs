//! The **cross-reference** of a document.
//!
//! A block may **reference** another: it stands for the text of a
//! block that carries a **bookmark**. The text it shows is
//! **derived** — worked out from the block it points at — and it is
//! never written down, so a reference cannot be out of date.
//!
//! The reference is an **edge**: from the block that references to the
//! block that is referenced, and the name of the bookmark is the
//! payload. The name is a fact of the document, and the edge is the
//! reference.
//!
//! The rules:
//!
//! - a **bookmark** that is not in the document is refused: a
//!   reference to a name that is not there points at nothing;
//! - a block that points at a block with **no text** shows **nothing**,
//!   and nothing is not a text of zero characters and a half;
//! - the text of a block that is **not** a reference is the text it
//!   stores.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **kind** of the edge that says a block references another.
pub const REFERENCES: &str = "prose.references";

/// The **kind** of a block that references another.
pub const REFERENCE_KIND: &str = "prose.block.reference";

/// The block a reference **points at**.
pub fn target_of(document: &craftkit_sdk::Document, block: u64) -> Option<u64> {
    document
        .relations
        .iter()
        .find(|relation| relation.kind == REFERENCES && relation.from == block)
        .map(|relation| relation.to)
}

/// The **text** of a block.
///
/// For a reference it is the text of the block it points at, and it is
/// **derived**: it is worked out every time, and it is never stored.
pub fn text_of(document: &craftkit_sdk::Document, block: &Node) -> Json {
    if block.kind != REFERENCE_KIND {
        return block.data.get("text").cloned().unwrap_or(Json::Null);
    }
    let Some(target) = target_of(document, block.id) else { return Json::Null };
    let Some(node) = document.node(target) else { return Json::Null };
    node.data.get("text").cloned().unwrap_or(Json::Null)
}

/// The block of a document that carries a **bookmark** of that name.
fn bookmarked(document: &craftkit_sdk::Document, parent: u64, name: &str) -> Option<u64> {
    document
        .node(parent)?
        .children
        .iter()
        .filter(|child| child.kind.starts_with("prose.block."))
        .find(|child| crate::bookmarks::bookmark_of(child).as_deref() == Some(name))
        .map(|child| child.id)
}

/// **Add** a cross-reference to a document.
///
/// The reference is a block that stands for the text of the block that
/// carries the bookmark, and its text is derived from that block.
#[derive(Command)]
#[command(
    id = "prose.block.reference.add",
    area = "prose",
    layer = "prose",
    icon = "plus",
    mutates,
    undoable
)]
pub struct AddReference {
    /// The document it goes in.
    pub document: NodeRef,
    /// The **bookmark** it points at.
    pub target: Text,
}

impl AddReference {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.reference.add".to_string();
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
        let name = self.target.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a reference points at a bookmark, and a bookmark has a name".to_string(),
            });
        }
        let Some(target) = bookmarked(session.document(), document, &name) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no bookmark `{name}` in the document"),
            });
        };
        let block = session.document_mut()?.add_node_with_under(
            Some(document),
            name.clone(),
            REFERENCE_KIND,
            None,
            json!({ "target": name.clone() }),
        );
        session.document_mut()?.add_relation(craftkit_sdk::Relation::with_payload(
            block,
            target,
            REFERENCES,
            json!({ "bookmark": name.clone() }),
        ));
        Ok(Outcome::changed(format!("the document references `{name}`"))
            .with_data(json!({ "block": block, "target": target, "bookmark": name })))
    }
}

/// **Point** a cross-reference at another bookmark.
///
/// The text of the reference changes with the block it points at, and
/// the text it had is gone: a reference is never out of date.
#[derive(Command)]
#[command(
    id = "prose.block.reference.set",
    area = "prose",
    layer = "prose",
    icon = "pencil",
    mutates,
    undoable
)]
pub struct SetReference {
    /// The reference.
    pub block: NodeRef,
    /// The **bookmark** it points at.
    pub target: Text,
}

impl SetReference {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.reference.set".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        if node.kind != REFERENCE_KIND {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{block}` is a `{}` and not a reference", node.kind),
            });
        }
        let name = self.target.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a reference points at a bookmark, and a bookmark has a name".to_string(),
            });
        }
        // The **document** the reference is in.
        let document = session
            .document()
            .walk()
            .find(|node| node.kind == "prose.document" && node.children.iter().any(|child| child.id == block))
            .map(|node| node.id);
        let Some(document) = document else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the reference `{block}` is in no document"),
            });
        };
        let Some(target) = bookmarked(session.document(), document, &name) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no bookmark `{name}` in the document"),
            });
        };
        session
            .document_mut()?
            .remove_relations(|relation| relation.kind == REFERENCES && relation.from == block);
        if let Some(node) = session.document_mut()?.node_mut(block) {
            node.data["target"] = json!(name.clone());
        }
        session.document_mut()?.add_relation(craftkit_sdk::Relation::with_payload(
            block,
            target,
            REFERENCES,
            json!({ "bookmark": name.clone() }),
        ));
        Ok(Outcome::changed(format!("the reference points at `{name}`"))
            .with_data(json!({ "block": block, "target": target, "bookmark": name })))
    }
}

/// **Read** the text of a block: the text it shows, which for a
/// reference is the text of the block it points at.
#[derive(Command)]
#[command(id = "prose.block.reference.read", area = "prose", layer = "prose", icon = "list")]
pub struct ReadReference {
    /// The block.
    pub block: NodeRef,
}

impl ReadReference {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.reference.read".to_string();
        let block = self.block.0;
        let Some(node) = session.document().node(block).cloned() else {
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
        let document = session.document();
        let text = text_of(document, &node);
        let target = target_of(document, block);
        Ok(Outcome::new(format!("the block `{block}` has a text")).with_data(json!({
            "block": block,
            "kind": node.kind,
            "text": text,
            "target": match target {
                Some(target) => json!(target),
                None => Json::Null,
            },
        })))
    }
}
