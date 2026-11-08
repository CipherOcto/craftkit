//! The **turn** of an item.
//!
//! An item may be **turned**: a number of degrees about the middle of
//! its shape. The turn is a fact of the item, and it is written in
//! **degrees** — the way a person writes an angle — and the
//! **radians** are derived from it, because the tree's angle is a
//! radian (§ the numeric battery's angle) and the tree's unit is
//! never lost and never written down twice.
//!
//! The rules:
//!
//! - a turn that is **not a number** is refused, and so is a turn so
//!   large that the radians of it are not a number;
//! - an item that is **locked** may not be turned, like every other
//!   edit;
//! - the **turned box** of an item is **derived**: it is the box of
//!   the item's shape turned about its middle, and it is the box the
//!   item occupies once it is turned.
//!
//! # What the port found
//!
//! - **The tree's turn is an affine**, and the tree builds it in
//!   radians — `Affine::rotate(30f64.to_radians())` — and the tree's
//!   user interface writes degrees. Here the degrees are the fact and
//!   the radians are derived.
//! - **The tree's turn is about the middle of the item**, and so is
//!   this one: the place of the item does not move when it is turned,
//!   which is the same bargain as the scale of an item (§482).
//! - **The tree's turn snaps**, in steps of a quarter and of a degree
//!   at a time, when a key is held down; and the tree's turn may be
//!   any angle at all. Here the turn is any number of degrees.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session};

use crate::items::{item_of, may_edit};

/// The **turn** of an item, in degrees. An item that is not turned is
/// turned by no degrees at all.
pub fn rotation_of(item: &Node) -> f64 {
    item.data
        .get("rotation")
        .and_then(|rotation| rotation.as_f64())
        .filter(|rotation| rotation.is_finite())
        .unwrap_or(0.0)
}

/// The **turn** of an item, in radians — the tree's unit.
pub fn radians_of(item: &Node) -> f64 {
    rotation_of(item).to_radians()
}

/// The **turned box** of an item: the box its shape takes when it is
/// turned about its middle.
///
/// It is `None` when the item has no shape to turn.
pub fn turned_of(item: &Node) -> Option<(f64, f64, f64, f64)> {
    let rect = item.data.get("rect")?.as_object()?;
    let x = rect.get("x")?.as_f64()?;
    let y = rect.get("y")?.as_f64()?;
    let width = rect.get("width")?.as_f64()?;
    let height = rect.get("height")?.as_f64()?;
    let centre_x = x + width / 2.0;
    let centre_y = y + height / 2.0;
    let (sin, cos) = radians_of(item).sin_cos();
    let mut left = f64::INFINITY;
    let mut top = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut bottom = f64::NEG_INFINITY;
    for (corner_x, corner_y) in [(x, y), (x + width, y), (x, y + height), (x + width, y + height)] {
        let dx = corner_x - centre_x;
        let dy = corner_y - centre_y;
        let turned_x = centre_x + dx * cos - dy * sin;
        let turned_y = centre_y + dx * sin + dy * cos;
        left = left.min(turned_x);
        top = top.min(turned_y);
        right = right.max(turned_x);
        bottom = bottom.max(turned_y);
    }
    Some((left, top, right - left, bottom - top))
}

/// The turned box of an item, as it is reported.
pub fn turned_json(item: &Node) -> Json {
    match turned_of(item) {
        Some((x, y, width, height)) => {
            json!({ "x": x, "y": y, "width": width, "height": height })
        }
        None => Json::Null,
    }
}

/// **Turn** an item.
///
/// The turn is about the middle of the item's shape, and the place of
/// the item does not move.
#[derive(Command)]
#[command(id = "layout.item.rotate", area = "layout", layer = "layout", icon = "rect", mutates, undoable)]
pub struct RotateItem {
    /// The item.
    pub item: NodeRef,
    /// The turn, in degrees. A turn of ninety is a quarter turn, and
    /// the turn is clockwise on the page.
    pub degrees: Number,
}

impl RotateItem {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.rotate".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(session, &node, &id)?;
        let degrees = self.degrees.0;
        if !degrees.is_finite() {
            return Err(EngineError::Failed {
                id,
                message: format!("{degrees} is not a turn: a turn is a number of degrees"),
            });
        }
        if !degrees.to_radians().is_finite() {
            return Err(EngineError::Failed {
                id,
                message: format!("the turn {degrees} degrees is not a number of radians"),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(item) {
            node.data["rotation"] = json!(degrees);
        }
        Ok(Outcome::changed(format!("the item `{item}` is turned {degrees} degrees"))
            .with_data(json!({ "item": item, "degrees": degrees, "radians": degrees.to_radians() })))
    }
}
