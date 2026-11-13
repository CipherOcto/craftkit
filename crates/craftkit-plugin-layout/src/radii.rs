//! The **radius** of an item: the corners of a rectangle that are
//! rounded.
//!
//! An item may have a **radius**: the corners of its rectangle are
//! drawn as arcs of that radius. The radius is a fact of the item, in
//! points, and the **corners** are **derived**: they are the centres
//! of the four corner arcs — the corners of the box inset by the
//! radius — so they cannot be out of date.
//!
//! The rules:
//!
//! - a radius that is **not a number**, or that is **negative**, is
//!   refused;
//! - a radius **beyond half the shorter side** of the box is refused:
//!   the tree clamps it, and this tree refuses it — a corner that is
//!   rounded more than half the width is not a corner;
//! - an item whose shape is **not a rectangle** may not have a radius:
//!   a radius is a rectangle's own.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome};

use crate::items::{item_of, may_edit};

/// The **radius** of an item, if it has one.
pub fn radius_of(item: &Node) -> Option<f64> {
    item.data
        .get("radius")
        .and_then(|radius| radius.as_f64())
        .filter(|radius| radius.is_finite() && *radius >= 0.0)
}

/// The **corners** of an item: the centres of the four corner arcs.
///
/// They are `None` when the item has no radius or no box.
pub fn corners_of(item: &Node) -> Option<Vec<(f64, f64)>> {
    let radius = radius_of(item)?;
    let rect = item.data.get("rect")?.as_object()?;
    let x = rect.get("x")?.as_f64()?;
    let y = rect.get("y")?.as_f64()?;
    let width = rect.get("width")?.as_f64()?;
    let height = rect.get("height")?.as_f64()?;
    let (left, right) = (x + radius, x + width - radius);
    let (top, bottom) = (y + radius, y + height - radius);
    Some(vec![(left, top), (right, top), (right, bottom), (left, bottom)])
}

/// The corners of an item, as they are reported.
pub fn corners_json(item: &Node) -> Json {
    match corners_of(item) {
        Some(corners) => {
            json!(corners.into_iter().map(|(x, y)| json!({ "x": x, "y": y })).collect::<Vec<_>>())
        }
        None => Json::Null,
    }
}

commands! {
    /// **Round** the corners of an item.
    layout.item.radius {
        area: "layout",
        layer: "layout",
        icon: "rect",
        mutates,
        undoable,
        params: {
            /// The item.
            item: NodeRef,
            /// The radius of the corners, in points.
            radius: Number,
        },
        run: {
            let id = "layout.item.radius".to_string();
            let item = self.item.0;
            let node = item_of(session, &id, item)?;
            may_edit(session, &node, &id)?;
            let radius = self.radius.0;
            if !radius.is_finite() || radius < 0.0 {
                return Err(EngineError::Failed {
                    id,
                    message: format!("{radius} is not a radius: a radius is a number and not a negative one"),
                });
            }
            let shape = node.data.get("shape").and_then(|shape| shape.as_str()).unwrap_or_default();
            if shape != "rectangle" {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "a radius is a rectangle's own, and the item `{}` is a `{shape}`",
                        node.name
                    ),
                });
            }
            let rect = node.data.get("rect").and_then(|rect| rect.as_object());
            let size = |name: &str| rect.and_then(|rect| rect.get(name)).and_then(|value| value.as_f64());
            let (Some(width), Some(height)) = (size("width"), size("height")) else {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the item `{}` has no box to round", node.name),
                });
            };
            let shorter = width.min(height);
            if radius > shorter / 2.0 {
                return Err(EngineError::Failed {
                    id,
                    message: format!(
                        "a radius of {radius} is more than half the shorter side of the box, which is {shorter}"
                    ),
                });
            }
            if let Some(node) = session.document_mut()?.node_mut(item) {
                node.data["radius"] = json!(radius);
            }
            Ok(Outcome::changed(format!("the item `{item}` is rounded by {radius}"))
                .with_data(json!({ "item": item, "radius": radius })))

        }
    }
}
