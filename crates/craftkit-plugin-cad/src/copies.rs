//! The **copy** of an entity.
//!
//! A copy is a second entity of the **same kind**, with the same
//! geometry and the same layer, put somewhere else. It is not a
//! second name for the first one: it is a thing of its own that may
//! be moved, turned and taken away without the first one feeling it.
//!
//! # What the port found
//!
//! - **The tree's copy is a command with a base point and a target**,
//!   and it copies a whole **selection**. Here one entity is copied
//!   at a time, and the **offset** is the vector it is copied by: a
//!   copy that is in the same place as the thing it copies is a copy
//!   that cannot be told from it, and the caller says where the copy
//!   goes.
//! - **The tree's copy carries the layer with it.** Here the same: the
//!   copy is on the same **layer** as the entity it copies, and the
//!   layer is an **edge** and so the edge is made again for the copy.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Relation, Session, Text};

/// The **entity** a node is, if it is one.
fn the_entity<'a>(
    session: &'a Session,
    command: &str,
    node: u64,
) -> Result<&'a craftkit_sdk::Node, EngineError> {
    let Some(found) = session.document().node(node) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no entity `{node}`"),
        });
    };
    if !found.kind.starts_with("cad.entity.") {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{node}` is a `{}` and not an entity", found.kind),
        });
    }
    Ok(found)
}

/// A **point**, read from the caller's JSON.
fn point_of(value: &Json, id: &str, what: &str) -> Result<(f64, f64), EngineError> {
    let x = value.get("x").and_then(|x| x.as_f64()).unwrap_or(f64::NAN);
    let y = value.get("y").and_then(|y| y.as_f64()).unwrap_or(f64::NAN);
    if !x.is_finite() || !y.is_finite() {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("{what} is a point in the world's units"),
        });
    }
    Ok((x, y))
}

/// The **block** an entity is in.
///
/// A node does not say where it is: the tree says so, and the tree is
/// the parent. So the block is found by walking the document for the
/// node that holds the entity.
fn block_of(document: &Document, entity: u64) -> Option<u64> {
    document.roots().iter().find_map(|root| parent_in(root, entity))
}

/// The node that **holds** a node, if it is held by one.
fn parent_in(node: &Node, entity: u64) -> Option<u64> {
    for child in node.children.iter() {
        if child.id == entity {
            return Some(node.id);
        }
        if let Some(found) = parent_in(child, entity) {
            return Some(found);
        }
    }
    None
}

commands! {
    /// **Copy** an entity.
    cad.entity.copy {
        area: "cad",
        layer: "cad",
        icon: "copy",
        mutates,
        undoable,
        params: {
            /// The entity to copy.
            entity: NodeRef,
            /// What to call the copy.
            name: Text,
            /// The vector the copy is made **by**.
            by: Json,
            /// The block the copy goes in. The block of the entity itself when
            /// it is not said.
            block: Option<NodeRef>,
        },
        run: {
            let id = "cad.entity.copy".to_string();
            let entity = self.entity.0;
            let node = the_entity(session, &id, entity)?;
            crate::editable::may_edit(session.document(), entity, &id)?;
            let kind = node.kind.clone();
            let geometry = node.data.get("geometry").cloned().ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the entity `{entity}` has no geometry"),
            })?;
            let (dx, dy) = point_of(&self.by, &id, "the vector")?;
            let Some(new) = crate::modify::moved(&kind, &geometry, dx, dy) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("an entity of the kind `{kind}` may not be copied"),
                });
            };
            let block = match self.block {
                Some(block) => block.0,
                None => block_of(session.document(), entity).ok_or_else(|| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the entity `{entity}` is in no block"),
                })?,
            };
            // The **layer** of the entity, and the **block** an insert
            // refers to: the copy carries them, because a copy of a thing
            // on a layer that is not on that layer is not a copy of it.
            let layer = session
                .document()
                .relations_from(entity, Some(craftkit_sdk::layer::ON_LAYER))
                .first()
                .map(|relation| relation.to);
            let inserts = session
                .document()
                .relations_from(entity, Some("cad.inserts"))
                .first()
                .map(|relation| (relation.to, relation.payload.clone()));
            let copy = crate::add_entity(session, block, &self.name.0, &kind, new, "cad.entity.copy")?;
            if let Some(layer) = layer {
                let name = session.document().node(layer).map(|node| node.name.clone()).unwrap_or_default();
                session.document_mut()?.add_relation(Relation::with_payload(
                    copy,
                    layer,
                    craftkit_sdk::layer::ON_LAYER,
                    json!({ "layer": name }),
                ));
            }
            if let Some((block_node, payload)) = inserts {
                session.document_mut()?.add_relation(Relation::with_payload(
                    copy,
                    block_node,
                    "cad.inserts",
                    payload,
                ));
            }
            Ok(Outcome::changed(format!("copied the entity `{entity}`"))
                .with_data(json!({ "entity": entity, "copy": copy, "name": self.name.0 })))

        }
    }
}
