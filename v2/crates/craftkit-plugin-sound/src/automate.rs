//! The **automation**: the lanes of a track, and what they do to its
//! sound.
//!
//! A lane is a curve on the track's row (§127) and it is **derived**:
//! the breakpoints are in the document and the curve is worked out every
//! time somebody asks. This module is the other half of that — the
//! value of a lane **at a time**, and the gain and the balance that
//! value means.
//!
//! # The parameters
//!
//! The parameter of a lane is a **string**, so anybody may automate
//! anything a node's payload names. Three are worked out here, because
//! a sound is made of them:
//!
//! - **volume**, in **decibels** — like the gain of a clip (§ the
//!   clip's payload);
//! - **pan**, between minus one and one — the balance between the
//!   channels, and a **mono** sound is not balanced, so a lane of pan
//!   over a mono track does nothing;
//! - **mute** — a value of one or more mutes the track, and a value of
//!   less than one does not.
//!
//! A lane of any other parameter is **drawn** and is not **heard**: the
//! kit does not know what the parameter means, and it does not guess.

use crate::mixer::LanePayload;
use craftkit_sdk::{Document, Node};

/// The **automation** of a track: its lanes, by parameter.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Automation {
    /// The volume, in decibels, over time.
    pub volume: Option<Vec<(i64, f64)>>,
    /// The pan, between minus one and one, over time.
    pub pan: Option<Vec<(i64, f64)>>,
    /// The mute, over time: one mutes and less does not.
    pub mute: Option<Vec<(i64, f64)>>,
    /// The track's own **mute**: whether it is silenced when nothing
    /// automates it.
    ///
    /// The tree's mixer keeps a mute of its own beside the mute lane,
    /// and the lane is the mute's default: a lane with breakpoints
    /// overrides it, and a lane without one is the mute.
    pub muted: bool,
    /// The track's **pan**, between minus one and one: the balance of
    /// the track when nothing automates it.
    ///
    /// The tree's pan is the **default** of the pan lane, as its fader
    /// is the default of the volume lane: a lane with breakpoints
    /// overrides it, and a lane without one is the pan.
    pub pan_value: f64,
    /// The track's **fader**, in decibels: the level of the track when
    /// nothing automates it.
    ///
    /// The tree's fader is the **default** of the volume lane: a lane
    /// with breakpoints overrides it, and a lane without one is the
    /// fader. It is here and not in the lane because it belongs to the
    /// track and not to the curve.
    pub fader_db: f64,
}

/// The automation of a track: the lanes under it.
pub fn automation_of(document: &Document, track: &Node) -> Automation {
    let mut automation = Automation {
        // The **fader** and the **pan** are the track's own: a clip
        // has neither, and a track's are what it is heard at when no
        // lane says else.
        fader_db: crate::TrackPayload::from_json(&track.data).map(|payload| payload.fader_db).unwrap_or(0.0),
        pan_value: crate::TrackPayload::from_json(&track.data).map(|payload| payload.pan).unwrap_or(0.0),
        muted: crate::TrackPayload::from_json(&track.data).map(|payload| payload.mute).unwrap_or(false),
        ..Automation::default()
    };
    for lane in document.walk().filter(|node| node.kind == "sound.lane") {
        // A lane automates the track it is under.
        if crate::timeline::parent_of(document, lane.id).map(|parent| parent.id) != Some(track.id) {
            continue;
        }
        let Ok(payload) = LanePayload::from_json(&lane.data) else { continue };
        if payload.points.is_empty() {
            continue;
        }
        match payload.param.as_str() {
            "volume" => automation.volume = Some(payload.points),
            "pan" => automation.pan = Some(payload.points),
            "mute" => automation.mute = Some(payload.points),
            _ => {}
        }
    }
    automation
}

/// The value of a curve **at a time**.
///
/// The value before the first breakpoint is the first value, and the
/// value after the last is the last one: a lane holds the value it has
/// until the lane says otherwise. Between two breakpoints the value is
/// the **line** between them.
pub fn value_at(points: &[(i64, f64)], at: i64) -> f64 {
    let Some((first_at, first_value)) = points.first() else { return 0.0 };
    if at <= *first_at {
        return *first_value;
    }
    let Some((last_at, last_value)) = points.last() else { return 0.0 };
    if at >= *last_at {
        return *last_value;
    }
    for window in points.windows(2) {
        let (from_at, from_value) = window[0];
        let (to_at, to_value) = window[1];
        if at >= from_at && at <= to_at {
            let span = (to_at - from_at) as f64;
            if span <= 0.0 {
                return to_value;
            }
            let fraction = (at - from_at) as f64 / span;
            return from_value + (to_value - from_value) * fraction;
        }
    }
    *last_value
}

/// The samples of a track, with the automation **applied**.
///
/// The automation is worked out **per sample**, because that is what a
/// curve is: the volume at the sample's own time, and not the volume at
/// the start of the clip. A clip that runs across a breakpoint is loud
/// at one end and quiet at the other, and the sound says so.
///
/// The stage is in the tree's order: the **mute**, then the **fader**,
/// then the **balance**. Each of the three is a step of its own, and
/// the steps are separate functions, because a send may take its copy
/// of the signal from between them.
pub fn apply(automation: &Automation, samples: &[f32], channels: usize, start: i64) -> Vec<f32> {
    let muted = apply_mute(automation, samples, channels, start);
    let faded = apply_fader(automation, &muted, channels, start);
    apply_pan(automation, &faded, channels, start)
}

/// The **mute**: a value of one or more silences the track.
///
/// The mute gates the audio **and** the track's pre-fader tap: a
/// track that is muted is not heard anywhere, and a copy of it is a
/// copy of silence.
pub fn apply_mute(automation: &Automation, samples: &[f32], channels: usize, start: i64) -> Vec<f32> {
    if channels == 0 {
        return samples.to_vec();
    }
    // No **lane**: the track's own mute is the mute.
    let Some(points) = &automation.mute else {
        if !automation.muted {
            return samples.to_vec();
        }
        return vec![0.0_f32; samples.len()];
    };
    let mut out = samples.to_vec();
    for index in 0..samples.len() / channels {
        let at = start + index as i64;
        if value_at(points, at) < 0.5 {
            continue;
        }
        for channel in 0..channels {
            let at_index = index * channels + channel;
            if at_index < out.len() {
                out[at_index] = 0.0;
            }
        }
    }
    out
}

/// The **fader**: the volume lane, in decibels.
///
/// The fader is the track's own level, and it comes after the track's
/// inserts: the tree's mixer runs the inserts and then the fader.
pub fn apply_fader(automation: &Automation, samples: &[f32], channels: usize, start: i64) -> Vec<f32> {
    if channels == 0 {
        return samples.to_vec();
    }
    // No **lane**: the fader is the level. The tree's mixer runs the
    // lane when there is one and the fader when there is not.
    let Some(points) = &automation.volume else {
        let level = craftkit_audio::db_to_linear(automation.fader_db) as f32;
        if level == 1.0 {
            return samples.to_vec();
        }
        return samples.iter().map(|sample| sample * level).collect();
    };
    let mut out = samples.to_vec();
    for index in 0..samples.len() / channels {
        let at = start + index as i64;
        let volume = craftkit_audio::db_to_linear(value_at(points, at)) as f32;
        if volume == 1.0 {
            continue;
        }
        for channel in 0..channels {
            let at_index = index * channels + channel;
            if at_index < out.len() {
                out[at_index] *= volume;
            }
        }
    }
    out
}

/// The **balance**: the pan lane, between the channels.
///
/// The balance is the track's own, and it is applied when the track is
/// **routed**: a copy that a send takes is balanced by the **send's**
/// own pan, and not by the track's.
pub fn apply_pan(automation: &Automation, samples: &[f32], channels: usize, start: i64) -> Vec<f32> {
    // A **mono** sound has one channel and is not balanced.
    if channels < 2 {
        return samples.to_vec();
    }
    // No **lane**: the track's own pan is the balance.
    let Some(points) = &automation.pan else {
        let (left, right) = balance(automation.pan_value);
        return balanced(samples, channels, left, right);
    };
    let mut out = samples.to_vec();
    for index in 0..samples.len() / channels {
        let at = start + index as i64;
        let (left, right) = balance(value_at(points, at));
        for channel in 0..channels {
            let at_index = index * channels + channel;
            if at_index < out.len() {
                out[at_index] *= if channel % 2 == 0 { left } else { right };
            }
        }
    }
    out
}

/// The **balance** of a pan: the gain of the left and the gain of the
/// right.
///
/// This is the tree's **balance** law — the one the tree calls `Zero`
/// — and not its constant-power law. The constant-power law is a law
/// for *placing* a mono sound in a stereo field, and it takes three
/// decibels off the sound in the middle; a stereo sound that is
/// balanced is not quieter in the middle than it is at the side.
///
/// The pan is a balance: minus one is all to the left and one is all
/// to the right, and the middle is the middle.
pub fn balance(pan: f64) -> (f32, f32) {
    let pan = if pan.is_finite() { pan.clamp(-1.0, 1.0) } else { 0.0 };
    ((1.0 - pan).min(1.0) as f32, (1.0 + pan).min(1.0) as f32)
}

/// The sound with the two sides **balanced**.
fn balanced(samples: &[f32], channels: usize, left: f32, right: f32) -> Vec<f32> {
    let mut out = samples.to_vec();
    for index in 0..samples.len() / channels {
        for channel in 0..channels {
            let at = index * channels + channel;
            if at < out.len() {
                out[at] *= if channel % 2 == 0 { left } else { right };
            }
        }
    }
    out
}
