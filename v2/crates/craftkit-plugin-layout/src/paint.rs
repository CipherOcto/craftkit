//! The **paint** of an item: its fill and its stroke.
//!
//! An item may be **filled** with a colour and may have a **stroke**:
//! a colour and a weight. Both are facts of the item, and both may be
//! **nothing** — an item with no fill is not painted, and an item with
//! no stroke has no line around it.
//!
//! The rules:
//!
//! - a **colour** is written `#rrggbb`, the way a person writes one,
//!   and a colour that is not one is refused;
//! - a **weight** is a number of points that is not negative, and the
//!   weight of a stroke with no weight of its own is the tree's own
//!   one point;
//! - an item that is **locked** may not be painted, like every other
//!   edit;
//! - a colour that is **not said** leaves the paint as it was, and an
//!   **empty** colour takes it away — the same bargain as the header
//!   of a printed sheet (§471).
//!
//! # What the port found
//!
//! - **The tree's fill and stroke are swatches**: a named colour and a
//!   tint, and the tree has a list of swatches a document may choose
//!   from, and its fills may be gradients with an angle and a vector.
//!   Here a colour is a colour — `#rrggbb` — and a fill is a colour
//!   and nothing else, which is what the kit's own colour is.
//! - **The tree's stroke has a kind, an alignment, caps, joins and
//!   arrowheads**, and the tree's default stroke is one point of
//!   black. Here a stroke is a colour and a weight, and an item with
//!   no stroke has none.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::items::{item_of, may_edit};

/// The **weight** of a stroke with no weight of its own, in points:
/// the tree's own.
pub const DEFAULT_WEIGHT: f64 = 1.0;

/// The **most** a stroke may weigh, in points.
pub const MAX_WEIGHT: f64 = 1000.0;

/// The **fill** of an item: its colour, if it has one.
pub fn fill_of(item: &Node) -> Json {
    item.data.get("fill").cloned().unwrap_or(Json::Null)
}

/// The **stroke** of an item: its colour and its weight, if it has one.
pub fn stroke_of(item: &Node) -> Json {
    item.data.get("stroke").cloned().unwrap_or(Json::Null)
}

/// A **colour**, read the way a person writes one.
fn colour_of(text: &str, id: &str) -> Result<[f32; 4], EngineError> {
    match craftkit_sdk::Colour::from_hex(text.trim()) {
        Ok(colour) => Ok(colour.0),
        Err(why) => Err(EngineError::Failed { id: id.to_string(), message: why }),
    }
}

/// **Fill** an item with a colour — or take the fill away.
///
/// The colour is written `#rrggbb`. An empty colour takes the fill
/// away, and a colour that is not said leaves the fill as it was.
#[derive(Command)]
#[command(id = "layout.item.fill", area = "layout", layer = "layout", icon = "paint", mutates, undoable)]
pub struct SetFill {
    /// The item.
    pub item: NodeRef,
    /// The colour of the fill. An empty colour takes the fill away;
    /// without one, the fill stays as it is.
    pub colour: Option<Text>,
}

impl SetFill {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.fill".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(session, &node, &id)?;
        // An **empty** colour takes the fill away, and a colour that
        // is not said leaves the fill as it was.
        let fill = match &self.colour {
            Some(colour) if colour.0.trim().is_empty() => Json::Null,
            Some(colour) => {
                let colour = colour_of(&colour.0, &id)?;
                json!({ "colour": colour })
            }
            None => fill_of(&node),
        };
        if let Some(node) = session.document_mut()?.node_mut(item) {
            match &fill {
                Json::Null => {
                    if let Some(map) = node.data.as_object_mut() {
                        map.remove("fill");
                    }
                }
                fill => node.data["fill"] = fill.clone(),
            }
        }
        Ok(Outcome::changed(format!("the item `{item}` is painted")).with_data(json!({
            "item": item,
            "fill": fill,
        })))
    }
}

/// **Stroke** an item with a colour — or take the stroke away.
///
/// The colour is written `#rrggbb` and the weight is in points. An
/// empty colour takes the stroke away, a colour that is not said
/// leaves the stroke as it was, and a stroke with no weight of its
/// own weighs one point.
#[derive(Command)]
#[command(id = "layout.item.stroke", area = "layout", layer = "layout", icon = "paint", mutates, undoable)]
pub struct SetStroke {
    /// The item.
    pub item: NodeRef,
    /// The colour of the stroke. An empty colour takes the stroke
    /// away; without one, the stroke stays as it is.
    pub colour: Option<Text>,
    /// The **weight** of the stroke, in points. Without one, the
    /// weight stays as it is.
    pub weight: Option<Number>,
}

impl SetStroke {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.stroke".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(session, &node, &id)?;
        let colour = match &self.colour {
            Some(colour) if colour.0.trim().is_empty() => None,
            Some(colour) => Some(colour_of(&colour.0, &id)?),
            None => None,
        };
        let weight = match &self.weight {
            Some(weight) => {
                let weight = weight.0;
                if !weight.is_finite() || weight < 0.0 || weight > MAX_WEIGHT {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "{weight} is not a weight: it is a number from zero to {MAX_WEIGHT}"
                        ),
                    });
                }
                Some(weight)
            }
            None => None,
        };
        // The **weight** that is not said stays as it was, and the
        // weight of a stroke that has none is the tree's own one point.
        let weight = match weight {
            Some(weight) => Some(weight),
            None => {
                stroke_of(&node).get("weight").and_then(|weight| weight.as_f64()).or(Some(DEFAULT_WEIGHT))
            }
        };
        let stroke = match colour {
            Some(colour) => json!({
                "colour": colour,
                "weight": weight.unwrap_or(DEFAULT_WEIGHT),
            }),
            None if self.colour.is_some() => Json::Null,
            None => match stroke_of(&node) {
                Json::Null => Json::Null,
                stroke => {
                    let mut stroke = stroke;
                    if let Some(map) = stroke.as_object_mut() {
                        map.insert("weight".to_string(), json!(weight.unwrap_or(DEFAULT_WEIGHT)));
                    }
                    stroke
                }
            },
        };
        if let Some(node) = session.document_mut()?.node_mut(item) {
            match &stroke {
                Json::Null => {
                    if let Some(map) = node.data.as_object_mut() {
                        map.remove("stroke");
                    }
                }
                stroke => node.data["stroke"] = stroke.clone(),
            }
        }
        Ok(Outcome::changed(format!("the item `{item}` is stroked")).with_data(json!({
            "item": item,
            "stroke": stroke,
        })))
    }
}
