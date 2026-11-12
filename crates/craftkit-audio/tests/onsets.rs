//! The **onsets** of a sound, found in its **spectrum**.
//!
//! These are the tree's own tests, brought over: the tree's detector
//! watches the spectrum with a fast Fourier transform, and this is that
//! detector — with the transform written here, because the kit's crates
//! carry no dependencies at all.

use craftkit_audio::onsets;
use std::f32::consts::TAU;

/// The rate every sound here is made at.
const RATE: u32 = 48_000;

/// The sample at that many seconds.
fn at(seconds: f32) -> usize {
    (seconds * RATE as f32) as usize
}

/// A sound of that length with those parts in it, each with a short
/// fade at its end.
fn mix(length: f32, parts: &[(f32, Vec<f32>)]) -> Vec<f32> {
    let mut sound = vec![0.0; at(length)];
    for (start, signal) in parts {
        let fade = at(0.03).min(signal.len());
        for (index, value) in signal.iter().enumerate() {
            let left = signal.len() - index;
            let gain = if left < fade {
                0.5 - 0.5 * (std::f32::consts::PI * left as f32 / fade as f32).cos()
            } else {
                1.0
            };
            if let Some(slot) = sound.get_mut(at(*start) + index) {
                *slot += value * gain;
            }
        }
    }
    sound
}

/// A number that is not quite a number, and not the same one twice.
fn noise(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed as f32 / u32::MAX as f32 * 2.0 - 1.0
}

/// A **hit**: a burst of noise that decays.
fn hit(level: f32, decay: f32, seed: &mut u32) -> Vec<f32> {
    let mut previous = 0.0;
    (0..at(decay * 6.0))
        .map(|index| {
            let value = noise(seed);
            let high = value - previous;
            previous = value;
            level * high * (-(index as f32) / RATE as f32 / decay).exp()
        })
        .collect()
}

/// A **note**: a few harmonics that die away.
fn note(frequency: f32, level: f32, seconds: f32) -> Vec<f32> {
    (0..at(seconds))
        .map(|index| {
            let time = index as f32 / RATE as f32;
            let sound = (1..=8)
                .map(|k| {
                    (TAU * frequency * k as f32 * time).sin() * (-time * (0.6 + 0.4 * k as f32)).exp()
                        / k as f32
                })
                .sum::<f32>();
            level * sound * (time / 0.002).min(1.0)
        })
        .collect()
}

/// Six seconds of drums: a loud hit and three quiet ones after it,
/// over and over.
fn drums(length: f32) -> (Vec<f32>, Vec<usize>) {
    let mut seed = 7;
    let mut parts = Vec::new();
    let mut onsets = Vec::new();
    let mut time = 0.3_f32;
    while time < length - 0.5 {
        parts.push((time, hit(0.9, 0.04, &mut seed)));
        onsets.push(at(time));
        for k in 1..4 {
            parts.push((time + 0.14 * k as f32, hit(0.09, 0.03, &mut seed)));
            onsets.push(at(time + 0.14 * k as f32));
        }
        time += 0.56;
    }
    (mix(length, &parts), onsets)
}

/// Whether the found onsets are the wanted ones, to within a
/// tolerance.
fn near(found: &[usize], wanted: &[usize], tolerance: usize) -> bool {
    found.len() == wanted.len()
        && found.iter().zip(wanted).all(|(found, wanted)| found.abs_diff(*wanted) <= tolerance)
}

/// Notes that start over a ringing note are placed on their **attacks**.
#[test]
fn notes_starting_over_a_ringing_note_are_placed_on_their_attacks() {
    let starts = [0.3, 0.72, 1.14, 1.56, 1.98, 2.4];
    let frequencies = [196.0, 261.6, 196.0, 329.6, 246.9, 392.0];
    let parts: Vec<(f32, Vec<f32>)> = starts
        .iter()
        .zip(frequencies)
        .enumerate()
        .map(|(k, (start, frequency))| (*start, note(frequency, 0.2 + 0.1 * (k % 3) as f32, 1.1)))
        .collect();
    let found = onsets(&mix(4.0, &parts), 1, RATE, 0.5);
    let wanted: Vec<usize> = starts.iter().map(|start| at(*start)).collect();
    assert!(near(&found, &wanted, at(0.003)), "{found:?} and not {wanted:?}");
}

/// The **quieter** hits between the loud ones count: the threshold is a
/// fixed amount and not one relative to the loudest hit.
#[test]
fn quieter_hits_between_loud_ones_count() {
    let (sound, wanted) = drums(6.0);
    let found = onsets(&sound, 1, RATE, 0.5);
    assert!(near(&found, &wanted, at(0.001)), "{found:?} and not {wanted:?}");
}

/// A held chord with **vibrato** and chorus has no transients: a band
/// that rises while its neighbour rises more is not a rise of its own.
#[test]
fn a_held_chord_with_vibrato_and_chorus_has_no_transients() {
    let mut phase = [0.0_f32; 6];
    let sound: Vec<f32> = (0..at(6.0))
        .map(|index| {
            let time = index as f32 / RATE as f32;
            let mut sum = 0.0;
            for (j, frequency) in [220.0_f32, 277.2, 329.6].iter().enumerate() {
                for (d, sign) in [(0, 1.0_f32), (1, -1.0)] {
                    let p = &mut phase[j * 2 + d];
                    *p +=
                        TAU * frequency * (1.0 + sign * 0.004 * (TAU * (0.3 + 0.07 * j as f32) * time).sin())
                            / RATE as f32;
                    sum += (1..=6).map(|k| (*p * k as f32).sin() / k as f32).sum::<f32>();
                }
            }
            0.04 * sum * (time / 0.4).min(1.0) * (1.0 + 0.3 * (TAU * 4.0 * time).sin())
        })
        .collect();
    let found: Vec<usize> =
        onsets(&sound, 1, RATE, 0.5).into_iter().filter(|found| *found > at(0.5)).collect();
    assert!(found.is_empty(), "{found:?}");
}

/// The transients do not depend on **where** the analysed audio
/// starts.
#[test]
fn transients_do_not_depend_on_where_the_analysed_audio_starts() {
    let (mut sound, _) = drums(8.0);
    let from = at(3.3);
    sound.iter_mut().skip(from - at(0.4)).for_each(|value| *value *= 0.05);
    let whole = onsets(&sound, 1, RATE, 0.5);
    let part: Vec<usize> =
        onsets(sound.get(from..).unwrap_or_default(), 1, RATE, 0.5).into_iter().map(|at| at + from).collect();
    let settled =
        |found: &[usize]| found.iter().copied().filter(|found| *found > from + at(0.25)).collect::<Vec<_>>();
    assert!(!settled(&whole).is_empty());
    assert_eq!(settled(&whole), settled(&part), "{whole:?} and not {part:?}");
}

/// A **fast release** is not a transient: it smears energy between a
/// note's partials, and that looks like a rise.
#[test]
fn a_fast_release_is_not_a_transient() {
    let starts: Vec<f32> = (0..8).map(|k| 0.3 + k as f32 * 0.5).collect();
    let note: Vec<f32> = (0..at(0.4))
        .map(|index| {
            let time = index as f32 / RATE as f32;
            let level =
                if time < 0.15 { (time / 0.01).min(1.0) } else { 0.001_f32.powf((time - 0.15) / 0.04) };
            let saw: f32 = (1..=20).map(|k| (TAU * 659.0 * k as f32 * time).sin() / k as f32).sum();
            0.2 * level * saw
        })
        .collect();
    let parts: Vec<(f32, Vec<f32>)> = starts.iter().map(|start| (*start, note.clone())).collect();
    let found = onsets(&mix(4.5, &parts), 1, RATE, 0.5);
    let wanted: Vec<usize> = starts.iter().map(|start| at(*start)).collect();
    assert!(near(&found, &wanted, at(0.002)), "{found:?} and not {wanted:?}");
}

/// Channels that **cancel** when mixed still show their transients.
#[test]
fn channels_that_cancel_when_mixed_still_show_their_transients() {
    let (left, wanted) = drums(4.0);
    let mut stereo = Vec::with_capacity(left.len() * 2);
    for value in &left {
        stereo.push(*value);
        stereo.push(-*value);
    }
    let found = onsets(&stereo, 2, RATE, 0.5);
    assert!(near(&found, &wanted, at(0.001)), "{found:?} and not {wanted:?}");
}
