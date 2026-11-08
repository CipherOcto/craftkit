//! The **group** of items.
//!
//! A group holds items, and the items in it are drawn as the group is
//! drawn: the group is the thing that is moved, hidden or locked, and
//! what is inside it goes with it.
//!
//! # The rules
//!
//! These are the tree's own rules.
//!
//! - **At least two items** are grouped. One item is not a group.
//! - The items are on the **same spread** and at the **same level**:
//!   all of them in the spread, or all of them in one group. Items
//!   that are not are refused, and the refusal says why.
//! - The **order** of the items is kept: the group stands where the
//!   item at the back stood, and the items are in it in the order they
//!   were.
//! - The group is on the **layer** of the item at the back, and the
//!   items keep the layers they were on.
//! - A **locked** item may not be grouped: an edit of a locked item is
//!   refused, and the refusal says which item and why.
//! - **Ungrouping** puts the items back where the group stood, in the
//!   order they were in.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session, Text};

use crate::items::{item_of, may_edit};

/// The most items a group may hold.
pub const MAX_ITEMS: usize = 1_000;

/// The name a group has when it is not given one: the tree's.
pub const DEFAULT_NAME: &str = "Group";

/// The **parent** of an item: the node that holds it.
fn parent_of(session: &Session, item: u64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| node.children.iter().any(|child| child.id == item))
        .map(|node| node.id)
}

/// The **spread** an item is in: the spread that holds it, or the
/// spread that holds the group it is in.
fn spread_of(session: &Session, item: u64) -> Option<u64> {
    let parent = parent_of(session, item)?;
    let node = session.document().node(parent)?;
    if node.kind == "layout.spread" {
        return Some(parent);
    }
    parent_of(session, parent)
}

/// The **ids** a caller gives, and the error when they are not ids.
///
/// The ids come from outside, so they are read carefully: there are at
/// most a thousand of them, and they are whole numbers.
fn ids_of(value: &Json, command: &str) -> Result<Vec<u64>, EngineError> {
    let Some(list) = value.as_array() else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: "the items are a list of ids".to_string(),
        });
    };
    if list.len() > MAX_ITEMS {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there are {} items, and the most there may be is {MAX_ITEMS}", list.len()),
        });
    }
    let mut ids = Vec::with_capacity(list.len());
    for value in list {
        let Some(id) = value.as_u64() else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "the items are a list of ids".to_string(),
            });
        };
        if !ids.contains(&id) {
            ids.push(id);
        }
    }
    Ok(ids)
}

/// **Group** items.
///
/// The group stands where the item at the back stood, and the items
/// are in it in the order they were.
#[derive(Command)]
#[command(id = "layout.item.group", area = "layout", layer = "layout", icon = "group", mutates, undoable)]
pub struct GroupItems {
    /// The items to group: at least two.
    pub items: Json,
    /// What to call the group.
    pub name: Option<Text>,
}

impl GroupItems {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "layout.item.group";
        let ids = ids_of(&self.items, command)?;
        if ids.len() < 2 {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "a group holds at least two items".to_string(),
            });
        }
        // The **items**: they are items, and a locked one may not be
        // grouped.
        for id in &ids {
            let node = item_of(session, command, *id)?;
            may_edit(session, &node, command)?;
        }
        // The **parent**: the items are at the same level, and that is
        // the tree's own rule.
        let mut parent: Option<u64> = None;
        for id in &ids {
            let Some(held) = parent_of(session, *id) else {
                return Err(EngineError::Failed {
                    id: command.to_string(),
                    message: format!("the item `{id}` is in nothing"),
                });
            };
            match parent {
                None => parent = Some(held),
                Some(parent) if parent == held => {}
                Some(_) => {
                    return Err(EngineError::Failed {
                        id: command.to_string(),
                        message:
                            "the items to group must be on the same spread and at the same level (all of them in the spread, or all of them in one group)"
                                .to_string(),
                    });
                }
            }
        }
        let parent = parent.unwrap_or_default();
        let Some(parent_node) = session.document().node(parent).cloned() else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the items are in the node `{parent}`, and it is gone"),
            });
        };
        if parent_node.kind != "layout.spread" && parent_node.kind != "layout.item.group" {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the node `{parent}` is a `{}` and holds no items", parent_node.kind),
            });
        }
        let Some(spread) = spread_of(session, ids[0]) else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the item `{}` is not in a spread", ids[0]),
            });
        };
        // The **items** as the parent holds them: the ones that are
        // grouped, in the order they were drawn.
        let held: Vec<std::sync::Arc<Node>> =
            parent_node.children.iter().filter(|child| ids.contains(&child.id)).cloned().collect();
        if held.len() != ids.len() {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: "the items to group are not all in the same place".to_string(),
            });
        }
        // The **layer** of the item at the back: the group is on that
        // layer, and the items keep the layers they were on.
        let back = held.first().map(|item| item.id).unwrap_or_default();
        let layer = session
            .document()
            .relations
            .iter()
            .find(|relation| relation.from == back && relation.kind == craftkit_sdk::layer::ON_LAYER)
            .cloned();
        // The **group** is made in the place the items were.
        let name = self.name.as_ref().map(|name| name.0.trim().to_string()).filter(|name| !name.is_empty());
        let name = name.unwrap_or_else(|| DEFAULT_NAME.to_string());
        let group = session.document_mut()?.add_node_with_under(
            Some(parent),
            name.clone(),
            "layout.item.group",
            None,
            json!({}),
        );
        // The **children** of the parent, with the group in the place
        // the item at the back stood, and the items gone from it.
        let Some(parent_node) = session.document().node(parent).cloned() else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the items are in the node `{parent}`, and it is gone"),
            });
        };
        let group_node = parent_node.children.iter().find(|child| child.id == group).cloned();
        let Some(group_node) = group_node else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the group `{group}` was not made"),
            });
        };
        let mut children: Vec<std::sync::Arc<Node>> = Vec::with_capacity(parent_node.children.len());
        let mut placed = false;
        for child in parent_node.children.iter() {
            if child.id == group {
                // The group is placed where the item at the back stood.
                continue;
            }
            if !child.kind.starts_with("layout.item.") {
                children.push(child.clone());
                continue;
            }
            if ids.contains(&child.id) {
                if !placed {
                    children.push(group_node.clone());
                    placed = true;
                }
                continue;
            }
            children.push(child.clone());
        }
        if let Some(node) = session.document_mut()?.node_mut(parent) {
            node.children = children;
        }
        // The **items** are in the group, in the order they were drawn.
        if let Some(node) = session.document_mut()?.node_mut(group) {
            for item in held {
                node.children.push(item);
            }
        }
        // The **layer** of the group is the layer of the item at the
        // back: the tree's own rule.
        if let Some(layer) = layer {
            let mut relation = layer;
            relation.from = group;
            session.document_mut()?.add_relation(relation);
        }
        Ok(Outcome::changed(format!("grouped {} items into `{name}`", ids.len())).with_data(json!({
            "group": group,
            "name": name,
            "spread": spread,
            "items": ids,
        })))
    }
}

/// **Ungroup** a group: the items go back where the group stood.
#[derive(Command)]
#[command(id = "layout.item.ungroup", area = "layout", layer = "layout", icon = "minus", mutates, undoable)]
pub struct UngroupItems {
    /// The group.
    pub group: NodeRef,
}

impl UngroupItems {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let command = "layout.item.ungroup";
        let group = self.group.0;
        let node = item_of(session, command, group)?;
        if node.kind != "layout.item.group" {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the node `{group}` is a `{}` and not a group", node.kind),
            });
        }
        may_edit(session, &node, command)?;
        let Some(parent) = parent_of(session, group) else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the group `{group}` is in nothing"),
            });
        };
        let Some(parent_node) = session.document().node(parent).cloned() else {
            return Err(EngineError::Failed {
                id: command.to_string(),
                message: format!("the group `{group}` is in the node `{parent}`, and it is gone"),
            });
        };
        // The **items** of the group, in the order they are in.
        let held: Vec<std::sync::Arc<Node>> = node.children.clone();
        let items: Vec<u64> = held.iter().map(|child| child.id).collect();
        // The **children** of the parent, with the items of the group
        // where the group stood.
        let mut children: Vec<std::sync::Arc<Node>> =
            Vec::with_capacity(parent_node.children.len() + held.len());
        for child in parent_node.children.iter() {
            if child.id == group {
                for item in held.iter() {
                    children.push(item.clone());
                }
                continue;
            }
            children.push(child.clone());
        }
        if let Some(node) = session.document_mut()?.node_mut(parent) {
            node.children = children;
        }
        // The **group** is gone, and the layers of the items stay.
        session.document_mut()?.remove_node(group);
        Ok(Outcome::changed(format!("ungrouped {} item(s)", items.len())).with_data(json!({
            "group": group,
            "items": items,
        })))
    }
}
