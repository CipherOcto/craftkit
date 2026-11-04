//! The **bookmarks** of a document.
//!
//! A bookmark is a **name** for a place in a document, so that a link
//! may point at it: `#the-name`.
//!
//! The name is a fact of the **block** it is on — the tree's bookmark
//! is an inline object in the paragraph, and here it is the paragraph's
//! own payload — and the **bookmarks of a document** are derived: they
//! are the blocks that carry a name.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The most characters a bookmark's name may have.
///
/// Word's own bound, and the tree writes names in the file.
pub const MAX_NAME: usize = 40;

/// Whether a name is a name a bookmark may have.
///
/// The rule is Word's: a name begins with a letter or an underscore,
/// the rest of it is letters, digits and underscores, and it is not
/// longer than forty characters.
pub fn name_is_well_formed(name: &str) -> bool {
    if name.is_empty() || name.chars().count() > MAX_NAME {
        return false;
    }
    let mut characters = name.chars();
    let first = characters.next().unwrap_or(' ');
    if !(first.is_alphabetic() || first == '_') {
        return false;
    }
    characters.all(|character| character.is_alphanumeric() || character == '_')
}

/// The **bookmark** a block carries, if it carries one.
pub fn bookmark_of(node: &craftkit_sdk::Node) -> Option<String> {
    node.data.get("bookmark").and_then(|name| name.as_str()).map(|name| name.to_string())
}

/// The **bookmarks** of a document: the blocks that carry a name.
pub fn bookmarks_of(document: &craftkit_sdk::Document, document_node: u64) -> Vec<Json> {
    let Some(node) = document.node(document_node) else { return Vec::new() };
    node.children
        .iter()
        .filter(|child| child.kind.starts_with("prose.block."))
        .filter_map(|child| {
            let name = bookmark_of(child)?;
            Some(json!({ "block": child.id, "name": name }))
        })
        .collect()
}

/// Put a **bookmark** on a block.
///
/// The name must be one a bookmark may have, and it must not be a name
/// another block of the document already carries: two blocks with one
/// name are one place, and a link to that name would be a link to
/// nothing in particular.
#[derive(Command)]
#[command(id = "prose.block.bookmark", area = "prose", layer = "prose", icon = "tag", mutates, undoable)]
pub struct SetBookmark {
    /// The block.
    pub block: NodeRef,
    /// The name of the place.
    pub name: Text,
}

impl SetBookmark {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.bookmark".to_string();
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
        let name = self.name.0.trim().to_string();
        if !name_is_well_formed(&name) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "`{name}` is not a name a bookmark may have: a name begins with a letter or an underscore, the rest of it is letters, digits and underscores, and it is at most {MAX_NAME} characters long"
                ),
            });
        }
        // The **document** the block is in: the name is unique in it.
        let document = session
            .document()
            .walk()
            .find(|node| node.children.iter().any(|child| child.id == block))
            .map(|node| node.id);
        let Some(document) = document else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the block `{block}` is in no document"),
            });
        };
        for other in session.document().walk().filter(|node| node.kind == "prose.document") {
            for child in other.children.iter().filter(|child| child.kind.starts_with("prose.block.")) {
                if child.id == block {
                    continue;
                }
                if bookmark_of(child).as_deref() == Some(name.as_str()) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!("the name `{name}` is already in the document"),
                    });
                }
            }
        }
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        node.data["bookmark"] = Json::String(name.clone());
        Ok(Outcome::changed(format!("the block `{block}` is the place `{name}`"))
            .with_data(json!({ "block": block, "document": document, "name": name })))
    }
}

/// **Take away** the bookmark of a block.
///
/// A block with no bookmark has none to take away, and that is
/// refused.
#[derive(Command)]
#[command(
    id = "prose.block.bookmark.delete",
    area = "prose",
    layer = "prose",
    icon = "minus",
    mutates,
    undoable
)]
pub struct DeleteBookmark {
    /// The block.
    pub block: NodeRef,
}

impl DeleteBookmark {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.block.bookmark.delete".to_string();
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
        if bookmark_of(node).is_none() {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the block `{block}` has no bookmark"),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(block) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no block `{block}`"),
            });
        };
        node.data["bookmark"] = Json::Null;
        Ok(Outcome::changed(format!("the block `{block}` is no place")).with_data(json!({ "block": block })))
    }
}

/// **Read** the bookmarks of a document.
///
/// They are **derived**: they are the blocks that carry a name.
#[derive(Command)]
#[command(id = "prose.document.bookmarks", area = "prose", layer = "prose", icon = "list")]
pub struct ReadBookmarks {
    /// The document.
    pub document: NodeRef,
}

impl ReadBookmarks {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "prose.document.bookmarks".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document) else {
            return Err(EngineError::Failed { id, message: format!("there is no document `{document}`") });
        };
        if node.kind != "prose.document" {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{document}` is a `{}` and not a document", node.kind),
            });
        }
        let bookmarks = bookmarks_of(session.document(), document);
        Ok(Outcome::new(format!("the document `{document}` has {} bookmark(s)", bookmarks.len())).with_data(
            json!({
                "document": document,
                "bookmarks": bookmarks,
            }),
        ))
    }
}
