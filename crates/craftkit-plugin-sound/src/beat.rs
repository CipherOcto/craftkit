//! The **beat** inside a clip: where the beat falls within it.
//!
//! The tree's sound editor has a **sync point**: a place inside a clip
//! where the beat falls, and the thing a clip is lined up by. A clip
//! recorded against another one still lines up because both of them
//! say where their beat is.
//!
//! The tree keeps the sync point as a **number** in the clip, and the
//! number zero means *none*. That is one number saying two things: a
//! clip whose beat is at its very first sample cannot say so, because
//! zero is taken.
//!
//! Here the beat is an **option**: no beat, or a beat at some sample
//! in the clip. A beat of zero is a beat at the clip's head, and it is
//! not the same as no beat at all.
//!
//! # Where the beat sits
//!
//! The beat is an offset in **samples from the clip's start** — the
//! clip's own timeline, and not the source's. Two reasons:
//!
//! - it is where a person sees it: the clip's bar, and a mark on it;
//! - a stretched clip plays its source more slowly, so a source
//!   sample and a timeline sample are not the same distance apart, and
//!   a beat the person can see must be in the timeline's units.
//!
//! # What moves the beat
//!
//! The beat is a place in the clip's **audio**, and the audio does not
//! move when the clip is trimmed: trimming the head of a clip takes
//! the first samples away and leaves the rest where it was. So the
//! beat moves with the trim — it comes nearer the clip's head by the
//! number of samples that were cut — and it is **dropped** when the
//! trim leaves it outside the clip, because a beat past the end of
//! the clip is not a beat of that clip.
//!
//! # What this does not do
//!
//! - **The beat is not put on a grid.** The tree's editor quantises a
//!   clip to a grid — a division of the bar — and the grid is a setting
//!   of the session. There is a grid here too, and it puts a clip's
//!   beat on a grid line; but the beat itself does not know about the
//!   grid, and a beat that is not on a line is still a beat.
//! - **The beat is not shown.** A sound document is not a picture
//!   here (§ the tree's own notes), so the beat is a fact in the
//!   document and not a mark on a bar.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome};

use crate::{ClipPayload, session_of, tempo_map};

commands! {
    /// Set the **beat** of a clip.
    ///
    /// The beat is where the beat falls inside the clip, and it is set by
    /// saying where it falls: a place on the timeline.
    ///
    /// The place must be **inside** the clip. The tree's editor takes the
    /// place and **clamps** it into the clip, so a beat asked for outside
    /// the clip quietly lands at its edge; here the place outside the clip
    /// is refused, because a beat that was moved without being asked for is
    /// a beat the caller did not want.
    ///
    /// Without a place, the beat goes where the selection starts — the
    /// tree's editor does the same, and a selection of the clip's own
    /// first sample is a beat at the clip's head.
    sound.clip.sync.set {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Clip", "Sync Point"),
        params: {
            /// The clip to set the beat in.
            ///
            /// Without it, the clip that is selected.
            clip: Option<NodeRef>,
            /// Where the beat falls, in samples on the timeline.
            ///
            /// Without it, where the selection starts.
            at: Option<Number>,
        },
        run: {
            let id = "sound.clip.sync.set".to_string();
            let (clip, span) = match (self.clip, crate::selection_span(session)) {
                (Some(clip), span) => (clip.0, span.map(|(_, span)| span)),
                (None, Some((node, span))) => (node, Some(span)),
                (None, None) => {
                    return Err(EngineError::Failed {
                        id,
                        message: "which clip, and where in it does the beat fall?".to_string(),
                    });
                }
            };
            let node = session.document().node(clip).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            // Where the beat falls: the place the caller gave, or the start
            // of the selection, or the clip's own head.
            let at = match (self.at, span) {
                (Some(at), _) => at.0,
                (None, Some((from, _))) => from,
                (None, None) => payload.start as f64,
            };
            if !at.is_finite() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: "a place is made of numbers".to_string(),
                });
            }
            // The beat is **inside** the clip, and a place outside it is
            // refused rather than pulled to the edge.
            if at < payload.start as f64 || at > (payload.start + payload.length) as f64 {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!(
                        "the clip runs from {} to {} and the beat cannot fall at {at}",
                        payload.start,
                        payload.start + payload.length
                    ),
                });
            }
            let beat = at.round() as i64 - payload.start;
            let mut payload = payload;
            payload.sync = Some(beat);
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data = payload.to_json();
            }
            Ok(Outcome::changed(format!("the beat of the clip `{clip}` falls at {beat}"))
                .with_data(serde_json::json!({ "id": clip, "sync": beat, "at": at.round() as i64 })))

        }
    }

    /// Take the **beat** out of a clip.
    ///
    /// A clip with no beat is not changed, and a command that changes
    /// nothing leaves no undo step behind: the tree's editor does the same,
    /// and a caller that asks twice gets the same answer both times.
    sound.clip.sync.remove {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Clip", "No Sync Point"),
        params: {
            /// The clip to take the beat out of.
            ///
            /// Without it, the clip that is selected.
            clip: Option<NodeRef>,
        },
        run: {
            let id = "sound.clip.sync.remove".to_string();
            let clip = match (self.clip, crate::selection_span(session)) {
                (Some(clip), _) => clip.0,
                (None, Some((node, _))) => node,
                (None, None) => {
                    return Err(EngineError::Failed { id, message: "which clip?".to_string() });
                }
            };
            let node = session.document().node(clip).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            if payload.sync.is_none() {
                return Ok(Outcome::new(format!("the clip `{clip}` has no beat")));
            }
            let mut payload = payload;
            payload.sync = None;
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data = payload.to_json();
            }
            Ok(Outcome::changed(format!("the clip `{clip}` has no beat"))
                .with_data(serde_json::json!({ "id": clip, "sync": serde_json::Value::Null })))

        }
    }

    /// Read the **beat** of a clip.
    ///
    /// The beat is in the clip's payload and the **place** of it is
    /// derived: where it falls on the timeline, and where that is in the
    /// bar — the bar and the beat come from the tempo map, and they are
    /// worked out every time rather than kept.
    sound.clip.sync.read {
        area: "sound",
        layer: "sound",
        icon: "list",
        params: {
            /// The clip to read.
            ///
            /// Without it, the clip that is selected.
            clip: Option<NodeRef>,
        },
        run: {
            let id = "sound.clip.sync.read".to_string();
            let clip = match (self.clip, crate::selection_span(session)) {
                (Some(clip), _) => clip.0,
                (None, Some((node, _))) => node,
                (None, None) => {
                    return Err(EngineError::Failed { id, message: "which clip?".to_string() });
                }
            };
            let node = session.document().node(clip).ok_or_else(|| EngineError::Failed {
                id: id.clone(),
                message: format!("there is no node `{clip}`"),
            })?;
            if node.kind != "sound.clip" {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the node `{clip}` is a `{}` and not a clip", node.kind),
                });
            }
            let payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            let Some(beat) = payload.sync else {
                return Ok(Outcome::new(format!("the clip `{clip}` has no beat"))
                    .with_data(serde_json::json!({ "id": clip, "sync": serde_json::Value::Null })));
            };
            // The **place** of the beat, and the **bar** it falls in: the
            // tempo map says where a time is in the score, and it is
            // worked out here rather than kept.
            let at = payload.start + beat;
            let rate = session_of(session.document()).sample_rate;
            let map = tempo_map(session.document());
            let position = map.samples_to_ticks(at, rate).max(0);
            let bar_beat = map.bar_beat_at_tick(position);
            Ok(Outcome::new(format!("the beat of the clip `{clip}` falls at {beat}")).with_data(
                serde_json::json!({
                    "id": clip,
                    "sync": beat,
                    "at": at,
                    "position": format!("{}.{}.{}", bar_beat.bar, bar_beat.beat, bar_beat.tick),
                }),
            ))

        }
    }
}
