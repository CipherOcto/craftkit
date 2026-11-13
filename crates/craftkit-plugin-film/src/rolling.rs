//! The **roll** of a cut.
//!
//! A cut is the place where one clip ends and the next begins, and a
//! **roll** moves it: the clip before the cut gets longer and the clip
//! after it gets shorter, and the cut itself is somewhere else.
//!
//! It is the third way of moving the boundary between two clips, and
//! the other two are not like it: a **trim** moves one edge of one
//! clip and leaves its neighbour alone, and a **move** takes a clip
//! away from its place and puts it somewhere else. A roll moves the
//! boundary and **both** clips go with it.
//!
//! The tree **clamps** a roll: it works out the biggest delta the roll
//! may have, uses that, and the editor never says that it did. This
//! tree **refuses**: a roll that would cross a bound is an error that
//! says which bound, and the clips are as they were.
//!
//! The bounds are the **trim's** bounds, on both sides at once:
//!
//! - a clip is at least **one frame** long;
//! - a clip may not run **past the end** of its media, and the media
//!   a clip plays is its length multiplied by its **speed** (the
//!   module `speed`);
//! - a clip may not begin **before the media** it plays.
//!
//! And the **link** is here, and the roll honours it: the tree rolls
//! the linked cuts as well, so that the picture and the sound stay in
//! step, and this tree does the same (the module `linking`).
//!
//! # What the port found
//!
//! - **The tree's linked roll is clamped and this one is refused.** The
//!   tree's `clamp_roll_linked` tries the roll on a copy of the
//!   sequence for every pair and takes the delta that every pair can
//!   take, so a linked roll that cannot go the whole way goes part of
//!   the way and says nothing. Here every pair takes the same delta,
//!   and a pair that cannot take it refuses the roll — the rule the
//!   trim of a linked group already had (§299).
//!
//! # What it does not do
//!
//! - **The tree's linked selection is not here.** The tree rolls the
//!   linked cuts only when its linked selection is on, and this tree
//!   has no such a switch: the link is a fact of the clip and the roll
//!   honours it.
//! - **The tree's time remapping is not here**, and neither is its
//!   remapping of the media: a roll moves the cut and the media with
//!   it, and the media is the media the clip plays.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session};

commands! {
    /// **Roll** the cut between two clips.
    ///
    /// The clip before the cut gets longer and the clip after it gets
    /// shorter, and the media of both follows the cut.
    ///
    /// The roll is refused when it would cross a bound: the tree clamps
    /// such a roll and says nothing, and here the refusal says which
    /// bound.
    film.clip.roll {
        area: "film",
        layer: "film",
        icon: "split",
        mutates,
        undoable,
        params: {
            /// The clip before the cut.
            left: NodeRef,
            /// The clip after the cut.
            right: NodeRef,
            /// How far the cut moves, in frames. A negative number moves it
            /// the other way.
            delta: Number,
        },
        run: {
            let id = "film.clip.roll".to_string();
            let left = self.left.0;
            let right = self.right.0;
            for clip in [left, right] {
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
                    message: "a roll of zero frames is not a roll".to_string(),
                });
            }
            // The two clips must be **next to each other**: a roll is a
            // cut, and a cut is between two clips on one track.
            let track = crate::track_of(session, left).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{left}` is on no track"),
            })?;
            if crate::track_of(session, right) != Some(track) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{right}` is not on the track of the clip `{left}`"),
                });
            }
            // A **locked** track cannot be edited.
            if crate::is_locked(session, track) {
                return Err(crate::locked_error(&id, track));
            }
            // The **cut**: where the clip before it ends and the clip
            // after it begins.
            let cut = cut_of(session, left).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{left}` does not say where it is"),
            })?;
            let right_start = start_of(session, right).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{right}` does not say where it is"),
            })?;
            if cut != right_start {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip `{left}` ends at frame {cut}, and the clip `{right}` begins at frame {right_start}"
                    ),
                });
            }
            // The **link** is here: the tree rolls the linked cuts as
            // well, so that the picture and the sound stay in step.
            let pairs = pairs_to_roll(session, left, right, cut);
            // Every pair takes the same delta, and a pair that cannot take
            // it refuses the whole roll: the tree scales the roll of the
            // group down and says nothing.
            let mut planned: Vec<Planned> = Vec::new();
            for (before, after) in &pairs {
                let before_clock =
                    crate::trim::clock_after(session, *before, "out", delta).map_err(|message| {
                        EngineError::Failed { id: id.clone(), message: format!("the clip `{before}`: {message}") }
                    })?;
                let after_clock = crate::trim::clock_after(session, *after, "in", delta).map_err(|message| {
                    EngineError::Failed { id: id.clone(), message: format!("the clip `{after}`: {message}") }
                })?;
                planned.push(Planned {
                    before: *before,
                    after: *after,
                    before_clock: Clock::of(before_clock),
                    after_clock: Clock::of(after_clock),
                });
            }
            for planned in &planned {
                for (clip, clock) in
                    [(planned.before, planned.before_clock), (planned.after, planned.after_clock)]
                {
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
            for planned in &planned {
                crate::transitions::follow_cut(session, planned.before, planned.after, delta)?;
                crate::transitions::keep_within_clips(session, planned.before)?;
                crate::transitions::keep_within_clips(session, planned.after)?;
            }
            let rolled: Vec<u64> = pairs.iter().flat_map(|(before, after)| [*before, *after]).collect();
            Ok(Outcome::changed(format!("the cut at frame {cut} is rolled by {delta} frames")).with_data(
                serde_json::json!({
                    "left": left,
                    "right": right,
                    "cut": cut,
                    "delta": delta,
                    "pairs": pairs.iter().map(|(before, after)| serde_json::json!({
                        "left": before,
                        "right": after,
                    })).collect::<Vec<_>>(),
                    "clips": rolled,
                }),
            ))

        }
    }
}

/// The three numbers of a clip: where it starts, how long it is and
/// where in the media it begins.
#[derive(Clone, Copy)]
struct Clock {
    /// Where it starts on the timeline.
    start: u64,
    /// How long it is.
    duration: u64,
    /// Where in the media it begins.
    source_in: u64,
}

impl Clock {
    /// The clock of a clip, from the three numbers a trim worked out.
    fn of(clock: (i64, i64, i64)) -> Clock {
        Clock {
            start: clock.0.max(0) as u64,
            duration: clock.1.max(0) as u64,
            source_in: clock.2.max(0) as u64,
        }
    }
}

/// The new clocks of a pair of clips whose cut is rolled.
struct Planned {
    /// The clip before the cut.
    before: u64,
    /// The clip after the cut.
    after: u64,
    /// The clock of the clip before the cut.
    before_clock: Clock,
    /// The clock of the clip after the cut.
    after_clock: Clock,
}

/// Where a clip **starts**.
fn start_of(session: &Session, clip: u64) -> Option<u64> {
    let node = session.document().node(clip)?;
    node.data.get("start").and_then(|start| start.as_u64())
}

/// Where a clip **ends**: the cut it makes with the clip after it.
fn cut_of(session: &Session, clip: u64) -> Option<u64> {
    let node = session.document().node(clip)?;
    let start = node.data.get("start").and_then(|start| start.as_u64())?;
    let duration = node.data.get("duration").and_then(|duration| duration.as_u64())?;
    Some(start + duration)
}

/// The **pairs** of clips that roll together: the pair that was named,
/// and the **linked** pairs at the same cut.
///
/// The tree's rule: a pair of clips on another track rolls with the
/// pair that was named when the first clip of the pair is **linked** to
/// the first clip of the named pair and the second to the second, and
/// the cut is the same.
fn pairs_to_roll(session: &Session, left: u64, right: u64, cut: u64) -> Vec<(u64, u64)> {
    let mut pairs = vec![(left, right)];
    let left_link = session.document().node(left).and_then(|node| crate::linking::link_of(&node.data));
    let right_link = session.document().node(right).and_then(|node| crate::linking::link_of(&node.data));
    if left_link.is_none() && right_link.is_none() {
        return pairs;
    }
    let track = crate::track_of(session, left);
    for node in session.document().walk().filter(|node| node.kind == "film.track") {
        // A **locked** track cannot be edited, so its cut does not
        // roll: the tree's rule.
        if Some(node.id) == track || crate::is_locked(session, node.id) {
            continue;
        }
        let mut clips: Vec<(u64, u64, u64, Option<u64>)> = node
            .children
            .iter()
            .filter(|child| child.kind == "film.clip")
            .map(|child| {
                let start = child.data.get("start").and_then(|start| start.as_u64()).unwrap_or(0);
                let duration = child.data.get("duration").and_then(|duration| duration.as_u64()).unwrap_or(0);
                (child.id, start, duration, crate::linking::link_of(&child.data))
            })
            .collect();
        clips.sort_by_key(|(_, start, _, _)| *start);
        for pair in clips.windows(2) {
            let (before, before_start, before_duration, before_link) = pair[0];
            let (after, after_start, _, after_link) = pair[1];
            if before_start + before_duration != cut || after_start != cut {
                continue;
            }
            if before_link != left_link || after_link != right_link {
                continue;
            }
            if before == left && after == right {
                continue;
            }
            pairs.push((before, after));
        }
    }
    pairs
}
