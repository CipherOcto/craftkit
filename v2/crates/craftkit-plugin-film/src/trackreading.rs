//! **Reading** a track.
//!
//! A track may be read whole: the answer is everything the track holds
//! and everything that is said about it.
//!
//! The facts of a track are its **name**, its **kind** — a video track
//! or an audio one — and whether it is **locked**.
//!
//! Three of the facts of the answer are **derived**: the **clips** on
//! the track are its children in the order they **start**, the
//! **length** of the track is the end of its last clip, and the
//! **sequence** the track is in is its place in the tree.

use craftkit_derive::Command;

use craftkit_sdk::serde_json::{Value as Json, json};
use craftkit_sdk::{EngineError, NodeRef, Outcome, Session};

/// Read a **track**.
#[derive(Command)]
#[command(id = "film.track.get", area = "film", layer = "film", icon = "list")]
pub struct ReadTrack {
    /// The track to read.
    pub track: NodeRef,
}

impl ReadTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.track.get".to_string();
        let track = self.track.0;
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed { id, message: format!("there is no track `{track}`") });
        };
        if node.kind != "film.track" {
            return Err(EngineError::Failed {
                id,
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        let kind = node.data.get("kind").cloned().unwrap_or(Json::Null);
        let locked = crate::is_locked(session, track);
        // The **clips** of the track are in the order they **start**:
        // the order of the timeline is derived, and the order of the
        // tree is the order the clips were made in.
        let mut clips: Vec<(u64, u64, u64)> = node
            .children
            .iter()
            .filter(|child| child.kind == "film.clip")
            .map(|child| {
                let start = child.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                let duration = child.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                (start, duration, child.id)
            })
            .collect();
        clips.sort_by_key(|(start, _, _)| *start);
        let ids: Vec<u64> = clips.iter().map(|(_, _, id)| *id).collect();
        // The **length** of the track is the end of its last clip.
        let length = clips.iter().map(|(start, duration, _)| start + duration).max().unwrap_or(0);
        // The **sequence** the track is in is its place in the tree.
        let sequence = session
            .document()
            .walk()
            .find(|node| node.children.iter().any(|child| child.id == track))
            .map(|node| node.id);
        Ok(Outcome::new(format!("the track `{track}`")).with_data(json!({
            "id": track,
            "name": node.name,
            "kind": kind,
            "locked": locked,
            "clips": ids,
            "length": length,
            "sequence": sequence,
        })))
    }
}
