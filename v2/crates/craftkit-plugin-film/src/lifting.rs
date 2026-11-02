//! **Lifting** a range of a track.
//!
//! A range of a track may be **taken away**: the clips that are in it
//! go, and the **gap** they leave is a gap — the clips that were after
//! the range stay where they are, and the track is shorter by the
//! length of the range.
//!
//! The tree has two ways of taking a range away: the **lift**, which
//! leaves the gap, and the **extract**, which closes it. The lift is
//! here.
//!
//! The tree's lift **splits** the clips that span the edges of the
//! range, so that the range has two edges. Here that is **refused**,
//! and the refusal names the clip: splitting a clip is a command of
//! its own (§ the split), and a command that splits a clip while it
//! removes another one is a command that does two things and says one.
//!
//! # What it does not do
//!
//! - **The extract is not here**, and neither is the tree's
//!   **sync-locked** tracks: the tree's extract closes the gap on the
//!   tracks that are locked with the target as well.
//! - **The tree's locked tracks are not here**, and neither are its
//!   **links** and its **transitions**.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// **Lift** a range of a track: take the clips that are in it away.
///
/// The clips that are in the range are gone, and the gap they leave is
/// a gap. A clip that **spans** an edge of the range is refused: the
/// tree splits it, and the split is a command of its own.
#[derive(Command)]
#[command(id = "film.clip.lift", area = "film", layer = "film", icon = "minus", mutates, undoable)]
pub struct LiftRange {
    /// The track the range is on.
    pub track: NodeRef,
    /// Where the range starts, in frames.
    pub from: Number,
    /// Where the range ends, in frames.
    pub to: Number,
}

impl LiftRange {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.lift".to_string();
        let track = self.track.0;
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
        let from = frames(self.from, &id)?;
        let to = frames(self.to, &id)?;
        if to < from {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a range ends after it starts, and this one ends at frame {to}"),
            });
        }
        // The **clips** on the track: the ones that are in the range go,
        // and the ones that span an edge of it are refused.
        let mut inside: Vec<u64> = Vec::new();
        for child in session
            .document()
            .walk()
            .filter(|node| node.kind == "film.track" && node.id == track)
            .flat_map(|node| node.children.iter())
            .filter(|child| child.kind == "film.clip")
        {
            let Some(start) = child.data.get("start").and_then(|start| start.as_u64()) else {
                continue;
            };
            let Some(duration) = child.data.get("duration").and_then(|duration| duration.as_u64()) else {
                continue;
            };
            let end = start + duration;
            if start >= from && end <= to {
                inside.push(child.id);
                continue;
            }
            if start < to && from < end {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip on the track runs from frame {start} to frame {end}, and it spans an edge of the range"
                    ),
                });
            }
        }
        // The **gap** the clips leave is a gap: the clips that are
        // after the range do not move.
        let mut removed: Vec<serde_json::Value> = Vec::with_capacity(inside.len());
        for clip in inside {
            if !session.document_mut()?.remove_node(clip) {
                continue;
            }
            removed.push(serde_json::json!({ "id": clip }));
        }
        let count = removed.len();
        Ok(Outcome::changed(format!("{count} clip(s) lifted from frames {from} to {to}")).with_data(
            serde_json::json!({
                "track": track,
                "from": from,
                "to": to,
                "removed": removed,
            }),
        ))
    }
}

/// A number of frames.
fn frames(value: Number, id: &str) -> Result<u64, EngineError> {
    if !value.0.is_finite() || value.0 < 0.0 || value.0.fract() != 0.0 {
        return Err(EngineError::Failed {
            id: id.to_string(),
            message: "a number of frames is a whole number".to_string(),
        });
    }
    Ok(value.0 as u64)
}
