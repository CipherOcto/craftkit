//! The relations between the nodes, and the commands that make them.
//!
//! # What a relation is
//!
//! The document is a **tree**: a node holds other nodes, and that is
//! containment. A tree cannot say everything a real document has to say.
//! A clip in a sound document plays a source and is not *in* the source;
//! a track feeds a bus and is not *in* the bus; a track is under a
//! controller and is not *in* it either.
//!
//! Those are **relations**: a typed edge from one node to another. The
//! kind is a **string** — `source`, `route`, `vca`, `loop`, `asset`,
//! `symbol` — because the kinds of relation are as open as the kinds of
//! node, and the core does not match on them.
//!
//! # Why the commands are here
//!
//! The relations belong to the **document**, and the commands that change
//! them belong with it. So they are in the SDK rather than in an
//! application: every front end gets them, and they are the same commands
//! everywhere. A plugin that has a kind of relation it cares about — the
//! sound battery has `source` and `route` — reads the document's
//! relations and does not need a command of its own.
//!
//! # The rules
//!
//! - a relation names two nodes that are **in the document**: a relation
//!   to a node that is not there is a relation to nothing;
//! - the same edge with the same kind is the same relation, and it is not
//!   made twice;
//! - removing a relation does not remove the nodes it joined.
//!
//! # An edge can say something
//!
//! The kind is the **name** of the edge, and two edges of the same kind
//! can say different things: a PDF's reference says which key it is
//! under, a symbol instance says the transform it was placed with. So a
//! relation carries a **payload** as well, and the core does not read it.

use craftkit_derive::Command;

use crate::document::Relation;
use crate::{EngineError, NodeRef, Outcome, Session, Text};

/// Link two nodes.
#[derive(Command)]
#[command(id = "link.add", area = "link", icon = "link", mutates, undoable)]
pub struct LinkAdd {
    /// The node the link goes from.
    pub from: NodeRef,
    /// The node the link goes to.
    pub to: NodeRef,
    /// What the link is: `source`, `route`, `vca`, `loop`, …
    pub kind: Text,
    /// What the link says, if it says anything.
    pub payload: Option<crate::serde_json::Value>,
}

impl LinkAdd {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let from = self.from.0;
        let to = self.to.0;
        for id in [from, to] {
            if session.document().node(id).is_none() {
                return Err(EngineError::Failed {
                    id: "link.add".to_string(),
                    message: format!("there is no node `{id}`"),
                });
            }
        }
        if from == to {
            return Err(EngineError::Failed {
                id: "link.add".to_string(),
                message: format!("the node `{from}` is not linked to itself"),
            });
        }
        let kind = self.kind.0.clone();
        let payload = match &self.payload {
            Some(payload) if payload.is_null() => crate::serde_json::Value::Null,
            Some(payload) => payload.clone(),
            None => crate::serde_json::Value::Null,
        };
        let relation = Relation::with_payload(from, to, kind.clone(), payload);
        if !session.document_mut().add_relation(relation) {
            return Err(EngineError::Failed {
                id: "link.add".to_string(),
                message: format!("the node `{from}` is already linked to `{to}` as a `{kind}`"),
            });
        }
        Ok(Outcome::changed(format!("linked `{from}` to `{to}` as a `{kind}`"))
            .with_data(crate::serde_json::json!({ "from": from, "to": to, "kind": kind })))
    }
}

/// Unlink two nodes.
///
/// The kind may be left out, and then every link between the two nodes
/// goes; and one end may be left out, and then every link from that node
/// goes. What is left out is what the caller does not care about.
#[derive(Command)]
#[command(id = "link.remove", area = "link", icon = "link", mutates, undoable)]
pub struct LinkRemove {
    /// The node the link goes from.
    pub from: NodeRef,
    /// The node the link goes to, if the caller cares which one.
    pub to: Option<NodeRef>,
    /// What the link is, if the caller cares which kind.
    pub kind: Option<Text>,
}

impl LinkRemove {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let from = self.from.0;
        let to = self.to.map(|to| to.0);
        let kind = self.kind.as_ref().map(|kind| kind.0.clone());
        let removed = session.document_mut().remove_relations(|relation| {
            relation.from == from
                && to.is_none_or(|to| relation.to == to)
                && kind.as_ref().is_none_or(|kind| &relation.kind == kind)
        });
        Ok(Outcome::changed(format!("removed {removed} link(s)"))
            .with_data(crate::serde_json::json!({ "from": from, "removed": removed })))
    }
}

/// The links in the document.
///
/// A query: it does not change the document, and it is how an agent asks
/// what is attached to what.
#[derive(Command)]
#[command(id = "link.list", area = "link", icon = "link")]
pub struct LinkList {
    /// Only the links from this node.
    pub from: Option<NodeRef>,
    /// Only the links to this node.
    pub to: Option<NodeRef>,
    /// Only the links of this kind.
    pub kind: Option<Text>,
}

impl LinkList {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let from = self.from.map(|from| from.0);
        let to = self.to.map(|to| to.0);
        let kind = self.kind.as_ref().map(|kind| kind.0.clone());
        let links: Vec<crate::serde_json::Value> = session
            .document()
            .relations
            .iter()
            .filter(|relation| from.is_none_or(|from| relation.from == from))
            .filter(|relation| to.is_none_or(|to| relation.to == to))
            .filter(|relation| kind.as_ref().is_none_or(|kind| &relation.kind == kind))
            .map(|relation| {
                crate::serde_json::json!({
                    "from": relation.from,
                    "to": relation.to,
                    "kind": relation.kind,
                    "payload": relation.payload,
                })
            })
            .collect();
        Ok(Outcome::new(format!("{} link(s)", links.len()))
            .with_data(crate::serde_json::json!({ "links": links })))
    }
}
