//! The **label** of a clip.
//!
//! A label is a **colour** and not a number: the tree's own sixteen
//! names, in the tree's own order. A clip carries one, or none.
//!
//! The tree's finding is that a label is a **group**: the tree's
//! *select label group* command takes every clip that carries one of
//! the labels of the clips that are selected, and selects them. So the
//! group is **derived** from the labels and not written down, and it
//! changes when a label is put on a clip and not when it is taken off.
//!
//! And the tree's finding is that a label with **no name** is not a
//! label: the tree's list is closed, and a name that is not on the
//! list is refused — the tree's own rule, and a colour that the tree
//! does not have is not a colour.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session, Text};

/// The **labels**, in the tree's own order, with the tree's own names.
pub const LABELS: [&str; 16] = [
    "Violet",
    "Iris",
    "Caribbean",
    "Lavender",
    "Cerulean",
    "Forest",
    "Rose",
    "Mango",
    "Purple",
    "Blue",
    "Teal",
    "Magenta",
    "Tan",
    "Green",
    "Brown",
    "Yellow",
];

/// The label a name says, if it is one.
///
/// The tree's rule: the name is the tree's own, and the case does not
/// matter.
pub fn label_of(name: &str) -> Option<&'static str> {
    let wanted = name.trim();
    LABELS.iter().find(|label| label.eq_ignore_ascii_case(wanted)).copied()
}

/// Put a **label** on a clip.
///
/// The label is one of the tree's own sixteen, and a name that is not
/// one of them is refused.
#[derive(Command)]
#[command(id = "film.clip.label", area = "film", layer = "film", icon = "paint", mutates, undoable)]
pub struct SetLabel {
    /// The clip.
    pub clip: NodeRef,
    /// The label: `Violet`, `Iris`, …
    pub label: Text,
}

impl SetLabel {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.label".to_string();
        let clip = self.clip.0;
        let wanted = self.label.0.clone();
        let label = label_of(&wanted).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("there is no label `{wanted}`: the labels are {}", LABELS.join(", ")),
        })?;
        let Some(node) = session.document().node(clip) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no clip `{clip}`"),
            });
        };
        if node.kind != "film.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
            });
        }
        let mut data = node.data.clone();
        data["label"] = serde_json::json!(label);
        let Some(node) = session.document_mut()?.node_mut(clip) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no clip `{clip}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the clip `{clip}` is labelled `{label}`"))
            .with_data(serde_json::json!({ "clip": clip, "label": label })))
    }
}

/// Take the **label** off a clip.
///
/// A clip with no label is in no group, and that is the tree's own: a
/// clip that carries no colour is in no colour group.
#[derive(Command)]
#[command(id = "film.clip.label.delete", area = "film", layer = "film", icon = "minus", mutates, undoable)]
pub struct DeleteLabel {
    /// The clip.
    pub clip: NodeRef,
}

impl DeleteLabel {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.label.delete".to_string();
        let clip = self.clip.0;
        let Some(node) = session.document().node(clip) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no clip `{clip}`"),
            });
        };
        if node.kind != "film.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
            });
        }
        let had = node.data.get("label").and_then(|label| label.as_str()).map(|label| label.to_string());
        let mut data = node.data.clone();
        data["label"] = serde_json::Value::Null;
        let Some(node) = session.document_mut()?.node_mut(clip) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no clip `{clip}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the clip `{clip}` is in no group"))
            .with_data(serde_json::json!({ "clip": clip, "label": had })))
    }
}

/// The **group** of a clip: every clip that carries its label.
///
/// The group is **derived**: it changes when a label is put on a clip
/// and not when it is written down.
#[derive(Command)]
#[command(id = "film.clip.label.group", area = "film", layer = "film", icon = "list")]
pub struct LabelGroup {
    /// The clip whose label makes the group.
    pub clip: NodeRef,
}

impl LabelGroup {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.label.group".to_string();
        let mut wanted: Vec<String> = Vec::new();
        {
            let clip = self.clip.0;
            let Some(node) = session.document().node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no clip `{clip}`"),
                });
            };
            if node.kind != "film.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            if let Some(label) = node.data.get("label").and_then(|label| label.as_str()) {
                wanted.push(label.to_string());
            }
        }
        let group: Vec<u64> = session
            .document()
            .walk()
            .filter(|node| node.kind == "film.clip")
            .filter(|node| {
                node.data
                    .get("label")
                    .and_then(|label| label.as_str())
                    .map(|label| wanted.iter().any(|name| name == label))
                    .unwrap_or(false)
            })
            .map(|node| node.id)
            .collect();
        Ok(Outcome::new(format!("the group has {} clip(s)", group.len()))
            .with_data(serde_json::json!({ "labels": wanted, "clips": group })))
    }
}
