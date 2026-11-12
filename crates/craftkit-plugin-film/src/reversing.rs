//! **Reversing** a clip.
//!
//! A clip may play its media **backwards**. The tree's clip has a
//! `reverse` flag, and the flag is not a property of the media — the
//! media is the same — it is a property of the **direction** the clip
//! plays in, and it changes what the clip's **edges** do.
//!
//! The tree's `move_edge` is where the tree says it:
//!
//! - a clip that plays forwards consumes the media **after** the
//!   clip's in point, so trimming its in edge moves its source in
//!   point **forward**;
//! - a clip that plays backwards consumes the media **before** its out
//!   point, so trimming its out edge moves its source in point
//!   **backwards**.
//!
//! Here a clip's payload carries the flag, and the trim and the split
//! read it.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// **Reverse** a clip: play its media backwards.
///
/// The media is not touched and the clip does not move: the clip plays
/// the same media in the other direction, and its edges change what
/// they consume.
#[derive(Command)]
#[command(id = "film.clip.reverse", area = "film", layer = "film", icon = "repeat", mutates, undoable)]
pub struct ReverseClip {
    /// The clip.
    pub clip: NodeRef,
}

impl ReverseClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.reverse".to_string();
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
        let reversed = node.data.get("reverse").and_then(|reverse| reverse.as_bool()).unwrap_or(false);
        let mut data = node.data.clone();
        data["reverse"] = serde_json::json!(!reversed);
        if let Some(node) = session.document_mut()?.node_mut(clip) {
            node.data = data;
        }
        let message = if reversed {
            format!("the clip {clip} plays forwards now")
        } else {
            format!("the clip {clip} plays backwards now")
        };
        Ok(Outcome::changed(message).with_data(serde_json::json!({ "clip": clip, "reverse": !reversed })))
    }
}
