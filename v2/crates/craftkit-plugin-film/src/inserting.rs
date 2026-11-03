//! **Inserting** a clip.
//!
//! The tree moves a clip in two modes: the **overwrite**, which cuts
//! what is under the clip, and the **insert**, which opens a gap and
//! pushes the clips along. The overwrite is in `moving`, and this is
//! the insert.
//!
//! The insert **opens a gap** of the clip's length at the place the
//! clip goes, and every clip on the track that starts at or after that
//! place is pushed along by that length. So nothing is cut: the
//! material that was there is still there, and it is further along.
//!
//! The tree **splits** a clip that spans the place the gap opens at, so
//! that the gap has two edges. Here that is **refused**, and the
//! refusal names the clip that spans the place: splitting a clip is a
//! command of its own — the tree's razor — and a command that splits
//! a clip as a side effect of another one is a command that does two
//! things.
//!
//! # What it does not do
//!
//! - **The sync-locked tracks are not here.** The tree opens the gap on
//!   the tracks that are **sync-locked** with the target as well, so
//!   that the locked groups stay together. Here a track is a track.
//! - **The clip that spans the place is not split.** The tree splits
//!   it; here the insert is refused.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// **Insert** a clip at a place on a track.
///
/// A gap of the clip's length opens at the place, and the clips that
/// are there are pushed along by that length: nothing is cut. A clip
/// that **spans** the place is refused — the tree splits it, and
/// splitting is a command of its own.
#[derive(Command)]
#[command(id = "film.clip.insert", area = "film", layer = "film", icon = "arrow", mutates, undoable)]
pub struct InsertClip {
    /// The clip.
    pub clip: NodeRef,
    /// The track it goes on.
    pub track: NodeRef,
    /// Where it goes, in frames.
    pub start: Number,
}

impl InsertClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.insert".to_string();
        let clip = self.clip.0;
        let track = self.track.0;
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
        let Some(duration) = node.data.get("duration").and_then(|duration| duration.as_u64()) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` does not say how long it is"),
            });
        };
        let source_in = node.data.get("source_in").cloned().unwrap_or(serde_json::Value::Null);
        let Some(node) = session.document().node(track) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{track}`"),
            });
        };
        if node.kind != "film.track" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the node `{track}` is a `{}` and not a track", node.kind),
            });
        }
        let start = self.start.0;
        if !start.is_finite() || start.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a number of frames is a whole number".to_string(),
            });
        }
        if start < 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the timeline begins at frame 0".to_string(),
            });
        }
        let start = start as u64;
        // The clips that are on the track: the ones that are pushed
        // along, and the one that spans the place, which is refused.
        let mut pushed: Vec<(u64, u64)> = Vec::new();
        for child in session
            .document()
            .walk()
            .filter(|node| node.kind == "film.track" && node.id == track)
            .flat_map(|node| node.children.iter())
            .filter(|child| child.kind == "film.clip" && child.id != clip)
        {
            let Some(other_start) = child.data.get("start").and_then(|start| start.as_u64()) else {
                continue;
            };
            let Some(other_duration) = child.data.get("duration").and_then(|duration| duration.as_u64())
            else {
                continue;
            };
            let other_end = other_start + other_duration;
            if other_start < start && other_end > start {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip on the track runs from frame {other_start} to frame {other_end}, and it spans the place the gap opens at"
                    ),
                });
            }
            if other_start >= start {
                pushed.push((child.id, other_start));
            }
        }
        // The **gap**: the clips that are there are pushed along by the
        // length of the clip that is inserted.
        let mut moved: Vec<serde_json::Value> = Vec::new();
        for (other, other_start) in pushed {
            let new_start = other_start + duration;
            let source_in = source_in_of(session, other);
            let duration = duration_of(session, other);
            if let Some(node) = session.document_mut()?.node_mut(other) {
                node.data = crate::clocks_of(&node.data, new_start, duration, &source_in);
                node.name = format!("clip at {new_start}");
            }
            moved.push(serde_json::json!({ "id": other, "start": new_start }));
        }
        // The clip itself: its place, and the track it is on.
        let name = format!("clip at {start}");
        let from = session
            .document()
            .walk()
            .find(|node| node.kind == "film.track" && node.children.iter().any(|child| child.id == clip))
            .map(|node| node.id);
        if from == Some(track) {
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data = crate::clocks_of(&node.data, start, duration, &source_in);
                node.name = name;
            }
        } else {
            let Some(mut node) = session.document_mut()?.take_node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                });
            };
            node.data = crate::clocks_of(&node.data, start, duration, &source_in);
            node.name = name;
            let Some(track_node) = session.document_mut()?.node_mut(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{track}`"),
                });
            };
            track_node.add_child(node);
        }
        Ok(Outcome::changed(format!("the clip `{clip}` is inserted at frame {start} of the track `{track}`"))
            .with_data(serde_json::json!({
                "id": clip,
                "track": track,
                "start": start,
                "duration": duration,
                "pushed": moved,
            })))
    }
}

/// How long a clip is.
fn duration_of(session: &Session, clip: u64) -> u64 {
    session
        .document()
        .node(clip)
        .and_then(|node| node.data.get("duration").cloned())
        .and_then(|duration| duration.as_u64())
        .unwrap_or(0)
}

/// Where in the media a clip begins.
fn source_in_of(session: &Session, clip: u64) -> serde_json::Value {
    session
        .document()
        .node(clip)
        .and_then(|node| node.data.get("source_in").cloned())
        .unwrap_or(serde_json::Value::Null)
}
