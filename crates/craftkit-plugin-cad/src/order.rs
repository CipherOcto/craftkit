//! The **order** the entities of a block are drawn in.
//!
//! The entities of a block are its children, and the children are
//! drawn in the order they stand in: the first is drawn first and so
//! it is **at the back**, and the last is drawn last and so it is
//! **at the front**.
//!
//! The tree's rules, and they are the finding:
//!
//! - the tree's `front` puts the entities **last** — the front is the
//!   top of the drawing and the top is drawn last;
//! - the tree's `back` puts them **first**;
//! - an entity that is already at the front and is told to go to the
//!   front **stays** there.
//!
//! Here a command moves **one** entity, because a command that takes
//! a selection is a command that does not say what it does — the
//! tree's own rule is that a command does one thing (§ the registry).

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

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

/// The **block** an entity is in.
fn the_block_of(session: &Session, entity: u64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| node.kind == "cad.block" && node.children.iter().any(|child| child.id == entity))
        .map(|node| node.id)
}

/// The **entities** of a block, in the order they are drawn.
fn entities_of(session: &Session, block: u64) -> Vec<u64> {
    let Some(node) = session.document().node(block) else { return Vec::new() };
    node.children.iter().filter(|child| child.kind.starts_with("cad.entity.")).map(|child| child.id).collect()
}

/// **Move** an entity in the order of its block.
fn move_entity(session: &mut Session, id: &str, entity: u64, place: &str) -> Result<Outcome, EngineError> {
    the_entity(session, id, entity)?;
    let Some(block) = the_block_of(session, entity) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the entity `{entity}` is not in a block"),
        });
    };
    let Some(node) = session.document_mut()?.node_mut(block) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("there is no block `{block}`"),
        });
    };
    let Some(index) = node.children.iter().position(|child| child.id == entity) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the entity `{entity}` is not in the block `{block}`"),
        });
    };
    let last = node.children.len().saturating_sub(1);
    let target = match place {
        "front" => last,
        "back" => 0,
        "forward" => (index + 1).min(last),
        _ => index.saturating_sub(1),
    };
    if target != index {
        let child = node.children.remove(index);
        node.children.insert(target, child);
    }
    Ok(Outcome::changed(format!("the entity `{entity}` is `{place}`"))
        .with_data(json!({ "entity": entity, "block": block, "to": place })))
}

commands! {
    /// **Read** the entities of a block, in the order they are drawn.
    cad.entity.list {
        area: "cad",
        layer: "cad",
        icon: "list",
        params: {
            /// The block.

            block: NodeRef,
        },
        run: {

            let id = "cad.entity.list".to_string();
            let block = self.block.0;
            let Some(node) = session.document().node(block) else {
                return Err(EngineError::Failed { id, message: format!("there is no block `{block}`") });
            };
            if node.kind != "cad.block" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{block}` is a `{}` and not a block", node.kind),
                });
            }
            let entities = entities_of(session, block);
            Ok(Outcome::new(format!("the block `{block}` has {} entity(s)", entities.len()))
                .with_data(json!({ "block": block, "entities": entities })))

        }
    }
    /// **Bring** an entity to the **front**.
    cad.entity.front {
        area: "cad",
        layer: "cad",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The entity.

            entity: NodeRef,
        },
        run: {

            move_entity(session, "cad.entity.front", self.entity.0, "front")

        }
    }
    /// **Send** an entity to the **back**.
    cad.entity.back {
        area: "cad",
        layer: "cad",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The entity.

            entity: NodeRef,
        },
        run: {

            move_entity(session, "cad.entity.back", self.entity.0, "back")

        }
    }
    /// **Move** an entity **one step** towards the front.
    cad.entity.forward {
        area: "cad",
        layer: "cad",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The entity.

            entity: NodeRef,
        },
        run: {

            move_entity(session, "cad.entity.forward", self.entity.0, "forward")

        }
    }
    /// **Move** an entity **one step** towards the back.
    cad.entity.backward {
        area: "cad",
        layer: "cad",
        icon: "arrows",
        mutates,
        undoable,
        params: {
            /// The entity.

            entity: NodeRef,
        },
        run: {

            move_entity(session, "cad.entity.backward", self.entity.0, "backward")

        }
    }
}
