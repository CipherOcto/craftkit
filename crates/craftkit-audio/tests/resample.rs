//! The **resampler**: a windowed sinc, and what it is for.
//!
//! A resampler that reads with a line throws away everything above half
//! the old rate, and what is thrown away folds back into the sound as
//! a whistle. These are the tests of the better resampler: the tone
//! keeps its pitch, and what was above the new half is taken away.

use craftkit_audio::{Error, Limits};

/// The samples of a **tone** of that frequency, at that rate.
fn a_tone(rate: u32, frequency: f32, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|frame| {
            let t = frame as f64 / f64::from(rate);
            (2.0 * std::f64::consts::PI * f64::from(frequency) * t).sin() as f32
        })
        .collect()
}

/// Where the wave crosses zero **going up**, away from the edges.
///
/// The edges are the kernel's own: at the start and the end of the
/// sound the kernel reads outside it, and a crossing there is the
/// kernel's and not the wave's.
fn up_crossings(samples: &[f32]) -> Vec<usize> {
    let mut out = Vec::new();
    for index in samples.len() / 4..samples.len() * 3 / 4 {
        let before = samples[index - 1];
        let after = samples[index];
        if before <= 0.0 && after > 0.0 {
            out.push(index);
        }
    }
    out
}

/// The **period** of a wave: how many samples between its crossings.
fn period(samples: &[f32]) -> f64 {
    let crossings = up_crossings(samples);
    assert!(crossings.len() >= 2, "the wave crosses zero more than once: {crossings:?}");
    let first = crossings[0];
    let last = crossings[crossings.len() - 1];
    (last - first) as f64 / (crossings.len() - 1) as f64
}

/// How **loud** a sound is: the root of the mean of its squares.
fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|sample| f64::from(*sample) * f64::from(*sample)).sum();
    (sum / samples.len() as f64).sqrt()
}

/// A tone keeps its **pitch** when the rate is changed: a thousand
/// hertz is a thousand hertz however fast it is played.
#[test]
fn a_tone_keeps_its_pitch() {
    let limits = Limits::default();
    let tone = a_tone(8_000, 1_000.0, 400);
    assert!((period(&tone) - 8.0).abs() < 1e-6, "a thousand hertz at eight thousand is eight samples");
    let faster = craftkit_audio::resample(&tone, 1, 8_000, 16_000, &limits).expect("the sound is resampled");
    assert!(
        (period(&faster) - 16.0).abs() < 0.5,
        "and sixteen samples at sixteen thousand: {}",
        period(&faster)
    );
    let slower = craftkit_audio::resample(&tone, 1, 8_000, 4_000, &limits).expect("the sound is resampled");
    assert!((period(&slower) - 4.0).abs() < 0.5, "and four at four thousand: {}", period(&slower));
}

/// What was **above the new half** of the rate is taken away: it would
/// otherwise fold back into the sound as a whistle.
///
/// This is the whole reason the resampler is a filter and not a line.
#[test]
fn what_was_above_the_new_half_is_taken_away() {
    let limits = Limits::default();
    // Three thousand hertz at eight thousand: a tone that is above the
    // half of the rate it is played at (two thousand).
    let tone = a_tone(8_000, 3_000.0, 400);
    let quieter = craftkit_audio::resample(&tone, 1, 8_000, 4_000, &limits).expect("the sound is resampled");
    let before = rms(&tone);
    let after = rms(&quieter);
    assert!(after < before * 0.05, "the whistle is taken away: {before} became {after}");
}

/// A tone that is **inside** the new half is kept: the filter takes
/// the whistle and not the music.
#[test]
fn a_tone_inside_the_new_half_is_kept() {
    let limits = Limits::default();
    let tone = a_tone(8_000, 500.0, 400);
    let quieter = craftkit_audio::resample(&tone, 1, 8_000, 4_000, &limits).expect("the sound is resampled");
    let before = rms(&tone);
    let after = rms(&quieter);
    assert!(
        after > before * 0.8 && after < before * 1.2,
        "the music is kept as it is: {before} became {after}"
    );
}

/// A sound that would be **too long** at the new rate is refused: it
/// is not truncated, because a truncated sound is wrong in a way nobody
/// can hear.
#[test]
fn a_sound_that_would_be_too_long_is_refused() {
    let limits = Limits { max_frames: 10 };
    let tone = a_tone(8_000, 500.0, 100);
    let error = craftkit_audio::resample(&tone, 1, 8_000, 16_000, &limits)
        .expect_err("two hundred frames is longer than ten");
    assert_eq!(error, Error::SoundTooLong { frames: 200 });
}

/// The **channels** are kept apart: a frame of the new sound is a frame
/// of the old one.
#[test]
fn the_channels_are_kept_apart() {
    let limits = Limits::default();
    // A tone on the left and silence on the right.
    let tone = a_tone(8_000, 500.0, 200);
    let mut samples = Vec::with_capacity(tone.len() * 2);
    for sample in &tone {
        samples.push(*sample);
        samples.push(0.0);
    }
    let resampled =
        craftkit_audio::resample(&samples, 2, 8_000, 16_000, &limits).expect("the sound is resampled");
    assert_eq!(resampled.len(), 800, "two channels of four hundred frames");
    let right =
        resampled.iter().skip(1).step_by(2).fold(0.0_f32, |loudest, sample| loudest.max(sample.abs()));
    assert!(right < 1e-6, "the right channel is silent: {right}");
    let left = resampled.iter().step_by(2).fold(0.0_f32, |loudest, sample| loudest.max(sample.abs()));
    assert!(left > 0.5, "and the left channel is the tone: {left}");
}
