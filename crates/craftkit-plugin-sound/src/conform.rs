//! The **conform** of a clip: making the material fit the tempo.
//!
//! A recording is made at some tempo — the one the band was playing —
//! and a document may be at another. The tree's sound editor has a
//! command that **conforms** a clip to the document's tempo: the clip
//! is played at the rate that makes the material fit, and the clip is
//! as long as the material then lasts.
//!
//! # What the conform needs to know
//!
//! The tempo the material is at. The tree keeps it in the **edit
//! state** — in a map of values, under a key made of the clip's id —
//! and the length the clip had before it was ever conformed, in
//! another key beside it. So the two facts about a clip live outside
//! the clip, in a map that only the tree's editor knows about.
//!
//! Here the tempo the material is at is a **field of the clip**, and
//! the length it had before is not kept at all: the material's length
//! is the clip's length divided by its stretch, and that is worked out
//! every time.
//!
//! # The beat follows the audio
//!
//! The tree's conform leaves the clip's sync point where it was, and
//! the material's downbeat moves away from it — the clip is twice as
//! long and the beat is still where it was. Here the beat moves with
//! the material, the way it moves when the clip is trimmed (§ the beat
//! of a clip): the beat is a place in the clip, and the clip's
//! timeline is what changed.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{EngineError, NodeRef, Number, Outcome};

use crate::locks;
use crate::{ClipPayload, session_of, tempo_map};

/// The slowest a material may be, in beats a minute.
///
/// The tree's own bound: a tempo below this is not a tempo anybody
/// plays at, and a clip conformed to it would be longer than the
/// document.
pub(crate) const MIN_SOURCE_BPM: f64 = 5.0;

/// The fastest a material may be, in beats a minute.
pub(crate) const MAX_SOURCE_BPM: f64 = 1000.0;

commands! {
    /// **Conform** a clip to the tempo of the document.
    ///
    /// The clip is played at the rate that makes the material fit: a
    /// material recorded at 120 beats a minute in a document at 60 is
    /// played twice as slowly, so that the beats of the recording fall on
    /// the beats of the document.
    ///
    /// The tempo of the material is remembered in the clip: the first
    /// conform takes the tempo of the document where the clip starts, and
    /// a later conform uses the remembered one — so a document whose tempo
    /// changes can be conformed again and the material still fits.
    ///
    /// A clip whose **content** is locked is refused: a conform changes
    /// the sound.
    sound.clip.conform {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Clip", "Conform to Tempo"),
        params: {
            /// The clip to conform.
            clip: NodeRef,
            /// The tempo of the material, in beats a minute.
            ///
            /// Without it, the tempo the clip remembers, and the tempo of the
            /// document where the clip starts if it does not remember one.
            source_bpm: Option<Number>,
        },
        run: {
            let id = "sound.clip.conform".to_string();
            let clip = self.clip.0;
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
            let mut payload = ClipPayload::from_json(&node.data)
                .map_err(|error| EngineError::Failed { id: id.clone(), message: error })?;
            // A clip whose **content** is locked is finished.
            if locks::edit_is_locked(&payload) {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip}` is locked: its sound is finished"),
                });
            }
            // A clip with no **sound** has nothing to conform.
            if payload.source.is_none() {
                return Err(EngineError::Failed {
                    id: id.clone(),
                    message: format!("the clip `{clip}` has no sound to conform"),
                });
            }
            // The tempo of the **material**: what the caller says, or what
            // the clip remembers, or the tempo of the document where the
            // clip starts.
            let map = tempo_map(session.document());
            let rate = session_of(session.document()).sample_rate;
            let now = map.tempo_at_tick(map.samples_to_ticks(payload.start, rate));
            let source_bpm = match self.source_bpm {
                Some(bpm) => {
                    if !bpm.0.is_finite() || bpm.0 < MIN_SOURCE_BPM || bpm.0 > MAX_SOURCE_BPM {
                        return Err(EngineError::Failed {
                            id: id.clone(),
                            message: format!(
                                "the tempo of the material is from {MIN_SOURCE_BPM} to {MAX_SOURCE_BPM} beats a minute, and {} is not",
                                bpm.0
                            ),
                        });
                    }
                    bpm.0
                }
                None => payload.source_bpm.unwrap_or(now),
            };
            // The **ratio**: how much slower the material is played, so that
            // it fits the tempo of the document.
            let ratio = source_bpm / now;
            let stretch = crate::stretch_of(ratio, &id)?;
            // The material's length is the clip's length divided by its
            // stretch, and the clip is as long as the material lasts at the
            // new rate.
            let material = (payload.length as f64 / payload.stretch).round() as i64;
            let length = ((material as f64 * ratio).round() as i64).max(1);
            // The **beat** follows the audio: the clip's timeline is what
            // changed, and the beat is a place in it.
            let scale = if payload.length > 0 { length as f64 / payload.length as f64 } else { 1.0 };
            let beat = payload.sync.map(|beat| ((beat as f64 * scale).round() as i64).clamp(0, length));
            payload.source_bpm = Some(source_bpm);
            payload.stretch = stretch;
            payload.length = length;
            payload.sync = beat;
            payload.fade_in.len = payload.fade_in.len.min(length);
            payload.fade_out.len = payload.fade_out.len.min(length);
            if let Some(node) = session.document_mut()?.node_mut(clip) {
                node.data = payload.to_json();
            }
            Ok(Outcome::changed(format!("the clip `{clip}` is conformed to {now} beats a minute")).with_data(
                serde_json::json!({
                    "id": clip,
                    "source_bpm": source_bpm,
                    "bpm": now,
                    "stretch": stretch,
                    "length": length,
                }),
            ))

        }
    }
}
