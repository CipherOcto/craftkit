//! The **dimension** of a drawing.
//!
//! A dimension **measures** an entity: it says how long the entity
//! is, and it says it where the reader can see it.
//!
//! The **measurement** is derived: it is worked out from the geometry
//! of the entity every time, so a dimension cannot be out of date. And
//! the **text** is derived from the measurement, unless the dimension
//! carries a **text of its own** — the tree's own: a dimension may
//! say something other than what it measures, and when it says
//! nothing of its own it says the measurement.
//!
//! The reference to the measured entity is an **edge** — a dimension
//! and the entity it measures are two things, and the edge between
//! them says which — the same bargain as the insert and the block it
//! inserts.
//!
//! The rules:
//!
//! - a dimension measures the **length** of an entity, and an entity
//!   that has no length — a text, an insert — is refused;
//! - the **text** a dimension carries of its own is said with a
//!   command, and an **empty** text takes it away: the measurement
//!   shows again.

use craftkit_derive::{NodeKind, commands};

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome, Relation, Text};

/// The **edge** that says a dimension measures an entity.
const MEASURES: &str = "cad.measures";

/// The **decimals** a measurement is written with: the tree's own.
const DECIMALS: usize = 3;

/// A **dimension**: an entity that measures another one.
#[derive(NodeKind)]
#[node_kind(kind = "cad.entity.dimension", label = "Dimension", layer = "cad", evaluates)]
pub struct DimensionKind;

impl DimensionKind {
    /// Not drawn: a dimension is a fact here, and the drawing of a
    /// dimension needs a witness line and an arrowhead.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Not hit.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a dimension **means**: what it measures, how much, and
    /// what it says.
    fn evaluate(node: &Node, document: &Document) -> Option<Json> {
        let measured = measured_entity(document, node);
        let value = measured.and_then(length_of);
        Some(json!({
            "geometry": node.data.get("geometry").cloned().unwrap_or(Json::Null),
            "measures": measured.map(|entity| Json::String(entity.name.clone())).unwrap_or(Json::Null),
            "value": value.map(|value| json!(value)).unwrap_or(Json::Null),
            "text": Json::String(text_of(node, value)),
        }))
    }
}

/// The **entity** a dimension measures, if it is there.
pub fn measured_entity<'a>(document: &'a Document, dimension: &Node) -> Option<&'a Node> {
    let relations = document.relations_from(dimension.id, Some(MEASURES));
    let relation = relations.first()?;
    document.node(relation.to)
}

/// The **length** of an entity, if it has one.
fn length_of(entity: &Node) -> Option<f64> {
    let geometry = entity.data.get("geometry")?;
    crate::measurements::length_of(&entity.kind, geometry)
}

/// The **text** of a dimension: its own if it has one, and the
/// measurement if it has not.
pub fn text_of(dimension: &Node, value: Option<f64>) -> String {
    if let Some(text) = dimension.data.get("text").and_then(|text| text.as_str()) {
        return text.to_string();
    }
    match value {
        Some(value) => format!("{value:.precision$}", precision = DECIMALS),
        None => String::new(),
    }
}

commands! {
    /// **Add** a dimension to a block.
    ///
    /// The dimension measures the **length** of an entity, and an entity
    /// that has no length is refused.
    cad.dimension.add {
        area: "cad",
        layer: "cad",
        icon: "ruler",
        mutates,
        undoable,
        params: {
            /// The block it goes in.
            block: NodeRef,
            /// What to call it.
            name: Text,
            /// The entity it **measures**.
            entity: NodeRef,
            /// Where it is written.
            at: Json,
        },
        run: {
            let id = "cad.dimension.add".to_string();
            let measured = self.entity.0;
            let Some(entity) = session.document().node(measured).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no entity `{measured}`"),
                });
            };
            if !entity.kind.starts_with("cad.entity.") {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{measured}` is a `{}` and not an entity", entity.kind),
                });
            }
            if length_of(&entity).is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "a dimension measures the length of an entity, and a `{}` has no length",
                        entity.kind
                    ),
                });
            }
            let (x, y) = crate::point(&self.at, &id, "the insertion point")?;
            let geometry = json!({ "at": { "x": x, "y": y } });
            let name = self.name.0.trim().to_string();
            let dimension =
                crate::add_entity(session, self.block.0, &name, "cad.entity.dimension", geometry, &id)?;
            session.document_mut()?.add_relation(Relation::with_payload(
                dimension,
                measured,
                MEASURES,
                json!({ "measures": entity.name.clone() }),
            ));
            let value = length_of(&entity);
            Ok(Outcome::changed(format!("added the dimension `{name}`"))
                .with_data(json!({ "id": dimension, "name": name, "measures": measured, "value": value })))

        }
    }

    /// **Say** what a dimension says of its own.
    ///
    /// A text that is **empty** takes the text away, and the measurement
    /// shows again.
    cad.dimension.text {
        area: "cad",
        layer: "cad",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The dimension.
            dimension: NodeRef,
            /// What it says. Without it the measurement shows.
            text: Text,
        },
        run: {
            let id = "cad.dimension.text".to_string();
            let dimension = self.dimension.0;
            let Some(node) = session.document().node(dimension).cloned() else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no dimension `{dimension}`"),
                });
            };
            if node.kind != "cad.entity.dimension" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{dimension}` is a `{}` and not a dimension", node.kind),
                });
            }
            let text = self.text.0.clone();
            if let Some(node) = session.document_mut()?.node_mut(dimension) {
                if text.is_empty() {
                    node.data.as_object_mut().map(|data| data.remove("text"));
                } else {
                    node.data["text"] = json!(text.clone());
                }
            }
            Ok(Outcome::changed(format!("the dimension `{dimension}` says `{text}`"))
                .with_data(json!({ "dimension": dimension, "text": text })))

        }
    }
}
