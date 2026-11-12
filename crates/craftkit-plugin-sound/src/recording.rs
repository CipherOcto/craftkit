//! The **recording** of a track.
//!
//! A track may be **armed** for recording, and it may **monitor** its
//! input: the sound that comes in is heard in the mix, and the sound
//! that is there is not.
//!
//! The tree's transport asks the **host** to record, and the sound
//! that comes in is the host's: here there is no host and no live
//! input, so what is here is the **arming** and the **monitoring** —
//! the facts of a track that is ready to record.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Outcome, Session};

use crate::TrackPayload;

/// Arm a track for **recording**, or let it be.
///
/// The arm is the track's **own**: the tree's mixer shares the mute
/// and the solo with the group that mixes together, and the arm is
/// not one of them — a group of tracks is armed one at a time.
#[derive(Command)]
#[command(
    id = "sound.track.record",
    area = "sound",
    layer = "sound",
    icon = "record",
    menu("Track", "Record"),
    mutates,
    undoable
)]
pub struct RecordTrack {
    /// The track.
    pub track: NodeRef,
    /// Whether it is armed for recording.
    pub armed: Bool,
}

impl RecordTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.record".to_string();
        let track = self.track.0;
        let armed = self.armed.0;
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
        payload.record_arm = armed;
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no track `{track}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the track `{track}` is armed: {armed}"))
            .with_data(serde_json::json!({ "id": track, "record_arm": armed })))
    }
}

/// Let a track **hear its input**, or not.
///
/// The tree's monitor puts the **live input** in the mix in place of
/// the track's own sound — and here there is no live input, so the
/// monitor is the fact that says the track is listening.
#[derive(Command)]
#[command(
    id = "sound.track.monitor",
    area = "sound",
    layer = "sound",
    icon = "record",
    menu("Track", "Monitor"),
    mutates,
    undoable
)]
pub struct MonitorTrack {
    /// The track.
    pub track: NodeRef,
    /// Whether it hears its input.
    pub monitoring: Bool,
}

impl MonitorTrack {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "sound.track.monitor".to_string();
        let track = self.track.0;
        let monitoring = self.monitoring.0;
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
        payload.input_monitor = monitoring;
        let data = payload.to_json();
        let Some(node) = session.document_mut()?.node_mut(track) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no track `{track}`"),
            });
        };
        node.data = data;
        Ok(Outcome::changed(format!("the track `{track}` monitors its input: {monitoring}"))
            .with_data(serde_json::json!({ "id": track, "input_monitor": monitoring })))
    }
}
