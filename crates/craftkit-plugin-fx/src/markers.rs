//! The **markers** of a composition.
//!
//! A marker is a moment in a composition with something said about it:
//! the tree's marker is a time and a comment, and the tree's markers
//! are **sorted by time** — the tree's command sorts them as it adds
//! one.
//!
//! The tree's marker has more in it — a label, a chapter, a link, and
//! a duration for the protected ones that set the timing of a
//! responsive composition — and only the time and the comment are
//! here.

use craftkit_derive::{Evaluator, NodeKind, commands};

use craftkit_sdk::serde_json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Number, Outcome, Plan, PlanItem, Text};

/// The kind of a **marker**.
#[derive(NodeKind)]
#[node_kind(kind = "fx.marker", label = "Marker", layer = "fx", evaluates)]
pub struct MarkerKind;

impl MarkerKind {
    /// A marker is not drawn.
    fn paint(_node: &Node, _document: &Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a marker: never.
    fn hit(_node: &Node, _document: &Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a marker **means**: where it is, and what it says.
    fn evaluate(node: &Node, _document: &Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "frame": node.data.get("frame").cloned().unwrap_or(serde_json::Value::Null),
            "comment": node.data.get("comment").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }
}

/// The **payload** of a marker: a frame and a comment.
#[derive(Debug, Clone, PartialEq)]
pub struct MarkerPayload {
    /// Where the marker is, in frames.
    pub frame: u64,
    /// What it says.
    pub comment: String,
}

impl MarkerPayload {
    /// The payload as it is written.
    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "frame": self.frame,
            "comment": self.comment,
        })
    }

    /// The payload as it is read.
    pub fn from_json(value: &serde_json::Value) -> Result<MarkerPayload, String> {
        Ok(MarkerPayload {
            frame: value.get("frame").and_then(|frame| frame.as_u64()).unwrap_or(0),
            comment: value.get("comment").and_then(|comment| comment.as_str()).unwrap_or("").to_string(),
        })
    }
}

/// The **markers** of a composition, in the order they stand: sorted
/// by time, as the tree's command sorts them.
///
/// The order is **derived**: it changes when a marker moves, and
/// nobody writes it down.
pub fn markers_of(document: &Document, comp: u64) -> Vec<u64> {
    let Some(node) = document.node(comp) else { return Vec::new() };
    let mut markers: Vec<(u64, u64)> = node
        .children
        .iter()
        .filter(|child| child.kind == "fx.marker")
        .filter_map(|child| {
            let payload = MarkerPayload::from_json(&child.data).ok()?;
            Some((payload.frame, child.id))
        })
        .collect();
    markers.sort_by_key(|(frame, _)| *frame);
    markers.into_iter().map(|(_, id)| id).collect()
}

commands! {
    /// Put a **marker** in a composition.
    ///
    /// The marker is a frame and a comment, and the comment may be empty:
    /// the tree's comment is empty when nobody said anything.
    fx.marker.add {
        area: "fx",
        layer: "fx",
        icon: "plus",
        mutates,
        undoable,
        params: {
            /// The composition.
            comp: NodeRef,
            /// Where the marker is, in frames.
            frame: Number,
            /// What it says.
            comment: Option<Text>,
        },
        run: {
            let id = "fx.marker.add".to_string();
            let comp = self.comp.0;
            let Some(node) = session.document().node(comp) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no composition `{comp}`"),
                });
            };
            if node.kind != "fx.comp" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{comp}` is a `{}` and not a composition", node.kind),
                });
            }
            let frame = crate::frames(self.frame, "fx.marker.add")?;
            let comment = self.comment.as_ref().map(|comment| comment.0.clone()).unwrap_or_default();
            let payload = MarkerPayload { frame, comment: comment.clone() };
            let name = if comment.is_empty() { format!("marker {frame}") } else { comment.clone() };
            let marker = session.document_mut()?.add_node_with_under(
                Some(comp),
                name.clone(),
                "fx.marker",
                None,
                payload.to_json(),
            );
            Ok(Outcome::changed(format!("added a marker at frame {frame}"))
                .with_data(serde_json::json!({ "id": marker, "comp": comp, "frame": frame, "comment": comment })))

        }
    }
    /// Change a **marker**: where it is, or what it says.
    fx.marker.set {
        area: "fx",
        layer: "fx",
        icon: "pencil",
        mutates,
        undoable,
        params: {
            /// The marker to change.
            marker: NodeRef,
            /// Its new place, in frames.
            frame: Option<Number>,
            /// Its new comment.
            comment: Option<Text>,
        },
        run: {
            let id = "fx.marker.set".to_string();
            let marker = self.marker.0;
            let Some(node) = session.document().node(marker) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no marker `{marker}`"),
                });
            };
            if node.kind != "fx.marker" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{marker}` is a `{}` and not a marker", node.kind),
                });
            }
            let mut payload = MarkerPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if let Some(frame) = self.frame {
                payload.frame = crate::frames(frame, "fx.marker.set")?;
            }
            if let Some(comment) = &self.comment {
                payload.comment = comment.0.clone();
            }
            let name = if payload.comment.is_empty() { node.name.clone() } else { payload.comment.clone() };
            let Some(node) = session.document_mut()?.node_mut(marker) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no marker `{marker}`"),
                });
            };
            node.name = name.clone();
            node.data = payload.to_json();
            Ok(Outcome::changed(format!("changed the marker `{name}`")).with_data(
                serde_json::json!({ "id": marker, "frame": payload.frame, "comment": payload.comment }),
            ))

        }
    }
}

/// The evaluator that says which **markers** a composition has.
///
/// The order is **derived**: the markers stand in the order of their
/// time, as the tree's command sorts them, and the order changes when
/// a marker moves.
#[derive(Evaluator)]
#[evaluator(id = "fx.markers", label = "Markers", layer = "fx")]
pub struct MarkersEvaluator;

impl MarkersEvaluator {
    /// List the markers of every composition.
    fn evaluate(document: &Document) -> Plan {
        let mut plan = Plan::new();
        for comp in document.walk().filter(|node| node.kind == "fx.comp") {
            let markers: Vec<serde_json::Value> = markers_of(document, comp.id)
                .into_iter()
                .filter_map(|id| document.node(id))
                .filter_map(|node| {
                    let payload = MarkerPayload::from_json(&node.data).ok()?;
                    Some(serde_json::json!({
                        "id": node.id,
                        "name": node.name,
                        "frame": payload.frame,
                        "comment": payload.comment,
                    }))
                })
                .collect();
            plan.items.push(PlanItem {
                node: comp.id,
                kind: "fx.markers".to_string(),
                value: serde_json::json!({ "markers": markers }),
            });
        }
        plan
    }
}
