//! The **transients** of a sound: the places where it starts again.
//!
//! A transient is an **onset**: a place where the sound begins
//! afresh — a drum, a plucked string, the start of a phrase. Finding
//! them is how a sound is cut into the pieces it is made of.
//!
//! # How they are found here
//!
//! By the **level**. The sound is read in short frames, each frame's
//! level is worked out, and an onset is a frame whose level **rises**
//! above the level around it: a rise of a frame or two over a local
//! average is a transient, and a sound that fades away is not one.
//!
//! The tree's detector looks at the **spectrum** — it takes a fast
//! Fourier transform of each frame and watches the high bands for a
//! jump. That is a better answer, and it needs a transform this kit
//! does not have: the kit's crates carry no dependencies at all, and a
//! fast Fourier transform is a piece of work of its own. So the level
//! is used here, and the difference is written down where the function
//! is, because a detector that is not the tree's detector should say
//! so.
//!
//! # What a transient is not
//!
//! It is not a **beat**. A beat is where the music is, and the kit
//! knows that from the tempo map and the meter (§19); a transient is
//! where the **sound** starts, and the two are often the same place and
//! often are not.

/// The most transients a sound may have.
///
/// Ten thousand is more than a person hears in a piece of music, and a
/// sound with more of them is a sound the detector does not trust.
pub const MAX_TRANSIENTS: usize = 10_000;

/// The **hop** between the frames the detector reads, in samples.
///
/// A hop of two hundred and fifty-six samples is about five
/// milliseconds at forty-eight kilohertz: short enough to tell a
/// transient from the note before it, and long enough that the level
/// of a frame means something.
const HOP: usize = 256;

/// The **window** the detector reads, in samples.
///
/// Four times the hop, so that a frame's level is not taken from one
/// moment but from a moment around it.
const WINDOW: usize = 1024;

/// The shortest time between two transients, in samples.
///
/// Twenty milliseconds is the shortest gap a person can hear as two
/// sounds and not one: below that, the two attacks are one attack.
const MIN_GAP_FRAMES: usize = 960;

/// The **sensitivity**: how small a rise may be and still be a
/// transient.
///
/// It is a number from zero to one, and it says how much of the
/// **whole sound's** level a frame must reach to count: a sensitivity
/// of one is a hundredth of it, and a sensitivity of zero is all of
/// it — and a sound that reaches all of its own level is only its
/// loudest moment.
///
/// The tree's sensitivity is a number from zero to one as well, and it
/// means the same kind of thing.
fn factor_of(sensitivity: f64) -> f64 {
    let sensitivity = if sensitivity.is_finite() { sensitivity.clamp(0.0, 1.0) } else { 0.5 };
    0.01 + (1.0 - sensitivity) * 0.99
}

/// How many frames of **background** a transient is measured against.
///
/// Eight hops is about two thousand samples: long enough to be the
/// level the sound was at before the transient, and short enough that
/// a slow passage is not measured against the loud hit before it.
const BACKGROUND: usize = 8;

/// The **level** of each short frame of a sound.
///
/// The frames are a **hop** apart and each one reads a **window**: the
/// level of a frame is the root of the mean of the squares of the
/// samples in its window, so it is a level and not a peak.
///
/// The samples are **interleaved**: the sound's channels come one
/// after another, and the level of a frame is worked out across the
/// channels of it.
pub fn levels(samples: &[f32], channels: usize) -> Vec<f64> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    let mut levels: Vec<f64> = Vec::with_capacity(frames / HOP + 2);
    let mut index = 0;
    while index < frames {
        let end = (index + WINDOW).min(frames);
        let mut sum = 0.0_f64;
        let mut count = 0_u32;
        for frame in index..end {
            for channel in 0..channels {
                let value = samples.get(frame * channels + channel).copied().unwrap_or(0.0);
                if value.is_finite() {
                    sum += f64::from(value) * f64::from(value);
                    count += 1;
                }
            }
        }
        let level = if count == 0 { 0.0 } else { (sum / f64::from(count)).sqrt() };
        levels.push(level);
        index += HOP;
    }
    levels
}

/// The **loud** parts of a sound, as ranges of its samples.
///
/// A part is loud when its level is within `threshold_db` of the
/// **loudest** part of the sound: a threshold of forty-eight decibels
/// is a part that is at least a fortieth of the peak, and a threshold
/// of zero is the peak itself.
///
/// A part shorter than `min_length` is not a part: it is a click, and
/// a click that is kept is a clip of its own that says nothing. The
/// length is in samples, and a length of nothing keeps every part.
///
/// The ranges are in the sound's own samples, and they do not touch:
/// two parts with a gap between them are two parts.
pub fn loud_ranges(
    samples: &[f32],
    channels: usize,
    threshold_db: f64,
    min_length: i64,
) -> Vec<(usize, usize)> {
    let levels = levels(samples, channels);
    if levels.is_empty() {
        return Vec::new();
    }
    let peak = levels.iter().copied().fold(0.0_f64, f64::max);
    if peak <= 0.0 {
        return Vec::new();
    }
    let decibels = if threshold_db.is_finite() { threshold_db.clamp(-96.0, 0.0) } else { -48.0 };
    let threshold = peak * 10.0_f64.powf(decibels / 20.0);
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (position, level) in levels.iter().enumerate() {
        let loud = *level >= threshold;
        match (loud, start) {
            (true, None) => start = Some(position),
            (false, Some(from)) => {
                ranges.push((from * HOP, position * HOP));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(from) = start {
        ranges.push((from * HOP, levels.len() * HOP));
    }
    // The **short** parts are dropped: a click is not a part.
    ranges.retain(|(from, to)| to.saturating_sub(*from) >= min_length.max(0) as usize);
    ranges
}

/// The **onsets** of a sound, in samples from its start.
///
/// The samples are **interleaved**: the sound's channels come one
/// after another, and the levels of the frames are worked out across
/// the channels of each frame.
///
/// The sensitivity is a number from zero to one: the higher it is, the
/// smaller a rise may be and still be a transient. A number outside
/// that range is **clamped** into it — the sensitivity is a knob and
/// not a fact about the sound, and a knob that is turned past its end
/// is at its end.
///
/// The first frame of the sound is not an onset: the sound starts
/// there, and there is nothing before it to rise from.
pub fn transients(samples: &[f32], channels: usize, rate: u32, sensitivity: f64) -> Vec<usize> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    if frames < 2 {
        return Vec::new();
    }
    let factor = factor_of(sensitivity);
    // The level of the **whole** sound: what a frame is measured
    // against.
    let mut sum = 0.0_f64;
    let mut count = 0_u64;
    for sample in samples {
        if sample.is_finite() {
            sum += f64::from(*sample) * f64::from(*sample);
            count += 1;
        }
    }
    let overall = if count == 0 { 0.0 } else { (sum / count as f64).sqrt() };
    let threshold = overall * factor;
    let levels = levels(samples, channels);
    if levels.len() < 3 {
        return Vec::new();
    }
    // The **gap** between two transients, in samples: the same gap at
    // every rate would be a different time at every rate.
    let gap = ((MIN_GAP_FRAMES * rate.max(1) as usize) / 48_000).max(1);
    let mut onsets: Vec<usize> = Vec::new();
    for position in 1..levels.len().saturating_sub(1) {
        let level = levels[position];
        // A transient is loud enough: it reaches the threshold the
        // sensitivity sets.
        if level < threshold {
            continue;
        }
        // It is a **rise**: the frame before it was quieter, and the
        // frame after it is not louder — a transient is a peak.
        if level < levels[position - 1] || level < levels.get(position + 1).copied().unwrap_or(level) {
            continue;
        }
        // And it is a rise **out of the level around it**: the sound
        // was quieter before it than it is at it.
        let from = position.saturating_sub(BACKGROUND);
        let background = levels[from..position].iter().sum::<f64>() / (position - from).max(1) as f64;
        if level < background * 1.5 + 1e-9 {
            continue;
        }
        let at = position * HOP;
        if let Some(last) = onsets.last()
            && at.saturating_sub(*last) < gap
        {
            continue;
        }
        onsets.push(at);
        if onsets.len() >= MAX_TRANSIENTS {
            break;
        }
    }
    onsets
}
