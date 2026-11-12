//! The **relation**: an edge between two nodes.
//!
//! The document is a **tree**: a node holds other nodes, and that is
//! containment. A tree cannot say everything a real document has to
//! say — a clip plays a source and is not *in* the source, a track
//! feeds a bus and is not *in* it, a shape is on a layer and is not
//! *in* it — and the edge is what says the rest.
//!
//! The edge has a **kind**, and the kind is the name of what the edge
//! means: `cad.on_layer` says that one node is on another, and
//! `core.on_layer` says the same thing for every battery that has
//! layers (§103).
//!
//! The edge may carry a **payload**: what the edge says beyond its
//! kind. The payload is the edge's own, and the core never looks
//! inside it.

use craftkit_derive::Command;

use crate::document::Relation;
use crate::serde_json::{Value as Json, json};
use crate::{EngineError, NodeRef, Outcome, Session};

/// **Add** an edge between two nodes.
///
/// The edge is refused when one of its ends is not there: an edge to
/// nothing is not an edge, and a document that says a clip plays a
/// source that is not in the document says something false.
///
/// The **kind** is the name of what the edge means, and the payload is
/// what it says beyond that.
#[derive(Command)]
#[command(id = "relation.add", area = "relation", icon = "link", menu("Relation"), mutates, undoable)]
pub struct AddRelation {
    /// Where the edge starts.
    pub from: NodeRef,
    /// Where the edge ends.
    pub to: NodeRef,
    /// What the edge means.
    pub kind: crate::Text,
    /// What it says beyond that.
    pub payload: Option<Json>,
}

impl AddRelation {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let from = self.from.0;
        let to = self.to.0;
        if session.document().node(from).is_none() {
            return Err(EngineError::Failed {
                id: "relation.add".to_string(),
                message: format!("there is no node `{from}`"),
            });
        }
        if session.document().node(to).is_none() {
            return Err(EngineError::Failed {
                id: "relation.add".to_string(),
                message: format!("there is no node `{to}`"),
            });
        }
        let kind = self.kind.0.trim().to_string();
        if kind.is_empty() {
            return Err(EngineError::Failed {
                id: "relation.add".to_string(),
                message: "an edge says what it means".to_string(),
            });
        }
        let payload = self.payload.clone().unwrap_or(Json::Null);
        let added = session.document_mut()?.add_relation(Relation::with_payload(from, to, &kind, payload));
        if !added {
            return Err(EngineError::Failed {
                id: "relation.add".to_string(),
                message: format!("there is already an edge from `{from}` to `{to}`"),
            });
        }
        Ok(Outcome::changed(format!("added the edge `{kind}`"))
            .with_data(json!({ "from": from, "to": to, "kind": kind })))
    }
}
