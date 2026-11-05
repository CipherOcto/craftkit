//! The **style** of an item.
//!
//! A **style** is a named set of paint facts — a fill and a stroke —
//! and an item may be put **on** a style. A style may be **based on**
//! another, and the base's paint sits **under** the style's own, as
//! the character of a prose style sits under it (§468).
//!
//! The paint an item is drawn with is **derived**: it is the item's
//! own paint when the item has one, and the style's paint when it does
//! not. An item that overrides its style's fill is painted with its
//! own fill, and that is the tree's own bargain — an item follows the
//! style until its own paint differs from the style's.
//!
//! The rules:
//!
//! - a style that is **based on** itself, or on a style that is based
//!   on it, ends the chain instead of hanging the program — the chain
//!   is **bounded**;
//! - a node that is **not a style** is refused;
//! - an item that is **locked** may not be put on a style.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

use crate::items::{item_of, may_edit};
use crate::paint::{fill_of, stroke_of};

/// The **kind** of a style.
pub const STYLE_KIND: &str = "layout.style";

/// The **kind** of the edge that says a style is based on another.
pub const BASED_ON: &str = "layout.based_on";

/// The **kind** of the edge that says an item is on a style.
pub const STYLED_BY: &str = "layout.styled_by";

/// The most a chain of styles may be **long**.
pub const MAX_CHAIN: usize = 32;

/// The **style** an item is on, if it is on one.
pub fn style_of(document: &craftkit_sdk::Document, item: u64) -> Option<u64> {
    document
        .relations
        .iter()
        .find(|relation| relation.kind == STYLED_BY && relation.from == item)
        .map(|relation| relation.to)
}

/// The **chain** of a style, from the furthest base to the style
/// itself.
///
/// The walk is **bounded** and it stops at a style it has already
/// seen, so a cycle ends the chain instead of hanging the program.
pub fn chain_of(document: &craftkit_sdk::Document, style: u64) -> Vec<u64> {
    let mut chain: Vec<u64> = Vec::new();
    let mut seen: Vec<u64> = vec![style];
    let mut current = style;
    for _ in 0..MAX_CHAIN {
        let Some(relation) =
            document.relations.iter().find(|relation| relation.kind == BASED_ON && relation.from == current)
        else {
            break;
        };
        let base = relation.to;
        chain.push(base);
        if seen.contains(&base) {
            break;
        }
        seen.push(base);
        current = base;
    }
    chain.reverse();
    chain.push(style);
    chain
}

/// The **fill** of a style, with the fill of its base under it.
pub fn effective_fill(document: &craftkit_sdk::Document, style: u64) -> Json {
    let mut fill = Json::Null;
    for id in chain_of(document, style) {
        let Some(node) = document.node(id) else { continue };
        let own = fill_of(node);
        if !own.is_null() {
            fill = own;
        }
    }
    fill
}

/// The **stroke** of a style, with the stroke of its base under it.
pub fn effective_stroke(document: &craftkit_sdk::Document, style: u64) -> Json {
    let mut stroke = Json::Null;
    for id in chain_of(document, style) {
        let Some(node) = document.node(id) else { continue };
        let own = stroke_of(node);
        if !own.is_null() {
            stroke = own;
        }
    }
    stroke
}

/// The **paint** an item is drawn with: its own, or its style's.
pub fn effective_paint(document: &craftkit_sdk::Document, item: &Node) -> (Json, Json) {
    let own_fill = fill_of(item);
    let own_stroke = stroke_of(item);
    let Some(style) = style_of(document, item.id) else { return (own_fill, own_stroke) };
    let fill = if own_fill.is_null() { effective_fill(document, style) } else { own_fill };
    let stroke = if own_stroke.is_null() { effective_stroke(document, style) } else { own_stroke };
    (fill, stroke)
}

/// **Add** a style to a document.
///
/// A style may be **based on** another, and the base's paint sits
/// under the style's own.
#[derive(Command)]
#[command(id = "layout.style.add", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct AddStyle {
    /// The document it goes in.
    pub document: NodeRef,
    /// What to call it.
    pub name: Text,
    /// The style it is based on.
    pub based_on: Option<NodeRef>,
}

impl AddStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.style.add".to_string();
        let document = self.document.0;
        let Some(node) = session.document().node(document).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no document `{document}`"),
            });
        };
        if node.kind != "layout.document" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{document}` is a `{}` and not a document", node.kind),
            });
        }
        let name = self.name.0.trim().to_string();
        if name.is_empty() {
            return Err(EngineError::Failed { id: id.clone(), message: "a style has a name".to_string() });
        }
        let base = self.based_on.map(|base| base.0);
        if let Some(base) = base {
            let Some(node) = session.document().node(base) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no style `{base}`"),
                });
            };
            if node.kind != STYLE_KIND {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{base}` is a `{}` and not a style", node.kind),
                });
            }
        }
        let style = session.document_mut()?.add_node_with_under(
            Some(document),
            name.clone(),
            STYLE_KIND,
            None,
            json!({}),
        );
        if let Some(base) = base {
            session.document_mut()?.add_relation(craftkit_sdk::Relation::with_payload(
                style,
                base,
                BASED_ON,
                json!({ "base": base }),
            ));
        }
        Ok(Outcome::changed(format!("the style `{name}` is added")).with_data(json!({
            "style": style,
            "document": document,
            "name": name,
        })))
    }
}

/// **Fill** a style with a colour — or take the fill away.
///
/// An empty colour takes the fill away, and a colour that is not said
/// leaves the fill as it was.
#[derive(Command)]
#[command(id = "layout.style.fill", area = "layout", layer = "layout", icon = "paint", mutates, undoable)]
pub struct SetStyleFill {
    /// The style.
    pub style: NodeRef,
    /// The colour of the fill.
    pub colour: Option<Text>,
}

impl SetStyleFill {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.style.fill".to_string();
        let style = self.style.0;
        let Some(node) = session.document().node(style).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no style `{style}`"),
            });
        };
        if node.kind != STYLE_KIND {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{style}` is a `{}` and not a style", node.kind),
            });
        }
        let fill = match &self.colour {
            Some(colour) if colour.0.trim().is_empty() => Json::Null,
            Some(colour) => match craftkit_sdk::Colour::from_hex(colour.0.trim()) {
                Ok(colour) => json!({ "colour": colour.0 }),
                Err(why) => return Err(EngineError::Failed { id: id.clone(), message: why }),
            },
            None => fill_of(&node),
        };
        if let Some(node) = session.document_mut()?.node_mut(style) {
            match &fill {
                Json::Null => {
                    if let Some(map) = node.data.as_object_mut() {
                        map.remove("fill");
                    }
                }
                fill => node.data["fill"] = fill.clone(),
            }
        }
        Ok(Outcome::changed(format!("the style `{style}` is filled")).with_data(json!({
            "style": style,
            "fill": fill,
        })))
    }
}

/// **Stroke** a style with a colour — or take the stroke away.
///
/// The colour is written `#rrggbb` and the weight is in points. A
/// stroke with no weight of its own weighs one point.
#[derive(Command)]
#[command(id = "layout.style.stroke", area = "layout", layer = "layout", icon = "paint", mutates, undoable)]
pub struct SetStyleStroke {
    /// The style.
    pub style: NodeRef,
    /// The colour of the stroke.
    pub colour: Option<Text>,
    /// The **weight** of the stroke, in points.
    pub weight: Option<Number>,
}

impl SetStyleStroke {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.style.stroke".to_string();
        let style = self.style.0;
        let Some(node) = session.document().node(style).cloned() else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no style `{style}`"),
            });
        };
        if node.kind != STYLE_KIND {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{style}` is a `{}` and not a style", node.kind),
            });
        }
        let colour = match &self.colour {
            Some(colour) if colour.0.trim().is_empty() => None,
            Some(colour) => match craftkit_sdk::Colour::from_hex(colour.0.trim()) {
                Ok(colour) => Some(colour.0),
                Err(why) => return Err(EngineError::Failed { id: id.clone(), message: why }),
            },
            None => None,
        };
        let weight = match &self.weight {
            Some(weight) => {
                let weight = weight.0;
                if !weight.is_finite() || weight < 0.0 || weight > crate::paint::MAX_WEIGHT {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "{weight} is not a weight: it is a number from zero to {}",
                            crate::paint::MAX_WEIGHT
                        ),
                    });
                }
                Some(weight)
            }
            None => None,
        };
        let stroke = match colour {
            Some(colour) => json!({
                "colour": colour,
                "weight": weight.unwrap_or(crate::paint::DEFAULT_WEIGHT),
            }),
            None if self.colour.is_some() => Json::Null,
            None => {
                let mut stroke = stroke_of(&node);
                let weight = weight.or_else(|| {
                    stroke
                        .get("weight")
                        .and_then(|weight| weight.as_f64())
                        .or(Some(crate::paint::DEFAULT_WEIGHT))
                });
                if let Some(map) = stroke.as_object_mut() {
                    map.insert("weight".to_string(), json!(weight.unwrap_or(crate::paint::DEFAULT_WEIGHT)));
                }
                stroke
            }
        };
        if let Some(node) = session.document_mut()?.node_mut(style) {
            match &stroke {
                Json::Null => {
                    if let Some(map) = node.data.as_object_mut() {
                        map.remove("stroke");
                    }
                }
                stroke => node.data["stroke"] = stroke.clone(),
            }
        }
        Ok(Outcome::changed(format!("the style `{style}` is stroked")).with_data(json!({
            "style": style,
            "stroke": stroke,
        })))
    }
}

/// **Put** an item on a style — or take it off.
///
/// The paint the item is drawn with is **derived**: its own when it
/// has one, and the style's when it does not.
#[derive(Command)]
#[command(id = "layout.item.style", area = "layout", layer = "layout", icon = "plus", mutates, undoable)]
pub struct SetItemStyle {
    /// The item.
    pub item: NodeRef,
    /// The style it is on. Without one, the item is on no style.
    pub style: Option<NodeRef>,
}

impl SetItemStyle {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "layout.item.style".to_string();
        let item = self.item.0;
        let node = item_of(session, &id, item)?;
        may_edit(&node, &id)?;
        let style = self.style.map(|style| style.0);
        if let Some(style) = style {
            let Some(node) = session.document().node(style) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no style `{style}`"),
                });
            };
            if node.kind != STYLE_KIND {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{style}` is a `{}` and not a style", node.kind),
                });
            }
        }
        session
            .document_mut()?
            .remove_relations(|relation| relation.kind == STYLED_BY && relation.from == item);
        if let Some(style) = style {
            let name = session.document().node(style).map(|node| node.name.clone());
            session.document_mut()?.add_relation(craftkit_sdk::Relation::with_payload(
                item,
                style,
                STYLED_BY,
                json!({ "style": name }),
            ));
        }
        Ok(Outcome::changed(format!("the item `{item}` is on a style")).with_data(json!({
            "item": item,
            "style": match style {
                Some(style) => json!(style),
                None => Json::Null,
            },
        })))
    }
}
