//! The tests of the mixer.
//!
//! The mixer is arithmetic, and arithmetic has one rule: the sum is
//! the sum. So the tests are the sum, the gain, the length — and the
//! refusals, because a track's start and its gain came from a document
//! and a document can say anything.

use craftkit_audio::{Error, Limits, Track, db_to_linear, mix};

/// The limits every test uses.
fn limits() -> Limits {
    Limits::default()
}

/// Two tracks are **summed**: the second one is added to the first.
#[test]
fn two_tracks_are_summed() {
    let first: Vec<f32> = vec![0.5, 0.5];
    let second: Vec<f32> = vec![0.25, 0.25];
    let tracks =
        [Track { start: 0, samples: &first, gain: 1.0 }, Track { start: 0, samples: &second, gain: 1.0 }];
    let mixed = mix(&tracks, 48_000, &limits()).expect("the tracks are mixed");
    assert_eq!(mixed.frames, 2);
    assert!((mixed.samples[0] - 0.75).abs() < 1e-6, "the first sample is the sum");
    assert!((mixed.samples[1] - 0.75).abs() < 1e-6, "and so is the second");
}

/// A track that starts later lands **later**.
#[test]
fn a_track_that_starts_later_lands_later() {
    let late: Vec<f32> = vec![1.0];
    let tracks = [Track { start: 3, samples: &late, gain: 1.0 }];
    let mixed = mix(&tracks, 48_000, &limits()).expect("the track is mixed");
    assert_eq!(mixed.frames, 4, "the mix is as long as the track's end");
    assert!((mixed.samples[0] - 0.0).abs() < 1e-6, "nothing is at the beginning");
    assert!((mixed.samples[3] - 1.0).abs() < 1e-6, "the track is where it said");
}

/// A track's **gain** multiplies it.
#[test]
fn a_gain_multiplies_the_track() {
    let track: Vec<f32> = vec![1.0, 1.0];
    let tracks = [Track { start: 0, samples: &track, gain: 0.5 }];
    let mixed = mix(&tracks, 48_000, &limits()).expect("the track is mixed");
    assert!((mixed.samples[0] - 0.5).abs() < 1e-6, "the sample is halved");
}

/// The gain in **decibels** is the gain of a ratio.
#[test]
fn a_decibel_is_a_ratio() {
    assert!((db_to_linear(0.0) - 1.0).abs() < 1e-9, "no decibel is no change");
    assert!((db_to_linear(-6.0206) - 0.5).abs() < 1e-4, "six decibels down is about half");
    assert!((db_to_linear(6.0206) - 2.0).abs() < 1e-4, "and six up is about twice");
}

/// A track that starts **before the beginning** is refused.
#[test]
fn a_track_that_starts_before_the_beginning_is_refused() {
    let track: Vec<f32> = vec![1.0];
    let tracks = [Track { start: -1, samples: &track, gain: 1.0 }];
    let error = mix(&tracks, 48_000, &limits()).expect_err("that is before the beginning");
    assert_eq!(error, Error::NegativeStart { start: -1 });
}

/// A gain that is not a number is refused.
#[test]
fn a_gain_that_is_not_a_number_is_refused() {
    let track: Vec<f32> = vec![1.0];
    let tracks = [Track { start: 0, samples: &track, gain: f64::NAN }];
    let error = mix(&tracks, 48_000, &limits()).expect_err("that is not a gain");
    assert_eq!(error, Error::BadGain);
}

/// A mix longer than the limits allow is refused: the bound is the
/// point.
#[test]
fn a_mix_longer_than_the_limits_allow_is_refused() {
    let track: Vec<f32> = vec![0.0; 16];
    let tracks = [Track { start: 0, samples: &track, gain: 1.0 }];
    let limits = Limits { max_frames: 8 };
    let error = mix(&tracks, 48_000, &limits).expect_err("that is longer than the limit");
    assert_eq!(error, Error::TooLong { frames: 16 });
}

/// The **clipped** samples are within the range a file may hold.
#[test]
fn the_clipped_samples_are_within_the_range() {
    let samples = vec![2.0_f32, -2.0, 0.5];
    let clipped = craftkit_audio::clipped(&samples);
    assert_eq!(clipped[0], 1.0);
    assert_eq!(clipped[1], -1.0);
    assert_eq!(clipped[2], 0.5, "and what fits is untouched");
}

/// The **resampler** keeps the sound and changes its rate.
///
/// The resampling is linear, and the test says what that means: a ramp
/// stays a ramp, and the number of frames is in proportion.
#[test]
fn the_resampler_keeps_the_sound_and_changes_its_rate() {
    // A ramp: a quarter, a half, three quarters.
    let samples = vec![0.25_f32, 0.5, 0.75];
    let faster = craftkit_audio::resample(&samples, 1, 8_000, 16_000);
    assert_eq!(faster.len(), 6, "twice the rate is twice the frames");
    // The ramp is still a ramp: the first sample is the first, and the
    // second is between the first and the second.
    assert!((faster[0] - 0.25).abs() < 1e-6, "the first sample is the first");
    assert!((faster[1] - 0.375).abs() < 1e-6, "the second is between: {}", faster[1]);
    let slower = craftkit_audio::resample(&samples, 1, 16_000, 8_000);
    assert_eq!(slower.len(), 2, "half the rate is half the frames");
    assert!((slower[0] - 0.25).abs() < 1e-6, "and the sound is the same one");
}

/// The resampler keeps the **channels** apart.
#[test]
fn the_resampler_keeps_the_channels_apart() {
    // Two channels: a ramp on the left and silence on the right.
    let samples = vec![0.25_f32, 0.0, 0.5, 0.0, 0.75, 0.0];
    let faster = craftkit_audio::resample(&samples, 2, 8_000, 16_000);
    assert_eq!(faster.len(), 12, "two channels of six frames");
    assert!((faster[0] - 0.25).abs() < 1e-6, "the left channel is the ramp");
    assert!((faster[1] - 0.0).abs() < 1e-6, "and the right channel is silent");
}
