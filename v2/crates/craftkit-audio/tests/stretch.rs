//! The **time stretch**: the sound that is longer and the same pitch.
//!
//! A **varispeed** plays the sound faster and the pitch falls with it;
//! a **stretch** makes the sound longer and the pitch stays. It is the
//! difference between a record played at the wrong speed and a record
//! played slowly, and the second is the one a musician asks for.
//!
//! The method is **WSOLA** — waveform-similarity overlap-add — and these
//! are the tests of it: the pitch stays, the loudness stays, the
//! channels stay apart, and a stretch of one is the sound itself.

use craftkit_audio::time_stretch;

/// The samples of a **tone** of that frequency.
fn a_tone(frequency: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|frame| {
            let t = frame as f64 / 48_000.0;
            (2.0 * std::f64::consts::PI * f64::from(frequency) * t).sin() as f32
        })
        .collect()
}

/// The **period** of the sound: the lag at which it looks most like
/// itself.
///
/// The autocorrelation is the measure, and not the crossings: a
/// stretched sound joins its frames where they join best, and the
/// joins move the wave a little, so the crossings wander even when
/// the pitch does not. The period does not wander.
fn period(samples: &[f32]) -> f64 {
    let middle = &samples[samples.len() / 4..samples.len() * 3 / 4];
    let mut best_lag = 1_usize;
    let mut best = f64::NEG_INFINITY;
    // The lags are from eight samples up: a sound at these rates does
    // not have a period of two samples, and a lag that short is alike
    // everything.
    for lag in 8..400 {
        let mut sum = 0.0_f64;
        let mut energy = 0.0_f64;
        for index in 0..middle.len() - lag {
            let here = f64::from(middle[index]);
            let there = f64::from(middle[index + lag]);
            sum += here * there;
            energy += here * here;
        }
        let correlation = sum / energy.max(1e-12).sqrt();
        if correlation > best {
            best = correlation;
            best_lag = lag;
        }
    }
    best_lag as f64
}

/// A stretched tone **keeps its pitch**: the sound is twice as long
/// and the wave crosses zero at the same rate.
#[test]
fn a_stretched_tone_keeps_its_pitch() {
    let tone = a_tone(440.0, 48_000);
    let stretched = time_stretch(&tone, 1, 2.0, 48_000);
    assert_eq!(stretched.len(), 96_000, "the sound is twice as long");
    let before = period(&tone);
    let after = period(&stretched);
    let difference = (before - after).abs();
    assert!(difference <= 2.0, "the same period: {before} and {after}");
}

/// The sound is **louder** than it was: the frames overlap and the
/// windows are divided by what they add up to, and what is left is the
/// sound itself.
#[test]
fn a_stretched_tone_is_as_loud_as_it_was() {
    let tone = a_tone(440.0, 48_000);
    let stretched = time_stretch(&tone, 1, 2.0, 48_000);
    let loudest = |samples: &[f32]| samples.iter().fold(0.0_f32, |most, s| most.max(s.abs()));
    let before = loudest(&tone);
    let after = loudest(&stretched);
    assert!(after > before * 0.8 && after < before * 1.2, "as loud: {before} and {after}");
}

/// A stretch of **one** is the sound itself.
#[test]
fn a_stretch_of_one_is_the_sound() {
    let tone = a_tone(440.0, 4_800);
    assert_eq!(time_stretch(&tone, 1, 1.0, 48_000), tone);
}

/// The **channels** are kept apart.
#[test]
fn the_channels_are_kept_apart() {
    let left = a_tone(440.0, 4_800);
    let right = a_tone(660.0, 4_800);
    let mut both = Vec::with_capacity(left.len() * 2);
    for (l, r) in left.iter().zip(right.iter()) {
        both.push(*l);
        both.push(*r);
    }
    let stretched = time_stretch(&both, 2, 2.0, 48_000);
    let left_of = |samples: &[f32]| -> Vec<f32> { samples.chunks(2).map(|frame| frame[0]).collect() };
    let right_of = |samples: &[f32]| -> Vec<f32> { samples.chunks(2).map(|frame| frame[1]).collect() };
    assert_eq!(left_of(&stretched).len(), 9_600, "the sound is twice as long");
    assert!(period(&left_of(&stretched)) > 50.0, "the left channel is still there");
    assert!(period(&right_of(&stretched)) > 30.0, "the right channel is still there");
}

/// A stretch of **half** is the sound played backwards in time: the
/// sound is half as long and the pitch stays.
#[test]
fn a_stretch_of_half_is_shorter_and_the_same_pitch() {
    let tone = a_tone(440.0, 48_000);
    let stretched = time_stretch(&tone, 1, 0.5, 48_000);
    assert_eq!(stretched.len(), 24_000, "the sound is half as long");
    let before = period(&tone);
    let after = period(&stretched);
    assert!((before - after).abs() <= 2.0, "the same period: {before} and {after}");
}

/// A stretch that is **out of range** is **clamped**: the method can
/// only do so much, and the clamp is the last line of defence and not
/// the policy. The caller refuses such a stretch; the method does not
/// refuse it.
#[test]
fn a_stretch_out_of_range_is_clamped() {
    let tone = a_tone(440.0, 4_800);
    let stretched = time_stretch(&tone, 1, 1_000.0, 48_000);
    assert_eq!(stretched.len(), 48_000, "the sound is ten times as long, and not a thousand");
}

/// An empty sound has nothing to stretch.
#[test]
fn an_empty_sound_stays_empty() {
    assert!(time_stretch(&[], 1, 2.0, 48_000).is_empty(), "there is no sound to stretch");
}
