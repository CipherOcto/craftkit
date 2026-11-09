//! The **pitch** of a sound: the note it sings, if it sings one.
//!
//! The tree's own detector: **YIN**, the cumulative mean normalised
//! difference of de Cheveigné and Kawahara (2002). A frame of sound is
//! compared with itself at every **lag** it could be, the differences
//! are made **cumulative and normalised**, and the lag where that
//! measure falls below a **threshold** is the period of the note.
//!
//! The frames are then put in a row, and a **run** of frames that sing
//! the same note is a **note**: it has a start, an end, a pitch and a
//! velocity that is read from how loud the run was.
//!
//! # The numbers are hostile
//!
//! A sound comes from a file, and a file can say anything. So the
//! frames are **capped**, the sample rate is **checked**, and a frame
//! that cannot carry a pitch has **no pitch** rather than a wrong one.

/// The **most frames** the detector will look at.
///
/// The tree's own cap: a sound that is longer than this is read as far
/// as it goes and no further.
pub const MAX_FRAMES: usize = 2_000_000;

/// The **least** number of samples a frame may have.
pub const MIN_FRAME: usize = 64;

/// The **least** number of frames a note must sing for.
pub const MIN_FRAMES: usize = 3;

/// The **lowest** pitch the detector looks for, in hertz.
pub const MIN_HZ: f32 = 50.0;

/// The **highest** pitch the detector looks for, in hertz.
pub const MAX_HZ: f32 = 1600.0;

/// The **threshold** of the detector: below it the measure is a period.
pub const THRESHOLD: f32 = 0.15;

/// A **note** that was heard: where it is and what it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DetectedNote {
    /// The first sample of the note.
    pub start: usize,
    /// The sample after the last one of the note.
    pub end: usize,
    /// The **pitch** of the note: the middle A is sixty-nine.
    pub pitch: u8,
    /// How **loud** the note was, from one to a hundred and twenty-seven.
    pub velocity: u8,
}

/// The **pitch** of one frame of sound, in hertz.
///
/// `None` when the frame does not sing one: it is too short, or the
/// measure never falls below the threshold.
pub fn yin(frame: &[f32], sample_rate: f32, min_hz: f32, max_hz: f32, threshold: f32) -> Option<f32> {
    if !(sample_rate.is_finite() && sample_rate > 0.0) || frame.len() < MIN_FRAME {
        return None;
    }
    let max_tau = ((sample_rate / min_hz.max(20.0)) as usize).min(frame.len() / 2);
    let min_tau = ((sample_rate / max_hz.max(min_hz + 1.0)) as usize).max(2);
    if max_tau <= min_tau.saturating_add(2) {
        return None;
    }
    // The **difference** of the frame with itself at every lag.
    let width = frame.len() - max_tau;
    let mut difference = vec![0.0f32; max_tau + 1];
    for (lag, value) in difference.iter_mut().enumerate().skip(1) {
        let mut sum = 0.0f32;
        for index in 0..width {
            let (Some(one), Some(other)) = (frame.get(index), frame.get(index + lag)) else { break };
            let gap = one - other;
            sum += gap * gap;
        }
        *value = sum;
    }
    // The **cumulative mean normalised difference**.
    let mut measure = vec![1.0f32; max_tau + 1];
    let mut running = 0.0f32;
    for lag in 1..=max_tau {
        let value = difference.get(lag).copied().unwrap_or(0.0);
        running += value;
        if let Some(slot) = measure.get_mut(lag) {
            *slot = if running > 0.0 { value * lag as f32 / running } else { 1.0 };
        }
    }
    let mut lag = min_tau;
    while lag < max_tau {
        let value = measure.get(lag).copied().unwrap_or(1.0);
        if value < threshold {
            // The **bottom** of the measure: the walk goes to the place
            // where it stops falling.
            while lag + 1 < max_tau
                && measure.get(lag + 1).copied().unwrap_or(1.0) < measure.get(lag).copied().unwrap_or(1.0)
            {
                lag += 1;
            }
            // The **parabola** through the three points around it says
            // where the bottom is, between two of them.
            let left = measure.get(lag.wrapping_sub(1)).copied().unwrap_or(1.0);
            let middle = measure.get(lag).copied().unwrap_or(1.0);
            let right = measure.get(lag + 1).copied().unwrap_or(1.0);
            let denominator = left - 2.0 * middle + right;
            let shift = if denominator.abs() > 1.0e-9 { 0.5 * (left - right) / denominator } else { 0.0 };
            let period = lag as f32 + shift.clamp(-1.0, 1.0);
            return (period > 0.0).then_some(sample_rate / period);
        }
        lag += 1;
    }
    None
}

/// The **notes** of a sound.
///
/// The tree's own rule: the sound is cut into frames, each frame is
/// asked for its pitch, and a **run** of frames that sing the same
/// note is a note — and a run of fewer than three frames is not.
pub fn audio_to_notes(audio: &[f32], sample_rate: f32) -> Vec<DetectedNote> {
    let rate = if sample_rate.is_finite() && sample_rate > 0.0 { sample_rate } else { 48_000.0 };
    let window = ((rate * 0.046) as usize).clamp(256, 8192);
    let hop = window / 4;
    let mut frames: Vec<(usize, Option<u8>, f32)> = Vec::new();
    let mut position = 0usize;
    while position + window <= audio.len() && frames.len() < MAX_FRAMES {
        let Some(frame) = audio.get(position..position + window) else { break };
        let rms = (frame.iter().map(|sample| sample * sample).sum::<f32>() / window as f32).sqrt();
        // A **quiet** frame does not sing: there is no note in a silence.
        let pitch = if rms > 0.01 {
            yin(frame, rate, MIN_HZ, MAX_HZ, THRESHOLD)
                .map(|hz| (69.0 + 12.0 * (hz / 440.0).log2()).round().clamp(0.0, 127.0) as u8)
        } else {
            None
        };
        frames.push((position, pitch, rms));
        position += hop;
    }
    let mut notes: Vec<DetectedNote> = Vec::new();
    let mut index = 0usize;
    while index < frames.len() {
        let Some((start, Some(pitch), _)) = frames.get(index).copied() else {
            index += 1;
            continue;
        };
        let mut next = index;
        let mut peak = 0.0f32;
        while let Some((_, Some(other), rms)) = frames.get(next).copied() {
            if other != pitch {
                break;
            }
            peak = peak.max(rms);
            next += 1;
        }
        if next - index >= MIN_FRAMES {
            let end = frames.get(next).map_or(audio.len(), |frame| frame.0);
            let velocity = (40.0 + 87.0 * (peak / 0.5).min(1.0)) as u8;
            notes.push(DetectedNote { start, end, pitch, velocity: velocity.clamp(1, 127) });
        }
        index = next.max(index + 1);
    }
    notes
}
