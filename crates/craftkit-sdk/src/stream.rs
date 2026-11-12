//! The **stream** of a node: bytes that belong to the node and are not
//! JSON.
//!
//! # The shape this is for
//!
//! A node has a **dictionary** — its payload — and it may also have a
//! **stream**. That is the shape of a PDF object, and of a great many
//! formats that grew out of PostScript: a dictionary says what the thing
//! is, and a stream says what it contains.
//!
//! # Why the bytes are not in the document
//!
//! JSON cannot hold arbitrary bytes. The answer that was there before was
//! base64 in the payload, and it works, and it costs a **third more
//! space**: a document that carries fifty megabytes of streams carries
//! sixty-seven megabytes of text, and the text is what every reader has
//! to parse.
//!
//! So the bytes travel **beside** the document, in the bundle (see
//! [`crate::bundle`]), and the node says which file is its stream. The
//! document stays small, the bytes come back byte for byte, and a program
//! can say what is missing before it starts editing.
//!
//! # The commands
//!
//! - `node.stream.set` — this node's stream is this file;
//! - `node.stream.clear` — this node has no stream;
//! - `node.stream` — what the node's stream is, and what the document is
//!   missing.

use craftkit_derive::Command;

use crate::{EngineError, NodeRef, Outcome, Session, Text};

/// Say that a node's stream is a file.
#[derive(Command)]
#[command(id = "node.stream.set", area = "node", icon = "link", menu("Node"), mutates, undoable)]
pub struct SetStream {
    /// The node.
    pub node: NodeRef,
    /// Where the bytes are, relative to the document.
    pub path: Text,
}

impl SetStream {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let path = self.path.0.clone();
        if path.trim().is_empty() {
            return Err(EngineError::Failed {
                id: "node.stream.set".to_string(),
                message: "a stream has a path".to_string(),
            });
        }
        let id = self.node.0;
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "node.stream.set".to_string(),
                message: format!("there is no node {id} in the document"),
            });
        };
        node.stream = Some(path.clone());
        Ok(Outcome::changed(format!("the stream of node {id} is `{path}`"))
            .with_data(crate::serde_json::json!({ "node": id, "path": path })))
    }
}

/// A node has no stream.
#[derive(Command)]
#[command(id = "node.stream.clear", area = "node", icon = "minus", menu("Node"), mutates, undoable)]
pub struct ClearStream {
    /// The node.
    pub node: NodeRef,
}

impl ClearStream {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.node.0;
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "node.stream.clear".to_string(),
                message: format!("there is no node {id} in the document"),
            });
        };
        let Some(path) = node.stream.take() else {
            return Err(EngineError::Failed {
                id: "node.stream.clear".to_string(),
                message: format!("node {id} has no stream"),
            });
        };
        Ok(Outcome::changed(format!("node {id} has no stream any more"))
            .with_data(crate::serde_json::json!({ "node": id, "path": path })))
    }
}

/// What a node's stream is, and what the document is missing.
///
/// A query: it does not change the document.
#[derive(Command)]
#[command(id = "node.stream", area = "node", icon = "list", menu("Node"))]
pub struct ReadStream {
    /// The node.
    pub node: NodeRef,
}

impl ReadStream {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.node.0;
        let Some(node) = session.document().node(id) else {
            return Err(EngineError::Failed {
                id: "node.stream".to_string(),
                message: format!("there is no node {id} in the document"),
            });
        };
        let stream = node.stream.clone();
        let message = match &stream {
            Some(path) => format!("the stream of node {id} is `{path}`"),
            None => format!("node {id} has no stream"),
        };
        Ok(Outcome::new(message).with_data(crate::serde_json::json!({ "node": id, "stream": stream })))
    }
}
