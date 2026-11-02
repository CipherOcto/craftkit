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
//! - **The clip's speed is not here.** The tree's trim counts the
//!   media a trim takes in the media's own ticks, which are the
//!   timeline's ticks multiplied by the speed. Here a frame is a frame
//!   in both clocks.

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
        let Some(clock) = Clock::of(&node.data) else {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!("the clip `{clip}` does not say where it is"),
            });
        };
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
        // The **neighbours**: the clips on the same track, before and
        // after this one.
        let Neighbours { before, after } = neighbours(session, clip, clock.start);
        // The **media**: how long the item the clip plays is.
        let media = media_of(session, clip);

        let (start, duration, source_in) = if edge == "in" {
            let duration = clock.duration as i64 - delta;
            let source_in = clock.source_in as i64 + delta;
            let start = clock.start as i64 + delta;
            if duration < 1 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip would be {duration} frames long, and a clip is at least one frame"
                    ),
                });
            }
            if source_in < 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip would begin at {} frames before the media begins", -source_in),
                });
            }
            if start < 0 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip would start at frame {}, and the timeline begins at frame 0",
                        -start
                    ),
                });
            }
            (start, duration, source_in)
        } else {
            let duration = clock.duration as i64 + delta;
            if duration < 1 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip would be {duration} frames long, and a clip is at least one frame"
                    ),
                });
            }
            (clock.start as i64, duration, clock.source_in as i64)
        };

        if let Some(media) = media
            && source_in + duration > media as i64
        {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the clip would run to frame {} of the media, and the media is {media} frames long",
                    source_in + duration
                ),
            });
        }
        if let Some((_, end)) = before
            && start < end as i64
        {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the clip would start at frame {start}, and the clip before it ends at frame {end}"
                ),
            });
        }
        if let Some((next_start, _)) = after
            && start + duration > next_start as i64
        {
            return Err(EngineError::Failed {
                id: id.clone(),
                message: format!(
                    "the clip would end at frame {}, and the clip after it starts at frame {next_start}",
                    start + duration
                ),
            });
        }

        let data = serde_json::json!({
            "start": start,
            "duration": duration,
            "source_in": source_in,
        });
        if let Some(node) = session.document_mut()?.node_mut(clip) {
            node.data = data;
            node.name = format!("clip at {start}");
        }
        Ok(Outcome::changed(format!("the clip `{clip}` is trimmed by {delta} frames")).with_data(
            serde_json::json!({
                "id": clip,
                "start": start,
                "duration": duration,
                "source_in": source_in,
            }),
        ))
    }
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

/// How long the **media** the clip plays is, if it is known.
///
/// The item's own duration is the media's length; an item that has none
/// does not bound the clip.
fn media_of(session: &Session, clip: u64) -> Option<u64> {
    let item = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == clip)
        .map(|relation| relation.to)?;
    let node = session.document().node(item)?;
    let duration = node.data.get("duration")?.as_u64()?;
    if duration == 0 { None } else { Some(duration) }
}
