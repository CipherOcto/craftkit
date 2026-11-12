//! The **enable** of a clip.
//!
//! A clip may be **disabled**: it is on the track and it takes its
//! place there, and it does not **play**. The tree's clip has the flag
//! and the tree's canvas skips a clip that is not enabled — the clip
//! is there and it is not drawn.
//!
//! So the flag is a **fact** of the clip, and what plays is **derived**:
//! the timeline says which clips are enabled and how many of them
//! play, and it is the derivation that says the flag has a meaning.
//!
//! # What the port found
//!
//! - **The tree flips the flag and this tree sets it.** The tree's
//!   `clip.enable` inverts the flag of every clip of the selection, so
//!   a selection that holds a mix of enabled and disabled clips ends
//!   up with the mix **inverted** and not normalised. Here the command
//!   takes the clip and says whether it plays, and a command that says
//!   nothing is a command that says nothing — so the flag is set to
//!   what the caller asked for, and the command says which way it went.
//! - **The tree's track has an enable flag too**, and a disabled track
//!   plays nothing at all. Here a track's kind is in its payload and
//!   its clips are its children, and there is no flag on the track —
//!   a clip that does not play is a clip that does not play, and the
//!   track is not a thing that plays.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome};

/// Whether a clip **plays**.
pub fn is_enabled(data: &serde_json::Value) -> bool {
    data.get("enabled").and_then(|enabled| enabled.as_bool()).unwrap_or(true)
}

commands! {
    /// **Enable** a clip, or disable it.
    ///
    /// Without the `on` the flag is **toggled**, and the answer says which
    /// way it went.
    film.clip.enable {
        area: "film",
        layer: "film",
        icon: "power",
        mutates,
        undoable,
        params: {
            /// The clip.

            clip: NodeRef,
            /// Whether the clip plays. Without it the flag is toggled.

            on: Option<Bool>,
        },
        run: {

            let id = "film.clip.enable".to_string();
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
            let was = is_enabled(&node.data);
            let now = self.on.map(|on| on.0).unwrap_or(!was);
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data["enabled"] = serde_json::json!(now);
            }
            let message =
                if now { format!("the clip `{clip}` plays") } else { format!("the clip `{clip}` does not play") };
            Ok(Outcome::changed(message).with_data(serde_json::json!({ "clip": clip, "enabled": now })))

        }
    }
}
