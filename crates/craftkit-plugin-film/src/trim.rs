//! **Trimming** a clip.
//!
//! A clip has two clocks — where it starts on the timeline and where it
//! begins in the media — and a trim moves one of the two **edges** of
//! it. The **in** edge is the head: trimming it changes how long the
//! clip is, where it begins in the media, and where it starts on the
//! timeline. The **out** edge is the tail: trimming it changes how long
//! the clip is and nothing else.
//!
//! The tree **clamps** a trim: it computes the biggest delta the trim
//! may have and uses that, so a drag that goes too far stops at the
//! bound and the editor never says that it did. This tree **refuses**:
//! a trim that would cross a bound is an error that says which bound
//! stopped it, and the clip is as it was.
//!
//! The bounds are four:
//!
//! - a clip is **at least one frame** long;
//! - a clip may not begin **before the media** it plays, and may not
//!   run **past the end** of it;
//! - a clip may not **overlap** the clip before it or the clip after
//!   it on the same track;
//! - a clip may not start **before the timeline**.
//!
//! # What it does not do
//!
//! - **The ripple is not here.** The tree's ripple trim moves the
//!   clips after the trimmed one, so that the gap closes. Here the
//!   trim moves the clip and nothing else, and the gap it leaves is a
//!   gap.
//! - **The clip's speed is here, and the trim counts the media
//!   through it.** The tree's trim counts the media a trim takes in the
//!   media's own ticks, which are the timeline's ticks multiplied by
//!   the speed (the module `speed`).
//! - **The frame hold is here, and the trim honours it.** A held clip
//!   has no out edge in the media and no head handle, so a trim of its
//!   in edge moves the clip and leaves the media where it is, and the
//!   media does not bound it — the tree's `media_len` is nothing when a
//!   clip holds a frame (the module `hold`).
//! - **The transitions are here, and the trim honours them.** A
//!   transition may not leave the two clips it joins, so a trim that
//!   made a clip shorter brings the transition back inside them and
//!   shortens it when they are now shorter than it (the module
//!   `transitions`).
//! - **The link is here, and the trim honours it.** The tree's linked
//!   clips are trimmed as **one edit**: every member of the group takes
//!   the same delta, and a member that cannot take it refuses the trim
//!   (the module `linking`).

use craftkit_derive::Command;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome, Session, Text};

/// The **clip**'s three numbers, as the payload holds them.
struct Clock {
    /// Where it starts on the timeline, in frames.
    start: u64,
    /// How long it is, in frames.
    duration: u64,
    /// Where in the media it begins, in frames.
    source_in: u64,
}

impl Clock {
    /// The clock of a clip, if the payload holds one.
    fn of(data: &serde_json::Value) -> Option<Clock> {
        Some(Clock {
            start: data.get("start")?.as_u64()?,
            duration: data.get("duration")?.as_u64()?,
            source_in: data.get("source_in")?.as_u64()?,
        })
    }
}

/// **Trim** one edge of a clip.
///
/// The **in** edge is the head: the clip gets shorter or longer, the
/// media it begins at moves, and the clip moves on the timeline with
/// it. The **out** edge is the tail: the clip gets shorter or longer
/// and stays where it is.
///
/// A trim that would cross a bound is **refused**, and the refusal
/// says which bound: a clip is at least one frame long, a clip may not
/// run past the end of the media or begin before it, a clip may not
/// overlap its neighbours, and a clip may not start before the
/// timeline.
#[derive(Command)]
#[command(id = "film.clip.trim", area = "film", layer = "film", icon = "scissors", mutates, undoable)]
pub struct TrimClip {
    /// The clip.
    pub clip: NodeRef,
    /// The edge that is trimmed: `in` or `out`.
    pub edge: Text,
    /// How much it is trimmed, in frames. A negative number trims the
    /// other way.
    pub delta: Number,
}

impl TrimClip {
    fn run(&self, session: &mut Session) -> Result<Outcome, EngineError> {
        let id = "film.clip.trim".to_string();
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
        // A clip that is **linked** is trimmed with its partners: the
        // tree trims the group as one edit, with the delta that every
        // member can take.
        let members = crate::linking::group(session, clip);
        let Some(clock) = Clock::of(&node.data) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` does not say where it is"),
            });
        };
        // A **locked** track cannot be edited.
        if let Some(track) = crate::track_of(session, clip)
            && crate::is_locked(session, track)
        {
            return Err(crate::locked_error(&id, track));
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
                message: "a trim of zero frames is not a trim".to_string(),
            });
        }
        // Every member of the group is trimmed by the same delta, and
        // a member that cannot take it refuses the whole trim.
        let mut planned: Vec<(u64, i64, i64, i64)> = Vec::new();
        for member in &members {
            let (start, duration, source_in) =
                trimmed(session, *member, &edge, delta).map_err(|message| EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{member}`: {message}"),
                })?;
            planned.push((*member, start, duration, source_in));
        }
        for (member, start, duration, source_in) in &planned {
            let Some(node) = session.document_mut()?.node_mut(*member) else {
                continue;
            };
            let source_in = serde_json::json!(*source_in as u64);
            node.data = crate::clocks_of(&node.data, *start as u64, *duration as u64, &source_in);
            crate::set_clip_name(node, *start as u64);
        }
        // The **transitions** that join the trimmed clips stay within
        // the two clips they join: a trim that made a clip shorter
        // brings a transition back inside the clips (the module
        // `transitions`).
        for member in &members {
            crate::transitions::keep_within_clips(session, *member)?;
        }
        let (start, duration, source_in) = planned
            .first()
            .map(|(_, start, duration, source_in)| (*start, *duration, *source_in))
            .unwrap_or((clock.start as i64, clock.duration as i64, clock.source_in as i64));
        let partners: Vec<u64> = members.iter().copied().filter(|member| *member != clip).collect();
        Ok(Outcome::changed(format!("the clip `{clip}` is trimmed by {delta} frames")).with_data(
            serde_json::json!({
                "id": clip,
                "start": start,
                "duration": duration,
                "source_in": source_in,
                "group": members,
                "partners": partners,
            }),
        ))
    }
}

/// The **new clock** of a clip trimmed by a delta: where it starts, how
/// long it is and where in the media it begins.
///
/// The answer is the clock, or the **reason** the trim cannot be made.
fn trimmed(session: &Session, clip: u64, edge: &str, delta: i64) -> Result<(i64, i64, i64), String> {
    let (start, duration, source_in) = clock_after(session, clip, edge, delta)?;
    // The **neighbours**: the clips on the same track, before and
    // after this one, taken from where the clip **is** and not from
    // where it would be.
    let old_start = session
        .document()
        .node(clip)
        .and_then(|node| node.data.get("start").and_then(|start| start.as_u64()))
        .unwrap_or(0);
    let Neighbours { before, after } = neighbours(session, clip, old_start);
    if let Some((_, end)) = before
        && start < end as i64
    {
        return Err(format!(
            "the clip would start at frame {start}, and the clip before it ends at frame {end}"
        ));
    }
    if let Some((next_start, _)) = after
        && start + duration > next_start as i64
    {
        return Err(format!(
            "the clip would end at frame {}, and the clip after it starts at frame {next_start}",
            start + duration
        ));
    }
    Ok((start, duration, source_in))
}

/// The **new clock** of a clip whose edge is trimmed by a delta, with
/// every bound but the **neighbours'**.
///
/// The neighbours are the clips on the same track, and whether the
/// trimmed clip runs into one of them is the caller's business: a
/// **trim** refuses it and a **roll** moves the neighbour with it (the
/// module `rolling`).
pub fn clock_after(session: &Session, clip: u64, edge: &str, delta: i64) -> Result<(i64, i64, i64), String> {
    let Some(node) = session.document().node(clip) else {
        return Err(format!("there is no clip `{clip}`"));
    };
    let Some(clock) = Clock::of(&node.data) else {
        return Err(format!("the clip `{clip}` does not say where it is"));
    };
    // A clip that plays **backwards** consumes the media at the
    // other end of its range.
    let reversed = crate::is_reversed(&node.data);
    // The **speed**: the media a frame of the clip plays is the
    // speed's worth of the media, so a trim of the in edge moves
    // the media by that much and not by one frame.
    let speed = crate::speed::speed_of(&node.data);
    // The **frame hold**: the frame of the media the clip holds to the
    // end of it. A held clip has no out edge in the media and no head
    // handle, so the trim moves the clip and not the media.
    let hold = crate::hold::hold_of(&node.data);
    // The **media**: the range of the item's frames a clip may play.
    let media = media_of(session, clip);
    let (media_start, media_marked) = match media {
        Some(media) => (media.start as i64, media.marked),
        None => (0, false),
    };

    // A clip that plays **backwards** consumes the media at the
    // other end of its range: its in edge takes the media *after*
    // its out point and its out edge takes the media *before* its
    // in point.
    // The **media** a trim of that many frames takes.
    let media_delta = (delta as f64 * speed).round() as i64;
    let (start, duration, source_in) = if edge == "in" {
        let duration = clock.duration as i64 - delta;
        // A **held** clip has no head handle: the media before its in
        // point is not there to be taken, so the in edge moves the clip
        // and leaves the media where it is.
        let source_in = if hold.is_some() || reversed {
            clock.source_in as i64
        } else {
            clock.source_in as i64 + media_delta
        };
        let start = clock.start as i64 + delta;
        if duration < 1 {
            return Err(format!(
                "the clip would be {duration} frames long, and a clip is at least one frame"
            ));
        }
        if source_in < media_start {
            return Err(if media_marked {
                format!(
                    "the clip would begin at frame {}, and the media begins at frame {media_start}",
                    source_in
                )
            } else {
                format!("the clip would begin at {} frames before the media begins", -source_in)
            });
        }
        if start < 0 {
            return Err(format!(
                "the clip would start at frame {}, and the timeline begins at frame 0",
                -start
            ));
        }
        (start, duration, source_in)
    } else {
        let duration = clock.duration as i64 + delta;
        if duration < 1 {
            return Err(format!(
                "the clip would be {duration} frames long, and a clip is at least one frame"
            ));
        }
        // A clip that plays backwards takes the media *before* its
        // in point when its out edge grows.
        let source_in = if reversed { clock.source_in as i64 - media_delta } else { clock.source_in as i64 };
        if source_in < media_start {
            return Err(if media_marked {
                format!(
                    "the clip would begin at frame {}, and the media begins at frame {media_start}",
                    source_in
                )
            } else {
                format!("the clip would begin at {} frames before the media begins", -source_in)
            });
        }
        (clock.start as i64, duration, source_in)
    };

    // The **media** the clip plays: its length multiplied by its
    // speed, as the tree counts it.
    // A **held** clip plays its material up to the hold and holds that
    // frame to the end, so the material it plays is **shorter** than the
    // clip. The tree does not bound a held clip by the media at all —
    // its `media_len` is nothing when a clip holds a frame — so the
    // bound is the hold and the media does not stop it.
    if let Some(media) = media
        && hold.is_none()
    {
        let played = crate::speed::media_frames(duration as u64, speed) as i64;
        if source_in + played > media.end as i64 {
            return Err(format!(
                "the clip would run to frame {} of the media, and {}",
                source_in + played,
                media.bound()
            ));
        }
    }
    Ok((start, duration, source_in))
}

/// The clips **around** a clip on its own track.
struct Neighbours {
    /// The clip before it: its start and its end.
    before: Option<(u64, u64)>,
    /// The clip after it: its start and its end.
    after: Option<(u64, u64)>,
}

/// The neighbours of a clip on its own track: the one before it and the
/// one after it, with their starts and their ends.
fn neighbours(session: &Session, clip: u64, start: u64) -> Neighbours {
    let mut before: Option<(u64, u64)> = None;
    let mut after: Option<(u64, u64)> = None;
    // The track the clip is on: the one that holds it.
    let Some(track) = session
        .document()
        .walk()
        .find(|node| node.kind == "film.track" && node.children.iter().any(|child| child.id == clip))
    else {
        return Neighbours { before, after };
    };
    for child in track.children.iter().filter(|child| child.kind == "film.clip" && child.id != clip) {
        let Some(other_start) = child.data.get("start").and_then(|start| start.as_u64()) else {
            continue;
        };
        let Some(duration) = child.data.get("duration").and_then(|duration| duration.as_u64()) else {
            continue;
        };
        let other_end = other_start + duration;
        if other_end <= start && before.map(|(_, end)| other_end > end).unwrap_or(true) {
            before = Some((other_start, other_end));
        } else if other_start >= start && after.map(|(next, _)| other_start < next).unwrap_or(true) {
            after = Some((other_start, other_end));
        }
    }
    Neighbours { before, after }
}

/// The **media** the clip plays: the range of its frames, if it is
/// known.
///
/// An item that has no duration does not bound the clip.
fn media_of(session: &Session, clip: u64) -> Option<crate::MediaRange> {
    crate::media_range_of(session, clip)
}
