//! **Locking** a track.
//!
//! A locked track cannot be **edited**: no clip on it may be trimmed,
//! moved, split, taken away or rippled, and no clip may be put on it.
//!
//! The tree's locked track, and the tree's refusal: the tree's
//! `unlocked_track` is asked for the track of every clip that moves,
//! and a locked track is the error `Locked`.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome};

commands! {
    /// **Lock** a track, or unlock it.
    ///
    /// A locked track is a track that cannot be edited: the clips on it
    /// keep their places and their lengths, and nothing may be put on it.
    film.track.lock {
        area: "film",
        layer: "film",
        icon: "lock",
        mutates,
        undoable,
        params: {
            /// The track.
            track: NodeRef,
            /// Whether the track is locked.
            locked: Bool,
        },
        run: {
            let id = "film.track.lock".to_string();
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
            let mut data = node.data.clone();
            data["locked"] = serde_json::json!(self.locked.0);
            if let Some(node) = session.document_mut()?.node_mut(track) {
                node.data = data;
            }
            let message = if self.locked.0 {
                format!("the track `{track}` is locked")
            } else {
                format!("the track `{track}` is unlocked")
            };
            Ok(Outcome::changed(message)
                .with_data(serde_json::json!({ "track": track, "locked": self.locked.0 })))

        }
    }
}
