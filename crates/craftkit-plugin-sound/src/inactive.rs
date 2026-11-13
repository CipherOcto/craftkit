//! The **voice** of a track: whether it is heard at all.
//!
//! The tree's track carries a **voice**: the track's header says "dyn"
//! or "off", and "off" makes the track **inactive**. An inactive track
//! is not heard — not in the mix, not on a bus and not as a stem — and
//! it is not gone: it is in the document, with its clips and its
//! settings, and a command can make it play again.
//!
//! The tree's commit makes the old track inactive and hidden when it
//! writes the track's sound to a file (§ the commit of a track). The
//! **hidden** is not here: this tree draws no track headers, and a
//! thing that nothing draws and nothing reads is not a fact.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome};

commands! {
    /// **Make** a track inactive, or make it play again.
    ///
    /// An inactive track is not heard: it is not in the mix, not on a bus
    /// and not a stem. The track itself is not touched — its clips and its
    /// settings are where they were — and making it active again brings
    /// it back exactly as it was.
    sound.track.inactive {
        area: "sound",
        layer: "sound",
        icon: "eye",
        mutates,
        undoable,
        menu: ("Track", "Make Inactive"),
        params: {
            /// The track.
            track: NodeRef,
            /// Whether the track is inactive.
            inactive: Bool,
        },
        run: {
            let id = self.track.0;
            let Some(node) = session.document().node(id) else {
                return Err(EngineError::Failed {
                    id: "sound.track.inactive".to_string(),
                    message: format!("there is no node `{id}`"),
                });
            };
            if node.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: "sound.track.inactive".to_string(),
                    message: format!("the node `{id}` is a `{}` and not a track", node.kind),
                });
            }
            let inactive = self.inactive.0;
            let Some(node) = session.document_mut()?.node_mut(id) else {
                return Err(EngineError::Failed {
                    id: "sound.track.inactive".to_string(),
                    message: format!("there is no node `{id}`"),
                });
            };
            let mut payload = crate::TrackPayload::from_json(&node.data).map_err(|error| {
                EngineError::Failed { id: "sound.track.inactive".to_string(), message: error }
            })?;
            payload.inactive = inactive;
            node.data = payload.to_json();
            Ok(Outcome::changed(format!("the track `{id}` is inactive: {inactive}"))
                .with_data(json!({ "id": id, "inactive": inactive })))

        }
    }
}
