//! The **loudness** of a sound: how loud it is to a listener.
//!
//! The tree's own meter: the **ITU-R BS.1770** loudness, as **EBU R128**
//! asks for it, with the **true peak** beside it.
//!
//! # What is measured
//!
//! - The sound is **weighted**: the K-weighting of BS.1770 — a shelf
//!   and a high-pass — is what a listener's ear is said to do.
//! - The weighted sound is measured in **blocks of a tenth of a
//!   second**, and the blocks are put in a row:
//!   - the **momentary** loudness is the mean of the last **four**
//!     blocks — four hundred milliseconds;
//!   - the **short-term** loudness is the mean of the last **thirty**
//!     blocks — three seconds;
//!   - the **integrated** loudness is the mean of every block that is
//!     **above the gate**: first the blocks above **minus seventy**
//!     LUFS, then those above **ten LU below** that mean.
//! - The **true peak** is the highest point of the sound **between** the
//!   samples: the sound is looked at **four times** over, so that a
//!   peak between two samples is not missed.
//!
//! # The numbers are hostile
//!
//! A sound comes from a file. So the sample rate is **checked**, the
//! channels are **capped**, and a number that is not a number is
//! **nothing** rather than a wrong one.

use std::f64::consts::PI;

/// The **least** loudness a meter says, in decibels.
pub const MIN_DB: f32 = -144.0;

/// The **most** channels a meter will hold state for.
pub const MAX_CHANNELS: usize = 64;

/// The **blocks** of a tenth of a second that are kept.
const SUB_BLOCKS: usize = 30;

/// The **lowest** loudness the histogram keeps.
const HIST_MIN: f32 = -70.0;

/// The **width** of a bin of the histogram.
const HIST_STEP: f32 = 0.05;

/// The **bins** of the histogram.
const HIST_BINS: usize = 1800;

/// The **phases** of the filter that finds the true peak.
const TP_PHASES: usize = 4;

/// The **taps** of that filter.
const TP_TAPS: usize = 12;

/// The **coefficients** of a second-order filter, with `a0` of one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coeffs {
    /// The first tap of the numerator.
    pub b0: f32,
    /// The second tap of the numerator.
    pub b1: f32,
    /// The third tap of the numerator.
    pub b2: f32,
    /// The first tap of the denominator.
    pub a1: f32,
    /// The second tap of the denominator.
    pub a2: f32,
}

/// A **filter**: a second-order section, one sample at a time.
#[derive(Debug, Clone)]
pub struct Biquad {
    coeffs: Coeffs,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// A filter with the given coefficients.
    pub fn new(coeffs: Coeffs) -> Self {
        Self { coeffs, z1: 0.0, z2: 0.0 }
    }

    /// The filter of **one** sample.
    pub fn process_sample(&mut self, x: f32) -> f32 {
        let coeffs = self.coeffs;
        let y = coeffs.b0 * x + self.z1;
        self.z1 = flush(coeffs.b1 * x - coeffs.a1 * y + self.z2);
        self.z2 = flush(coeffs.b2 * x - coeffs.a2 * y);
        if y.is_finite() {
            y
        } else {
            self.z1 = 0.0;
            self.z2 = 0.0;
            0.0
        }
    }
}

/// A number too small to be a number is nothing: the tree's own rule.
fn flush(x: f32) -> f32 {
    let size = x.abs();
    if size > 1.0e-25 && size.is_finite() { x } else { 0.0 }
}

/// A **gain** as decibels: the tree's own rule.
pub fn gain_to_db(gain: f32) -> f32 {
    let size = gain.abs();
    if size.is_nan() {
        return MIN_DB;
    }
    if size.is_infinite() {
        return f32::MAX;
    }
    (20.0 * size.log10()).max(MIN_DB)
}

/// The **K-weighting** of BS.1770: a shelf and a high-pass, designed
/// for the sample rate at hand.
fn k_weighting(rate: f64) -> [Coeffs; 2] {
    // The **prewarp**: a corner at or past Nyquist wraps the tangent
    // negative, so it is held just below it.
    let prewarp = |f0: f64| (PI * f0.min(rate * 0.49) / rate).tan();
    // The shelf.
    let (f0, g, q) = (1_681.974_450_955_533, 3.999_843_853_973_347, 0.707_175_236_955_419_6);
    let k = prewarp(f0);
    let vh = 10f64.powf(g / 20.0);
    let vb = vh.powf(0.499_666_774_154_541_6);
    let a0 = 1.0 + k / q + k * k;
    let shelf = Coeffs {
        b0: ((vh + vb * k / q + k * k) / a0) as f32,
        b1: (2.0 * (k * k - vh) / a0) as f32,
        b2: ((vh - vb * k / q + k * k) / a0) as f32,
        a1: (2.0 * (k * k - 1.0) / a0) as f32,
        a2: ((1.0 - k / q + k * k) / a0) as f32,
    };
    // The high-pass.
    let (f0, q) = (38.135_470_876_024_44, 0.500_327_037_323_877_3);
    let k = prewarp(f0);
    let a0 = 1.0 + k / q + k * k;
    let high_pass = Coeffs {
        b0: 1.0,
        b1: -2.0,
        b2: 1.0,
        a1: (2.0 * (k * k - 1.0) / a0) as f32,
        a2: ((1.0 - k / q + k * k) / a0) as f32,
    };
    [shelf, high_pass]
}

/// The loudness of a mean-square energy: BS.1770's own equation.
fn lufs(energy: f64) -> f32 {
    if energy <= 1.0e-20 { MIN_DB } else { (-0.691 + 10.0 * energy.log10()) as f32 }
}

/// The **loudness** of a sound.
///
/// The tree's own meter: the momentary, the short-term and the gated
/// integrated loudness, and the true peak beside them.
#[derive(Debug, Clone)]
pub struct LoudnessMeter {
    rate: f32,
    weights: Vec<f32>,
    filters: Vec<[Biquad; 2]>,
    block_len: usize,
    block_pos: usize,
    block_acc: Vec<f64>,
    sub: [f64; SUB_BLOCKS],
    sub_idx: usize,
    sub_filled: usize,
    hist_count: Vec<u64>,
    hist_energy: Vec<f64>,
    tp_coefs: [[f32; TP_PHASES]; TP_TAPS],
    tp_hist: Vec<[f32; 2 * TP_TAPS]>,
    tp_pos: Vec<usize>,
    true_peak: f32,
    max_momentary: f32,
}

impl LoudnessMeter {
    /// A meter for a sound of a sample rate and a number of channels.
    ///
    /// The **weights** of the channels are the tree's own: every
    /// channel counts once, except in a **six-channel** sound, where
    /// the low-frequency effects channel counts not at all and the two
    /// surrounds count one and a half times.
    pub fn new(sample_rate: f32, channels: usize) -> Self {
        let rate = if sample_rate.is_finite() && sample_rate >= 1000.0 {
            sample_rate.min(1_536_000.0)
        } else {
            48_000.0
        };
        let channels = channels.clamp(1, MAX_CHANNELS);
        let weights = if channels == 6 { vec![1.0, 1.0, 1.0, 0.0, 1.41, 1.41] } else { vec![1.0; channels] };
        let [shelf, high_pass] = k_weighting(f64::from(rate));
        // The **filter that finds the true peak**: a windowed sinc at
        // the Nyquist of the sound, in four phases.
        let mut tp_coefs = [[0.0f32; TP_PHASES]; TP_TAPS];
        let total = TP_TAPS * TP_PHASES;
        let center = (total as f64 - 1.0) / 2.0;
        for index in 0..total {
            let t = (index as f64 - center) / TP_PHASES as f64;
            let sinc = if t.abs() < 1.0e-12 { 1.0 } else { (PI * t).sin() / (PI * t) };
            let x = index as f64 / (total as f64 - 1.0);
            let window = 0.42 - 0.5 * (2.0 * PI * x).cos() + 0.08 * (4.0 * PI * x).cos();
            if let Some(slot) =
                tp_coefs.get_mut(index / TP_PHASES).and_then(|tap| tap.get_mut(index % TP_PHASES))
            {
                *slot = (sinc * window) as f32;
            }
        }
        for phase in 0..TP_PHASES {
            let sum: f32 = tp_coefs.iter().filter_map(|tap| tap.get(phase)).sum();
            if sum.abs() > 1.0e-6 {
                tp_coefs.iter_mut().filter_map(|tap| tap.get_mut(phase)).for_each(|slot| *slot /= sum);
            }
        }
        Self {
            rate,
            weights,
            filters: vec![[Biquad::new(shelf), Biquad::new(high_pass)]; channels],
            block_len: ((rate as f64 * 0.1).round() as usize).max(1),
            block_pos: 0,
            block_acc: vec![0.0; channels],
            sub: [0.0; SUB_BLOCKS],
            sub_idx: 0,
            sub_filled: 0,
            hist_count: vec![0; HIST_BINS],
            hist_energy: vec![0.0; HIST_BINS],
            tp_coefs,
            tp_hist: vec![[0.0; 2 * TP_TAPS]; channels],
            tp_pos: vec![0; channels],
            true_peak: 0.0,
            max_momentary: MIN_DB,
        }
    }

    /// The **sample rate** the meter reads at.
    pub fn sample_rate(&self) -> f32 {
        self.rate
    }

    /// The **weight** of a channel: the tree's own rule.
    pub fn set_channel_weight(&mut self, channel: usize, weight: f32) {
        if let Some(slot) = self.weights.get_mut(channel) {
            *slot = if weight.is_finite() { weight.clamp(0.0, 4.0) } else { 1.0 };
        }
    }

    /// **Feed** the meter a sound: the channels are given one after
    /// another, and the number of frames is the number of samples of
    /// the first channel.
    pub fn process(&mut self, channels: &[Vec<f32>]) {
        let count = self.filters.len().min(channels.len());
        let frames = channels.iter().take(count).map(Vec::len).min().unwrap_or(0);
        for frame in 0..frames {
            for channel in 0..count {
                let sample = channels
                    .get(channel)
                    .and_then(|lane| lane.get(frame))
                    .copied()
                    .filter(|value| value.is_finite())
                    .unwrap_or(0.0);
                if let Some([first, second]) = self.filters.get_mut(channel) {
                    let value = second.process_sample(first.process_sample(sample));
                    if let Some(acc) = self.block_acc.get_mut(channel) {
                        *acc += f64::from(value) * f64::from(value);
                    }
                }
                self.true_peak_sample(channel, sample);
            }
            self.block_pos += 1;
            if self.block_pos >= self.block_len {
                self.finish_block();
            }
        }
    }

    /// The **true peak** of one sample of one channel.
    fn true_peak_sample(&mut self, channel: usize, sample: f32) {
        let (Some(history), Some(position)) = (self.tp_hist.get_mut(channel), self.tp_pos.get_mut(channel))
        else {
            return;
        };
        *position = (*position + TP_TAPS - 1) % TP_TAPS;
        let position = *position;
        for slot in [position, position + TP_TAPS] {
            if let Some(value) = history.get_mut(slot) {
                *value = sample;
            }
        }
        let Some(window) = history.get(position..position + TP_TAPS) else { return };
        let mut values = [-0.0f32; TP_PHASES];
        for (coeffs, sample) in self.tp_coefs.iter().zip(window) {
            for (value, coefficient) in values.iter_mut().zip(coeffs) {
                *value += coefficient * sample;
            }
        }
        let mut peak = sample.abs();
        for value in values {
            peak = peak.max(value.abs());
        }
        if peak > self.true_peak {
            self.true_peak = peak;
        }
    }

    /// The **end** of a block of a tenth of a second.
    fn finish_block(&mut self) {
        let length = self.block_pos.max(1) as f64;
        let mut energy = 0.0;
        for (acc, weight) in self.block_acc.iter_mut().zip(&self.weights) {
            energy += f64::from(*weight) * *acc / length;
            *acc = 0.0;
        }
        self.block_pos = 0;
        if let Some(slot) = self.sub.get_mut(self.sub_idx) {
            *slot = energy;
        }
        self.sub_idx = (self.sub_idx + 1) % SUB_BLOCKS;
        self.sub_filled = (self.sub_filled + 1).min(SUB_BLOCKS);
        if self.sub_filled >= 4 {
            let energy = self.recent_energy(4);
            let loudness = lufs(energy);
            self.max_momentary = self.max_momentary.max(loudness);
            if loudness > HIST_MIN {
                let bin = (((loudness - HIST_MIN) / HIST_STEP) as usize).min(HIST_BINS - 1);
                if let (Some(count), Some(total)) =
                    (self.hist_count.get_mut(bin), self.hist_energy.get_mut(bin))
                {
                    *count += 1;
                    *total += energy;
                }
            }
        }
    }

    /// The **mean energy** of the last `n` blocks.
    fn recent_energy(&self, n: usize) -> f64 {
        let n = n.min(self.sub_filled);
        if n == 0 {
            return 0.0;
        }
        let mut sum = 0.0;
        for step in 1..=n {
            let index = (self.sub_idx + SUB_BLOCKS - step) % SUB_BLOCKS;
            sum += self.sub.get(index).copied().unwrap_or(0.0);
        }
        sum / n as f64
    }

    /// The **momentary** loudness: the last four hundred milliseconds,
    /// in LUFS. It is the least loudness until four hundred
    /// milliseconds have been measured.
    pub fn momentary_lufs(&self) -> f32 {
        if self.sub_filled < 4 { MIN_DB } else { lufs(self.recent_energy(4)) }
    }

    /// The **short-term** loudness: the last three seconds, or what
    /// there is of it, in LUFS.
    pub fn short_term_lufs(&self) -> f32 {
        if self.sub_filled < 4 { MIN_DB } else { lufs(self.recent_energy(SUB_BLOCKS)) }
    }

    /// The **highest** momentary loudness the meter has seen.
    pub fn max_momentary_lufs(&self) -> f32 {
        self.max_momentary
    }

    /// The **integrated** loudness: the mean of the blocks that are
    /// above the gate, in LUFS.
    ///
    /// The gate is the tree's own: first the blocks above **minus
    /// seventy** LUFS, and then those above **ten LU below** the mean
    /// of those.
    pub fn integrated_lufs(&self) -> f32 {
        let (mut count, mut energy) = (0u64, 0.0f64);
        for (blocks, total) in self.hist_count.iter().zip(&self.hist_energy) {
            count += blocks;
            energy += total;
        }
        if count == 0 {
            return MIN_DB;
        }
        let gate = lufs(energy / count as f64) - 10.0;
        let (mut kept, mut kept_energy) = (0u64, 0.0f64);
        for (index, (blocks, total)) in self.hist_count.iter().zip(&self.hist_energy).enumerate() {
            let centre = HIST_MIN + (index as f32 + 0.5) * HIST_STEP;
            if centre > gate {
                kept += blocks;
                kept_energy += total;
            }
        }
        if kept == 0 { MIN_DB } else { lufs(kept_energy / kept as f64) }
    }

    /// The **true peak** of the sound, in decibels above full scale.
    pub fn true_peak_db(&self) -> f32 {
        gain_to_db(self.true_peak)
    }
}
