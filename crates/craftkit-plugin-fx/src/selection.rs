//! The **selection** of layers.
//!
//! The selection is what a person has picked out, or what an agent
//! has named. It is the **session's** own state and not a part of the
//! document — the document is the truth, and the selection is of the
//! document.
//!
//! The tree's rules, and they are the finding:
//!
//! - the tree's selection is a **list of layers**, and a layer that
//!   is not in the document is not selected;
//! - the tree's `deselect` **takes the layers out** of the selection
//!   and does not take the selection away, and the tree's `select`
//!   **replaces** it;
//! - the tree's `selectNext` and `selectPrevious` walk the stack —
//!   the layer above and the layer below the one that is selected.
//!
//! Here a command selects **layers**, and the stack that the tree
//! walks is the stack of the composition (§ §431).

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{Bool, EngineError, Outcome, Session};

/// The **layers** a value holds: a list of ids.
fn layers_of(value: &Json) -> Vec<u64> {
    value
        .as_array()
        .map(|layers| layers.iter().filter_map(|layer| layer.as_u64()).collect())
        .unwrap_or_default()
}

/// **Select** layers.
#[derive(Command)]
#[command(id = "fx.layer.select", area = "fx", layer = "fx", icon = "select")]
pub struct SelectLayers {
    /// The layers: their ids.
    pub layers: Json,
}

impl SelectLayers {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.layer.select".to_string();
        let layers = layers_of(&self.layers);
        for layer in &layers {
            let Some(node) = session.document().node(*layer) else {
                return Err(EngineError::Failed { id, message: format!("there is no layer `{layer}`") });
            };
            if node.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
                });
            }
        }
        session.select_ids(&layers);
        Ok(Outcome::changed(format!("selected {} layer(s)", layers.len()))
            .with_data(json!({ "layers": layers })))
    }
}

/// **Take** layers **out** of the selection.
#[derive(Command)]
#[command(id = "fx.layer.deselect", area = "fx", layer = "fx", icon = "select")]
pub struct DeselectLayers {
    /// The layers: their ids.
    pub layers: Json,
}

impl DeselectLayers {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "fx.layer.deselect".to_string();
        let layers = layers_of(&self.layers);
        for layer in &layers {
            let Some(node) = session.document().node(*layer) else {
                return Err(EngineError::Failed { id, message: format!("there is no layer `{layer}`") });
            };
            if node.kind != "fx.layer" {
                return Err(EngineError::Failed {
                    id,
                    message: format!("the node `{layer}` is a `{}` and not a layer", node.kind),
                });
            }
        }
        let kept: Vec<u64> =
            session.selected_ids().into_iter().filter(|selected| !layers.contains(selected)).collect();
        session.select_ids(&kept);
        Ok(Outcome::changed(format!("took {} layer(s) out of the selection", layers.len()))
            .with_data(json!({ "layers": layers })))
    }
}

/// **Read** the selection.
#[derive(Command)]
#[command(id = "fx.selection.get", area = "fx", layer = "fx", icon = "list")]
pub struct GetSelection {
    /// Whether to say which layers they are and not only how many.
    pub detailed: Option<Bool>,
}

impl GetSelection {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let layers = session.selected_ids();
        let detailed = self.detailed.map(|detailed| detailed.0).unwrap_or(false);
        let named: Vec<Json> = if detailed {
            layers
                .iter()
                .filter_map(|layer| {
                    let node = session.document().node(*layer)?;
                    Some(json!({ "id": node.id, "name": node.name, "kind": node.kind }))
                })
                .collect()
        } else {
            Vec::new()
        };
        Ok(Outcome::new(format!("{} thing(s) are selected", layers.len()))
            .with_data(json!({ "layers": layers, "selected": named })))
    }
}
