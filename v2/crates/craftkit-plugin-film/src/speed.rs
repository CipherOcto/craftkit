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
//!
//! # The ripple of the change
//!
//! A speed change makes a clip longer or shorter, and the space after
//! it may not be there. The tree has two ways out: the new length is
//! **bounded** by the space there is — and the tree clamps it — or the
//! change **ripples**: the clips that were after the clip move **up**
//! by the change in its length, and the time the clip took is time
//! that is free.
//!
//! Here the first way is a **refusal**: a clip that would run into its
//! neighbour is refused rather than shortened. And the second way is
//! here: the command takes a `ripple`, and the clips that were after
//! the clip move by the change, and the **transitions** follow the
//! cuts.
//!
//! # What it does not do
//!
//! - **The sync-locked tracks are not here.** The tree's ripple moves
//!   the tracks that are **sync-locked** with the track that changed,
//!   so that a group of tracks stays together. Here only the track
//!   the clip is on moves.
//! - **The tree's end of the timeline is not here.** The tree refuses
//!   a change that would move later clips past the end of the
//!   timeline; here the length of a track is derived and there is no
//!   end to pass.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, NodeRef, Number, Outcome, Session};

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
///
/// The tree has a second way: the change may **ripple**. The clips
/// that were after the clip move **up** by the change in its length,
/// and the time the clip took is time that is free — which is the
/// tree's ripple delete, done by a speed change.
#[derive(Command)]
#[command(id = "film.clip.speed", area = "film", layer = "film", icon = "repeat", mutates, undoable)]
pub struct SetSpeed {
    /// The clip.
    pub clip: NodeRef,
    /// The speed: one is the media as it is, two is twice as fast.
    pub speed: Number,
    /// Whether the clips that were after it move up by the change in
    /// its length.
    ///
    /// Leave it out and the change is bounded by the space there is:
    /// a clip that would run into its neighbour is refused.
    pub ripple: Option<Bool>,
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
        let old_duration = duration;
        let duration = (media as f64 / speed).round() as i64;
        if duration < 1 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "the clip would be less than one frame long".to_string(),
            });
        }
        // The clip may not **overlap** the clip after it: the tree
        // bounds the new length by the space there is. A **ripple**
        // moves the clips that were after it, so there is no bound.
        let ripple = self.ripple.map(|ripple| ripple.0).unwrap_or(false);
        if !ripple
            && let Some(next) = the_next_start(session, clip, start)
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
        // The **ripple**: the clips that were after it move up by the
        // change in its length, and the transitions follow the cuts.
        let moved =
            if ripple { ripple_after(session, clip, start, old_duration, duration)? } else { Vec::new() };
        Ok(Outcome::changed(format!("the clip `{clip}` plays at {speed} times")).with_data(
            serde_json::json!({
                "clip": clip,
                "speed": speed,
                "duration": duration,
                "start": start,
                "source_in": source_in,
                "ripple": ripple,
                "moved": moved,
            }),
        ))
    }
}

/// The **ripple** of a speed change: the clips that were after the
/// clip move up by the change in its length.
///
/// The tree's rule: the clips that start at or after the **end** of
/// the clip that changed are the ones that move, and they move by the
/// change in its length. The **transitions** follow the cuts: one
/// whose cut moved moves with it, and one that no longer fits inside
/// the two clips it joins is brought back inside them.
fn ripple_after(
    session: &mut Session,
    clip: u64,
    start: u64,
    old_duration: u64,
    new_duration: i64,
) -> Result<Vec<u64>, EngineError> {
    let delta = new_duration - old_duration as i64;
    if delta == 0 {
        return Ok(Vec::new());
    }
    let Some(track) = crate::track_of(session, clip) else { return Ok(Vec::new()) };
    let Some(node) = session.document().node(track) else { return Ok(Vec::new()) };
    let old_end = start + old_duration;
    // The **clips** that move: the ones that start at or after the end
    // of the clip that changed.
    let moving: Vec<(u64, u64)> = node
        .children
        .iter()
        .filter(|child| child.kind == "film.clip" && child.id != clip)
        .filter_map(|child| {
            let child_start = child.data.get("start").and_then(|start| start.as_u64())?;
            Some((child.id, child_start))
        })
        .filter(|(_, child_start)| *child_start >= old_end)
        .collect();
    if moving.is_empty() {
        // No clip moved, but the clip that changed is a different
        // length now, and a transition that joins it may no longer fit
        // inside the two clips.
        crate::transitions::keep_within_clips(session, clip)?;
        return Ok(Vec::new());
    }
    // The **moved** clips, and the **transitions** that follow them.
    let moved_ids: Vec<u64> = moving.iter().map(|(id, _)| *id).collect();
    let transitions: Vec<u64> = node
        .children
        .iter()
        .filter(|child| child.kind == "film.transition")
        .filter(|child| {
            let from = child.data.get("from").and_then(|from| from.as_u64());
            let to = child.data.get("to").and_then(|to| to.as_u64());
            from.is_some_and(|from| moved_ids.contains(&from)) || to.is_some_and(|to| moved_ids.contains(&to))
        })
        .map(|child| child.id)
        .collect();
    // The clips move first.
    for (id, child_start) in &moving {
        let new_start = *child_start as i64 + delta;
        if new_start < 0 {
            return Err(EngineError::Failed {
                id: "film.clip.speed".to_string(),
                message: "the speed change would move a clip before the start of the timeline".to_string(),
            });
        }
        let Some(node) = session.document_mut()?.node_mut(*id) else { continue };
        let duration = node.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
        let source_in = node.data.get("source_in").cloned().unwrap_or(serde_json::Value::Null);
        node.data = crate::clocks_of(&node.data, new_start as u64, duration, &source_in);
        node.name = format!("clip at {}", new_start as u64);
    }
    // And the transitions follow the cuts.
    for transition in &transitions {
        let Some(node) = session.document_mut()?.node_mut(*transition) else { continue };
        let start = node.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
        let new_start = start as i64 + delta;
        if new_start < 0 {
            return Err(EngineError::Failed {
                id: "film.clip.speed".to_string(),
                message: "the speed change would move a transition before the start of the timeline"
                    .to_string(),
            });
        }
        node.data["start"] = serde_json::json!(new_start as u64);
    }
    // And a transition that no longer fits inside the two clips it
    // joins is brought back inside them.
    crate::transitions::keep_within_clips(session, clip)?;
    for id in &moved_ids {
        crate::transitions::keep_within_clips(session, *id)?;
    }
    Ok(moved_ids)
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
