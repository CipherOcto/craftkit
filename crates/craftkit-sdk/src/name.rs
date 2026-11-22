//! The **name** of a node.
//!
//! # What a name is
//!
//! A node has a name, and the name is what a person calls it. The
//! document's own kinds are open strings, and a node's name is a piece
//! of text beside them: the name says what a person calls it, and the
//! kind says what it is.
//!
//! # The commands
//!
//! - `node.name.set` — this node is called this.
//!
//! # Why the kit holds it
//!
//! Every battery has names in it — a track, a scene, a layer, a clip —
//! and a document whose names cannot be changed is a document whose
//! names are wrong forever. The name is **the node's own**, so one
//! command renames a track, a scene and a clip alike: a battery that
//! wanted a command of its own for every kind of name would have a
//! second path for one behaviour.

use craftkit_derive::Command;

use crate::{EngineError, NodeRef, Outcome, Session, Text};

/// Say what a node is called.
#[derive(Command)]
#[command(id = "node.name.set", area = "node", icon = "pencil", menu("Node"), mutates, undoable)]
pub struct SetName {
    /// The node.
    pub node: NodeRef,
    /// What it is called.
    pub name: Text,
}

impl SetName {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.node.0;
        let name = self.name.0.trim().to_string();
        // **A node has a name.** An empty name is not a name, and a
        // document of nameless nodes cannot be asked what anything is
        // called.
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "node.name.set".to_string(),
                message: "a node has a name".to_string(),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(id) else {
            return Err(EngineError::Failed {
                id: "node.name.set".to_string(),
                message: format!("there is no node {id} in the document"),
            });
        };
        node.name = name.clone();
        Ok(Outcome::changed(format!("node {id} is called `{name}`"))
            .with_data(crate::serde_json::json!({ "node": id, "name": name })))
    }
}
