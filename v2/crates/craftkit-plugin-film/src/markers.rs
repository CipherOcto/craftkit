//! The **markers** of a sequence.
//!
//! A marker is a **named place** on the timeline: a point, or a span,
//! with a name and a comment and a colour, and it belongs to the
//! **sequence** and not to a track. It is the tree's `Marker`, and it
//! is a thing of its own with an id of its own.
//!
//! The tree's marker carries:
//!
//! - the **place**: where it starts, and how long it is;
//! - a **name** and a **comment**;
//! - a **colour**, and the colour is one of the tree's own sixteen
//!   labels (the module `labels`);
//! - a **kind**: the tree has five of them — `Comment`, `Chapter`,
//!   `Segmentation`, `WebLink` and `FlashCue` — and the one a marker
//!   is made is `Comment`.
//!
//! # What the port found
//!
//! - **The tree's markers are the sequence's own list.** The tree
//!   keeps them in a vector on the sequence and sorts them by their
//!   place, and the marker is not a child of anything. Here a marker
//!   is a **node** under the sequence, and the place is in its
//!   payload, so that a marker is a thing in the tree and not a row
//!   in a list.
//! - **The tree's playhead commands are not here.** The tree's
//!   `markers.goNext` and `markers.goPrev` move the playhead to the
//!   next marker, and the playhead is the state of the editor and not
//!   a fact of the document. Here the markers are facts and the
//!   playhead is not a thing at all.
//!
//! # What it does not do
//!
//! - **The tree's ripple of the markers is not here.** The tree may
//!   move the markers with the clips when a ripple edit closes a gap,
//!   and that is a setting of the tree. Here a marker stays where it
//!   is when the clips around it move: it is a place the caller gave
//!   it, and nothing moves it but a command that says so.
//! - **The tree's export of the markers is not here.** The tree may
//!   write the markers of a sequence out as a CSV file. That is a
//!   file and not a fact of the document.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, Node, NodeRef, Number, Outcome, Session, Text};

/// The **kinds** of a marker, in the tree's own order.
pub const KINDS: [&str; 5] = ["Comment", "Chapter", "Segmentation", "WebLink", "FlashCue"];

/// The kind a name says, if it is one.
///
/// The tree's rule: the name is the tree's own, and the case does not
/// matter.
pub fn kind_of(name: &str) -> Option<&'static str> {
    let wanted = name.trim();
    KINDS.iter().find(|kind| kind.eq_ignore_ascii_case(wanted)).copied()
}

/// A **marker**: a named place on the timeline of a sequence.
#[derive(craftkit_derive::NodeKind)]
#[node_kind(kind = "film.marker", label = "Marker", layer = "film", evaluates)]
pub struct MarkerKind;

impl MarkerKind {
    /// A marker is not drawn.
    fn paint(_node: &Node, _document: &craftkit_sdk::Document, _canvas: &mut dyn craftkit_sdk::Canvas) {}

    /// Whether a point is inside a marker: never.
    fn hit(_node: &Node, _document: &craftkit_sdk::Document, _point: craftkit_sdk::Point) -> bool {
        false
    }

    /// What a marker **means**: where it is, what it is called and what
    /// colour it is.
    fn evaluate(node: &Node, _document: &craftkit_sdk::Document) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "start": node.data.get("start").cloned().unwrap_or(serde_json::Value::Null),
            "duration": node.data.get("duration").cloned().unwrap_or(serde_json::Value::Null),
            "name": node.data.get("name").cloned().unwrap_or(serde_json::Value::Null),
            "comment": node.data.get("comment").cloned().unwrap_or(serde_json::Value::Null),
            "color": node.data.get("color").cloned().unwrap_or(serde_json::Value::Null),
            "kind": node.data.get("kind").cloned().unwrap_or(serde_json::Value::Null),
        }))
    }
}

/// **Add** a marker to a sequence.
///
/// The marker is a named place on the timeline of the sequence.
#[derive(Command)]
#[command(id = "film.marker.add", area = "film", layer = "film", icon = "flag", mutates, undoable)]
pub struct AddMarker {
    /// The sequence the marker goes on.
    pub sequence: NodeRef,
    /// Where it starts, in frames.
    pub start: Number,
    /// How long it is, in frames.
    pub duration: Option<Number>,
    /// What to call it.
    pub name: Option<Text>,
    /// What to say about it.
    pub comment: Option<Text>,
    /// Its colour: one of the tree's own sixteen labels.
    pub color: Option<Text>,
    /// What kind of marker it is: `Comment`, `Chapter`, `Segmentation`,
    /// `WebLink` or `FlashCue`.
    pub kind: Option<Text>,
}

impl AddMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.marker.add".to_string();
        let sequence = self.sequence.0;
        let Some(node) = session.document().node(sequence) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{sequence}`"),
            });
        };
        if node.kind != "film.item" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sequence}` is a `{}` and not a sequence", node.kind),
            });
        }
        let start = frames(self.start, &id)?;
        let duration = match &self.duration {
            Some(duration) => frames(*duration, &id)?,
            None => 0,
        };
        let color = match &self.color {
            Some(color) => Some(crate::labels::label_of(&color.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no label `{}`: the labels are {}",
                    color.0.trim(),
                    crate::labels::LABELS.join(", ")
                ),
            })?),
            None => None,
        };
        let kind = match &self.kind {
            Some(kind) => Some(kind_of(&kind.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no kind `{}`: the kinds are {}", kind.0.trim(), KINDS.join(", ")),
            })?),
            None => None,
        };
        let name = self.name.as_ref().map(|name| name.0.clone()).unwrap_or_default();
        let comment = self.comment.as_ref().map(|comment| comment.0.clone()).unwrap_or_default();
        let data = serde_json::json!({
            "start": start,
            "duration": duration,
            "name": name,
            "comment": comment,
            "color": color.map(|color| serde_json::json!(color)).unwrap_or(serde_json::Value::Null),
            "kind": kind.map(|kind| serde_json::json!(kind)).unwrap_or(serde_json::json!("Comment")),
        });
        let marker = session.document_mut()?.add_node_with_under(
            Some(sequence),
            name.clone(),
            "film.marker",
            None,
            data,
        );
        Ok(Outcome::changed(format!("the marker at frame {start} is added"))
            .with_data(serde_json::json!({ "id": marker, "sequence": sequence, "start": start })))
    }
}

/// **Change** a marker.
///
/// Only what is named is changed: the name, the comment, the colour,
/// the kind and the length. A marker with no name of its own keeps
/// the name it has.
#[derive(Command)]
#[command(id = "film.marker.set", area = "film", layer = "film", icon = "pencil", mutates, undoable)]
pub struct SetMarker {
    /// The marker.
    pub marker: NodeRef,
    /// What to call it.
    pub name: Option<Text>,
    /// What to say about it.
    pub comment: Option<Text>,
    /// Its colour: one of the tree's own sixteen labels.
    pub color: Option<Text>,
    /// What kind of marker it is.
    pub kind: Option<Text>,
    /// How long it is, in frames.
    pub duration: Option<Number>,
}

impl SetMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.marker.set".to_string();
        let marker = self.marker.0;
        let Some(node) = session.document().node(marker) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{marker}`"),
            });
        };
        if node.kind != "film.marker" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{marker}` is a `{}` and not a marker", node.kind),
            });
        }
        let color = match &self.color {
            Some(color) => Some(crate::labels::label_of(&color.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "there is no label `{}`: the labels are {}",
                    color.0.trim(),
                    crate::labels::LABELS.join(", ")
                ),
            })?),
            None => None,
        };
        let kind = match &self.kind {
            Some(kind) => Some(kind_of(&kind.0).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no kind `{}`: the kinds are {}", kind.0.trim(), KINDS.join(", ")),
            })?),
            None => None,
        };
        let duration = match &self.duration {
            Some(duration) => Some(frames(*duration, &id)?),
            None => None,
        };
        let name = self.name.as_ref().map(|name| name.0.clone());
        let comment = self.comment.as_ref().map(|comment| comment.0.clone());
        let Some(node) = session.document_mut()?.node_mut(marker) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{marker}`"),
            });
        };
        if let Some(name) = name {
            node.data["name"] = serde_json::json!(name);
            node.name = name;
        }
        if let Some(comment) = comment {
            node.data["comment"] = serde_json::json!(comment);
        }
        if let Some(color) = color {
            node.data["color"] = serde_json::json!(color);
        }
        if let Some(kind) = kind {
            node.data["kind"] = serde_json::json!(kind);
        }
        if let Some(duration) = duration {
            node.data["duration"] = serde_json::json!(duration);
        }
        Ok(Outcome::changed(format!("the marker `{marker}` is changed"))
            .with_data(serde_json::json!({ "id": marker })))
    }
}

/// **Take** a marker away.
#[derive(Command)]
#[command(id = "film.marker.remove", area = "film", layer = "film", icon = "trash", mutates, undoable)]
pub struct RemoveMarker {
    /// The marker.
    pub marker: NodeRef,
}

impl RemoveMarker {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.marker.remove".to_string();
        let marker = self.marker.0;
        let Some(node) = session.document().node(marker) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{marker}`"),
            });
        };
        if node.kind != "film.marker" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{marker}` is a `{}` and not a marker", node.kind),
            });
        }
        if !session.document_mut()?.remove_node(marker) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{marker}`"),
            });
        }
        Ok(Outcome::changed(format!("the marker `{marker}` is taken away")).with_data(serde_json::json!({
            "id": marker,
        })))
    }
}

/// **Take away** every marker of a sequence.
#[derive(Command)]
#[command(id = "film.marker.clear", area = "film", layer = "film", icon = "trash", mutates, undoable)]
pub struct ClearMarkers {
    /// The sequence.
    pub sequence: NodeRef,
}

impl ClearMarkers {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.marker.clear".to_string();
        let sequence = self.sequence.0;
        let Some(node) = session.document().node(sequence) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{sequence}`"),
            });
        };
        if node.kind != "film.item" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{sequence}` is a `{}` and not a sequence", node.kind),
            });
        }
        let markers: Vec<u64> =
            node.children.iter().filter(|child| child.kind == "film.marker").map(|child| child.id).collect();
        for marker in &markers {
            if !session.document_mut()?.remove_node(*marker) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{marker}`"),
                });
            }
        }
        Ok(Outcome::changed(format!("the markers of the sequence `{sequence}` are taken away"))
            .with_data(serde_json::json!({ "sequence": sequence, "markers": markers })))
    }
}

/// A number of frames.
fn frames(value: Number, id: &str) -> Result<u64, EngineError> {
    if !value.0.is_finite() || value.0 < 0.0 || value.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: format!("a number of frames is a whole number that is there: {}", value.0),
        });
    }
    Ok(value.0 as u64)
}
