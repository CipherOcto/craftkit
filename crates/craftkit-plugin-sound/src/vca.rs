//! The **VCA** of a track.
//!
//! A **VCA master** is a track that carries no sound of its own: its
//! fader **offsets** the faders of the tracks assigned to it, and its
//! **mute** mutes them. It is a second fader over a group, and it is
//! the tree's own.
//!
//! The tree's finding is that the VCA is **not** a group: a group
//! shares a mute and a solo, and a VCA shares neither — it adds its
//! own level to the members and it silences them when it is muted.
//!
//! The tree's VCA master is a **kind** of track, and the kinds of a
//! track are open strings here, so `vca` is one of them. A VCA master
//! holds no clips and is not in the mix: the tree's mixer leaves the
//! VCA master out of the mix and asks it for its level instead.
//!
//! # What it does not do
//!
//! - **The tree's VCA master is a track of its own kind, and here the
//!   kind is the string `vca`.** Nothing else about it is the tree's.
//! - **The tree's VCA master may be automated**, and here the VCA's
//!   own volume lane is read the way any other track's is.

use craftkit_derive::commands;

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Document, EngineError, Node, NodeRef, Outcome};

use crate::TrackPayload;

/// The **kind** of a track that is a VCA master.
pub const VCA_KIND: &str = "vca";

/// Whether a track is a **VCA master**: a track that carries no sound
/// of its own and whose fader offsets the faders of the tracks
/// assigned to it.
pub fn is_vca(node: &Node) -> bool {
    TrackPayload::from_json(&node.data).map(|payload| payload.track_kind == VCA_KIND).unwrap_or(false)
}

/// The **VCA master** of a track, if it has one.
///
/// The tree's mix asks the track for its VCA and looks the master up;
/// a master that is not there is no master, and the member is heard
/// at its own level.
pub fn vca_of(document: &Document, track: &Node) -> Option<Node> {
    let payload = TrackPayload::from_json(&track.data).ok()?;
    let master = payload.vca?;
    let node = document.node(master)?;
    if node.kind != "sound.track" {
        return None;
    }
    Some(node.clone())
}

/// Put a track's sound through its **VCA master**.
///
/// The master's **fader** offsets the member's level, and a muted
/// master mutes the member: the tree's mixer adds the master's level
/// to the member's and asks the master whether it is muted.
///
/// The master's **volume lane** is read the way any other track's is:
/// a lane with breakpoints overrides the master's own fader, and a
/// lane without one is the fader.
///
/// The master's sound is not in the mix and neither is the member's
/// copy of it: the VCA is a **fader over a group** and not a sound
/// of its own, and a **pre-fader** send takes the member's signal
/// before the VCA, the way the tree's mixer takes it before the
/// fader.
pub fn apply_vca(document: &Document, track: &Node, samples: Vec<f32>, channels: usize) -> Vec<f32> {
    let Some(master) = vca_of(document, track) else { return samples };
    // Only a **VCA master** is a master: a track that is not one is
    // no master at all, and the member is heard at its own level.
    if !is_vca(&master) {
        return samples;
    }
    // The master's own **mute**: the tree's mixer asks the master
    // whether it is muted, and a muted master mutes its members. The
    // question is the master's **own** mute and not its mute lane:
    // the tree's mixer reads the mixer mute of the master.
    let muted = TrackPayload::from_json(&master.data).map(|payload| payload.mute).unwrap_or(false);
    if muted {
        return vec![0.0_f32; samples.len()];
    }
    let automation = crate::automate::automation_of(document, &master);
    crate::automate::apply_fader(&automation, &samples, channels, 0)
}

commands! {
    /// Put a track under a **VCA master**, or take it out again.
    ///
    /// The master must be a track whose kind is `vca`: the tree's engine
    /// refuses any other track, and so does this one. A track that is put
    /// under a master is under **that** master and not under another one:
    /// a track has one VCA.
    sound.track.vca {
        area: "sound",
        layer: "sound",
        icon: "sliders",
        mutates,
        undoable,
        menu: ("Track", "VCA"),
        params: {
            /// The track to put under the master.

            track: NodeRef,
            /// The VCA master.

            vca: NodeRef,
        },
        run: {

            let id = "sound.track.vca".to_string();
            let track = self.track.0;
            let vca = self.vca.0;
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
            let Some(master) = session.document().node(vca) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{vca}`"),
                });
            };
            if master.kind != "sound.track" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{vca}` is a `{}` and not a track", master.kind),
                });
            }
            if !is_vca(master) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the track `{vca}` is not a VCA master"),
                });
            }
            if vca == track {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the track `{track}` cannot be its own VCA master"),
                });
            }
            let mut payload = TrackPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error.to_string() })?;
            payload.vca = Some(vca);
            let data = payload.to_json();
            let Some(node) = session.document_mut()?.node_mut(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{track}`"),
                });
            };
            node.data = data;
            Ok(Outcome::changed(format!("the track `{track}` is under the VCA `{vca}`"))
                .with_data(json!({ "track": track, "vca": vca })))

        }
    }
    /// Take a track out of its **VCA master**.
    ///
    /// A track with no master is heard at its own level, and that is the
    /// tree's own: a member of no VCA is not offset by anything.
    sound.track.vca.delete {
        area: "sound",
        layer: "sound",
        icon: "minus",
        mutates,
        undoable,
        menu: ("Track", "VCA"),
        params: {
            /// The track to take out.

            track: NodeRef,
        },
        run: {

            let id = "sound.track.vca.delete".to_string();
            let track = self.track.0;
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
            let had = payload.vca;
            payload.vca = None;
            let data = payload.to_json();
            let Some(node) = session.document_mut()?.node_mut(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no track `{track}`"),
                });
            };
            node.data = data;
            Ok(Outcome::changed(format!("the track `{track}` is under no VCA"))
                .with_data(json!({ "track": track, "vca": had })))

        }
    }
}
