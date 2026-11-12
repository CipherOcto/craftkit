//! The selection: what is picked out, and the commands that pick it out.
//!
//! # What the selection is
//!
//! The **session** holds it: the things that are picked out. It is the
//! session's own state, the way the document is, and it is not the
//! document's business — a node does not know whether it is selected.
//!
//! A thing is a node, and it may be picked out **whole** or **in part**.
//!
//! # The part
//!
//! A part is a range in the node's **own units**: samples for a clip in a
//! sound document, and whatever a shape's axis is for a shape. The core
//! does not know what the units are — it carries them, and the kind
//! reads them. That is the same bargain the payload makes, one level
//! down, and it is what a range needs: there is no universal axis, and a
//! kit that pretended otherwise would be wrong in every domain but one.
//!
//! The sound application this port comes from has an **edit range** — a
//! span of time on the timeline — and half of its editing commands are
//! about that range and not about a node at all: a trim, a split, a
//! nudge, a razor. A selection of ids cannot say any of those things.
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

use crate::{EngineError, NodeRef, Number, Outcome, Session};

/// One thing that is selected.
///
/// A node, and the part of it that is picked out.
#[derive(Debug, Clone, PartialEq)]
pub struct Selected {
    /// The node.
    pub node: u64,
    /// Where in it the selection starts, in the node's own units.
    ///
    /// `None` when the whole node is selected.
    pub from: Option<f64>,
    /// Where in it the selection ends, in the node's own units.
    pub to: Option<f64>,
}

impl Selected {
    /// A whole node.
    pub fn node(node: u64) -> Self {
        Selected { node, from: None, to: None }
    }

    /// A part of a node.
    ///
    /// The range is in the node's own units, and it is **half open**:
    /// `from` is in the selection and `to` is not, the way every range
    /// in a sound document is.
    pub fn span(node: u64, from: f64, to: f64) -> Self {
        Selected { node, from: Some(from), to: Some(to) }
    }

    /// Whether the whole node is selected.
    pub fn is_whole(&self) -> bool {
        self.from.is_none() && self.to.is_none()
    }
}

/// Select every node in the document.
#[derive(Command)]
#[command(id = "select.all", area = "select", icon = "select")]
pub struct SelectAll {}

impl SelectAll {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let ids: Vec<u64> = session.document().walk().map(|node| node.id).collect();
        let count = ids.len();
        session.select_ids(&ids);
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
        session.select_ids(&[id]);
        Ok(Outcome::new(format!("selected the node `{id}`"))
            .with_data(crate::serde_json::json!({ "id": id })))
    }
}

/// Select a **part** of a node.
///
/// The range is in the node's own units, and it is half open: `from` is
/// in the selection and `to` is not.
#[derive(Command)]
#[command(id = "select.span", area = "select", icon = "select")]
pub struct SelectSpan {
    /// The node.
    pub node: NodeRef,
    /// Where the part starts.
    pub from: Number,
    /// Where the part ends.
    pub to: Number,
}

impl SelectSpan {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = self.node.0;
        if session.document().node(id).is_none() {
            return Err(EngineError::Failed {
                id: "select.span".to_string(),
                message: format!("there is no node `{id}`"),
            });
        }
        let from = self.from.0;
        let to = self.to.0;
        if !from.is_finite() || !to.is_finite() {
            return Err(EngineError::Failed {
                id: "select.span".to_string(),
                message: "a range is made of numbers".to_string(),
            });
        }
        if to < from {
            return Err(EngineError::Failed {
                id: "select.span".to_string(),
                message: format!("the range ends at {to} and starts at {from}"),
            });
        }
        session.select(&[crate::Selected::span(id, from, to)]);
        Ok(Outcome::new(format!("selected part of the node `{id}`"))
            .with_data(crate::serde_json::json!({ "id": id, "from": from, "to": to })))
    }
}
