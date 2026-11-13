//! The **layers** of a layout document.
//!
//! A layer is a node, and the items on it are said to be on it by an
//! **edge**. A layer may be **deleted**, the layers with nothing on
//! them may be deleted, the items of two layers may be **merged** into
//! one, and a layer may be **moved** in the order of the layers.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - A document needs **at least one layer**: the last layer of a
//!   document may not be deleted.
//! - The **items** of a layer go with it when it is deleted.
//! - The layers with **nothing on them** may be deleted together, and
//!   one layer always stays.
//! - The **name** of a layer is not empty, and no two layers of a
//!   document have the same name.
//! - The layers are **ordered**, and the order is the tree's: the
//!   first is the frontmost.
//!
//! # What it does not do
//!
//! - **The layers are not the kit's layers.** The edge of this
//!   battery is its own, and so the kit's rule — that a hidden layer
//!   hides what is on it — does not reach these layers.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The **edge** that says an item is on a layer.
const ON_LAYER: &str = craftkit_sdk::layer::ON_LAYER;

/// The **layers** of a document, in the order they stand in.
fn layers_of(session: &Session, document: u64) -> Vec<u64> {
    let Some(node) = session.document().node(document) else { return Vec::new() };
    node.children.iter().filter(|child| child.kind == "layout.layer").map(|child| child.id).collect()
}

/// The **items** on a layer: the edges that say so.
fn items_on(session: &Session, layer: u64) -> Vec<u64> {
    let mut items = Vec::new();
    for relation in session.document().relations.iter() {
        if relation.kind == ON_LAYER && relation.to == layer && !items.contains(&relation.from) {
            items.push(relation.from);
        }
    }
    items
}

/// The **layer** a command is about, and whether it is a layer.
fn layer_of(session: &Session, command: &str, layer: u64) -> Result<u64, EngineError> {
    let Some(node) = session.document().node(layer) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no layer `{layer}`"),
        });
    };
    if node.kind != "layout.layer" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
        });
    }
    // The **document** the layer is in.
    Ok(session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == layer))
        .map(|node| node.id)
        .unwrap_or(layer))
}

commands! {
    /// **Rename** a layer.
    ///
    /// The name of a layer is not empty, and no two layers of a document
    /// have the same name.
    layout.layer.rename {
        area: "layout",
        layer: "layout",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
            /// Its new name.
            name: Text,
        },
        run: {
            let command = "layout.layer.rename";
            let layer = self.layer.0;
            let document = layer_of(session, command, layer)?;
            let name = self.name.0.trim().to_string();
            if name.is_empty() {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a layer has a name".to_string(),
                });
            }
            for other in layers_of(session, document) {
                if other == layer {
                    continue;
                }
                let held = session.document().node(other).map(|node| node.name.clone());
                if held.as_deref() == Some(name.as_str()) {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: format!("the layer `{name}` is already there"),
                    });
                }
            }
            let Some(node) = session.document_mut()?.node_mut(layer) else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no layer `{layer}`"),
                });
            };
            node.name = name.clone();
            Ok(Outcome::changed(format!("the layer is `{name}`"))
                .with_data(json!({ "layer": layer, "name": name })))

        }
    }

    /// **Delete** a layer.
    ///
    /// The items on the layer go with it, and a document needs at least one
    /// layer.
    layout.layer.delete {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
        },
        run: {
            let command = "layout.layer.delete";
            let layer = self.layer.0;
            let document = layer_of(session, command, layer)?;
            let layers = layers_of(session, document);
            if layers.len() < 2 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "a document needs at least one layer".to_string(),
                });
            }
            // The **items** on the layer go with it.
            let items = items_on(session, layer);
            let mut gone = Vec::new();
            for item in &items {
                if session.document_mut()?.remove_node(*item) {
                    gone.push(*item);
                }
            }
            // The **layer** is gone.
            session.document_mut()?.remove_node(layer);
            Ok(Outcome::changed(format!("the layer `{layer}` is gone")).with_data(json!({
                "layer": layer,
                "items": gone,
                "layers": layers.len() - 1,
            })))

        }
    }

    /// **Delete** the layers of a document that have **nothing on them**.
    ///
    /// One layer always stays.
    layout.layer.unused.delete {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The document.
            document: NodeRef,
        },
        run: {
            let command = "layout.layer.unused.delete";
            let document = self.document.0;
            if session.document().node(document).is_none() {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no document `{document}`"),
                });
            }
            let layers = layers_of(session, document);
            let mut doomed = Vec::new();
            for layer in &layers {
                if items_on(session, *layer).is_empty() {
                    doomed.push(*layer);
                }
            }
            // One **layer** always stays: the first one.
            if doomed.len() == layers.len() {
                doomed.pop();
            }
            let mut gone = Vec::new();
            for layer in &doomed {
                if session.document_mut()?.remove_node(*layer) {
                    gone.push(*layer);
                }
            }
            Ok(Outcome::changed(format!("{} layer(s) are gone", gone.len())).with_data(json!({
                "document": document,
                "layers": gone,
                "count": gone.len(),
            })))

        }
    }

    /// **Move** a layer in the order of the layers.
    ///
    /// The order is the tree's: the first is the frontmost.
    layout.layer.move {
        area: "layout",
        layer: "layout",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
            /// The place it goes: the first is the frontmost.
            to: craftkit_sdk::Number,
        },
        run: {
            let command = "layout.layer.move";
            let layer = self.layer.0;
            let document = layer_of(session, command, layer)?;
            let layers = layers_of(session, document);
            let to = self.to.0;
            if !to.is_finite() || to.fract() != 0.0 || to < 0.0 || to >= layers.len() as f64 {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the place is a number from 0 to {}, and {to} is not one", layers.len()),
                });
            }
            let to = to as usize;
            // The **children** of the document, with the layer moved: the
            // order of the layers is counted from the front, and the
            // children are counted from the back.
            let Some(node) = session.document().node(document).cloned() else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("there is no document `{document}`"),
                });
            };
            // The **slots** the layers take in the children, and the order
            // of the layers from the front.
            let slots: Vec<usize> = node
                .children
                .iter()
                .enumerate()
                .filter(|(_, child)| child.kind == "layout.layer")
                .map(|(index, _)| index)
                .collect();
            let front: Vec<u64> = layers.iter().rev().copied().collect();
            let Some(index) = front.iter().position(|held| *held == layer) else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the layer `{layer}` is not in the document `{document}`"),
                });
            };
            let mut moved = front.clone();
            moved.remove(index);
            moved.insert(to.min(moved.len()), layer);
            // The **children**, with the layers in their new places.
            let mut children = node.children.clone();
            for (slot, held) in slots.iter().zip(moved.iter().rev()) {
                let Some(node) = session.document().node(*held).cloned() else { continue };
                children[*slot] = std::sync::Arc::new(node);
            }
            if let Some(node) = session.document_mut()?.node_mut(document) {
                node.children = children;
            }
            Ok(Outcome::changed(format!("the layer `{layer}` is at {to}")).with_data(json!({
                "layer": layer,
                "to": to,
                "layers": layers_of(session, document),
            })))

        }
    }

    /// **Merge** layers: the items of them move to one, and the others are
    /// gone.
    ///
    /// The tree's own rule: the items move to the layer named, and the
    /// layers that were merged are deleted.
    layout.layer.merge {
        area: "layout",
        layer: "layout",
        icon: "group",
        mutates,
        undoable,
        params: {
            /// The layers to merge: at least one.
            layers: Json,
            /// The layer the items go to: the first of them when it is not
            /// named.
            into: Option<NodeRef>,
        },
        run: {
            let command = "layout.layer.merge";
            let Some(list) = self.layers.as_array() else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "the layers are a list of ids".to_string(),
                });
            };
            let mut layers: Vec<u64> = Vec::new();
            for value in list {
                let Some(id) = value.as_u64() else {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message: "the layers are a list of ids".to_string(),
                    });
                };
                if !layers.contains(&id) {
                    layers.push(id);
                }
            }
            if layers.is_empty() {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "there are no layers to merge".to_string(),
                });
            }
            layer_of(session, command, layers[0])?;
            for layer in &layers {
                layer_of(session, command, *layer)?;
            }
            let into = match self.into {
                Some(into) => {
                    layer_of(session, command, into.0)?;
                    into.0
                }
                None => layers[0],
            };
            if !layers.contains(&into) {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: "the layer the items go to is not one of the layers that are merged".to_string(),
                });
            }
            // The **items** of the layers that go away move to the layer
            // that stays.
            let mut moved = Vec::new();
            for layer in layers.iter().filter(|layer| **layer != into) {
                for item in items_on(session, *layer) {
                    let already = session.document().relations.iter().any(|relation| {
                        relation.kind == ON_LAYER && relation.from == item && relation.to == into
                    });
                    if already {
                        session.document_mut()?.remove_relations(|relation| {
                            relation.kind == ON_LAYER && relation.from == item && relation.to == *layer
                        });
                        continue;
                    }
                    let name = session.document().node(into).map(|node| node.name.clone()).unwrap_or_default();
                    session.document_mut()?.add_relation(craftkit_sdk::Relation::with_payload(
                        item,
                        into,
                        ON_LAYER,
                        json!({ "layer": name }),
                    ));
                    session.document_mut()?.remove_relations(|relation| {
                        relation.kind == ON_LAYER && relation.from == item && relation.to == *layer
                    });
                    moved.push(item);
                }
            }
            // The **layers** that were merged are gone.
            let mut gone = Vec::new();
            for layer in layers.iter().filter(|layer| **layer != into) {
                if session.document_mut()?.remove_node(*layer) {
                    gone.push(*layer);
                }
            }
            Ok(Outcome::changed(format!("{} layer(s) are merged into one", gone.len())).with_data(json!({
                "into": into,
                "layers": gone,
                "items": moved,
            })))

        }
    }

    /// **Lock** a layer: what is on it may not be edited.
    ///
    /// The kit's own rule: a locked layer locks everything on it.
    layout.layer.lock {
        area: "layout",
        layer: "layout",
        icon: "lock",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
        },
        run: {
            set_flag(session, "layout.layer.lock", self.layer.0, true)

        }
    }

    /// **Unlock** a layer: what is on it may be edited again.
    layout.layer.unlock {
        area: "layout",
        layer: "layout",
        icon: "lock",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
        },
        run: {
            set_flag(session, "layout.layer.unlock", self.layer.0, false)

        }
    }

    /// **Hide** a layer: what is on it is not drawn.
    layout.layer.hide {
        area: "layout",
        layer: "layout",
        icon: "minus",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
        },
        run: {
            set_flag(session, "layout.layer.hide", self.layer.0, true)

        }
    }

    /// **Show** a layer: what is on it is drawn again.
    layout.layer.show {
        area: "layout",
        layer: "layout",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
        },
        run: {
            set_flag(session, "layout.layer.show", self.layer.0, false)

        }
    }
}

/// Set the **locked** or the **hidden** flag of a layer.
///
/// The flags are the kit's own: a locked layer locks what is on it,
/// and a hidden layer hides it.
fn set_flag(session: &mut Session, command: &str, layer: u64, on: bool) -> Result<Outcome, EngineError> {
    layer_of(session, command, layer)?;
    let Some(node) = session.document_mut()?.node_mut(layer) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no layer `{layer}`"),
        });
    };
    let name = node.name.clone();
    if command.ends_with("lock") {
        node.locked = on;
    } else {
        node.hidden = on;
    }
    let what = if command.ends_with("lock") { "locked" } else { "hidden" };
    Ok(Outcome::changed(format!("the layer `{name}` is {what}")).with_data(json!({
        "layer": layer,
        "name": name,
        what: on,
    })))
}
