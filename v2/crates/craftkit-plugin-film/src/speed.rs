//! The **speed** of a clip.
//!
//! A clip may play its media **faster or slower** than it is. The
//! tree's clip has a `speed` — one is a hundred per cent, and the
//! media a clip consumes is the media it plays in the media's own
//! frames: a clip that plays twice as fast consumes twice the media in
//! half the time.
//!
//! So the speed is the **third** thing that joins the two clocks: the
//! timeline's frames, the media's frames, and the rate the media is
//! played at. And the tree's rule for changing it is that the **media
//! stays the same**: a clip that plays twice as fast is half as long,
//! and it plays the same material.
//!
//! Here a clip's payload carries the speed, and the trim and the split
//! count the media through it.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

/// The **speed** of a clip, as the payload holds it.
///
/// One is the media as it is. A clip with no speed plays as it is.
pub fn speed_of(data: &serde_json::Value) -> f64 {
    data.get("speed")
        .and_then(|speed| speed.as_f64())
        .filter(|speed| speed.is_finite() && *speed > 0.0)
        .unwrap_or(1.0)
}

/// The **media** a clip of that length consumes at that speed.
///
/// The tree's rule: the media a clip plays is its length multiplied by
/// its speed, and the multiplication is rounded.
pub fn media_frames(duration: u64, speed: f64) -> u64 {
    (duration as f64 * speed).round().max(0.0) as u64
}

/// **Set** the speed of a clip.
///
/// The clip's **length** follows: a clip that plays twice as fast is
/// half as long, and it plays the same material.
///
/// A clip may not **overlap** the clip after it: the tree bounds the
/// new length by the space there is, and a clip that would run into
/// its neighbour is refused rather than shortened.
#[derive(Command)]
#[command(id = "film.clip.speed", area = "film", layer = "film", icon = "repeat", mutates, undoable)]
pub struct SetSpeed {
    /// The clip.
    pub clip: NodeRef,
    /// The speed: one is the media as it is, two is twice as fast.
    pub speed: Number,
}

impl SetSpeed {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.speed".to_string();
        let clip = self.clip.0;
        let speed = self.speed.0;
        if !speed.is_finite() || speed <= 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("a speed is a number that is there and is more than nothing: {speed}"),
            });
        }
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
        let Some(duration) = node.data.get("duration").and_then(|duration| duration.as_u64()) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` does not say how long it is"),
            });
        };
        let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
        let source_in = node.data.get("source_in").and_then(|source_in| source_in.as_u64()).unwrap_or(0);
        // The **media** the clip plays now, and the media it will play:
        // the same material, at another rate.
        let media = media_frames(duration, speed_of(&node.data));
        let duration = (media as f64 / speed).round() as i64;
        if duration < 1 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the clip would be less than one frame long".to_string(),
            });
        }
        // The clip may not **overlap** the clip after it: the tree
        // bounds the new length by the space there is.
        if let Some(next) = the_next_start(session, clip, start)
            && start as i64 + duration > next as i64
        {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the clip would end at frame {}, and the clip after it starts at frame {next}",
                    start as i64 + duration
                ),
            });
        }
        // The media may not run past the **end** of the item the clip
        // plays.
        if let Some(media_length) = crate::media_length_of(session, clip)
            && source_in as i64 + media_frames(duration as u64, speed) as i64 > media_length as i64
        {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the clip would run to frame {} of the media, and the media is {media_length} frames long",
                    source_in as i64 + media_frames(duration as u64, speed) as i64
                ),
            });
        }
        let mut data = node.data.clone();
        data["duration"] = serde_json::json!(duration);
        data["speed"] = serde_json::json!(speed);
        if let Some(node) = session.document_mut()?.node_mut(clip) {
            node.data = data;
            node.name = format!("clip at {start}");
        }
        Ok(Outcome::changed(format!("the clip `{clip}` plays at {speed} times")).with_data(
            serde_json::json!({
                "clip": clip,
                "speed": speed,
                "duration": duration,
                "start": start,
                "source_in": source_in,
            }),
        ))
    }
}

/// Where the clip **after** this one starts.
fn the_next_start(session: &Session, clip: u64, start: u64) -> Option<u64> {
    let track = crate::track_of(session, clip)?;
    let node = session.document().node(track)?;
    node.children
        .iter()
        .filter(|child| child.kind == "film.clip" && child.id != clip)
        .filter_map(|child| {
            let child_start = child.data.get("start").and_then(|start| start.as_u64())?;
            let child_duration = child.data.get("duration").and_then(|duration| duration.as_u64())?;
            Some((child_start, child_duration))
        })
        .filter(|(child_start, _)| *child_start >= start)
        .map(|(child_start, _)| child_start)
        .min()
}
