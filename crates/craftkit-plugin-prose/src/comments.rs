//! The **comment** on a block.
//!
//! A comment is something somebody said about a piece of a document:
//! **who** said it and **what** they said. The tree's comment belongs
//! to the **document** and **anchors** to a range of words; here a
//! comment belongs to the document and names the **block** it is on.
//!
//! So the comments live in the **payload** of the document: they are
//! numerous, and they hold nothing — which is the rule this battery
//! found, with the grid's cells and the CAD's geometry and the
//! paragraph's runs behind it.
//!
//! And a comment goes with the **block** it is on: a block that is
//! taken away takes its comment with it, and a comment on a block that
//! is not there is a comment on nothing.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

/// The **key** a block's comment is kept under.
fn key(block: u64) -> String {
    block.to_string()
}

/// The **comments** of a document, as they are kept.
fn comments_of(document: &Node) -> Json {
    document.data.get("comments").cloned().unwrap_or_else(|| json!({}))
}

/// The **block** a comment is on, and the document it is in.
///
/// The block must be a block, and it must be in a document: a comment
/// belongs to a document, and a block that is in no document has
/// nowhere to keep one.
fn the_block(session: &Session, command: &str, block: u64) -> Result<u64, EngineError> {
    let Some(node) = session.document().node(block) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no block `{block}`"),
        });
    };
    if !node.kind.starts_with("prose.block.") {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{block}` is a `{}` and not a block", node.kind),
        });
    }
    let document = session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == block))
        .map(|node| node.id);
    match document {
        Some(document) => Ok(document),
        None => Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the block `{block}` is in no document"),
        }),
    }
}

commands! {
    /// **Put** a comment on a block.
    ///
    /// A block has at most one comment, and a comment put on it again
    /// **replaces** the one that was there.
    prose.comment.set {
        area: "prose",
        layer: "prose",
        icon: "quote",
        mutates,
        undoable,
        params: {
            /// The block the comment is on.

            block: NodeRef,
            /// Who said it.

            author: Text,
            /// What they said.

            text: Text,
        },
        run: {

            let id = "prose.comment.set".to_string();
            let block = self.block.0;
            let document = the_block(session, &id, block)?;
            let author = self.author.0.trim().to_string();
            if author.is_empty() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a comment has an author".to_string(),
                });
            }
            let text = self.text.0.clone();
            let comment = json!({ "author": author, "text": text });
            let Some(node) = session.document_mut()?.node_mut(document) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no document `{document}`"),
                });
            };
            let mut comments = comments_of(node);
            if !comments.is_object() {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the comments of the document `{document}` are not a group"),
                });
            }
            comments[key(block)] = comment.clone();
            node.data["comments"] = comments;
            Ok(Outcome::changed(format!("the block `{block}` has a comment from {author}"))
                .with_data(json!({ "block": block, "document": document, "comment": comment })))

        }
    }
    /// **Read** the comment on a block.
    ///
    /// A block with no comment has no comment, and that is said as
    /// nothing and not as an error.
    prose.comment.get {
        area: "prose",
        layer: "prose",
        icon: "list",
        params: {
            /// The block.

            block: NodeRef,
        },
        run: {

            let id = "prose.comment.get".to_string();
            let block = self.block.0;
            let document = the_block(session, &id, block)?;
            let comment = session
                .document()
                .node(document)
                .map(comments_of)
                .and_then(|comments| comments.get(key(block).as_str()).cloned())
                .unwrap_or(Json::Null);
            Ok(Outcome::new(format!("the block `{block}`")).with_data(json!({
                "block": block,
                "document": document,
                "comment": comment,
            })))

        }
    }
    /// **Take away** the comment on a block.
    ///
    /// A block with no comment has no comment to take away, and that is
    /// refused.
    prose.comment.delete {
        area: "prose",
        layer: "prose",
        icon: "trash",
        mutates,
        undoable,
        params: {
            /// The block.

            block: NodeRef,
        },
        run: {

            let id = "prose.comment.delete".to_string();
            let block = self.block.0;
            let document = the_block(session, &id, block)?;
            let Some(node) = session.document_mut()?.node_mut(document) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no document `{document}`"),
                });
            };
            let mut comments = comments_of(node);
            if !comments.is_object() || comments.get(key(block).as_str()).is_none() {
                return Err(EngineError::Failed { id, message: format!("the block `{block}` has no comment") });
            }
            let mut taken = false;
            if let Some(map) = comments.as_object_mut() {
                taken = map.remove(&key(block)).is_some();
            }
            if taken {
                node.data["comments"] = comments;
            }
            Ok(Outcome::changed(format!("the block `{block}` has no comment now"))
                .with_data(json!({ "block": block, "document": document })))

        }
    }
}

/// **Take away** the comment of a block that is being taken away.
///
/// The comment goes with the block: a comment on a block that is not
/// there is a comment on nothing.
pub fn drop_comment(session: &mut Session, document: u64, block: u64) {
    let Ok(document_node) = session.document_mut() else { return };
    let Some(node) = document_node.node_mut(document) else { return };
    let mut comments = comments_of(node);
    let mut taken = false;
    if let Some(map) = comments.as_object_mut() {
        taken = map.remove(&key(block)).is_some();
    }
    if taken {
        node.data["comments"] = comments;
    }
}
