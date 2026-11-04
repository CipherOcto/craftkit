//! **Taking away** a track.
//!
//! A track may be taken away from its sequence, and the clips on it go
//! with it: they are its children, and a child goes with its parent.
//!
//! A **locked** track may not be taken away: a locked track is a track
//! that cannot be edited, and the tree's locked track refuses every
//! edit — including the taking away of the track itself.
//!
//! # What the port found
//!
//! - **The tree's track is a lane and this tree's track is a node.** The
//!   tree holds its tracks in two lists — the video tracks and the
//!   audio tracks — and takes one away by dropping it from its list.
//!   Here a track is a **node** under the sequence, and the clips on it
//!   are its **children**, so taking the track away takes the clips
//!   with it and the tree of the document says so by itself.
//! - **The tree's lock is checked by the tree's edit.** The tree's
//!   `edit_sequence` asks for the track of every edit and refuses a
//!   locked one. Here every command that edits a clip asks for the
//!   track of the clip, and this command asks for the track it is
//!   taking away.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// **Take away** a track.
///
/// The track is gone, and the clips that were on it go with it.
#[derive(Command)]
#[command(id = "film.track.remove", area = "film", layer = "film", icon = "minus", mutates, undoable)]
pub struct RemoveTrack {
    /// The track.
    pub track: NodeRef,
}

impl RemoveTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.track.remove".to_string();
        let track = self.track.0;
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no track `{track}`"),
            });
        };
        if node.kind != "film.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        // A **locked** track cannot be edited, and taking it away is an
        // edit.
        if crate::is_locked(session, track) {
            return Err(crate::locked_error(&id, track));
        }
        let name = node.name.clone();
        let clips: Vec<u64> =
            node.children.iter().filter(|child| child.kind == "film.clip").map(|child| child.id).collect();
        // The node and its children go with it: the clips and the edges
        // that say which item they played.
        if !session.document_mut()?.remove_node(track) {
            return Err(EngineError::Failed { id, message: format!("there is no track `{track}`") });
        }
        Ok(Outcome::changed(format!("the track `{name}` is gone")).with_data(serde_json::json!({
            "track": track,
            "name": name,
            "clips": clips,
        })))
    }
}
