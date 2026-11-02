//! **Taking away** a clip.
//!
//! A clip may be taken off its track. What goes with it goes with it:
//! the edge that says which item the clip plays dies with the node, and
//! the **gap** the clip leaves is a gap — the clips that were after it
//! stay where they are, and the track is shorter by the length of the
//! clip that went.
//!
//! The tree has two ways of taking a clip away: the **delete**, which
//! leaves the gap, and the **ripple delete**, which closes it. The
//! delete is here, and the ripple is a command of its own.
//!
//! # What it does not do
//!
//! - **The gap is not closed.** The tree's `ripple_delete_items`
//!   closes the gap and refuses when it cannot: "no gap can close: the
//!   other clips on these tracks cover the deleted time".
//! - **The tree's links and transitions are not here**, and neither is
//!   the tree's **linked** clip, which the tree removes with its
//!   partner.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// **Take away** a clip.
///
/// The clip is gone, the edge that says which item it played goes with
/// it, and the gap it leaves is a gap.
#[derive(Command)]
#[command(id = "film.clip.remove", area = "film", layer = "film", icon = "minus", mutates, undoable)]
pub struct RemoveClip {
    /// The clip.
    pub clip: NodeRef,
}

impl RemoveClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.remove".to_string();
        let clip = self.clip.0;
        let Some(node) = session.document().node(clip) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            });
        };
        if node.kind != "film.clip" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
            });
        }
        let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
        let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
        if !session.document_mut()?.remove_node(clip) {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            });
        }
        Ok(Outcome::changed(format!("the clip `{clip}` is gone"))
            .with_data(serde_json::json!({ "id": clip, "start": start, "duration": duration })))
    }
}
