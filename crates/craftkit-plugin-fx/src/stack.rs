//! The **stack of layers** of a composition.
//!
//! A composition is a stack: the layers stand in an order, and the
//! **front** of the stack is drawn over the rest of it. The tree's
//! order is the front first — the layer that was added last is at the
//! front — and here the children of the composition are in the order
//! they were added, and so the **front** is the last of them.
//!
//! The **arrange** commands move a layer in the stack:
//!
//! - `forward` — one step towards the front, and a layer that is
//!   already at the front stays there;
//! - `backward` — one step towards the back, and a layer that is
//!   already at the back stays there;
//! - `front` — to the front of the stack;
//! - `back` — to the back of the stack.
//!
//! # What the port found
//!
//! - **The tree arranges a selection of layers at once** and its
//!   rules for a selection that is not contiguous are careful: each
//!   layer moves one step and the layers keep their order among
//!   themselves. Here a command arranges **one** layer, because a
//!   command that takes a selection is a command that does not say
//!   what it does — and the tree's own rule is that a command does
//!   one thing (§ the registry).
//! - **The tree's `front` and `back` keep the order of the layers
//!   that are moved**, and here one layer is moved and there is no
//!   order to keep.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// Where a layer may be moved to.
const PLACES: [&str; 4] = ["forward", "backward", "front", "back"];

/// The **composition** a node is, if it is one.
fn the_comp<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no composition `{node}`"),
        });
    };
    if found.kind != "fx.comp" {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not a composition", found.kind),
        });
    }
    Ok(found)
}

/// The **layer** of a composition, if it is one.
fn the_layer(session: &Session, command: &str, comp: u64, layer: u64) -> Result<(), EngineError> {
    let node = the_comp(session, command, comp)?;
    if !node.children.iter().any(|child| child.id == layer && child.kind == "fx.layer") {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no layer `{layer}` in the composition `{comp}`"),
        });
    }
    Ok(())
}

/// The **layers** of a composition, the front first.
fn stack_of(session: &Session, comp: u64) -> Vec<u64> {
    let Some(node) = session.document().node(comp) else {
        return Vec::new();
    };
    node.children.iter().filter(|child| child.kind == "fx.layer").map(|child| child.id).rev().collect()
}

commands! {
    /// **Read** the stack of layers of a composition.
    fx.layer.list {
        area: "fx",
        layer: "fx",
        icon: "list",
        params: {
            /// The composition.
            comp: NodeRef,
        },
        run: {
            let id = "fx.layer.list".to_string();
            let comp = self.comp.0;
            the_comp(session, &id, comp)?;
            let layers = stack_of(session, comp);
            Ok(Outcome::new(format!("the composition `{comp}` has {} layer(s)", layers.len()))
                .with_data(json!({ "comp": comp, "layers": layers })))

        }
    }

    /// **Move** a layer in the stack.
    fx.layer.arrange {
        area: "fx",
        layer: "fx",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The composition.
            comp: NodeRef,
            /// The layer.
            layer: NodeRef,
            /// Where to: `forward`, `backward`, `front` or `back`.
            to: Text,
        },
        run: {
            let id = "fx.layer.arrange".to_string();
            let comp = self.comp.0;
            let layer = self.layer.0;
            the_layer(session, &id, comp, layer)?;
            let wanted = self.to.0.trim().to_string();
            let place =
                PLACES.iter().find(|place| place.eq_ignore_ascii_case(&wanted)).copied().ok_or_else(|| {
                    EngineError::Failed {
                        id: id.clone(),
                        message: format!("there is no place `{wanted}`: the places are {}", PLACES.join(", ")),
                    }
                })?;
            let Some(node) = session.document_mut()?.node_mut(comp) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no composition `{comp}`"),
                });
            };
            let Some(index) = node.children.iter().position(|child| child.id == layer) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("there is no layer `{layer}` in the composition `{comp}`"),
                });
            };
            let last = node.children.len().saturating_sub(1);
            // The front of the stack is the last of the children.
            let target = match place {
                "forward" => (index + 1).min(last),
                "backward" => index.saturating_sub(1),
                "front" => last,
                _ => 0,
            };
            if target != index {
                let child = node.children.remove(index);
                node.children.insert(target, child);
            }
            Ok(Outcome::changed(format!("the layer `{layer}` is `{place}`"))
                .with_data(json!({ "comp": comp, "layer": layer, "to": place })))

        }
    }

    /// **Rename** a layer.
    fx.layer.rename {
        area: "fx",
        layer: "fx",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The layer.
            layer: NodeRef,
            /// The name.
            name: Text,
        },
        run: {
            let id = "fx.layer.rename".to_string();
            let layer = self.layer.0;
            let Some(node) = session.document().node(layer) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no layer `{layer}`"),
                });
            };
            if node.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
                });
            }
            let name = self.name.0.clone();
            if name.trim().is_empty() {
                return Err(EngineError::Failed {
                    id,
                    message: "a layer has a name, and an empty one is not a name".to_string(),
                });
            }
            let Some(node) = session.document_mut()?.node_mut(layer) else {
                return Err(EngineError::Failed {
                    id: "fx.layer.rename".to_string(),
                    message: format!("there is no layer `{layer}`"),
                });
            };
            node.name = name.clone();
            Ok(Outcome::changed(format!("the layer `{layer}` is `{name}`"))
                .with_data(json!({ "layer": layer, "name": name })))

        }
    }
}
