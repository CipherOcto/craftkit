//! The **item** of a spread.
//!
//! An item is a frame, a graphic or a group, and it has **facts** of
//! its own: how **solid** it is, whether it is **hidden**, whether it
//! is **locked**, and how **big** it is.
//!
//! - the **opacity** is a percentage, the way the vector battery's is,
//!   and it is kept as the fraction it is;
//! - an item may be **hidden**: it is not drawn, and that is the kit's
//!   own word for it — a muted node is not drawn;
//! - an item may be **locked**, and a locked item may not be **edited**:
//!   every command that changes it is refused, and the refusal says
//!   which item and why;
//! - the **scale** of an item is a factor on the size of its shape, and
//!   a factor that is not a number above zero is refused — and so is
//!   a size that would not be a number.
//!
//! # What the port found
//!
//! - **The tree's locked objects are skipped, not refused.** Its
//!   commands filter the locked ones out of the set they work on. Here
//!   an edit of a locked item is **refused**: a command that silently
//!   does nothing to half of what it was given is a command whose
//!   answer is a lie.
//! - **The tree's opacity is a percentage and so is this one.** The
//!   tree's scale is a factor and so is this one. The numbers are the
//!   tree's numbers, and the second spelling of a number is a second
//!   thing to get wrong.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

/// The **item** a command is about, and whether it is an item.
pub fn item_of(session: &Session, command: &str, item: u64) -> Result<Node, EngineError> {
    let Some(node) = session.document().node(item) else {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("there is no item `{item}`"),
        });
    };
    if !node.kind.starts_with("layout.item.") {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the node `{item}` is a `{}` and not an item", node.kind),
        });
    }
    Ok(node.clone())
}

/// Whether an item is **locked**: it may not be edited.
pub fn is_locked(item: &Node) -> bool {
    item.data.get("locked").and_then(|locked| locked.as_bool()).unwrap_or(false)
}

/// Whether an item may be **edited**.
///
/// It may not, when it is locked — and the refusal says which item and
/// why, because a refusal that says only "no" is a refusal that sends
/// the reader looking.
pub fn may_edit(item: &Node, command: &str) -> Result<(), EngineError> {
    if is_locked(item) {
        return Err(EngineError::Failed {
            id: command.to_string(),
            message: format!("the item `{}` is locked, and a locked item may not be edited", item.name),
        });
    }
    Ok(())
}

/// The **opacity** of an item, as the fraction it is.
pub fn opacity_of(item: &Node) -> f64 {
    item.data
        .get("opacity")
        .and_then(|opacity| opacity.as_f64())
        .filter(|opacity| opacity.is_finite() && (0.0..=1.0).contains(opacity))
        .unwrap_or(1.0)
}

/// **Set** the opacity of an item.
///
/// The opacity is a **percentage**, from zero to a hundred, and a
/// number outside it is refused.
#[derive(Command)]
#[command(id = "layout.item.opacity", area = "layout", layer = "layout", icon = "paint", mutates, undoable)]
pub struct SetOpacity {
    /// The item.
    pub item: NodeRef,
    /// The opacity, as a percentage: from zero to a hundred.
    pub opacity: Number,
}

impl SetOpacity {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.opacity".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(&node, &id)?;
        let opacity = self.opacity.0;
        if !opacity.is_finite() || !(0.0..=100.0).contains(&opacity) {
            return Err(EngineError::Failed {
                id,
                message: "an opacity is a percentage from zero to a hundred".to_string(),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(item) {
            node.data["opacity"] = json!(opacity / 100.0);
        }
        Ok(Outcome::changed(format!("the item `{item}` is {opacity} per cent solid"))
            .with_data(json!({ "item": item, "opacity": opacity })))
    }
}

/// **Hide** an item.
///
/// A hidden item is not drawn.
#[derive(Command)]
#[command(id = "layout.item.hide", area = "layout", layer = "layout", icon = "minus", mutates, undoable)]
pub struct HideItem {
    /// The item.
    pub item: NodeRef,
}

impl HideItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.hide".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(&node, &id)?;
        let name = node.name.clone();
        if let Some(node) = session.document_mut()?.node_mut(item) {
            node.muted = true;
        }
        Ok(Outcome::changed(format!("the item `{name}` is hidden")).with_data(json!({ "item": item })))
    }
}

/// **Show** an item.
#[derive(Command)]
#[command(id = "layout.item.show", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct ShowItem {
    /// The item.
    pub item: NodeRef,
}

impl ShowItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.show".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(&node, &id)?;
        let name = node.name.clone();
        if let Some(node) = session.document_mut()?.node_mut(item) {
            node.muted = false;
        }
        Ok(Outcome::changed(format!("the item `{name}` is shown")).with_data(json!({ "item": item })))
    }
}

/// **Lock** an item.
///
/// A locked item may not be edited, and every command that changes it
/// is refused until it is unlocked.
#[derive(Command)]
#[command(id = "layout.item.lock", area = "layout", layer = "layout", icon = "lock", mutates, undoable)]
pub struct LockItem {
    /// The item.
    pub item: NodeRef,
}

impl LockItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.lock".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        let name = node.name.clone();
        if let Some(node) = session.document_mut()?.node_mut(item) {
            node.data["locked"] = json!(true);
        }
        Ok(Outcome::changed(format!("the item `{name}` is locked")).with_data(json!({ "item": item })))
    }
}

/// **Unlock** an item.
#[derive(Command)]
#[command(id = "layout.item.unlock", area = "layout", layer = "layout", icon = "lock", mutates, undoable)]
pub struct UnlockItem {
    /// The item.
    pub item: NodeRef,
}

impl UnlockItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.unlock".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        let name = node.name.clone();
        if let Some(node) = session.document_mut()?.node_mut(item) {
            node.data["locked"] = json!(false);
        }
        Ok(Outcome::changed(format!("the item `{name}` is unlocked")).with_data(json!({ "item": item })))
    }
}

/// **Scale** an item.
///
/// The factor is on the **size** of the shape, and the place of the
/// item does not move. A factor that is not a number above zero is
/// refused, and so is a size that would not be a number.
#[derive(Command)]
#[command(id = "layout.item.scale", area = "layout", layer = "layout", icon = "rect", mutates, undoable)]
pub struct ScaleItem {
    /// The item.
    pub item: NodeRef,
    /// The factor: twice is twice as big.
    pub factor: Number,
}

impl ScaleItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.scale".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(&node, &id)?;
        let factor = self.factor.0;
        if !factor.is_finite() || factor <= 0.0 {
            return Err(EngineError::Failed {
                id,
                message: format!("{factor} is not a factor: a factor is a number above zero"),
            });
        }
        let Some(rect) = node.data.get("rect").and_then(|rect| rect.as_object()) else {
            return Err(EngineError::Failed {
                id,
                message: format!("the item `{}` has no shape to scale", node.name),
            });
        };
        let number = |name: &str| rect.get(name).and_then(|value| value.as_f64());
        let (Some(width), Some(height)) = (number("width"), number("height")) else {
            return Err(EngineError::Failed {
                id,
                message: format!("the item `{}` has no shape to scale", node.name),
            });
        };
        let new_width = width * factor;
        let new_height = height * factor;
        if !new_width.is_finite() || !new_height.is_finite() {
            return Err(EngineError::Failed {
                id,
                message: format!("the shape of the item `{}` would not be a number", node.name),
            });
        }
        let mut new_rect = rect.clone();
        new_rect.insert("width".to_string(), Json::from(new_width));
        new_rect.insert("height".to_string(), Json::from(new_height));
        if let Some(node) = session.document_mut()?.node_mut(item) {
            node.data["rect"] = Json::Object(new_rect);
        }
        Ok(Outcome::changed(format!("the item `{item}` is scaled by {factor}"))
            .with_data(json!({ "item": item, "factor": factor, "width": new_width, "height": new_height })))
    }
}
