//! The **frozen** track.
//!
//! The tree's freeze sets a flag on a track, and the flag says that
//! the track is **frozen**. That is all it says: the tree's engine
//! reads the flag nowhere else, and the only thing that shows it is
//! the track's header, which draws a frozen track with the face of a
//! bypassed one.
//!
//! So the tree's freeze **freezes nothing** — and the honest port is
//! the flag itself, as a fact of the track. What a frozen track means
//! to the mix, and what a person may do to it, is the tree's business
//! and not here.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome};

use crate::TrackPayload;

commands! {
    /// **Freeze** a track, or let it be.
    ///
    /// The tree's freeze is a flag and not a behaviour, and this one is the
    /// same: the track says it is frozen, and the tree's engine does
    /// nothing else with it.
    sound.track.freeze {
        area: "sound",
        layer: "sound",
        icon: "lock",
        mutates,
        undoable,
        menu: ("Track", "Freeze"),
        params: {
            /// The track.

            track: NodeRef,
            /// Whether it is frozen.

            frozen: Bool,
        },
        run: {

            let id = "sound.track.freeze".to_string();
            let track = self.track.0;
            let frozen = self.frozen.0;
            let Some(node) = session.document().node(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{track}`"),
                });
            };
            if node.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{track}` is a `{}` and not a track", node.kind),
                });
            }
            let mut payload = TrackPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            payload.frozen = frozen;
            let data = payload.to_json();
            let Some(node) = session.document_mut()?.node_mut(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{track}`"),
                });
            };
            node.data = data;
            Ok(Outcome::changed(format!("the track `{track}` is frozen: {frozen}"))
                .with_data(json!({ "id": track, "frozen": frozen })))

        }
    }
}
