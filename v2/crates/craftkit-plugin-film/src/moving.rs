//! **Moving** a clip.
//!
//! A clip has a place on a track, and the place may change: the clip
//! moves along the track, or it moves to another track. The tree moves
//! a clip in two modes — the **overwrite**, which cuts what is under
//! the clip, and the **insert**, which pushes it along — and here the
//! move is the **plain** one: the clip goes where it is told to go,
//! and a clip that would land on another clip is **refused**.
//!
//! The refusal is the point. The tree's overwrite is a thing a command
//! does **quietly**: a clip that is dragged over another one cuts it,
//! and the cut is not in the answer the tree gives back. Here the
//! answer says what would have been cut, and the caller may do it in a
//! command of its own.
//!
//! # What it does not do
//!
//! - **The insert is not here.** The tree's insert mode pushes the
//!   clips after the moved one along the track, so that nothing is
//!   cut. That is a different command.
//! - **The ripple is not here**, and neither is the **link**: the
//!   tree's linked clips move together, and here a clip is a clip.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// **Move** a clip to a place on a track.
///
/// The clip goes where it is told to go, and a clip that would land on
/// another clip is **refused**: the tree's overwrite cuts the clip
/// that is under it and says nothing about the cut, and here the
/// refusal names the clip that would have been cut.
#[derive(Command)]
#[command(id = "film.clip.move", area = "film", layer = "film", icon = "arrow", mutates, undoable)]
pub struct MoveClip {
    /// The clip.
    pub clip: NodeRef,
    /// The track it goes on. The track it is on, if it stays there.
    pub track: NodeRef,
    /// Where it starts on that track, in frames.
    pub start: Number,
}

impl MoveClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.move".to_string();
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
        let end = start + duration;
        // The **clips** that are on the track it goes on: the move may
        // not land on any of them.
        for other in session
            .document()
            .walk()
            .filter(|node| node.kind == "film.track")
            .filter(|node| node.id == track)
            .flat_map(|node| node.children.iter())
            .filter(|child| child.kind == "film.clip" && child.id != clip)
        {
            let Some(other_start) = other.data.get("start").and_then(|start| start.as_u64()) else {
                continue;
            };
            let Some(other_duration) = other.data.get("duration").and_then(|duration| duration.as_u64())
            else {
                continue;
            };
            let other_end = other_start + other_duration;
            if start < other_end && other_start < end {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip would run from frame {start} to frame {end}, and the clip on the track runs from frame {other_start} to frame {other_end}"
                    ),
                });
            }
        }
        // The **track** it is on now, if it is another one.
        let from = session
            .document()
            .walk()
            .find(|node| node.kind == "film.track" && node.children.iter().any(|child| child.id == clip))
            .map(|node| node.id);
        let data = serde_json::json!({
            "start": start,
            "duration": duration,
            "source_in": source_in,
        });
        let name = format!("clip at {start}");
        if from == Some(track) {
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data = data;
                node.name = name;
            }
        } else {
            // The clip **changes track**: it is taken out of the tree
            // and put under the other track, with the same id, so that
            // the edge that says which item it plays still says it.
            let Some(mut node) = session.document_mut()?.take_node(clip) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{clip}`"),
                });
            };
            node.data = data;
            node.name = name;
            let Some(track_node) = session.document_mut()?.node_mut(track) else {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("there is no node `{track}`"),
                });
            };
            track_node.add_child(node);
        }
        Ok(Outcome::changed(format!("the clip `{clip}` is at frame {start} of the track `{track}`"))
            .with_data(
                serde_json::json!({ "id": clip, "track": track, "start": start, "duration": duration }),
            ))
    }
}
