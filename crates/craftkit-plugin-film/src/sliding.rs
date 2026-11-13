//! The **slide** of a clip between its neighbours.
//!
//! A clip on a track may be **slid** between the clips around it: the
//! clip moves, and the clips around it **give way** — the one before it
//! grows or shrinks, and the one after it moves and shrinks or grows —
//! so that the span they share stays **without a gap**.
//!
//! It is the third way of moving a boundary between two clips, and the
//! other two are not like it: a **trim** moves one edge of one clip and
//! leaves its neighbour alone, a **roll** moves the boundary between two
//! clips and both of them go with it, and a **slide** moves the clip in
//! the middle and its neighbours go with it.
//!
//! The tree **clamps** a slide: it works out the biggest delta the
//! slide may have, uses that, and the editor never says that it did.
//! This tree **refuses**: a slide that would cross a bound is an error
//! that says which bound, and the clips are as they were.
//!
//! The bounds are the **trim's** bounds, on the neighbours:
//!
//! - a clip is at least **one frame** long;
//! - a clip may not run **past the end** of its media, and the media
//!   a clip plays is its length multiplied by its **speed** (the
//!   module `speed`);
//! - a clip may not begin **before the media** it plays;
//! - a clip may not start **before the timeline**.
//!
//! And a neighbour that is not **next to** the slid clip — a gap
//! between them — is a bound of its own: the slid clip may not run
//! into it.
//!
//! And the **link** is here, and the slide honours it: the linked
//! clips slide together, by the same delta on their own tracks, and a
//! member that cannot take it refuses the whole slide (the module
//! `linking`).
//!
//! # What the port found
//!
//! - **The tree's linked slide is scaled down and this one is refused.**
//!   The tree's `slide_items` slides every clip of the group by the
//!   delta the **most restrictive** one can take, so the group slides a
//!   little and nothing says so. Here every member takes the same
//!   delta, and a member that cannot take it refuses the slide — the
//!   rule the trim of a linked group already had (§299).
//!
//! # What it does not do
//!
//! - **The tree's rate stretch is not here.** The tree may change how
//!   fast a clip plays its media by dragging one of its edges, and the
//!   linked partners stretch with it. That is an edit of the media and
//!   not of the place, and it is a command of its own.
//! - **The tree's time remapping is not here**, and neither is its
//!   remapping of the media: a slide moves the clip and its
//!   neighbours, and the media of the slid clip is the media it was
//!   showing.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

commands! {
    /// **Slide** a clip between its neighbours.
    ///
    /// The clip moves and the clips around it give way, so that the span
    /// they share stays without a gap.
    ///
    /// The slide is refused when it would cross a bound: the tree clamps
    /// such a slide and says nothing, and here the refusal says which
    /// bound.
    film.clip.slide {
        area: "film",
        layer: "film",
        icon: "slide",
        mutates,
        undoable,
        params: {
            /// The clip.
            clip: NodeRef,
            /// How far it slides, in frames. A negative number slides it the
            /// other way.
            delta: Number,
        },
        run: {
            let id = "film.clip.slide".to_string();
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
                    message: "a slide of zero frames is not a slide".to_string(),
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
            // The **link** is here: the linked clips slide together, by the
            // same delta on their own tracks.
            let members = crate::linking::group(session, clip);
            // Every member takes the same delta, and a member that cannot
            // take it refuses the whole slide: the tree scales the group
            // down and says nothing.
            let mut plans: Vec<Plan> = Vec::new();
            for member in &members {
                let plan = plan(session, *member, delta, &members)
                    .map_err(|message| EngineError::Failed { id: id.clone(), message })?;
                plans.push(plan);
            }
            for plan in &plans {
                for (clip, clock) in clocks_of(plan) {
                    if let Some(node) = session.document_mut()?.node_mut(clip) {
                        let source_in = serde_json::json!(clock.source_in);
                        node.data = crate::clocks_of(&node.data, clock.start, clock.duration, &source_in);
                        crate::set_clip_name(node, clock.start);
                    }
                }
            }
            // The **transitions** that sit over a cut that moved follow the
            // cut, and they stay inside the two clips they join (the
            // module `transitions`).
            for plan in &plans {
                if let Some((before, _)) = plan.before {
                    crate::transitions::follow_cut(session, before, plan.clip, delta)?;
                }
                if let Some((after, _)) = plan.after {
                    crate::transitions::follow_cut(session, plan.clip, after, delta)?;
                }
                crate::transitions::keep_within_clips(session, plan.clip)?;
                if let Some((before, _)) = plan.before {
                    crate::transitions::keep_within_clips(session, before)?;
                }
                if let Some((after, _)) = plan.after {
                    crate::transitions::keep_within_clips(session, after)?;
                }
            }
            let slid: Vec<u64> = plans.iter().map(|plan| plan.clip).collect();
            Ok(Outcome::changed(format!("the clip `{clip}` slides by {delta} frames")).with_data(
                serde_json::json!({
                    "id": clip,
                    "delta": delta,
                    "clips": slid,
                    "group": members,
                }),
            ))

        }
    }
}

/// The **clock** of a clip: where it starts, how long it is and where
/// in the media it begins.
#[derive(Clone, Copy)]
struct Clock {
    /// Where it starts on the timeline.
    start: u64,
    /// How long it is.
    duration: u64,
    /// Where in the media it begins.
    source_in: u64,
}

/// What a slide does to one clip and its neighbours.
struct Plan {
    /// The clip that slides.
    clip: u64,
    /// Where it goes.
    clock: Clock,
    /// The clip before it, and where it goes.
    before: Option<(u64, Clock)>,
    /// The clip after it, and where it goes.
    after: Option<(u64, Clock)>,
}

/// The **clips** a plan touches: the clip that slides and its
/// neighbours.
fn clocks_of(plan: &Plan) -> Vec<(u64, Clock)> {
    let mut clocks = vec![(plan.clip, plan.clock)];
    if let Some((before, clock)) = plan.before {
        clocks.push((before, clock));
    }
    if let Some((after, clock)) = plan.after {
        clocks.push((after, clock));
    }
    clocks
}

/// The **plan** of one clip that slides: where it goes, and where its
/// neighbours go.
///
/// The answer is the plan, or the **reason** the slide cannot be made.
fn plan(session: &Session, clip: u64, delta: i64, members: &[u64]) -> Result<Plan, String> {
    let Some(node) = session.document().node(clip) else {
        return Err(format!("there is no clip `{clip}`"));
    };
    let Some(start) = node.data.get("start").and_then(|start| start.as_u64()) else {
        return Err(format!("the clip `{clip}` does not say where it is"));
    };
    let Some(duration) = node.data.get("duration").and_then(|duration| duration.as_u64()) else {
        return Err(format!("the clip `{clip}` does not say how long it is"));
    };
    let source_in = node.data.get("source_in").and_then(|source_in| source_in.as_u64()).unwrap_or(0);
    let Some(track) = crate::track_of(session, clip) else {
        return Err(format!("the clip `{clip}` is on no track"));
    };
    if crate::is_locked(session, track) {
        return Err(format!("the track `{track}` is locked"));
    }
    let end = start + duration;
    let (before, after) = crate::neighbours_of(session, clip);
    // A **linked** neighbour is not a neighbour that may give way: the
    // linked clip slides by itself, and two clips that are linked and
    // neighbours on one track have no single answer.
    if let Some(neighbour) = &before
        && members.contains(&neighbour.id)
    {
        return Err(format!(
            "the clip `{}` is linked to the clip `{clip}` and they are neighbours on the same track",
            neighbour.id
        ));
    }
    if let Some(neighbour) = &after
        && members.contains(&neighbour.id)
    {
        return Err(format!(
            "the clip `{}` is linked to the clip `{clip}` and they are neighbours on the same track",
            neighbour.id
        ));
    }
    // The clip itself: it **moves** and its media is the media it was
    // showing.
    let moved = start as i64 + delta;
    if moved < 0 {
        return Err(format!("the clip would start at frame {}, and the timeline begins at frame 0", -moved));
    }
    let moved = moved as u64;
    // The clip **before** it: when it is next to the slid clip it gives
    // way, and when there is a gap between them the gap is the bound.
    let before = match before {
        Some(neighbour) => {
            if neighbour.end == start {
                let clock = crate::trim::clock_after(session, neighbour.id, "out", delta)
                    .map_err(|message| format!("the clip `{}`: {message}", neighbour.id))?;
                Some((neighbour.id, Clock::of(clock)))
            } else {
                if moved < neighbour.end {
                    return Err(format!(
                        "the clip would start at frame {moved}, and the clip before it ends at frame {}",
                        neighbour.end
                    ));
                }
                None
            }
        }
        None => None,
    };
    // The clip **after** it: the same, the other way.
    let after = match after {
        Some(neighbour) => {
            if neighbour.start == end {
                let clock = crate::trim::clock_after(session, neighbour.id, "in", delta)
                    .map_err(|message| format!("the clip `{}`: {message}", neighbour.id))?;
                Some((neighbour.id, Clock::of(clock)))
            } else {
                if moved + duration > neighbour.start {
                    return Err(format!(
                        "the clip would end at frame {}, and the clip after it starts at frame {}",
                        moved + duration,
                        neighbour.start
                    ));
                }
                None
            }
        }
        None => None,
    };
    Ok(Plan { clip, clock: Clock { start: moved, duration, source_in }, before, after })
}

impl Clock {
    /// The clock of a clip, from the three numbers the trim worked out.
    fn of(clock: (i64, i64, i64)) -> Clock {
        Clock {
            start: clock.0.max(0) as u64,
            duration: clock.1.max(0) as u64,
            source_in: clock.2.max(0) as u64,
        }
    }
}
