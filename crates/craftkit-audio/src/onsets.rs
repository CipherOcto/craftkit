//! The **onsets** of a sound, found in its **spectrum**.
//!
//! The tree's detector watches how much of each **band** of the
//! spectrum rises from one frame to the next. A drum is a wide burst
//! of energy and the bands rise together; a bowed string is a
//! sustained tone and the bands do not. The rise of the bands is
//! called the **flux**, and a peak in the flux is an onset.
//!
//! # How it is done here
//!
//! - The sound is read in **frames** of about twelve milliseconds, and
//!   the frames overlap: each one starts a quarter of a frame after
//!   the one before it.
//! - Each frame is **transformed** (§ the transform), and the power at
//!   each frequency is gathered into **bands**: twenty-four to the
//!   octave, from thirty hertz to twenty kilohertz, each band a
//!   triangle over the bins it covers.
//! - Each band's level is the **logarithm** of its power, so that a
//!   quiet band and a loud one are measured the same way.
//! - The **flux** of a frame is how much each band rose against the
//!   loudest of its **neighbours**: a band that rose while the one
//!   beside it rose more is not a rise of its own, and a note that
//!   bends is not an onset.
//! - A frame is an onset when its flux stands out from the flux
//!   around it by a **fixed amount** — and not by an amount relative
//!   to the loudest hit, so that a quiet hit still counts.
//! - The onset is then placed at the **attack**: the place where the
//!   energy first rises, which the frame's own start may be a little
//!   after.

use crate::fft;

/// The most onsets a sound may have.
///
/// Ten thousand is more than a person hears in a piece of music, and a
/// sound with more of them is a sound the detector does not trust.
pub const MAX_ONSETS: usize = 10_000;

/// The most channels a detector reads.
const MAX_CHANNELS: usize = 32;

/// The **onsets** of a sound, in samples from its start.
///
/// The samples are **interleaved**: the sound's channels come one after
/// another, and the spectrum of a frame is worked out across the
/// channels of it.
///
/// The sensitivity is a number from zero to one: the higher it is, the
/// smaller a rise may be and still be an onset. A number outside that
/// range is **clamped** into it — the sensitivity is a knob and not a
/// fact about the sound, and a knob that is turned past its end is at
/// its end.
pub fn onsets(samples: &[f32], channels: usize, rate: u32, sensitivity: f64) -> Vec<usize> {
    let channels = channels.max(1);
    let frames = samples.len() / channels;
    let rate = rate.max(1);
    let sensitivity = if sensitivity.is_finite() { sensitivity.clamp(0.0, 1.0) } else { 0.5 };
    // The **frame**: about twelve milliseconds, and a power of two, so
    // that the transform is the transform of a power of two.
    let size = (((rate as f64 * 0.012) as usize).clamp(64, 8192)).next_power_of_two();
    let hop = size / 4;
    if frames < size || size < 2 {
        return Vec::new();
    }
    // The **channels**: each one on its own, and the samples that are
    // not numbers are silence.
    let mut planes: Vec<Vec<f32>> = Vec::with_capacity(channels.min(MAX_CHANNELS));
    for channel in 0..channels.min(MAX_CHANNELS) {
        let plane: Vec<f32> = (0..frames)
            .map(|frame| {
                let value = samples.get(frame * channels + channel).copied().unwrap_or(0.0);
                if value.is_finite() { value } else { 0.0 }
            })
            .collect();
        planes.push(plane);
    }
    if planes.is_empty() {
        return Vec::new();
    }
    let count = planes.len() as f64;
    let bands = log_bands(size, rate as f64);
    // The **window**: a raised cosine, so that the ends of a frame do
    // not make a click of their own.
    let window: Vec<f32> = (0..size)
        .map(|index| (0.5 - 0.5 * (2.0 * std::f64::consts::PI * index as f64 / size as f64).cos()) as f32)
        .collect();
    // The magnitudes are relative to a full-scale sine: below about
    // minus eighty decibels a band's changes barely count.
    let gain = 1.0e4 / (size as f32 / 4.0);
    let count_frames = (frames - size) / hop + 1;
    let mut frame_samples: Vec<f32> = vec![0.0; size];
    let mut previous: Vec<f32> = Vec::new();
    let mut flux: Vec<f32> = Vec::with_capacity(count_frames);
    let mut energy: Vec<f32> = Vec::with_capacity(count_frames);
    for index in 0..count_frames {
        let start = index * hop;
        // The **power** of the frame: the powers of the channels added
        // together and averaged.
        let mut powers = vec![0.0_f32; size / 2 + 1];
        for plane in &planes {
            for (position, value) in frame_samples.iter_mut().enumerate() {
                *value = plane.get(start + position).copied().unwrap_or(0.0) * window[position];
            }
            let spectrum = fft::power(&frame_samples);
            for (power, value) in powers.iter_mut().zip(&spectrum) {
                *power += value / count as f32;
            }
        }
        energy.push(powers.iter().sum());
        // The **bands**: the level of each one, in logarithms.
        let current: Vec<f32> = bands
            .iter()
            .map(|band| {
                let sum: f32 = band
                    .iter()
                    .map(|(bin, weight)| powers.get(*bin).copied().unwrap_or(0.0).sqrt() * *weight)
                    .sum();
                (1.0 + gain * sum).log10()
            })
            .collect();
        // The **flux**: how much each band rose against the loudest of
        // its neighbours.
        let rise = |band: usize, value: f32| {
            let from = band.saturating_sub(1);
            let to = (band + 2).min(previous.len());
            let loudest = previous
                .get(from..to)
                .map(|around| around.iter().copied().fold(0.0_f32, f32::max))
                .unwrap_or(0.0);
            (value - loudest).max(0.0)
        };
        flux.push(if previous.is_empty() {
            0.0
        } else {
            current.iter().enumerate().map(|(band, value)| rise(band, *value)).sum()
        });
        previous = current;
    }
    if flux.is_empty() {
        return Vec::new();
    }
    // The **windows** the flux is measured in, in frames.
    let frames_of = |seconds: f64| (((seconds * rate as f64) as usize) / hop).max(1);
    let (near, before, after, gap) = (frames_of(0.03), frames_of(0.1), frames_of(0.07), frames_of(0.03));
    // The **amount** a peak must stand out by: a fixed amount and not
    // one relative to the loudest hit, so that a quiet hit counts.
    let delta = 2.5 * 2.0_f32.powf((0.5 - sensitivity as f32) * 4.0);
    let mut found: Vec<usize> = Vec::new();
    let mut last: Option<usize> = None;
    for (index, current) in flux.iter().enumerate() {
        let span = |before: usize, after: usize| {
            flux.get(index.saturating_sub(before)..(index + after + 1).min(flux.len())).unwrap_or(&[])
        };
        let peak = span(near, near).iter().copied().fold(0.0_f32, f32::max);
        let around = span(before, after);
        let mean = around.iter().sum::<f32>() / around.len().max(1) as f32;
        if *current <= 0.0 || *current < peak || *current < mean + delta {
            continue;
        }
        if last.is_some_and(|last| index - last < gap) {
            continue;
        }
        let (at, contrast) = attack_start(&planes, index * hop, size, rate as f64);
        // A fast release smears energy between a note's partials, which
        // looks like a rise. Its attack edge is weak and the energy
        // does not grow, unlike a real onset's.
        let was = energy.get(index.saturating_sub(1)).copied().unwrap_or(0.0);
        let grows = energy
            .get(index..(index + 2).min(energy.len()))
            .is_some_and(|around| around.iter().any(|value| *value > was));
        if contrast < std::f64::consts::LN_2 && !grows {
            continue;
        }
        last = Some(index);
        if found.last().is_none_or(|last| at > *last) {
            found.push(at);
            if found.len() >= MAX_ONSETS {
                break;
            }
        }
    }
    found
}

/// The **bands**: twenty-four to the octave, from thirty hertz to
/// twenty kilohertz, over the bins of a transform of `size` points.
///
/// Each band is a triangle over the bins it covers, and the weights
/// of a band sum to one. Where the bins are coarser than the bands,
/// the bands that would fall on the same bin are left out.
fn log_bands(size: usize, rate: f64) -> Vec<Vec<(usize, f32)>> {
    let bin_hz = rate / size as f64;
    let top = 20_000.0_f64.min(rate * 0.45);
    let mut centres: Vec<usize> = Vec::new();
    let mut frequency = 30.0_f64;
    while frequency <= top {
        let bin = (frequency / bin_hz).round() as usize;
        if centres.last().is_none_or(|last| bin > *last) {
            centres.push(bin);
        }
        frequency *= 2.0_f64.powf(1.0 / 24.0);
    }
    centres
        .windows(3)
        .filter_map(|window| {
            let (low, middle, high) = (window[0], window[1], window[2]);
            let triangle: Vec<(usize, f32)> = (low..=high)
                .map(|bin| {
                    let weight = if bin <= middle {
                        (bin - low) as f32 / (middle - low).max(1) as f32
                    } else {
                        (high - bin) as f32 / (high - middle).max(1) as f32
                    };
                    (bin, weight)
                })
                .filter(|(_, weight)| *weight > 0.0)
                .collect();
            let sum: f32 = triangle.iter().map(|(_, weight)| weight).sum();
            if sum <= 0.0 {
                return None;
            }
            Some(triangle.into_iter().map(|(bin, weight)| (bin, weight / sum)).collect())
        })
        .collect()
}

/// Where the attack of an onset **begins**, and how strong it is.
///
/// The log spectrum jumps as soon as the window reaches an attack, so
/// the frame's own start may be a little after the attack. The step
/// edge of the high-passed energy is found — the sample where the next
/// three milliseconds gain most over the six milliseconds before — and
/// the attack is then walked back to where the energy first crosses
/// the midpoint, in decibels, between the two.
fn attack_start(planes: &[Vec<f32>], frame: usize, size: usize, rate: f64) -> (usize, f64) {
    let (before, after, short) = (
        ((rate * 0.006) as usize).max(1),
        ((rate * 0.003) as usize).max(1),
        ((rate * 0.0005) as usize).max(1),
    );
    let from = frame.saturating_sub(before);
    let to = frame + size + size / 2 + after;
    let mut cumulative = vec![0.0_f64; to - from + 1];
    for (offset, index) in (from..to).enumerate() {
        let energy: f32 = planes
            .iter()
            .map(|plane| {
                let value = plane.get(index).copied().unwrap_or(0.0);
                let previous =
                    index.checked_sub(1).and_then(|before| plane.get(before)).copied().unwrap_or(0.0);
                let difference = value - previous;
                difference * difference
            })
            .sum();
        let sum = cumulative.get(offset).copied().unwrap_or(0.0) + f64::from(energy);
        if let Some(slot) = cumulative.get_mut(offset + 1) {
            *slot = sum;
        }
    }
    let at = |index: usize| cumulative.get(index.saturating_sub(from)).copied().unwrap_or(0.0);
    let mean = |a: usize, b: usize| (at(b) - at(a)) / b.saturating_sub(a).max(1) as f64;
    let contrast = |index: usize| {
        ((mean(index, index + after) + 1e-12) / (mean(index.saturating_sub(before), index) + 1e-12)).ln()
    };
    let best = (frame..frame + size + size / 2)
        .max_by(|a, b| contrast(*a).total_cmp(&contrast(*b)))
        .unwrap_or(frame);
    let level = (mean(best.saturating_sub(before), best) * mean(best, best + after)).sqrt();
    let attack =
        (best.saturating_sub(after)..=best).find(|index| mean(*index, index + short) > level).unwrap_or(best);
    (attack, contrast(best))
}
