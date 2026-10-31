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
