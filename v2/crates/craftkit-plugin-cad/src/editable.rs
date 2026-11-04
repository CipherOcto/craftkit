//! What may be **edited**.
//!
//! A layer may be **locked** and it may be **frozen**, and both of
//! them are about what may be changed:
//!
//! - a layer that is **locked** is drawn, and what is on it may not
//!   be **edited**;
//! - a layer that is **frozen** is not drawn at all, and what is on
//!   it may not be edited either.
//!
//! So the lock of a layer is a lock on **what is on it**, and the
//! question "may this be changed?" is asked of the layer, not of the
//! thing.
//!
//! # What the port found
//!
//! - **The tree's locked objects are skipped, not refused.** Its cut
//!   copies the objects that are not on locked layers and says how
//!   many were left alone; its edits filter the locked ones out of
//!   the set they work on. Here an edit of a locked entity is
//!   **refused**: a command that silently does nothing to half of what
//!   it was given is a command whose answer is a lie, and an entity
//!   that cannot be edited should say so.
//! - **The tree's layer is a name and the lock is a property of the
//!   name.** Here the layer is a **node**, the lock is a fact in its
//!   payload, and an entity is on a layer through an **edge** — so
//!   the question is answered by following the edge.

use craftkit_sdk::{Document, EngineError, Node};

/// Whether a layer is **locked**: what is on it may not be edited.
pub(crate) fn is_locked(layer: &Node) -> bool {
    layer.data.get("locked").and_then(|locked| locked.as_bool()).unwrap_or(false)
}

/// Whether a layer is **frozen**: it is not drawn, and what is on it
/// may not be edited.
pub(crate) fn is_frozen(layer: &Node) -> bool {
    layer.data.get("frozen").and_then(|frozen| frozen.as_bool()).unwrap_or(false)
}

/// The **layer** an entity is on, if it is on one.
///
/// The layer is an **edge**, and the edge says what it is: it points
/// at a layer.
pub(crate) fn layer_of(document: &Document, entity: u64) -> Option<&Node> {
    let relations = document.relations_from(entity, Some(craftkit_sdk::layer::ON_LAYER));
    let relation = relations.first()?;
    let layer = document.node(relation.to)?;
    (layer.kind == "cad.layer").then_some(layer)
}

/// Whether an entity may be **edited**.
///
/// It may not, when the layer it is on is locked or frozen — and the
/// refusal says which layer and why, because a refusal that says only
/// "no" is a refusal that sends the reader looking.
pub(crate) fn may_edit(document: &Document, entity: u64, command: &str) -> Result<(), EngineError> {
    let Some(layer) = layer_of(document, entity) else { return Ok(()) };
    if is_locked(layer) {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the entity `{entity}` is on the locked layer `{}`", layer.name),
        });
    }
    if is_frozen(layer) {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the entity `{entity}` is on the frozen layer `{}`", layer.name),
        });
    }
    Ok(())
}
