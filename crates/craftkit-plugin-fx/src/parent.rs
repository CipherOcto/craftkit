//! The **parent** of a layer.
//!
//! A layer may be under another layer: the parent's transform carries
//! the child with it. The parent is a **fact of the layer**, and the
//! chain of parents is a fact too.
//!
//! The tree's finding is that the chain may not be a **circle**: a
//! layer under a layer under itself would carry itself forever, and
//! the tree's command refuses it before anything is changed. The
//! refusal is a **refusal**: the tree's own check refuses a parent
//! chain that already holds a cycle, and a parent that would close
//! one, and it does it before the edit — so a refused parenting
//! changes nothing.
//!
//! # What it does not do
//!
//! - **Parenting does not keep the layer where it is here.** The tree's
//!   parenting re-expresses the child's transform in the new parent's
//!   space, so the picture does not move when the parent changes; that
//!   re-expression is not here, and the parent here is the fact and not
//!   the transform.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome};

/// The **parent** of a layer, as the document holds it.
pub fn parent_of(node: &Node) -> Option<u64> {
    node.data.get("parent").and_then(|parent| parent.as_u64())
}

/// The **chain** of parents above a layer, nearest first.
///
/// The chain is **derived**: it changes when a layer is reparented, and
/// nobody writes it down. A chain that holds a cycle does not end, and
/// the walk stops at the layer it has already seen.
pub fn ancestors_of(document: &Document, layer: u64) -> Vec<u64> {
    let mut chain = Vec::new();
    let mut current = layer;
    while let Some(node) = document.node(current) {
        let Some(parent) = parent_of(node) else { break };
        if chain.contains(&parent) {
            break;
        }
        chain.push(parent);
        current = parent;
    }
    chain
}

/// Whether parenting a layer under a parent would make a **circle**.
///
/// A circle is made when the layer is already **above** the parent:
/// the chain above the parent reaches the layer, and putting the
/// layer under the parent would close it.
pub fn would_cycle(document: &Document, layer: u64, parent: u64) -> bool {
    if layer == parent {
        return true;
    }
    ancestors_of(document, parent).contains(&layer)
}

commands! {
    /// Put a layer **under** another layer.
    ///
    /// The parent must be a layer of the **same composition**: the tree's
    /// command finds the parent in the composition and refuses a layer
    /// that is not there.
    ///
    /// A parent that would make a **circle** is refused, and nothing is
    /// changed.
    fx.layer.parent {
        area: "fx",
        layer: "fx",
        icon: "link",
        mutates,
        undoable,
        params: {
            /// The layer to put under the parent.
            layer: NodeRef,
            /// The parent.
            parent: NodeRef,
        },
        run: {
            let id = "fx.layer.parent".to_string();
            let layer = self.layer.0;
            let parent = self.parent.0;
            let Some(node) = session.document().node(layer) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no layer `{layer}`"),
                });
            };
            if node.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
                });
            }
            let Some(master) = session.document().node(parent) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no layer `{parent}`"),
                });
            };
            if master.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{parent}` is a `{}` and not a layer", master.kind),
                });
            }
            if would_cycle(session.document(), layer, parent) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "parenting the layer `{layer}` under the layer `{parent}` would make a circle"
                    ),
                });
            }
            let Some(node) = session.document_mut()?.node_mut(layer) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no layer `{layer}`"),
                });
            };
            node.data["parent"] = serde_json::Value::from(parent);
            Ok(Outcome::changed(format!("the layer `{layer}` is under the layer `{parent}`"))
                .with_data(serde_json::json!({ "layer": layer, "parent": parent })))

        }
    }

    /// Take a layer **out** of its parent.
    ///
    /// A layer with no parent is a layer of its own, and that is the
    /// tree's own: nothing carries it but the composition.
    fx.layer.parent.delete {
        area: "fx",
        layer: "fx",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The layer to take out.
            layer: NodeRef,
        },
        run: {
            let id = "fx.layer.parent.delete".to_string();
            let layer = self.layer.0;
            let Some(node) = session.document().node(layer) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no layer `{layer}`"),
                });
            };
            if node.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
                });
            };
            let had = parent_of(node);
            let Some(node) = session.document_mut()?.node_mut(layer) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no layer `{layer}`"),
                });
            };
            node.data["parent"] = serde_json::Value::Null;
            Ok(Outcome::changed(format!("the layer `{layer}` is under no layer"))
                .with_data(serde_json::json!({ "layer": layer, "parent": had })))

        }
    }
}
