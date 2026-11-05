//! The **order** the items of a spread are drawn in.
//!
//! The items of a spread are its children, and the children are drawn
//! in the order they stand in: the first is drawn first and so it is
//! **at the back**, and the last is drawn last and so it is **at the
//! front**.
//!
//! The rules:
//!
//! - a command moves **one** item, because a command that takes a
//!   selection is a command that does not say what it does;
//! - the pages of a spread are **not** items, and they keep their
//!   places: only the order of the items among themselves changes;
//! - a **locked** item may not be edited, and the order is an edit;
//! - an item that is already where it was told to go **does not
//!   move**, and the command says so rather than saying that
//!   something changed.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{EngineError, Node, NodeRef, Outcome, Session};

use crate::items::{item_of, may_edit};

/// The **spread** an item is in.
fn spread_of(session: &Session, item: u64) -> Option<u64> {
    session
        .document()
        .walk()
        .find(|node| node.kind == "layout.spread" && node.children.iter().any(|child| child.id == item))
        .map(|node| node.id)
}

/// The **items** of a spread, in the order they are drawn.
fn items_of(session: &Session, spread: u64) -> Vec<u64> {
    let Some(node) = session.document().node(spread) else { return Vec::new() };
    node.children
        .iter()
        .filter(|child| child.kind.starts_with("layout.item."))
        .map(|child| child.id)
        .collect()
}

/// **Move** an item in the order of its spread.
fn move_item(session: &mut Session, id: &str, item: u64, place: &str) -> Result<Outcome, EngineError> {
    let node = item_of(session, id, item)?;
    may_edit(&node, id)?;
    let Some(spread) = spread_of(session, item) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the item `{item}` is not in a spread"),
        });
    };
    let items = items_of(session, spread);
    let Some(index) = items.iter().position(|other| *other == item) else {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("the item `{item}` is not in the spread `{spread}`"),
        });
    };
    let last = items.len().saturating_sub(1);
    let target = match place {
        "front" => last,
        "back" => 0,
        "forward" => (index + 1).min(last),
        _ => index.saturating_sub(1),
    };
    if target == index {
        return Ok(Outcome::new(format!("the item `{item}` is already `{place}`"))
            .with_data(json!({ "item": item, "spread": spread, "to": place, "moved": false })));
    }
    // The **order** of the items, with the item moved — and the pages
    // of the spread where they are.
    let mut order = items.clone();
    let moved = order.remove(index);
    order.insert(target, moved);
    // The children are `Arc<Node>`, so the order is written by hand:
    // the items take the places the items had.
    let slots: Vec<usize> = session
        .document()
        .node(spread)
        .map(|node| {
            node.children
                .iter()
                .enumerate()
                .filter(|(_, child)| child.kind.starts_with("layout.item."))
                .map(|(index, _)| index)
                .collect()
        })
        .unwrap_or_default();
    let mut new_children: Vec<(usize, std::sync::Arc<Node>)> = Vec::new();
    let mut order = order.into_iter();
    for slot in slots {
        let id = order.next().unwrap_or_default();
        let Some(child) = session.document().node(id).cloned() else { continue };
        new_children.push((slot, std::sync::Arc::new(child)));
    }
    if let Some(node) = session.document_mut()?.node_mut(spread) {
        for (slot, child) in new_children {
            node.children[slot] = child;
        }
    }
    Ok(Outcome::changed(format!("the item `{item}` is `{place}`"))
        .with_data(json!({ "item": item, "spread": spread, "to": place, "moved": true })))
}

/// **Read** the items of a spread, in the order they are drawn.
#[derive(Command)]
#[command(id = "layout.item.list", area = "layout", layer = "layout", icon = "list")]
pub struct ListItems {
    /// The spread.
    pub spread: NodeRef,
}

impl ListItems {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.list".to_string();
        let spread = self.spread.0;
        let Some(node) = session.document().node(spread) else {
            return Err(EngineError::Failed { id, message: format!("there is no spread `{spread}`") });
        };
        if node.kind != "layout.spread" {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{spread}` is a `{}` and not a spread", node.kind),
            });
        }
        let items = items_of(session, spread);
        Ok(Outcome::new(format!("the spread `{spread}` has {} item(s)", items.len()))
            .with_data(json!({ "spread": spread, "items": items })))
    }
}

/// **Bring** an item to the **front**.
#[derive(Command)]
#[command(id = "layout.item.front", area = "layout", layer = "layout", icon = "arrows", mutates, undoable)]
pub struct BringToFront {
    /// The item.
    pub item: NodeRef,
}

impl BringToFront {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        move_item(session, "layout.item.front", self.item.0, "front")
    }
}

/// **Send** an item to the **back**.
#[derive(Command)]
#[command(id = "layout.item.back", area = "layout", layer = "layout", icon = "arrows", mutates, undoable)]
pub struct SendToBack {
    /// The item.
    pub item: NodeRef,
}

impl SendToBack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        move_item(session, "layout.item.back", self.item.0, "back")
    }
}

/// **Move** an item **one step** towards the front.
#[derive(Command)]
#[command(id = "layout.item.forward", area = "layout", layer = "layout", icon = "arrows", mutates, undoable)]
pub struct ForwardItem {
    /// The item.
    pub item: NodeRef,
}

impl ForwardItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        move_item(session, "layout.item.forward", self.item.0, "forward")
    }
}

/// **Move** an item **one step** towards the back.
#[derive(Command)]
#[command(id = "layout.item.backward", area = "layout", layer = "layout", icon = "arrows", mutates, undoable)]
pub struct BackwardItem {
    /// The item.
    pub item: NodeRef,
}

impl BackwardItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        move_item(session, "layout.item.backward", self.item.0, "backward")
    }
}
