//! The **frame hold** of a clip.
//!
//! A clip may **hold** a frame: it plays its media up to a point in
//! the media and then holds that frame to the end of the clip. The
//! hold is a **source** frame — a frame of the media — and not a frame
//! of the timeline, because it is the media that stops and not the
//! timeline.
//!
//! So a held clip has no **out edge in the media**: its material ends
//! at the hold, and the rest of the clip is that one frame. And it has
//! no **head handle**: the media before its in point is not there to
//! be taken, so a trim of the in edge moves the clip and not the
//! media.
//!
//! The tree's rule, and it is the finding: a held clip is a clip whose
//! material is **shorter** than the clip.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The **frame** a clip holds: the source frame it holds to the end.
///
/// A clip with no hold plays its media to the end of the clip.
pub fn hold_of(data: &serde_json::Value) -> Option<u64> {
    data.get("frame_hold").and_then(|hold| hold.as_u64())
}

/// **Hold** a frame of a clip.
///
/// The clip plays its media up to that frame of the media and then
/// holds that frame to the end of the clip. The frame must be inside
/// the media the clip plays.
#[derive(Command)]
#[command(id = "film.clip.hold", area = "film", layer = "film", icon = "pin", mutates, undoable)]
pub struct SetHold {
    /// The clip.
    pub clip: NodeRef,
    /// The **source** frame the clip holds: the frame at that point of
    /// the media, held to the end of the clip.
    pub at: Number,
}

impl SetHold {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.hold".to_string();
        let clip = self.clip.0;
        let at = self.at.0;
        if !at.is_finite() || at.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a number of frames is a whole number".to_string(),
            });
        }
        let at = at as u64;
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
        // A **locked** track cannot be edited.
        if let Some(track) = crate::track_of(session, clip)
            && crate::is_locked(session, track)
        {
            return Err(crate::locked_error(&id, track));
        }
        let source_in = node.data.get("source_in").and_then(|source_in| source_in.as_u64()).unwrap_or(0);
        let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
        // The **speed**: the media a frame of the clip plays is the
        // speed's worth of the media.
        let speed = crate::speed::speed_of(&node.data);
        let media = crate::speed::media_frames(duration, speed);
        let end = source_in + media;
        // The hold must be **inside the media** the clip plays: the
        // frame it holds is a frame of that media.
        if at < source_in || at > end {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the frame {at} is not in the media the clip plays, which runs from frame {source_in} to frame {end}"
                ),
            });
        }
        if let Some(node) = session.document_mut()?.node_mut(clip) {
            node.data["frame_hold"] = serde_json::json!(at);
        }
        Ok(Outcome::changed(format!("the clip `{clip}` holds the frame at {at} of the media"))
            .with_data(serde_json::json!({ "clip": clip, "at": at, "source_in": source_in })))
    }
}

/// **Take away** the frame hold of a clip.
///
/// The clip plays its media to the end of the clip again.
#[derive(Command)]
#[command(id = "film.clip.hold.delete", area = "film", layer = "film", icon = "unpin", mutates, undoable)]
pub struct DeleteHold {
    /// The clip.
    pub clip: NodeRef,
}

impl DeleteHold {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.hold.delete".to_string();
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
        // A **locked** track cannot be edited.
        if let Some(track) = crate::track_of(session, clip)
            && crate::is_locked(session, track)
        {
            return Err(crate::locked_error(&id, track));
        }
        let Some(at) = hold_of(&node.data) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` holds no frame"),
            });
        };
        if let Some(node) = session.document_mut()?.node_mut(clip) {
            node.data["frame_hold"] = serde_json::Value::Null;
        }
        Ok(Outcome::changed(format!("the clip `{clip}` holds no frame"))
            .with_data(serde_json::json!({ "clip": clip, "at": at })))
    }
}
