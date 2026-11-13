//! The **gain** of a clip: how loud the clip is.
//!
//! The tree's sound editor has a gain on every clip, and a command to
//! set it, a pair of commands to nudge it and a third to bypass it. The
//! v2 sound battery had a **gain field** in the clip's payload and
//! nothing that set it — every clip was played at unity, whatever the
//! document said.
//!
//! # What the gain is
//!
//! A level in **decibels**, from minus one hundred and forty-four to
//! thirty-six. It is the level of the clip itself: the track's trim
//! and the track's automation are other things, and a clip that is
//! loud and a track that is quiet are two different facts.
//!
//! # The tree clamps and this tree refuses
//!
//! The tree's `clip.gain` takes any number and **clamps** it into the
//! range: a gain of a hundred decibels becomes thirty-six, and a gain
//! of minus a thousand becomes minus one hundred and forty-four. A
//! gain asked for as a hundred decibels is not a gain, so here it is
//! **refused** (§ the kit's own rule: a bound is not negotiable, and a
//! refusal never truncates).
//!
//! # The tree's bypass is a reset and this one is a bypass
//!
//! The tree's `clip.gain_bypass` sets the clip's gain to zero and
//! throws the old gain away — the clip plays as it is, and the gain
//! that was there is gone for good. That is a **reset**, and the name
//! says otherwise.
//!
//! Here the gain is **bypassed** and **kept**: the clip plays as it is
//! while the bypass is on, and the gain comes back when the bypass is
//! taken off. A flag says it, and the flag is in the clip's payload,
//! so a file says whether the gain was in the path.

use craftkit_derive::commands;

use craftkit_sdk::serde_json;
use craftkit_sdk::{Bool, EngineError, Number, Outcome};

use crate::ClipPayload;

/// The quietest a clip may be, in decibels.
///
/// Minus one hundred and forty-four is the tree's own floor: it is
/// below anything anybody can hear, and a clip that is quieter than
/// that is silent.
pub(crate) const MIN_GAIN_DB: f64 = -144.0;

/// The loudest a clip may be, in decibels.
pub(crate) const MAX_GAIN_DB: f64 = 36.0;

commands! {
    /// **Set the gain** of clips.
    ///
    /// The gain is set absolutely — `db` — or by a change — `delta_db` —
    /// and one of the two is needed: a command with neither does not know
    /// what the gain should be.
    ///
    /// A gain outside the range is **refused** and the clips keep the gain
    /// they had: the tree's command clamps such a gain into the range, and
    /// a gain asked for as a hundred decibels is not a gain. A **change**
    /// that would take a clip out of the range is refused as well, because
    /// the change is what was asked for and the result is not a gain.
    sound.clip.gain {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Clip", "Gain"),
        params: {
            /// The clips to set the gain of: their ids.
            clips: serde_json::Value,
            /// The gain, in decibels.
            db: Option<Number>,
            /// The change in the gain, in decibels.
            delta_db: Option<Number>,
        },
        run: {
            let id = "sound.clip.gain".to_string();
            let clips = crate::clip_ids_of(&self.clips, &id)?;
            let (absolute, delta) = match (self.db, self.delta_db) {
                (Some(db), _) => (Some(db.0), None),
                (None, Some(delta)) => (None, Some(delta.0)),
                (None, None) => {
                    return Err(EngineError::Failed {
                        id,
                        message: "the gain in decibels, or the change in it, is needed".to_string(),
                    });
                }
            };
            if let Some(value) = absolute
                && !value.is_finite()
            {
                return Err(EngineError::Failed { id, message: "a gain is a number of decibels".to_string() });
            }
            if let Some(value) = delta
                && !value.is_finite()
            {
                return Err(EngineError::Failed {
                    id,
                    message: "a change in the gain is a number of decibels".to_string(),
                });
            }
            let mut set = 0;
            for clip in &clips {
                let node = session.document().node(*clip).ok_or_else(|| EngineError::Failed {
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
                let gain = match absolute {
                    Some(gain) => gain,
                    None => payload.gain_db + delta.unwrap_or(0.0),
                };
                if !(MIN_GAIN_DB..=MAX_GAIN_DB).contains(&gain) {
                    return Err(EngineError::Failed {
                        id: id.clone(),
                        message: format!(
                            "a gain is from {MIN_GAIN_DB} to {MAX_GAIN_DB} decibels, and {gain} is not one"
                        ),
                    });
                }
                if gain == payload.gain_db {
                    continue;
                }
                payload.gain_db = gain;
                if let Some(node) = session.document_mut()?.node_mut(*clip) {
                    node.data = payload.to_json();
                }
                set += 1;
            }
            if set == 0 {
                return Ok(Outcome::new("the clips are at that gain already"));
            }
            Ok(Outcome::changed(format!("the gain of {set} clips is set"))
                .with_data(serde_json::json!({ "clips": set })))

        }
    }

    /// **Bypass the gain** of clips.
    ///
    /// While the bypass is on, the clip plays as it is: the gain is not in
    /// the path. The gain itself is **kept**, so taking the bypass off
    /// brings it back — the tree's bypass throws the gain away, which is a
    /// reset and not a bypass.
    ///
    /// Without a bypass given, the bypass is **toggled**.
    sound.clip.gain.bypass {
        area: "sound",
        layer: "sound",
        icon: "sound",
        mutates,
        undoable,
        menu: ("Clip", "Bypass Gain"),
        params: {
            /// The clips to bypass the gain of: their ids.
            clips: serde_json::Value,
            /// Whether the gain is bypassed. Without it, the bypass is toggled.
            bypassed: Option<Bool>,
        },
        run: {
            let id = "sound.clip.gain.bypass".to_string();
            let clips = crate::clip_ids_of(&self.clips, &id)?;
            let mut set = 0;
            for clip in &clips {
                let node = session.document().node(*clip).ok_or_else(|| EngineError::Failed {
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
                let bypassed = self.bypassed.map(|bypassed| bypassed.0).unwrap_or(!payload.gain_bypassed);
                if bypassed == payload.gain_bypassed {
                    continue;
                }
                payload.gain_bypassed = bypassed;
                if let Some(node) = session.document_mut()?.node_mut(*clip) {
                    node.data = payload.to_json();
                }
                set += 1;
            }
            if set == 0 {
                return Ok(Outcome::new("the clips are as they are"));
            }
            Ok(Outcome::changed(format!("the gain of {set} clips is bypassed"))
                .with_data(serde_json::json!({ "clips": set })))

        }
    }
}
