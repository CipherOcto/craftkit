//! The selection: what is picked out, and the commands that pick it out.
//!
//! # What the selection is
//!
//! The **session** holds it: the ids of the nodes that are picked out. It
//! is the session's own state, the way the document is, and it is not the
//! document's business — a node does not know whether it is selected.
//!
//! # Why the commands are here
//!
//! The selection belongs to the session, and the commands that change it
//! belong with it. So they are in the SDK rather than in an application:
//! every front end — the studio, the bridge, a script — gets them, and
//! they are the same commands everywhere.
//!
//! A command that acts on "the selection" — the battery's path operations,
//! for one — reads [`Session::selection`], and that is the whole contract
//! between a person, an agent and a command.

use craftkit_derive::Command;

use crate::{EngineError, NodeRef, Outcome, Session};

/// Select every node in the document.
#[derive(Command)]
#[command(id = "select.all", area = "select", icon = "select")]
pub struct SelectAll {}

impl SelectAll {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let ids: Vec<u64> = session.document().walk().map(|node| node.id).collect();
        let count = ids.len();
        session.select(&ids);
        Ok(Outcome::new(format!("selected {count} node(s)"))
            .with_data(crate::serde_json::json!({ "selected": count })))
    }
}

/// Select nothing.
#[derive(Command)]
#[command(id = "select.none", area = "select", icon = "select")]
pub struct SelectNone {}

impl SelectNone {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        session.clear_selection();
        Ok(Outcome::new("selected nothing"))
    }
}

/// Select one node.
#[derive(Command)]
#[command(id = "select.node", area = "select", icon = "select")]
pub struct SelectNode {
    /// The node's id.
    pub node: NodeRef,
}

impl SelectNode {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.node.0;
        if session.document().node(id).is_none() {
            return Err(EngineError::Failed {
                id: "select.node".to_string(),
                message: format!("there is no node `{id}`"),
            });
        }
        session.select(&[id]);
        Ok(Outcome::new(format!("selected the node `{id}`"))
            .with_data(crate::serde_json::json!({ "id": id })))
    }
}
