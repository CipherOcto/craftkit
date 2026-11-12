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
    let tracks = [
        Track { start: 0, samples: first.clone(), gain: 1.0 },
        Track { start: 0, samples: second.clone(), gain: 1.0 },
    ];
    let mixed = mix(&tracks, 48_000, &limits()).expect("the tracks are mixed");
    assert_eq!(mixed.frames, 2);
    assert!((mixed.samples[0] - 0.75).abs() < 1e-6, "the first sample is the sum");
    assert!((mixed.samples[1] - 0.75).abs() < 1e-6, "and so is the second");
}

/// A track that starts later lands **later**.
#[test]
fn a_track_that_starts_later_lands_later() {
    let late: Vec<f32> = vec![1.0];
    let tracks = [Track { start: 3, samples: late.clone(), gain: 1.0 }];
    let mixed = mix(&tracks, 48_000, &limits()).expect("the track is mixed");
    assert_eq!(mixed.frames, 4, "the mix is as long as the track's end");
    assert!((mixed.samples[0] - 0.0).abs() < 1e-6, "nothing is at the beginning");
    assert!((mixed.samples[3] - 1.0).abs() < 1e-6, "the track is where it said");
}

/// A track's **gain** multiplies it.
#[test]
fn a_gain_multiplies_the_track() {
    let track: Vec<f32> = vec![1.0, 1.0];
    let tracks = [Track { start: 0, samples: track.clone(), gain: 0.5 }];
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
    let tracks = [Track { start: -1, samples: track.clone(), gain: 1.0 }];
    let error = mix(&tracks, 48_000, &limits()).expect_err("that is before the beginning");
    assert_eq!(error, Error::NegativeStart { start: -1 });
}

/// A gain that is not a number is refused.
#[test]
fn a_gain_that_is_not_a_number_is_refused() {
    let track: Vec<f32> = vec![1.0];
    let tracks = [Track { start: 0, samples: track.clone(), gain: f64::NAN }];
    let error = mix(&tracks, 48_000, &limits()).expect_err("that is not a gain");
    assert_eq!(error, Error::BadGain);
}

/// A mix longer than the limits allow is refused: the bound is the
/// point.
#[test]
fn a_mix_longer_than_the_limits_allow_is_refused() {
    let track: Vec<f32> = vec![0.0; 16];
    let tracks = [Track { start: 0, samples: track.clone(), gain: 1.0 }];
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
/// The number of frames is in proportion, and the sound is the same
/// sound: a ramp is still a ramp, read with the kernel instead of a
/// line.
///
/// The ramp is long enough for the kernel: the kernel is wider than a
/// few samples, and at the **edges** of a sound shorter than the
/// kernel the kernel reads outside the sound — which is why the
/// middle is what is measured. And the ramp is a ramp only to within
/// the filter: a ramp has something at every frequency, and the
/// filter takes the top away.
#[test]
fn the_resampler_keeps_the_sound_and_changes_its_rate() {
    // A ramp: sixty-four samples, a hundredth apart.
    let samples: Vec<f32> = (0..64).map(|index| 0.25 + index as f32 * 0.01).collect();
    let limits = craftkit_audio::Limits::default();
    let faster =
        craftkit_audio::resample(&samples, 1, 8_000, 16_000, &limits).expect("the sound is resampled");
    assert_eq!(faster.len(), 128, "twice the rate is twice the frames");
    // The ramp is still a ramp: the middle of the new sound is the
    // middle of the old one, read at the new rate.
    let middle = faster.len() / 2;
    let wanted = 0.25 + (middle as f32 / 2.0) * 0.01;
    assert!((faster[middle] - wanted).abs() < 5e-3, "the ramp is the ramp: {}", faster[middle]);
    let slower =
        craftkit_audio::resample(&samples, 1, 16_000, 8_000, &limits).expect("the sound is resampled");
    assert_eq!(slower.len(), 32, "half the rate is half the frames");
    let middle = slower.len() / 2;
    let wanted = 0.25 + (middle as f32 * 2.0) * 0.01;
    assert!((slower[middle] - wanted).abs() < 5e-3, "and the sound is the same one: {}", slower[middle]);
}

/// The resampler keeps the **channels** apart.
#[test]
fn the_resampler_keeps_the_channels_apart() {
    // Two channels: a ramp on the left and silence on the right.
    let ramp: Vec<f32> = (0..64).map(|index| 0.25 + index as f32 * 0.01).collect();
    let mut samples = Vec::with_capacity(ramp.len() * 2);
    for sample in &ramp {
        samples.push(*sample);
        samples.push(0.0);
    }
    let limits = craftkit_audio::Limits::default();
    let faster =
        craftkit_audio::resample(&samples, 2, 8_000, 16_000, &limits).expect("the sound is resampled");
    assert_eq!(faster.len(), 256, "two channels of a hundred and twenty-eight frames");
    let frame = (faster.len() / 2) / 2;
    let wanted = 0.25 + (frame as f32 / 2.0) * 0.01;
    assert!((faster[frame * 2] - wanted).abs() < 5e-3, "the left channel is the ramp");
    assert!((faster[frame * 2 + 1] - 0.0).abs() < 1e-6, "and the right channel is silent");
}

/// The **varispeed** makes the sound longer and lower.
///
/// The factor is a length ratio: twice the factor is twice the frames,
/// and the sound is read at half the rate, so the pitch falls with it.
#[test]
fn the_varispeed_makes_the_sound_longer_and_lower() {
    // One cycle of a wave: up, down, up, down.
    let samples = vec![0.0_f32, 1.0, 0.0, -1.0];
    let slower = craftkit_audio::varispeed(&samples, 1, 2.0);
    assert_eq!(slower.len(), 8, "twice the factor is twice the frames");
    // The sound is read at half the rate: the second frame is halfway
    // between the first and the second.
    assert!((slower[0] - 0.0).abs() < 1e-6, "the first frame is the first");
    assert!((slower[1] - 0.5).abs() < 1e-6, "the second is halfway: {}", slower[1]);
    assert!((slower[2] - 1.0).abs() < 1e-6, "and the third is the second");
}

/// The varispeed keeps the **channels** apart.
#[test]
fn the_varispeed_keeps_the_channels_apart() {
    // Two channels: a ramp on the left and silence on the right.
    let samples = vec![0.25_f32, 0.0, 0.5, 0.0];
    let slower = craftkit_audio::varispeed(&samples, 2, 2.0);
    assert_eq!(slower.len(), 8, "two channels of four frames");
    assert!((slower[0] - 0.25).abs() < 1e-6, "the left channel is the ramp");
    assert!((slower[1] - 0.0).abs() < 1e-6, "and the right channel is silent");
}

/// A factor of **one** is no stretch at all.
#[test]
fn a_factor_of_one_changes_nothing() {
    let samples = vec![0.25_f32, 0.5, 0.75];
    let same = craftkit_audio::varispeed(&samples, 1, 1.0);
    assert_eq!(same, samples, "the sound is the sound it was");
}

/// A factor that is **not a number** plays the sound as it is.
///
/// The number came from outside, and a number that makes no sense is
/// not a reason to lose the sound.
#[test]
fn a_factor_that_is_not_a_number_changes_nothing() {
    let samples = vec![0.25_f32, 0.5, 0.75];
    assert_eq!(craftkit_audio::varispeed(&samples, 1, f64::NAN), samples, "not a number");
    assert_eq!(craftkit_audio::varispeed(&samples, 1, 0.0), samples, "and not a speed at all");
    assert_eq!(craftkit_audio::varispeed(&samples, 1, -2.0), samples, "and not backwards");
}

/// A fade shape is a **curve** from nothing to full.
///
/// The shape says how the curve bends, and a straight line is one of
/// the shapes and not the only one.
#[test]
fn a_fade_shape_is_a_curve_from_nothing_to_full() {
    use craftkit_audio::FadeShape;
    assert!((FadeShape::Linear.gain(0.0) - 0.0).abs() < 1e-6, "a fade begins at nothing");
    assert!((FadeShape::Linear.gain(0.5) - 0.5).abs() < 1e-6, "and a straight line is halfway");
    assert!((FadeShape::Linear.gain(1.0) - 1.0).abs() < 1e-6, "and ends at full");
    // The equal-power fade is quiet for longer: at the middle it is
    // the sine of a quarter turn.
    let equal = FadeShape::EqualPower.gain(0.5);
    assert!(
        (equal - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-5,
        "the equal-power fade is quieter in the middle: {equal}"
    );
    // A place outside the fade is the end it is past.
    assert!((FadeShape::Linear.gain(-1.0) - 0.0).abs() < 1e-6, "before the fade is nothing");
    assert!((FadeShape::Linear.gain(2.0) - 1.0).abs() < 1e-6, "and after it is full");
}

/// A fade shape is read by its **name**.
///
/// The name is read the way a person writes it: the case does not
/// matter, and a space, a dash and an underscore are the same thing.
#[test]
fn a_fade_shape_is_read_by_its_name() {
    use craftkit_audio::FadeShape;
    assert_eq!(FadeShape::from_label("equal_power"), Some(FadeShape::EqualPower));
    assert_eq!(FadeShape::from_label("Equal Power"), Some(FadeShape::EqualPower));
    assert_eq!(FadeShape::from_label("equal-power"), Some(FadeShape::EqualPower));
    assert_eq!(FadeShape::from_label("s-curve"), Some(FadeShape::SCurve));
    assert_eq!(FadeShape::from_label("wobble"), None, "and a name that is not a shape is nothing");
}

/// A **bend** pushes the curve away from itself.
///
/// A bend of zero is the shape exactly, and a bend of one makes the
/// curve steeper in the middle: the ends stay where they were, so that
/// a fade still begins at nothing and still ends at full.
#[test]
fn a_bend_pushes_the_curve_away_from_itself() {
    use craftkit_audio::Fade;
    let straight = Fade { len: 100, shape: craftkit_audio::FadeShape::Linear, curvature: 0.0 };
    let bent = Fade { len: 100, shape: craftkit_audio::FadeShape::Linear, curvature: 1.0 };
    let bent_back = Fade { len: 100, shape: craftkit_audio::FadeShape::Linear, curvature: -1.0 };
    assert!((straight.gain(0.5) - 0.5).abs() < 1e-6, "the straight fade is halfway");
    assert!(bent.gain(0.5) > 0.5, "the bent fade is louder in the middle: {}", bent.gain(0.5));
    assert!(bent_back.gain(0.5) < 0.5, "and the other way is quieter: {}", bent_back.gain(0.5));
    assert!((bent.gain(0.0) - 0.0).abs() < 1e-6, "and it still begins at nothing");
    assert!((bent.gain(1.0) - 1.0).abs() < 1e-6, "and still ends at full");
}

/// The samples are **faded** at the head.
///
/// A fade in is the sound coming up: the first sample is nothing and
/// the last is full.
#[test]
fn the_samples_are_faded_at_the_head() {
    let samples = vec![1.0_f32; 4];
    let fade = craftkit_audio::Fade { len: 2, shape: craftkit_audio::FadeShape::Linear, curvature: 0.0 };
    let faded = craftkit_audio::faded(&samples, 1, fade, craftkit_audio::Fade::default());
    assert!((faded[0] - 0.0).abs() < 1e-6, "the first sample is nothing");
    assert!((faded[1] - 0.5).abs() < 1e-6, "the second is halfway: {}", faded[1]);
    assert!((faded[2] - 1.0).abs() < 1e-6, "and the third is full");
    assert!((faded[3] - 1.0).abs() < 1e-6, "and so is the last");
}

/// The samples are **faded** at the tail.
#[test]
fn the_samples_are_faded_at_the_tail() {
    let samples = vec![1.0_f32; 4];
    let fade = craftkit_audio::Fade { len: 2, shape: craftkit_audio::FadeShape::Linear, curvature: 0.0 };
    let faded = craftkit_audio::faded(&samples, 1, craftkit_audio::Fade::default(), fade);
    assert!((faded[0] - 1.0).abs() < 1e-6, "the first sample is full");
    assert!((faded[2] - 1.0).abs() < 1e-6, "the third is full");
    assert!((faded[3] - 0.5).abs() < 1e-6, "and the last is halfway down: {}", faded[3]);
}

/// A fade of **no length** changes nothing.
#[test]
fn a_fade_of_no_length_changes_nothing() {
    let samples = vec![0.25_f32, 0.5, 0.75];
    let faded =
        craftkit_audio::faded(&samples, 1, craftkit_audio::Fade::default(), craftkit_audio::Fade::default());
    assert_eq!(faded, samples, "the sound is the sound it was");
}

/// A fade keeps the **channels** apart.
#[test]
fn a_fade_keeps_the_channels_apart() {
    // Two channels: a ramp on the left and silence on the right.
    let samples = vec![0.25_f32, 0.0, 0.5, 0.0, 0.75, 0.0];
    let fade = craftkit_audio::Fade { len: 3, shape: craftkit_audio::FadeShape::Linear, curvature: 0.0 };
    let faded = craftkit_audio::faded(&samples, 2, fade, craftkit_audio::Fade::default());
    assert_eq!(faded.len(), 6, "the fade does not change the sound's length");
    assert!((faded[0] - 0.0).abs() < 1e-6, "the left channel is faded");
    assert!((faded[1] - 0.0).abs() < 1e-6, "and the right channel is silent");
    assert!((faded[4] - 0.5).abs() < 1e-6, "the last frame is half: {}", faded[4]);
}

/// A **note** is a frequency.
///
/// The notes are the piano's, and the A above the middle C is 69 and
/// is 440 hertz; an octave up is 81 and is twice the frequency.
#[test]
fn a_note_is_a_frequency() {
    assert!((craftkit_audio::midi_to_hz(69.0) - 440.0).abs() < 1e-9, "the A is 440");
    assert!((craftkit_audio::midi_to_hz(81.0) - 880.0).abs() < 1e-9, "and the A an octave up is 880");
    assert!((craftkit_audio::midi_to_hz(57.0) - 220.0).abs() < 1e-9, "and the A below is 220");
}

/// A **blip** is added to the samples.
///
/// The blip is a sine that fades away, and it is added and not mixed
/// in: the click is a voice of its own.
#[test]
fn a_blip_is_added_to_the_samples() {
    let mut samples = vec![0.0_f32; 4_800];
    craftkit_audio::blip(&mut samples, 1, 0, 440.0, 1.0, 48_000);
    let loudest = samples.iter().fold(0.0_f32, |loudest, sample| loudest.max(sample.abs()));
    assert!(loudest > 0.5, "the blip is loud: {loudest}");
    // The blip fades away: the end is quieter than the start.
    let start = samples[..100].iter().fold(0.0_f32, |loudest, sample| loudest.max(sample.abs()));
    let end = samples[1_300..1_400].iter().fold(0.0_f32, |loudest, sample| loudest.max(sample.abs()));
    assert!(end < start, "and it fades: {end} and {start}");
}

/// A blip goes to **every channel**.
#[test]
fn a_blip_goes_to_every_channel() {
    let mut samples = vec![0.0_f32; 4_800];
    craftkit_audio::blip(&mut samples, 2, 0, 440.0, 1.0, 48_000);
    let left = samples[0..4_800].iter().step_by(2).fold(0.0_f32, |loudest, s| loudest.max(s.abs()));
    let right = samples[1..4_800].iter().step_by(2).fold(0.0_f32, |loudest, s| loudest.max(s.abs()));
    assert!(left > 0.5, "the left channel is blipped: {left}");
    assert!(right > 0.5, "and the right one: {right}");
}
