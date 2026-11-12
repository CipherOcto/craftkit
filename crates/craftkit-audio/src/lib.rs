//! `craftkit-audio` — **the mixer**: the samples of many tracks,
//! summed into one.
//!
//! # What this is
//!
//! A mixer takes **tracks** and makes one **mix**: each track is a
//! stretch of samples that starts somewhere, and the mix is the sum of
//! all of them, each at its own gain.
//!
//! That is the whole of it, and it is deliberately the whole of it: the
//! kit's mixer is a **graph** — buses, sends, inserts and automation
//! (§20.4) — and a graph of arithmetic is arithmetic. So the library
//! here sums, and the graph is the caller's.
//!
//! # Why it is a library and not a plugin
//!
//! Because the mixer is arithmetic over plain numbers: it takes samples
//! and gives back samples, and it knows nothing about a document, a
//! file or the kit. A battery that renders a document brings the tracks
//! and takes the mix.
//!
//! # The transients of a sound
//!
//! The library also finds the **onsets** of a sound — the places where
//! it starts again. There are two detectors here: one watches the
//! **level** of the sound and one watches its **spectrum**, and the
//! second is the tree's own: a fast Fourier transform of each frame,
//! the bands of the spectrum, and how much they rise. The transform is
//! here too, because this kit's crates carry no dependencies at all.
//!
//! # The numbers are hostile
//!
//! A track's start and its length came from a document, and a document
//! can say anything. So a track that starts **before the beginning** is
//! refused, a track whose length is not a number is refused, and a mix
//! longer than the limits allow is refused. The mixer does not
//! truncate and it does not guess: a refusal is a value.

#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::unimplemented,
    clippy::todo,
    clippy::unreachable
)]
#![forbid(unsafe_code)]

mod fft;
mod loudness;
mod onsets;
mod pitch;
mod transients;

pub use fft::{is_power_of_two, power};
pub use loudness::{LoudnessMeter, MAX_CHANNELS, MIN_DB};
pub use onsets::{MAX_ONSETS, onsets};
pub use pitch::{DetectedNote, MAX_FRAMES, audio_to_notes, yin};
pub use transients::{MAX_TRANSIENTS, levels, loud_ranges, transients};

/// The **limits** a mix is made within.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most frames a mix may hold.
    pub max_frames: i64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_frames: 100_000_000 }
    }
}

/// Why a mix was **refused**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A track starts before the beginning of the mix.
    NegativeStart {
        /// The start the track said.
        start: i64,
    },
    /// A track's gain is not a number.
    BadGain,
    /// The mix would be longer than the limits allow.
    TooLong {
        /// The number of frames the mix would have had.
        frames: i64,
    },
    /// The sound would be longer than the limits allow at the new rate.
    SoundTooLong {
        /// The number of frames the sound would have had.
        frames: i64,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NegativeStart { start } => write!(f, "a track cannot start at {start}"),
            Error::BadGain => write!(f, "a gain is not a number"),
            Error::TooLong { frames } => {
                write!(f, "a mix of {frames} frames is longer than the limits allow")
            }
            Error::SoundTooLong { frames } => {
                write!(f, "a sound of {frames} frames is longer than the limits allow")
            }
        }
    }
}

impl std::error::Error for Error {}

/// One **track**: a stretch of samples that starts somewhere.
///
/// The track **owns** its samples, and that is the point: the mixing
/// that runs on a lane may not allocate, and a track that borrowed its
/// samples would have to be built on the control thread and could not
/// be handed to the lane whole.
#[derive(Debug, Clone, PartialEq)]
pub struct Track {
    /// Where it starts, in samples from the beginning of the mix.
    pub start: i64,
    /// The samples, interleaved.
    pub samples: Vec<f32>,
    /// Its gain, **linear**: one is as it is, a half is half.
    pub gain: f64,
}

/// A **mix**: the samples of every track, summed.
#[derive(Debug, Clone, PartialEq)]
pub struct Mix {
    /// How fast it is played, in samples a second.
    pub rate: u32,
    /// How many frames it has: a frame is one sample of every channel.
    pub frames: i64,
    /// The samples, interleaved.
    pub samples: Vec<f32>,
}

/// The **linear** gain of a gain in **decibels**.
///
/// The decibel is a logarithm of a ratio, and the ratio is what a
/// multiplier is: twenty times the logarithm, because a decibel is a
/// tenth of a bel and a bel is a factor of ten of **power**, and power
/// is the square of a gain.
pub fn db_to_linear(db: f64) -> f64 {
    if !db.is_finite() {
        return 1.0;
    }
    10.0_f64.powf(db / 20.0)
}

/// Mix the tracks: sum them, each at its own gain.
///
/// The mix is as long as the **longest** track, and the samples are not
/// clipped: a clip is a decision about the output, and the caller makes
/// it when the mix is written.
pub fn mix(tracks: &[Track], rate: u32, limits: &Limits) -> Result<Mix, Error> {
    let frames = frames_of(tracks)?;
    if frames > limits.max_frames {
        return Err(Error::TooLong { frames });
    }
    let mut samples = vec![0.0_f32; frames as usize];
    mix_into(&mut samples, tracks);
    Ok(Mix { rate, frames, samples })
}

/// How many **frames** a set of tracks needs.
///
/// The tracks are checked as they are counted: a track that starts
/// before the beginning and a track with a gain that is not a number
/// are **refused**, and the refusal says which track.
pub fn frames_of(tracks: &[Track]) -> Result<i64, Error> {
    let mut frames = 0_i64;
    for track in tracks {
        if track.start < 0 {
            return Err(Error::NegativeStart { start: track.start });
        }
        if !track.gain.is_finite() {
            return Err(Error::BadGain);
        }
        let end = track.start.saturating_add(track.samples.len() as i64);
        if end > frames {
            frames = end;
        }
    }
    Ok(frames)
}

/// The samples of every track, summed, **into a buffer the caller owns**.
///
/// This is the mixing that may run on a **lane**: it allocates nothing,
/// it waits for nothing and it does bounded work. The buffer is the
/// caller's and the tracks are the caller's too.
///
/// A track that starts after the buffer ends is left out, and a track
/// that starts before it is summed into it from where it starts.
pub fn mix_into(output: &mut [f32], tracks: &[Track]) -> usize {
    for track in tracks {
        if track.start < 0 {
            continue;
        }
        let start = track.start as usize;
        if start >= output.len() {
            continue;
        }
        let gain = track.gain as f32;
        for (index, sample) in track.samples.iter().enumerate() {
            let at = start + index;
            if at >= output.len() {
                break;
            }
            output[at] += *sample * gain;
        }
    }
    output.len()
}

/// The samples of a mix, **clipped** to the range a file may hold.
///
/// The clip is here and not in the mixer because it is a decision about
/// the **output**: a mix may be louder than one, and a file cannot say
/// so.
pub fn clipped(samples: &[f32]) -> Vec<f32> {
    samples.iter().map(|sample| sample.clamp(-1.0, 1.0)).collect()
}

/// Stretch the sound **without changing its pitch**.
///
/// A **varispeed** plays the sound faster and the pitch falls with it;
/// a **stretch** makes the sound longer and the pitch stays. It is the
/// difference between a record played at the wrong speed and a record
/// played slowly, and the second is the one a musician asks for.
///
/// The method is **WSOLA** — waveform-similarity overlap-add. The sound
/// is cut into frames, each frame is written over the last, and the
/// place each frame is read from is chosen so that it joins the frame
/// before it as smoothly as it can. The frames are all of the same
/// pitch, so the pitch is kept and the length is what changes.
///
/// Three places decide where a frame is read from, and each of them
/// has a job:
///
/// - the **nominal** place is where the frame would be read from if
///   the sound were not joined at all: the output place divided by the
///   factor. The nominal is what keeps the **rate** right, and the
///   rate is what keeps the pitch;
/// - the **natural continuation** is the sound that follows the frame
///   before it, and it is what the frame has to carry on from;
/// - the **tolerance** is how far from the nominal a frame may be
///   read from to join well. A small tolerance keeps the rate right
///   and a large one lets the sound wander.
///
/// The factor is a **length ratio**, the same one the varispeed takes:
/// a factor of two makes the sound twice as long, and it is the
/// **pitch** that does not move.
///
/// The factor is **clamped** into a range the method can do: a stretch
/// of sixteen would ask for a frame every sixteenth of a window, and
/// the sound would be a noise. A caller that means such a stretch
/// should **refuse** it rather than get a clamped one — the clamp here
/// is the last line of defence, not the policy.
///
/// The **channels** are kept: each channel is played on its own, and
/// the place a frame is read from is the same for all of them, because
/// the channels are one sound.
pub fn time_stretch(samples: &[f32], channels: usize, factor: f64, rate: u32) -> Vec<f32> {
    let factor = factor.clamp(0.1, 10.0);
    let channels = channels.max(1);
    if samples.is_empty() || !factor.is_finite() {
        return Vec::new();
    }
    let frames = samples.len() / channels;
    if frames == 0 {
        return Vec::new();
    }
    let new_frames = ((frames as f64) * factor).round().max(1.0) as usize;
    if (factor - 1.0).abs() < 1e-9 {
        let mut same = samples.to_vec();
        same.resize(new_frames * channels, 0.0);
        return same;
    }
    // The **frame** is forty thousandths of a second, the **hop** is
    // half of it, and the **tolerance** is ten thousandths.
    let rate = f64::from(rate.max(1));
    let frame = (((rate * 0.04) as usize) & !1).clamp(64, 8192);
    let hop = frame / 2;
    let tolerance = ((rate * 0.010) as usize).clamp(8, 2048);
    // The **window**: a Hann over the frame, and the sum of the
    // windows of the frames that overlap is what the sound is divided
    // by at the end.
    let window: Vec<f32> = (0..frame)
        .map(|index| {
            let phase = 2.0 * std::f64::consts::PI * index as f64 / frame as f64;
            (0.5 - 0.5 * phase.cos()) as f32
        })
        .collect();
    // The **guide**: the mix of the channels, and it is the guide that
    // says where a frame is read from, because the channels are one
    // sound and they are read from the same place.
    let guide: Vec<f32> = (0..frames)
        .map(|frame_index| {
            let mut sum = 0.0_f32;
            for channel in 0..channels {
                sum += samples[frame_index * channels + channel];
            }
            sum / channels as f32
        })
        .collect();
    let guide_at = |index: isize| -> f32 {
        if index < 0 {
            return 0.0;
        }
        guide.get(index as usize).copied().unwrap_or(0.0)
    };
    let mut out = vec![0.0_f32; (new_frames + frame) * channels];
    let mut weights = vec![0.0_f32; new_frames + frame];
    let mut previous = 0_isize;
    let mut index = 0_usize;
    loop {
        let out_pos = index * hop;
        if out_pos >= new_frames {
            break;
        }
        let nominal = ((out_pos as f64 / factor).round() as isize).max(0);
        let from = if index == 0 {
            0
        } else {
            best_start(&guide_at, nominal, previous + hop as isize, tolerance as isize, hop)
        };
        // The frame is written, windowed, over what is there.
        for (offset, &weight) in window.iter().enumerate() {
            let read = from + offset as isize;
            for channel in 0..channels {
                let value = if read < 0 {
                    0.0
                } else {
                    samples.get(read as usize * channels + channel).copied().unwrap_or(0.0)
                };
                if let Some(slot) = out.get_mut((out_pos + offset) * channels + channel) {
                    *slot += value * weight;
                }
            }
            if let Some(slot) = weights.get_mut(out_pos + offset) {
                *slot += weight;
            }
        }
        previous = from;
        index += 1;
    }
    out.truncate(new_frames * channels);
    // The sound is divided by the windows that were written over it.
    for frame_index in 0..new_frames {
        let weight = weights.get(frame_index).copied().unwrap_or(0.0);
        if weight > 1e-3 {
            for channel in 0..channels {
                if let Some(slot) = out.get_mut(frame_index * channels + channel) {
                    *slot /= weight;
                }
            }
        }
    }
    out
}

/// The place a frame is read from.
///
/// It is the place **near the nominal** whose beginning is most like
/// the **natural continuation** — the sound that follows the frame
/// before it — and it is the one that joins the frames best.
///
/// The search is coarse first and then fine: the coarse pass finds the
/// neighbourhood and the fine pass finds the sample.
fn best_start(
    guide: &dyn Fn(isize) -> f32,
    nominal: isize,
    natural: isize,
    tolerance: isize,
    hop: usize,
) -> isize {
    let score = |at: isize| -> f32 {
        let mut products = 0.0_f32;
        let mut energy = 0.0_f32;
        let mut offset = 0_isize;
        while offset < hop as isize {
            let want = guide(natural + offset);
            let got = guide(at + offset);
            products += want * got;
            energy += got * got;
            offset += 2;
        }
        products / energy.sqrt().max(1e-9)
    };
    let low = (nominal - tolerance).max(0);
    let high = nominal + tolerance;
    let mut best = nominal.max(0);
    let mut best_score = f32::NEG_INFINITY;
    let mut at = low;
    while at <= high {
        let value = score(at);
        if value > best_score {
            best_score = value;
            best = at;
        }
        at += 4;
    }
    let fine_low = (best - 3).max(low);
    let fine_high = (best + 3).min(high);
    for at in fine_low..=fine_high {
        let value = score(at);
        if value > best_score {
            best_score = value;
            best = at;
        }
    }
    best
}

/// Play the samples at **another speed**.
///
/// A factor of two makes the sound **twice as long** and an octave
/// lower, because the pitch follows the speed: the sound is read at
/// half the rate it was written at. This is a **varispeed**, and it is
/// not the same thing as a time stretch: a time stretch keeps the
/// pitch and changes the length, and that is a much harder thing to do
/// well.
///
/// The factor is therefore a **length ratio**: the new sound is
/// `factor` times as long as the old one.
///
/// The reading is **linear**: the sample at a place in the new sound is
/// the line between the two samples it lies between, which is the same
/// line the resampler draws.
///
/// The **channels** are kept: each channel is played on its own.
pub fn varispeed(samples: &[f32], channels: usize, factor: f64) -> Vec<f32> {
    let channels = channels.max(1);
    if !factor.is_finite() || factor <= 0.0 {
        return samples.to_vec();
    }
    let frames = samples.len() / channels;
    if frames == 0 {
        return Vec::new();
    }
    let new_frames = ((frames as f64) * factor).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(new_frames * channels);
    for frame in 0..new_frames {
        // Where this frame is in the old sound: the new sound is
        // `factor` times longer, so the old sound is read at `factor`
        // times less than the new sound is played.
        let position = frame as f64 / factor;
        let index = position.floor() as usize;
        let fraction = (position - index as f64) as f32;
        let next = (index + 1).min(frames.saturating_sub(1));
        for channel in 0..channels {
            let at = index * channels + channel;
            let after = next * channels + channel;
            let first = samples.get(at).copied().unwrap_or(0.0);
            let second = samples.get(after).copied().unwrap_or(first);
            out.push(first + (second - first) * fraction);
        }
    }
    out
}

/// Resample the samples of a sound from one rate to another.
///
/// The resampling is a **windowed sinc**: the sample at a place in the
/// new rate is the old sound read with a low-pass filter, and the
/// filter's shape is the ideal one — a sinc — with a Kaiser window
/// over it so that the filter's edge falls to nothing instead of
/// ringing.
///
/// The reason for the filter is the **aliasing**: a sound that is
/// played at a lower rate has less room, and what was above the new
/// half of the rate folds back into the sound as a whistle if it is
/// not taken away. A line cannot take it away — a line has no edge to
/// take it with — and that is why the resampler is not a line.
///
/// The **channels** are kept: a frame of the new sound is a frame of
/// the old one, so the samples of each channel are resampled on their
/// own.
///
/// A sound that would be longer than the limits allow at the new rate
/// is **refused**: it is not truncated, because a truncated sound is a
/// sound that is wrong in a way nobody can hear.
pub fn resample(
    samples: &[f32],
    channels: usize,
    from_rate: u32,
    to_rate: u32,
    limits: &Limits,
) -> Result<Vec<f32>, Error> {
    let channels = channels.max(1);
    if from_rate == 0 || to_rate == 0 || channels == 0 {
        return Ok(samples.to_vec());
    }
    let frames = samples.len() / channels;
    if frames == 0 {
        return Ok(Vec::new());
    }
    if from_rate == to_rate {
        return Ok(samples.to_vec());
    }
    let ratio = f64::from(to_rate) / f64::from(from_rate);
    let new_frames = ((frames as f64) * ratio).round().max(1.0) as usize;
    if new_frames as i64 > limits.max_frames {
        return Err(Error::SoundTooLong { frames: new_frames as i64 });
    }
    // The **cutoff**: below the old rate's half, with a little room for
    // the kernel's own skirt. A sound that is played at a lower rate
    // has less room, and what was above the new half has to go or it
    // folds back into the sound as a whistle.
    let cutoff = if ratio < 1.0 { ratio * 0.94 } else { 0.97 };
    let kernel = Sinc::new(cutoff);
    // The **bank**: the kernel precomputed for every phase of a
    // rational rate change, so that an output is a dot product over
    // the samples and not a table read for every tap. A rate change
    // with no small common ratio has no bank worth having, and the
    // per-tap path is there for it.
    if let Some(bank) = Polyphase::new(from_rate, to_rate, &kernel) {
        return Ok(bank.run(samples, channels, new_frames));
    }
    Ok(resample_by_taps(samples, channels, frames, new_frames, ratio, &kernel))
}

/// The resampler read **one tap at a time**.
///
/// This is the reference: the bank reads the same kernel and the same
/// taps, and the two agree.
fn resample_by_taps(
    samples: &[f32],
    channels: usize,
    frames: usize,
    new_frames: usize,
    ratio: f64,
    kernel: &Sinc,
) -> Vec<f32> {
    let mut out = Vec::with_capacity(new_frames * channels);
    for frame in 0..new_frames {
        // Where this frame is in the old sound.
        let position = frame as f64 / ratio;
        let first = (position - kernel.half_width).ceil().max(0.0) as i64;
        let last = ((position + kernel.half_width).floor() as i64).min(frames as i64 - 1);
        for channel in 0..channels {
            let mut total = 0.0_f32;
            let mut index = first;
            while index <= last {
                if let Some(value) = samples.get(index as usize * channels + channel) {
                    total += value * kernel.at(position - index as f64);
                }
                index += 1;
            }
            out.push(total);
        }
    }
    out
}

/// The sound **backwards**.
///
/// Each channel is reversed on its own, so a stereo sound is still a
/// stereo sound and the two channels keep their places in it.
pub fn reverse(samples: &[f32], channels: usize) -> Vec<f32> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    let mut out = vec![0.0_f32; frames * channels];
    for frame in 0..frames {
        for channel in 0..channels {
            let at = frame * channels + channel;
            let from = (frames - 1 - frame) * channels + channel;
            out[at] = samples.get(from).copied().unwrap_or(0.0);
        }
    }
    out
}

/// The sound **upside down**: the sign of every sample is flipped.
///
/// It is the same sound and not the other one, and it is the phase
/// that is different — the same thing the tree's phase invert does to
/// a track.
pub fn invert(samples: &[f32]) -> Vec<f32> {
    samples.iter().map(|sample| -*sample).collect()
}

/// The sound at a **level**: its loudest sample is at that level.
///
/// The level is in decibels and it is bounded, from minus one hundred
/// and forty-four to twenty-four: a level above zero would ask for a
/// sound that is louder than a file can hold. A silent sound is left
/// alone, because there is no loudest sample to move.
///
/// The loudest sample of the **whole** sound is the one that counts,
/// and not the loudest of each channel: the tree's normalizer does
/// the same unless it is asked for the other way.
pub fn normalize(samples: &[f32], target_db: f64) -> Vec<f32> {
    let target_db = if target_db.is_finite() { target_db.clamp(-144.0, 24.0) } else { 0.0 };
    let target = db_to_linear(target_db);
    let mut loudest = 0.0_f32;
    for sample in samples {
        let value = sample.abs();
        if value.is_finite() && value > loudest {
            loudest = value;
        }
    }
    if loudest <= 0.0 {
        return samples.to_vec();
    }
    let gain = target as f32 / loudest;
    samples.iter().map(|sample| sample * gain).collect()
}

/// The sound without its **direct current**: the mean is taken away.
///
/// A sound that sits above zero is not louder, it is **offset**, and
/// the offset takes room from the sound without adding any. Each
/// channel is measured on its own, because a stereo sound may have a
/// different offset in each of them.
pub fn remove_dc(samples: &[f32], channels: usize) -> Vec<f32> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    let mut out = samples.to_vec();
    for channel in 0..channels {
        if frames == 0 {
            break;
        }
        let mut sum = 0.0_f64;
        for frame in 0..frames {
            let value = samples.get(frame * channels + channel).copied().unwrap_or(0.0);
            if value.is_finite() {
                sum += f64::from(value);
            }
        }
        let mean = (sum / frames as f64) as f32;
        for frame in 0..frames {
            let at = frame * channels + channel;
            if let Some(slot) = out.get_mut(at) {
                *slot -= mean;
            }
        }
    }
    out
}

/// Resample the sound by a **ratio**: the new sound is `ratio` times
/// as long as the old one.
///
/// The ratio is not a pair of rates, so there is no bank of weights to
/// read: the sound is read one tap at a time, which is the reference
/// path and the slower one. A rate change that has a bank should use
/// [`resample`] instead.
///
/// The **cutoff** follows the ratio: a sound that is made shorter is
/// played faster, and what was above the new half of the rate has to
/// go or it folds back as a whistle.
pub fn resample_ratio(samples: &[f32], channels: usize, ratio: f64, new_frames: usize) -> Vec<f32> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    if frames == 0 || new_frames == 0 || !ratio.is_finite() || ratio <= 0.0 {
        return Vec::new();
    }
    let cutoff = if ratio < 1.0 { ratio * 0.94 } else { 0.97 };
    let kernel = Sinc::new(cutoff);
    resample_by_taps(samples, channels, frames, new_frames, ratio, &kernel)
}

/// Shift the **pitch** of the sound and keep its length.
///
/// A pitch shift is the other half of a stretch: a stretch makes the
/// sound longer and the pitch stays, and a pitch shift makes the pitch
/// move and the sound stays as long as it was.
///
/// It is done in two steps, and each of them is a thing this crate
/// already has:
///
/// - the sound is **resampled** as if it were played faster, which
///   raises the pitch and makes it shorter;
/// - the sound is then **stretched** back to the length it was, and a
///   stretch keeps the pitch — so the pitch has moved and the length
///   has not.
///
/// The **semitones** are the tree's: a semitone is a twelfth of a
/// doubling, so twelve semitones are an octave. A shift of more than
/// forty-eight semitones is clamped here, because that is all the
/// method can do; a caller that means such a shift should **refuse**
/// it rather than get a clamped one — the clamp here is the last line
/// of defence, not the policy.
pub fn pitch_shift(samples: &[f32], channels: usize, semitones: f64, rate: u32) -> Vec<f32> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    if frames == 0 {
        return Vec::new();
    }
    let semitones = if semitones.is_finite() { semitones.clamp(-48.0, 48.0) } else { 0.0 };
    if semitones.abs() < 1e-4 {
        return samples.to_vec();
    }
    let factor = 2f64.powf(semitones / 12.0);
    // The pitch goes up by the factor: the sound is read as if it were
    // played faster, and it is shorter for it.
    let shorter =
        resample_ratio(samples, channels, 1.0 / factor, ((frames as f64) / factor).round().max(1.0) as usize);
    // The length is put back by a stretch, and a stretch keeps the
    // pitch.
    if (0.1..=10.0).contains(&factor) {
        return time_stretch(&shorter, channels, factor, rate);
    }
    // A stretch of sixteen would ask for a frame every sixteenth of a
    // window, and the sound would be a noise: the stretch is done in
    // two halves instead, and the sound is the same.
    let half = factor.sqrt();
    time_stretch(&time_stretch(&shorter, channels, half, rate), channels, half, rate)
}

/// The **bank**: the kernel precomputed for every phase of a rational
/// rate change.
///
/// The rates of a sound are two whole numbers, and they have a common
/// factor: the rate change is then a whole number of phases, and each
/// phase has its own set of weights. So an output is one dot product
/// over the samples and the weights, with no table read and no
/// interpolation for every tap.
///
/// A rate change with no small common factor has no bank worth having
/// — the phases would be one for every output frame — and there is
/// none: the caller falls back to the taps.
struct Polyphase {
    /// How many phases the rate change has.
    phases: u64,
    /// How far the input moves in one output frame, in phases.
    step: u64,
    /// Where the first tap is, from the frame's own place.
    first: i64,
    /// How many taps there are.
    taps: usize,
    /// The weights: `phases * taps` of them, phase by phase.
    weights: Vec<f32>,
}

/// The most **weights** a bank may hold.
///
/// The bank is a fast path and not a requirement: a rate change whose
/// bank would be enormous is read tap by tap instead. Sixteen thousand
/// phases of seventy taps is four million weights — sixteen megabytes
/// — and a rate change that needs more than that is not a rate change
/// anybody plays.
const MAX_POLYPHASE_WEIGHTS: usize = 1 << 22;

impl Polyphase {
    /// The bank for that change of rate, if it is worth having.
    fn new(from: u32, to: u32, kernel: &Sinc) -> Option<Polyphase> {
        let factor = gcd(from, to);
        let (step, phases) = (u64::from(from / factor), u64::from(to / factor));
        let half_width = kernel.half_width;
        let first = (-half_width).floor() as i64;
        let last = half_width.ceil() as i64 + 1;
        let taps = usize::try_from(last - first + 1).ok()?;
        if usize::try_from(phases).ok()?.checked_mul(taps)? > MAX_POLYPHASE_WEIGHTS {
            return None;
        }
        let mut weights = Vec::with_capacity(taps * phases as usize);
        for phase in 0..phases {
            let fraction = phase as f64 / phases as f64;
            for tap in first..first + taps as i64 {
                let distance = fraction - tap as f64;
                weights.push(if distance.abs() <= half_width { kernel.at(distance) } else { 0.0 });
            }
        }
        Some(Polyphase { phases, step, first, taps, weights })
    }

    /// The sound at the new rate, read with the bank.
    fn run(&self, samples: &[f32], channels: usize, new_frames: usize) -> Vec<f32> {
        let frames = samples.len() / channels;
        let mut out = Vec::with_capacity(new_frames * channels);
        for frame in 0..new_frames as u64 {
            let Some(position) = frame.checked_mul(self.step) else { break };
            let (index, phase) = ((position / self.phases) as i64, (position % self.phases) as usize);
            let start = index + self.first;
            for channel in 0..channels {
                let mut total = 0.0_f32;
                for tap in 0..self.taps {
                    let at = start + tap as i64;
                    if at < 0 || at as usize >= frames {
                        // A tap outside the sound is nothing, the same
                        // way it is nothing when the taps are read one
                        // at a time.
                        continue;
                    }
                    let Some(weight) = self.weights.get(phase * self.taps + tap) else { continue };
                    total += samples[at as usize * channels + channel] * weight;
                }
                out.push(total);
            }
        }
        out.resize(new_frames * channels, 0.0);
        out
    }
}

/// The greatest common factor of two numbers.
fn gcd(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let rest = left % right;
        left = right;
        right = rest;
    }
    left.max(1)
}

/// The **windowed sinc** kernel: the shape a resampler reads with.
///
/// A resampler that reads with a **line** throws away everything above
/// half the old rate, and a sound resampled to a much higher rate loses
/// its top. The kernel here is a sinc — the shape of an ideal low-pass
/// — with a **Kaiser** window over it, so that the kernel falls to
/// nothing at its edges and does not ring.
struct Sinc {
    /// The kernel, one side of it, sampled finely.
    table: Vec<f32>,
    /// How far the kernel reaches, in samples.
    half_width: f64,
}

/// How many **zero crossings** of the sinc the kernel spans.
///
/// The sinc is cut where it is small enough to be nothing: thirty-two
/// zero crossings is far past anything a listener can hear, and it is
/// what makes the kernel's edge quiet.
const SINC_ZERO_CROSSINGS: f64 = 32.0;

/// How finely the kernel is **sampled**.
///
/// The kernel is read between two samples as well as at them, so it is
/// sampled finely and the reading is interpolated.
const SINC_RESOLUTION: usize = 512;

/// The **Kaiser** window's beta: how sharp the kernel's edge is.
///
/// Nine is the usual choice for a resampler: the edge falls away
/// quickly enough to stop the ringing and slowly enough to keep the
/// band the sound is in flat.
const KAISER_BETA: f64 = 9.0;

impl Sinc {
    /// The kernel for that **cutoff**, which is a part of the old rate.
    fn new(cutoff: f64) -> Self {
        let cutoff = cutoff.clamp(0.01, 1.0);
        let half_width = SINC_ZERO_CROSSINGS / cutoff;
        let count = (half_width * SINC_RESOLUTION as f64).ceil() as usize + 2;
        let scale = bessel_i0(KAISER_BETA);
        let table = (0..count)
            .map(|index| {
                let t = index as f64 / SINC_RESOLUTION as f64;
                let x = t / half_width;
                if x >= 1.0 {
                    return 0.0;
                }
                let argument = std::f64::consts::PI * cutoff * t;
                let sinc = if argument.abs() < 1e-12 { 1.0 } else { argument.sin() / argument };
                let window = bessel_i0(KAISER_BETA * (1.0 - x * x).max(0.0).sqrt()) / scale;
                (cutoff * sinc * window) as f32
            })
            .collect();
        Sinc { table, half_width }
    }

    /// The kernel's weight at that **distance** from its centre.
    fn at(&self, distance: f64) -> f32 {
        let position = distance.abs() * SINC_RESOLUTION as f64;
        let index = position as usize;
        let fraction = (position - index as f64) as f32;
        match (self.table.get(index), self.table.get(index + 1)) {
            (Some(a), Some(b)) => a + (b - a) * fraction,
            (Some(a), None) => *a,
            _ => 0.0,
        }
    }
}

/// The **Bessel** function of the first kind, of order zero.
///
/// It is the Kaiser window's shape: the window is the Bessel function
/// of the distance from the centre, and the series is the way to work
/// it out.
fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let y = x * x / 4.0;
    for k in 1..64 {
        term *= y / (k as f64 * k as f64);
        sum += term;
        if term < sum * 1e-12 {
            break;
        }
    }
    sum
}

/// The **shape** of a fade.
///
/// A fade is a curve from silence to full, and there is more than one
/// curve. The shape is the curve itself, and it is a **choice**, not a
/// number: a linear fade is a straight line, an equal-power fade is
/// quiet for longer, an S-curve is gentle at both ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FadeShape {
    /// A straight line from silence to full.
    Linear,
    /// The **default**: the two halves of a fade carry the same power.
    ///
    /// The curve is the sine of a quarter turn, and that is what makes
    /// a crossfade of two equal-power fades hold its power in the
    /// middle.
    #[default]
    EqualPower,
    /// A smooth curve that is gentle at both ends.
    SCurve,
    /// Slow at the start and quick at the end.
    Exponential,
    /// Quick at the start and slow at the end.
    Logarithmic,
}

impl FadeShape {
    /// Every shape, in the order they are offered.
    pub const ALL: [FadeShape; 5] = [
        FadeShape::Linear,
        FadeShape::EqualPower,
        FadeShape::SCurve,
        FadeShape::Exponential,
        FadeShape::Logarithmic,
    ];

    /// The **gain** at a place in the fade, from nothing to full.
    ///
    /// The place is a number from zero to one, and a place outside the
    /// fade is the end it is past: a fade that has ended is full, and a
    /// fade that has not begun is nothing.
    pub fn gain(self, t: f32) -> f32 {
        let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
        match self {
            FadeShape::Linear => t,
            FadeShape::EqualPower => (t * std::f32::consts::FRAC_PI_2).sin(),
            FadeShape::SCurve => 0.5 - 0.5 * (t * std::f32::consts::PI).cos(),
            FadeShape::Exponential => t * t * t,
            FadeShape::Logarithmic => 1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t),
        }
    }

    /// The **name** of the shape.
    pub fn label(self) -> &'static str {
        match self {
            FadeShape::Linear => "linear",
            FadeShape::EqualPower => "equal_power",
            FadeShape::SCurve => "s_curve",
            FadeShape::Exponential => "exponential",
            FadeShape::Logarithmic => "logarithmic",
        }
    }

    /// The shape of that **name**.
    ///
    /// The name is read the way a person writes it: the case does not
    /// matter, and a space, a dash and an underscore are the same
    /// separator. A name that is not a shape is **nothing**, and not a
    /// shape that is close to it.
    pub fn from_label(label: &str) -> Option<FadeShape> {
        let wanted = normalise(label);
        FadeShape::ALL.into_iter().find(|shape| normalise(shape.label()) == wanted)
    }
}

/// The name of a shape, as a person writes it: no case, and no
/// difference between a space, a dash and an underscore.
fn normalise(label: &str) -> String {
    label.to_ascii_lowercase().replace(['-', '_', ' '], "")
}

/// A **fade**: a length, a shape and a bend.
///
/// The length is in **samples**, the shape is the curve, and the bend
/// is how much the curve is pushed away from itself: a bend of zero is
/// the shape exactly, and a bend of one makes the curve steeper in the
/// middle.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Fade {
    /// How long it is, in samples. Zero is no fade at all.
    pub len: u64,
    /// The curve it follows.
    pub shape: FadeShape,
    /// How much the curve is bent, from -1 to 1.
    pub curvature: f32,
}

impl Fade {
    /// A fade of that length, with the default shape.
    pub fn of(len: u64) -> Fade {
        Fade { len, shape: FadeShape::default(), curvature: 0.0 }
    }

    /// The **gain** at a place in the fade.
    ///
    /// The bend is applied to the shape's own gain, and it is applied as
    /// a **weight**: a bent curve is the shape's curve pushed towards
    /// the middle, and the ends of the curve stay where they were, so
    /// that a fade still begins at nothing and still ends at full.
    pub fn gain(self, t: f32) -> f32 {
        let base = self.shape.gain(t);
        let curvature = if self.curvature.is_finite() { self.curvature.clamp(-1.0, 1.0) } else { 0.0 };
        if curvature == 0.0 || base <= 0.0 || base >= 1.0 {
            return base;
        }
        let weighted = base * (curvature * 4.0).exp2();
        weighted / (weighted + 1.0 - base)
    }

    /// The gain at a sample at the **head** of a clip.
    ///
    /// A sample that is past the fade is full, and a fade of no length
    /// is full everywhere.
    pub fn gain_at_head(self, rel: u64) -> f32 {
        if self.len == 0 || rel >= self.len {
            return 1.0;
        }
        self.gain(rel as f32 / self.len as f32)
    }

    /// The gain at a sample at the **tail** of a clip of that many
    /// frames.
    ///
    /// The tail is counted **from the end**, because that is where the
    /// silence is: the last sample of a fade of two is a half of the
    /// way down, and the sample before the fade is full.
    pub fn gain_at_tail(self, rel: u64, frames: u64) -> f32 {
        if self.len == 0 {
            return 1.0;
        }
        let from_end = frames.saturating_sub(rel);
        if from_end > self.len {
            return 1.0;
        }
        self.gain(from_end as f32 / self.len as f32)
    }
}

/// The samples with a **fade** at each end.
///
/// The fade is in **frames of the clip**, and the samples are the
/// clip's own: the head fades in and the tail fades out, and the two
/// overlap when the clip is shorter than they are.
///
/// The **channels** are kept: every channel is faded the same way, and
/// a fade is a gain and not a thing that belongs to a channel.
pub fn faded(samples: &[f32], channels: usize, fade_in: Fade, fade_out: Fade) -> Vec<f32> {
    let channels = channels.max(1);
    if fade_in.len == 0 && fade_out.len == 0 {
        return samples.to_vec();
    }
    let frames = samples.len() / channels;
    let mut out = samples.to_vec();
    for frame in 0..frames {
        let gain = fade_in.gain_at_head(frame as u64) * fade_out.gain_at_tail(frame as u64, frames as u64);
        if gain == 1.0 {
            continue;
        }
        for channel in 0..channels {
            let at = frame * channels + channel;
            if let Some(sample) = out.get_mut(at) {
                *sample *= gain;
            }
        }
    }
    out
}

/// How long a **blip** lasts, in seconds.
///
/// It is short enough to be a click and long enough to be heard: a
/// tenth of a second is a knock and a thousandth is a tick.
pub const BLIP_SECONDS: f64 = 0.03;

/// The **frequency** of a note, in hertz.
///
/// A note is a number from 0 to 127, and the numbers are the piano's:
/// 69 is the A above the middle C, and it is 440 hertz. The frequency
/// doubles every twelve notes, because twelve notes are an octave and
/// an octave is a doubling.
pub fn midi_to_hz(note: f64) -> f64 {
    if !note.is_finite() {
        return 0.0;
    }
    440.0 * 2.0_f64.powf((note.clamp(0.0, 127.0) - 69.0) / 12.0)
}

/// A **blip**: a short tone that fades away.
///
/// The blip is the click of a metronome: a sine at a pitch, at a
/// level, decaying over thirty thousandths of a second. It is **added**
/// to the samples and not mixed in: the click is a voice of its own and
/// it goes on top of everything else.
///
/// The blip starts at the sample `at` and is **one** voice: it goes to
/// every channel, because a click in the left ear and not in the right
/// is a click with a fault in it.
pub fn blip(samples: &mut [f32], channels: usize, at: usize, frequency: f64, amplitude: f32, rate: u32) {
    let channels = channels.max(1);
    if rate == 0 || !frequency.is_finite() || frequency <= 0.0 || !amplitude.is_finite() || amplitude == 0.0 {
        return;
    }
    let length = ((BLIP_SECONDS * rate as f64).round() as usize).max(1);
    let step = std::f64::consts::TAU * frequency / rate as f64;
    for index in 0..length {
        let sample = at + index;
        if sample >= samples.len() {
            break;
        }
        // The envelope: an exponential decay, so the blip fades away
        // rather than stopping.
        let envelope = (-6.0 * index as f64 / length as f64).exp() as f32;
        let value = ((step * index as f64).sin() as f32) * envelope * amplitude;
        for channel in 0..channels {
            let where_ = sample * channels + channel;
            if let Some(sample) = samples.get_mut(where_) {
                *sample += value;
            }
        }
    }
}

#[cfg(test)]
mod bank_tests {
    use super::*;

    /// A **tone** of that frequency, at that rate.
    fn a_tone(rate: u32, frequency: f32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|frame| {
                let t = frame as f64 / f64::from(rate);
                (2.0 * std::f64::consts::PI * f64::from(frequency) * t).sin() as f32
            })
            .collect()
    }

    /// The sound at the new rate, read **one tap at a time**.
    fn by_taps(samples: &[f32], channels: usize, from: u32, to: u32) -> Vec<f32> {
        let ratio = f64::from(to) / f64::from(from);
        let frames = samples.len() / channels;
        let new_frames = ((frames as f64) * ratio).round().max(1.0) as usize;
        let cutoff = if ratio < 1.0 { ratio * 0.94 } else { 0.97 };
        let kernel = Sinc::new(cutoff);
        resample_by_taps(samples, channels, frames, new_frames, ratio, &kernel)
    }

    /// The **bank** and the taps are the same sound.
    ///
    /// The bank reads the kernel once for every phase and the taps read
    /// it for every tap of every frame, and the two must agree — or the
    /// fast path is a different resampler.
    #[test]
    fn the_bank_and_the_taps_agree() {
        let limits = Limits::default();
        let tone = a_tone(44_100, 1_000.0, 4_000);
        let bank = resample(&tone, 1, 44_100, 48_000, &limits).expect("the sound is resampled");
        let taps = by_taps(&tone, 1, 44_100, 48_000);
        assert_eq!(bank.len(), taps.len(), "the same number of frames");
        let mut worst = 0.0_f32;
        for (left, right) in bank.iter().zip(taps.iter()) {
            worst = worst.max((left - right).abs());
        }
        assert!(worst < 1e-5, "the same sound: the worst difference is {worst}");
    }

    /// A rate change with **no small common factor** has no bank worth
    /// having, and the taps are there for it.
    #[test]
    fn a_rate_change_without_a_bank_is_read_tap_by_tap() {
        let limits = Limits::default();
        let kernel = Sinc::new(0.97);
        assert!(Polyphase::new(100_000, 100_001, &kernel).is_none(), "a hundred thousand phases is no bank");
        let tone = a_tone(100_000, 1_000.0, 4_000);
        let resampled = resample(&tone, 1, 100_000, 100_001, &limits).expect("the sound is resampled");
        let taps = by_taps(&tone, 1, 100_000, 100_001);
        assert_eq!(resampled.len(), taps.len());
        let mut worst = 0.0_f32;
        for (left, right) in resampled.iter().zip(taps.iter()) {
            worst = worst.max((left - right).abs());
        }
        assert!(worst < 1e-5, "the same sound: the worst difference is {worst}");
    }
}

#[cfg(test)]
mod measurement {
    use super::*;

    /// A **measurement** of the two paths, and not a test: it prints
    /// and it asserts nothing, and it is bounded to two hundred
    /// thousand frames.
    #[test]
    #[ignore]
    fn the_two_paths() {
        let frames = 200_000;
        let tone: Vec<f32> = (0..frames)
            .map(|frame| {
                let t = frame as f64 / 44_100.0;
                (2.0 * std::f64::consts::PI * 1_000.0 * t).sin() as f32
            })
            .collect();
        let kernel = Sinc::new(0.97);
        let bank = Polyphase::new(44_100, 48_000, &kernel).expect("there is a bank");
        let start = std::time::Instant::now();
        let with = bank.run(&tone, 1, 217_959);
        let bank_time = start.elapsed();
        let start = std::time::Instant::now();
        let without = resample_by_taps(&tone, 1, frames, 217_959, 48_000.0 / 44_100.0, &kernel);
        let taps_time = start.elapsed();
        println!(
            "the bank: {bank_time:?} for {} samples; the taps: {taps_time:?}; the speed-up: {:.1}x",
            with.len(),
            taps_time.as_secs_f64() / bank_time.as_secs_f64().max(1e-9)
        );
        assert_eq!(with.len(), without.len());
    }
}
