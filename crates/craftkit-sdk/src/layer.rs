//! The **layer**: a group of things that share a name and a look.
//!
//! # What a layer is
//!
//! A layer is **a node**, and the things on it are said to be on it by
//! a **relation**: an edge from the thing to the layer. The edge is the
//! membership, and the kind of the edge is the name of the membership
//! — `core.on_layer`.
//!
//! Before this, the CAD battery had layers of its own and a relation
//! of its own, and the rule that said "an entity is drawn in its
//! layer's colour" lived in that battery. Every other battery that
//! wanted a layer would have wanted the same rule and written it again,
//! and two rules that say the same thing drift apart.
//!
//! # What the kit says about a layer
//!
//! - a node with **no appearance of its own** takes the **layer's**;
//! - a node on a **hidden** layer is **hidden**;
//! - the layer is **followed once**: a node on a layer that is itself on
//!   a layer takes the first layer's appearance.
//!
//! The layer's **kind** is not the kit's business: a CAD layer is a CAD
//! table row, an artist's layer is an artist's node, and the kit's own
//! `core.layer` is a plain node with a name. The kit cares about the
//! **edge**, and every kind of layer works with it.

use craftkit_derive::{Command, NodeKind};

use crate::document::{Canvas, Document, Node, Point};
use crate::serde_json::{Value as Json, json};
use crate::{EngineError, NodeRef, Outcome, Session, Text};

/// The **edge** that says a node is on a layer.
///
/// The kind of an edge is the name of what the edge means (§ the
/// relations), and this one is the kit's own: every battery that has
/// layers uses it, so that the rule that follows from it is one rule.
pub const ON_LAYER: &str = "core.on_layer";

/// A **layer**: a node that things are put on.
///
/// It draws nothing and is hit by nothing: a layer is not a thing, it
/// is where things are.
#[derive(NodeKind)]
#[node_kind(kind = "core.layer", label = "Layer", layer = "core", evaluates)]
pub struct LayerKind;

impl LayerKind {
    /// Not drawn.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: Point) -> bool {
        false
    }

    /// What a layer **means**: its name.
    fn evaluate(node: &Node, _document: &Document) -> Option<Json> {
        Some(json!({ "name": node.name }))
    }
}

/// **Add** a layer.
#[derive(Command)]
#[command(id = "core.layer.add", area = "core", icon = "plus", menu("Layer"), mutates, undoable)]
pub struct AddLayer {
    /// Its name.
    pub name: Text,
}

impl AddLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed {
                id: "core.layer.add".to_string(),
                message: "a layer has a name".to_string(),
            });
        }
        let id = session.document_mut()?.add_node(name.clone(), "core.layer", None);
        Ok(Outcome::changed(format!("added the layer `{name}`")).with_data(json!({ "id": id, "name": name })))
    }
}

/// **Put** a node on a layer.
///
/// The membership is an **edge**, and it says what it is. A node that
/// is already on the layer is refused: the same node on the same layer
/// twice is two memberships, and the second one says nothing.
#[derive(Command)]
#[command(id = "core.layer.on", area = "core", icon = "plus", menu("Layer"), mutates, undoable)]
pub struct PutOnLayer {
    /// The node.
    pub node: NodeRef,
    /// The layer.
    pub layer: NodeRef,
}

impl PutOnLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        let layer = self.layer.0;
        if session.document().node(node).is_none() {
            return Err(EngineError::Failed {
                id: "core.layer.on".to_string(),
                message: format!("there is no node `{node}`"),
            });
        }
        if session.document().node(layer).is_none() {
            return Err(EngineError::Failed {
                id: "core.layer.on".to_string(),
                message: format!("there is no node `{layer}`"),
            });
        }
        if node == layer {
            return Err(EngineError::Failed {
                id: "core.layer.on".to_string(),
                message: "a node cannot be on itself".to_string(),
            });
        }
        let already = session
            .document()
            .relations_from(node, Some(ON_LAYER))
            .iter()
            .any(|relation| relation.to == layer);
        if already {
            return Err(EngineError::Failed {
                id: "core.layer.on".to_string(),
                message: format!("the node `{node}` is already on the layer `{layer}`"),
            });
        }
        session.document_mut()?.add_relation(crate::document::Relation::with_payload(
            node,
            layer,
            ON_LAYER,
            json!(null),
        ));
        Ok(Outcome::changed(format!("the node `{node}` is on the layer `{layer}`"))
            .with_data(json!({ "node": node, "layer": layer })))
    }
}

/// **Take** a node off its layer.
///
/// A node can be on one layer at a time: the edge says which, and a
/// second edge would be two answers to one question. So taking a node
/// off takes it off every layer.
#[derive(Command)]
#[command(id = "core.layer.off", area = "core", icon = "minus", menu("Layer"), mutates, undoable)]
pub struct TakeOffLayer {
    /// The node.
    pub node: NodeRef,
}

impl TakeOffLayer {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let node = self.node.0;
        if session.document().node(node).is_none() {
            return Err(EngineError::Failed {
                id: "core.layer.off".to_string(),
                message: format!("there is no node `{node}`"),
            });
        }
        let removed = session
            .document_mut()?
            .remove_relations(|relation| relation.from == node && relation.kind == ON_LAYER);
        Ok(Outcome::changed(format!("the node `{node}` is off its layer"))
            .with_data(json!({ "node": node, "removed": removed })))
    }
}
