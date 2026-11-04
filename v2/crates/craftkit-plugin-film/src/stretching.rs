//! The **stretch** of a clip.
//!
//! A clip plays its media at a **speed**, and the speed says how much
//! of the media a frame of the clip shows. A **stretch** changes that
//! speed by dragging one of the clip's edges: the clip gets longer or
//! shorter and the speed changes the other way, so that the clip plays
//! **the same material**.
//!
//! It is not the same edit as the **speed**: the speed command is
//! given a rate and the length follows, and the stretch is given a
//! length and the rate follows. And it is not the same edit as the
//! **trim**: a trim moves an edge and the media it shows changes,
//! while a stretch moves an edge and the media it shows does not.
//!
//! The tree **clamps** a stretch: it works out the biggest delta the
//! stretch may have, uses that, and the editor never says that it
//! did. This tree **refuses**: a stretch that would cross a bound is
//! an error that says which bound, and the clip is as it was.
//!
//! The bounds are:
//!
//! - a clip is at least **one frame** long;
//! - a clip may not run **into** the clip after it, and may not begin
//!   before the clip before it ends;
//! - a clip may not start **before the timeline**;
//! - a clip that **holds a frame** may not be stretched, because the
//!   material it plays is shorter than the clip and there is no one
//!   number for it (the module `hold`).
//!
//! And the **link** is here, and the stretch honours it: the linked
//! clips stretch together, by the same delta on their own tracks, and
//! a member that cannot take it refuses the whole stretch (the module
//! `linking`).
//!
//! # What the port found
//!
//! - **The tree's linked stretch is scaled down and this one is
//!   refused.** The tree's `rate_stretch_items` clamps every member to
//!   the room the **tightest** neighbour leaves, so a group that
//!   cannot stretch as far as it was told stretches as far as it may
//!   and nothing says so. Here every member takes the same delta, and
//!   a member that cannot take it refuses the stretch — the rule the
//!   trim of a linked group already had (§299).
//!
//! # What it does not do
//!
//! - **The tree's speed and duration dialogue is not here.** The tree
//!   may set the speed and the length together, and may **ripple** the
//!   clips that were after the clip. Here the speed is a command of
//!   its own (the module `speed`), and it is the one that ripples.
//! - **The tree's sync-locked tracks are not here**, so the tracks that
//!   are locked to the track that changed do not move with it.

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// **Stretch** one edge of a clip.
///
/// The clip gets longer or shorter and its speed changes the other
/// way, so that it plays the same material.
#[derive(Command)]
#[command(id = "film.clip.stretch", area = "film", layer = "film", icon = "expand", mutates, undoable)]
pub struct StretchClip {
    /// The clip.
    pub clip: NodeRef,
    /// The edge that is stretched: `in` or `out`.
    pub edge: Text,
    /// How much it is stretched, in frames. A negative number stretches
    /// it the other way.
    pub delta: Number,
}

/// What a stretch does to one clip.
struct Plan {
    /// The clip that is stretched.
    clip: u64,
    /// Where it starts afterwards.
    start: u64,
    /// How long it is afterwards.
    duration: u64,
    /// How fast it plays its media afterwards.
    speed: f64,
}

impl StretchClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.stretch".to_string();
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
        let edge = self.edge.0.trim().to_lowercase();
        if edge != "in" && edge != "out" {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("`{}` is not an edge: an edge is `in` or `out`", self.edge.0.trim()),
            });
        }
        let delta = self.delta.0;
        if !delta.is_finite() || delta.fract() != 0.0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a number of frames is a whole number".to_string(),
            });
        }
        let delta = delta as i64;
        if delta == 0 {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: "a stretch of zero frames is not a stretch".to_string(),
            });
        }
        // A **locked** track cannot be edited.
        let track = crate::track_of(session, clip).ok_or_else(|| EngineError::Failed {
            id: id.clone(),
            message: format!("the clip `{clip}` is on no track"),
        })?;
        if crate::is_locked(session, track) {
            return Err(crate::locked_error(&id, track));
        }
        // The **link** is here: the linked clips stretch together, by
        // the same delta on their own tracks.
        let members = crate::linking::group(session, clip);
        // Every member takes the same delta, and a member that cannot
        // take it refuses the whole stretch: the tree scales the group
        // down and says nothing.
        let mut plans: Vec<Plan> = Vec::new();
        for member in &members {
            let plan = plan(session, *member, &edge, delta)
                .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
            plans.push(plan);
        }
        for plan in &plans {
            if let Some(node) = session.document_mut()?.node_mut(plan.clip) {
                node.data =
                    crate::clocks_of(&node.data, plan.start, plan.duration, &node.data["source_in"].clone());
                node.data["speed"] = serde_json::json!(plan.speed);
                crate::set_clip_name(node, plan.start);
            }
        }
        // The **transitions** that sit over a cut that moved follow the
        // cut, and they stay inside the two clips they join (the
        // module `transitions`).
        for plan in &plans {
            let (before, after) = crate::neighbours_of(session, plan.clip);
            if edge == "in" {
                if let Some(before) = before {
                    crate::transitions::follow_cut(session, before.id, plan.clip, delta)?;
                }
            } else if let Some(after) = after {
                crate::transitions::follow_cut(session, plan.clip, after.id, delta)?;
            }
            crate::transitions::keep_within_clips(session, plan.clip)?;
        }
        let stretched: Vec<u64> = plans.iter().map(|plan| plan.clip).collect();
        let speed = plans.first().map(|plan| plan.speed).unwrap_or(1.0);
        Ok(Outcome::changed(format!("the clip `{clip}` is stretched by {delta} frames")).with_data(
            serde_json::json!({
                "id": clip,
                "edge": edge,
                "delta": delta,
                "speed": speed,
                "clips": stretched,
                "group": members,
            }),
        ))
    }
}

/// The **plan** of one clip that is stretched: where it starts, how
/// long it is and how fast it plays its media.
///
/// The answer is the plan, or the **reason** the stretch cannot be
/// made.
fn plan(session: &Session, clip: u64, edge: &str, delta: i64) -> Result<Plan, String> {
    let Some(node) = session.document().node(clip) else {
        return Err(format!("there is no clip `{clip}`"));
    };
    let Some(start) = node.data.get("start").and_then(|start| start.as_u64()) else {
        return Err(format!("the clip `{clip}` does not say where it is"));
    };
    let Some(duration) = node.data.get("duration").and_then(|duration| duration.as_u64()) else {
        return Err(format!("the clip `{clip}` does not say how long it is"));
    };
    let speed = crate::speed::speed_of(&node.data);
    // A clip that **holds a frame** plays material that is shorter
    // than the clip, so there is no one number to stretch.
    if crate::hold::hold_of(&node.data).is_some() {
        return Err(format!("the clip `{clip}` holds a frame, and a held clip cannot be stretched"));
    }
    let Some(track) = crate::track_of(session, clip) else {
        return Err(format!("the clip `{clip}` is on no track"));
    };
    if crate::is_locked(session, track) {
        return Err(format!("the track `{track}` is locked"));
    }
    // The **material** the clip plays: the media it shows, which does
    // not change.
    let material = crate::speed::media_frames(duration, speed);
    if material == 0 {
        return Err(format!("the clip `{clip}` plays no media of its own, and there is nothing to stretch"));
    }
    // The **new length**: the out edge grows the clip and the in edge
    // takes from it.
    let new_duration = if edge == "in" { duration as i64 - delta } else { duration as i64 + delta };
    if new_duration < 1 {
        return Err(format!(
            "the clip would be {new_duration} frames long, and a clip is at least one frame"
        ));
    }
    let new_duration = new_duration as u64;
    // The **new speed**: the same material in the new length.
    let new_speed = material as f64 / new_duration as f64;
    if !new_speed.is_finite() || new_speed <= 0.0 {
        return Err("the clip would play its media at no speed at all".to_string());
    }
    // The **place**: the out edge leaves the clip where it is and the
    // in edge moves it, so that the end of the clip stays where it is.
    let new_start = if edge == "in" { start as i64 + delta } else { start as i64 };
    if new_start < 0 {
        return Err(format!(
            "the clip would start at frame {}, and the timeline begins at frame 0",
            -new_start
        ));
    }
    let new_start = new_start as u64;
    // The **neighbours**: the clip may not run into the one after it,
    // and may not begin before the one before it ends.
    let (before, after) = crate::neighbours_of(session, clip);
    if edge == "in"
        && let Some(before) = before
        && new_start < before.end
    {
        return Err(format!(
            "the clip would start at frame {new_start}, and the clip before it ends at frame {}",
            before.end
        ));
    }
    if edge == "out"
        && let Some(after) = after
        && new_start + new_duration > after.start
    {
        return Err(format!(
            "the clip would end at frame {}, and the clip after it starts at frame {}",
            new_start + new_duration,
            after.start
        ));
    }
    Ok(Plan { clip, start: new_start, duration: new_duration, speed: new_speed })
}
