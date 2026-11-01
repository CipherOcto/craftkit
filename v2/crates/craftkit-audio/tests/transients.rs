//! The **transients** of a sound: the places where it starts again.
//!
//! The detector here works on the **level** — the tree's watches the
//! spectrum with a fast Fourier transform, and this kit's crates carry
//! no dependencies at all — and a transient is a rise that stays up.

use craftkit_audio::transients;

/// A sound of silence, a hit, silence, a hit, silence.
///
/// The hits are a quarter of a second long and they are a hundred
/// milliseconds apart, so a person hears two sounds and not one.
fn two_hits() -> Vec<f32> {
    let mut samples: Vec<f32> = Vec::new();
    for _ in 0..2 {
        samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
        samples.extend(std::iter::repeat_n(1.0_f32, 12_000));
        samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    }
    samples
}

/// A sound that fades away and never comes back: there is nothing in
/// it that starts again.
fn a_fade() -> Vec<f32> {
    (0..48_000).map(|index| (1.0 - index as f32 / 48_000.0).max(0.0)).collect()
}

/// The hits of a sound are found: two of them, and not the silence
/// around them.
#[test]
fn the_hits_of_a_sound_are_found() {
    let onsets = transients(&two_hits(), 1, 48_000, 0.5);
    assert_eq!(onsets.len(), 2, "two hits and no more: {onsets:?}");
    assert!(onsets[0] >= 4_800 && onsets[0] <= 5_400, "the first hit is where it is: {onsets:?}");
    // The second hit starts at 26_400: the silence comes before each
    // of them.
    assert!(onsets[1] >= 26_400 && onsets[1] <= 27_500, "and the second: {onsets:?}");
}

/// A sound that fades away has **no** transients: nothing in it starts
/// again.
#[test]
fn a_fade_has_no_transients() {
    let onsets = transients(&a_fade(), 1, 48_000, 0.5);
    assert!(onsets.is_empty(), "a fade starts once and does not start again: {onsets:?}");
}

/// **Silence** has no transients either.
#[test]
fn silence_has_no_transients() {
    let onsets = transients(&vec![0.0_f32; 48_000], 1, 48_000, 0.5);
    assert!(onsets.is_empty(), "there is nothing in silence: {onsets:?}");
}

/// The **sensitivity** is how small a rise may be: a sensitive
/// detector finds the small hits that a blunt one does not.
#[test]
fn the_sensitivity_is_how_small_a_rise_may_be() {
    let mut samples: Vec<f32> = Vec::new();
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    // A hit that is quieter than the one before it.
    samples.extend(std::iter::repeat_n(1.0_f32, 12_000));
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    samples.extend(std::iter::repeat_n(0.05_f32, 12_000));
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    let blunt = transients(&samples, 1, 48_000, 0.0);
    let sensitive = transients(&samples, 1, 48_000, 1.0);
    assert_eq!(blunt.len(), 1, "the blunt detector hears only the loud hit: {blunt:?}");
    assert_eq!(sensitive.len(), 2, "the sensitive one hears the quiet hit too: {sensitive:?}");
}

/// Two hits that are **too close together** are one hit: the gap of a
/// frame of a drum is not two drums.
#[test]
fn two_hits_that_are_too_close_are_one_hit() {
    let mut samples: Vec<f32> = Vec::new();
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    samples.extend(std::iter::repeat_n(1.0_f32, 1_200));
    samples.extend(std::iter::repeat_n(0.0_f32, 480));
    samples.extend(std::iter::repeat_n(1.0_f32, 1_200));
    samples.extend(std::iter::repeat_n(0.0_f32, 4_800));
    let onsets = transients(&samples, 1, 48_000, 0.5);
    assert_eq!(onsets.len(), 1, "the two hits are one: {onsets:?}");
}

/// A sound of **two channels** is read across both of them, and the
/// transients are the same.
#[test]
fn a_stereo_sound_is_read_across_both_channels() {
    let mono = two_hits();
    let mut stereo: Vec<f32> = Vec::with_capacity(mono.len() * 2);
    for sample in mono {
        stereo.push(sample);
        stereo.push(sample);
    }
    let onsets = transients(&stereo, 2, 48_000, 0.5);
    assert_eq!(onsets.len(), 2, "the two hits: {onsets:?}");
}
