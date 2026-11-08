//! The **shadow** of an item.
//!
//! An item may have a **shadow**: a blurred copy of itself, painted
//! behind it, moved away from it. The shadow is a fact of the item —
//! the **angle** the light comes from, the **distance** the shadow is
//! moved, the **blur** it is softened by, its **colour** and its
//! **opacity** — and the **offset** and the **box** of the shadow are
//! **derived**: the offset is the distance along the angle, and the box
//! is the box of the item grown by the blur and moved by the offset.
//!
//! The rules:
//!
//! - an angle, a distance or a blur that is **not a number** is
//!   refused, and a distance or a blur that is **negative** is
//!   refused;
//! - a **colour** that is not `#rrggbb` is refused;
//! - an **opacity** that is not between zero and one is refused.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::items::{item_of, may_edit};

/// A **shadow**: the light, the distance, the blur, the colour and
/// the opacity.
#[derive(Clone, Debug, PartialEq)]
pub struct Shadow {
    /// The angle the light comes from, in degrees.
    pub angle: f64,
    /// How far the shadow is moved, in points.
    pub distance: f64,
    /// How much the shadow is softened by, in points.
    pub blur: f64,
    /// The colour of the shadow.
    pub colour: String,
    /// How solid the shadow is, from zero to one.
    pub opacity: f64,
}

impl Shadow {
    /// The **offset** of the shadow: where it is moved to.
    pub fn offset(&self) -> (f64, f64) {
        let radians = self.angle.to_radians();
        (self.distance * radians.cos(), -self.distance * radians.sin())
    }

    /// The shadow, as it is kept.
    pub fn to_json(&self) -> Json {
        json!({
            "angle": self.angle,
            "distance": self.distance,
            "blur": self.blur,
            "colour": self.colour.clone(),
            "opacity": self.opacity,
        })
    }

    /// The **box** of a shadow of an item's box: the box grown by the
    /// blur and moved by the offset.
    pub fn box_of(&self, box_of: (f64, f64, f64, f64)) -> (f64, f64, f64, f64) {
        let (x, y, width, height) = box_of;
        let (offset_x, offset_y) = self.offset();
        (
            x + offset_x - self.blur,
            y + offset_y - self.blur,
            width + 2.0 * self.blur,
            height + 2.0 * self.blur,
        )
    }
}

/// The **shadow** of an item, if it has one.
pub fn shadow_of(item: &Node) -> Option<Shadow> {
    let shadow = item.data.get("shadow")?;
    let angle = shadow.get("angle")?.as_f64()?;
    let distance = shadow.get("distance")?.as_f64()?;
    let blur = shadow.get("blur").and_then(|blur| blur.as_f64()).unwrap_or(0.0);
    let colour = shadow.get("colour").and_then(|colour| colour.as_str()).unwrap_or("#000000").to_string();
    let opacity = shadow.get("opacity").and_then(|opacity| opacity.as_f64()).unwrap_or(1.0);
    Some(Shadow { angle, distance, blur, colour, opacity })
}

/// The **box** of an item, if it has one.
fn box_of(item: &Node) -> Option<(f64, f64, f64, f64)> {
    let rect = item.data.get("rect")?.as_object()?;
    let x = rect.get("x")?.as_f64()?;
    let y = rect.get("y")?.as_f64()?;
    let width = rect.get("width")?.as_f64()?;
    let height = rect.get("height")?.as_f64()?;
    Some((x, y, width, height))
}

/// The **shadow** of an item, as it is reported.
pub fn shadow_json(item: &Node) -> Json {
    let Some(shadow) = shadow_of(item) else { return Json::Null };
    let offset = shadow.offset();
    let box_of = box_of(item).map(|box_of| shadow.box_of(box_of));
    json!({
        "angle": shadow.angle,
        "distance": shadow.distance,
        "blur": shadow.blur,
        "colour": shadow.colour,
        "opacity": shadow.opacity,
        "offset": { "x": offset.0, "y": offset.1 },
        "box": match box_of {
            Some((x, y, width, height)) => json!({ "x": x, "y": y, "width": width, "height": height }),
            None => Json::Null,
        },
    })
}

/// **Give** an item a shadow.
#[derive(Command)]
#[command(id = "layout.item.shadow", area = "layout", layer = "layout", icon = "paint", mutates, undoable)]
pub struct SetShadow {
    /// The item.
    pub item: NodeRef,
    /// The **angle** the light comes from, in degrees.
    pub angle: Number,
    /// The **distance** the shadow is moved, in points.
    pub distance: Number,
    /// The **blur** the shadow is softened by, in points.
    pub blur: Option<Number>,
    /// The **colour** of the shadow, as `#rrggbb`.
    pub colour: Option<Text>,
    /// The **opacity** of the shadow, from zero to one.
    pub opacity: Option<Number>,
}

impl SetShadow {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.shadow".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(session, &node, &id)?;
        let angle = self.angle.0;
        if !angle.is_finite() {
            return Err(EngineError::Failed {
                id,
                message: format!("{angle} is not an angle: an angle is a number of degrees"),
            });
        }
        let distance = self.distance.0;
        if !distance.is_finite() || distance < 0.0 {
            return Err(EngineError::Failed {
                id,
                message: format!(
                    "{distance} is not a distance: a distance is a number and not a negative one"
                ),
            });
        }
        let blur = self.blur.map(|blur| blur.0).unwrap_or(0.0);
        if !blur.is_finite() || blur < 0.0 {
            return Err(EngineError::Failed {
                id,
                message: format!("{blur} is not a blur: a blur is a number and not a negative one"),
            });
        }
        let colour = match &self.colour {
            Some(colour) => match craftkit_sdk::Colour::from_hex(colour.0.trim()) {
                Ok(_) => colour.0.trim().to_ascii_lowercase(),
                Err(why) => {
                    return Err(EngineError::Failed { id, message: why });
                }
            },
            None => "#000000".to_string(),
        };
        let opacity = self.opacity.map(|opacity| opacity.0).unwrap_or(1.0);
        if !opacity.is_finite() || !(0.0..=1.0).contains(&opacity) {
            return Err(EngineError::Failed {
                id,
                message: format!("{opacity} is not an opacity: an opacity is between zero and one"),
            });
        }
        let shadow = Shadow { angle, distance, blur, colour, opacity };
        if let Some(node) = session.document_mut()?.node_mut(item) {
            node.data["shadow"] = shadow.to_json();
        }
        Ok(Outcome::changed(format!("the item `{item}` has a shadow"))
            .with_data(json!({ "item": item, "shadow": shadow.to_json() })))
    }
}
