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
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NegativeStart { start } => write!(f, "a track cannot start at {start}"),
            Error::BadGain => write!(f, "a gain is not a number"),
            Error::TooLong { frames } => {
                write!(f, "a mix of {frames} frames is longer than the limits allow")
            }
        }
    }
}

impl std::error::Error for Error {}

/// One **track**: a stretch of samples that starts somewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Track<'a> {
    /// Where it starts, in samples from the beginning of the mix.
    pub start: i64,
    /// The samples, interleaved.
    pub samples: &'a [f32],
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
pub fn mix(tracks: &[Track<'_>], rate: u32, limits: &Limits) -> Result<Mix, Error> {
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
    if frames > limits.max_frames {
        return Err(Error::TooLong { frames });
    }
    let mut samples = vec![0.0_f32; frames as usize];
    for track in tracks {
        let start = track.start as usize;
        for (index, sample) in track.samples.iter().enumerate() {
            let at = start + index;
            if at >= samples.len() {
                break;
            }
            samples[at] += *sample * track.gain as f32;
        }
    }
    Ok(Mix { rate, frames, samples })
}

/// The samples of a mix, **clipped** to the range a file may hold.
///
/// The clip is here and not in the mixer because it is a decision about
/// the **output**: a mix may be louder than one, and a file cannot say
/// so.
pub fn clipped(samples: &[f32]) -> Vec<f32> {
    samples.iter().map(|sample| sample.clamp(-1.0, 1.0)).collect()
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
/// The resampling is **linear**: the sample at a place in the new rate
/// is the line between the two samples it lies between. That is not the
/// best resampler and it is honest about what it is — a sound that has
/// been resampled by a line is a sound that has been resampled.
///
/// The **channels** are kept: a frame of the new sound is a frame of
/// the old one, so the samples of each channel are resampled on their
/// own.
pub fn resample(samples: &[f32], channels: usize, from_rate: u32, to_rate: u32) -> Vec<f32> {
    let channels = channels.max(1);
    if from_rate == 0 || to_rate == 0 || channels == 0 {
        return samples.to_vec();
    }
    let frames = samples.len() / channels;
    if frames == 0 {
        return Vec::new();
    }
    let ratio = f64::from(to_rate) / f64::from(from_rate);
    let new_frames = ((frames as f64) * ratio).round().max(1.0) as usize;
    let mut out = Vec::with_capacity(new_frames * channels);
    for frame in 0..new_frames {
        // Where this frame is in the old sound.
        let position = frame as f64 / ratio;
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
