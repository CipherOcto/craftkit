//! The **limits of the audio crate**: the numbers it will not go past.
//!
//! The crate is a **reader of sounds**: it looks at a material and
//! finds what is in it, and a reader that stops at a bound must **say**
//! that it stopped. A limit that no test names is a limit nothing proves
//! holds.

use craftkit_audio::{
    LoudnessMeter, MAX_CHANNELS, MAX_FRAMES, MAX_HZ, MAX_ONSETS, MAX_TRANSIENTS, audio_to_notes, onsets,
    transients,
};

/// The rate every sound here is made at.
const RATE: u32 = 48_000;

/// A number that is not quite a number, and not the same one twice.
fn noise(seed: &mut u32) -> f32 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 17;
    *seed ^= *seed << 5;
    *seed as f32 / u32::MAX as f32 * 2.0 - 1.0
}

/// A sound of that many **bursts**, each of them noise, with silence
/// between them.
fn bursts(count: usize, burst: usize, gap: usize) -> Vec<f32> {
    let mut seed = 1u32;
    let mut sound = Vec::with_capacity(count * gap);
    for _ in 0..count {
        for _ in 0..burst {
            sound.push(noise(&mut seed));
        }
        sound.extend(std::iter::repeat_n(0.0, gap.saturating_sub(burst)));
    }
    sound
}

/// A sound with **more onsets** than `MAX_ONSETS` is stopped at, and the
/// reading says so.
#[test]
fn a_sound_with_too_many_onsets_says_so() {
    // A burst every thirty-two milliseconds at eight kilohertz: ten
    // thousand of them, and a few more.
    let sound = bursts(10_050, 128, 256);
    let found = onsets(&sound, 1, 8_000, 0.5);
    assert!(found.truncated, "the detector stopped at its bound");
    assert_eq!(found.items.len(), MAX_ONSETS, "the onsets that were found");
}

/// A sound with **more transients** than `MAX_TRANSIENTS` is stopped at,
/// and the reading says so.
#[test]
fn a_sound_with_too_many_transients_says_so() {
    // A burst every thirty-two milliseconds: eleven thousand of them,
    // because the detector does not hear every one of them.
    let sound = bursts(11_000, 512, 1536);
    let found = transients(&sound, 1, RATE, 0.5);
    assert!(found.truncated, "the detector stopped at its bound");
    assert_eq!(found.items.len(), MAX_TRANSIENTS, "the transients that were found");
}

/// A sound **within** the bounds is read whole, and the reading says
/// that nothing was left out.
#[test]
fn a_sound_within_the_bounds_is_read_whole() {
    let sound = bursts(50, 128, 256);
    let found = onsets(&sound, 1, 8_000, 0.5);
    assert!(!found.truncated, "nothing was left out");
    assert!(!found.items.is_empty(), "and the onsets are there: {:?}", found.items);
}

/// A pitch **above** `MAX_HZ` is not a note.
///
/// The detector looks for a pitch between its own bounds, and a tone
/// above the top of them is not one it claims to hear.
#[test]
fn a_pitch_above_the_bound_is_not_a_note() {
    let rate = 48_000_f32;
    // The tone is **above the bound** of the detector: a quarter of a
    // kilohertz above the top of what it looks for.
    let tone = MAX_HZ + 400.0;
    let mut sound = Vec::with_capacity(rate as usize);
    for index in 0..(rate as usize) {
        let time = index as f32 / rate;
        sound.push(0.5 * (2.0 * std::f32::consts::PI * tone * time).sin());
    }
    let found = audio_to_notes(&sound, rate);
    assert!(found.items.is_empty(), "no note is claimed: {:?}", found.items);
    assert!(!found.truncated, "and nothing was left out");
}

/// A sound that is **within** the frames of the detector is read whole.
///
/// The detector stops at `MAX_FRAMES`, and the frames of a sound of a
/// second are far fewer than that: the bound is the tree's, and this
/// test says what the tree reads and not what it would read.
#[test]
fn a_sound_within_the_frames_is_read_whole() {
    let rate = 48_000_f32;
    let mut sound = Vec::with_capacity(rate as usize);
    for index in 0..(rate as usize) {
        let time = index as f32 / rate;
        sound.push(0.5 * (2.0 * std::f32::consts::PI * 440.0 * time).sin());
    }
    let found = audio_to_notes(&sound, rate);
    assert!(!found.truncated, "nothing was left out");
    // The frames the detector read of this sound are far fewer than the
    // bound: the sound that would reach it is larger than the machine.
    let frames = sound.len() / 2_048;
    assert!(frames < MAX_FRAMES, "the sound is within the frames of the detector: {frames}");
    assert!(!found.items.is_empty(), "and the note is there: {:?}", found.items);
}

/// The meter holds state for **MAX_CHANNELS** channels, and not for
/// more.
///
/// A meter asked for more channels than the bound has is a meter of the
/// bound's channels: the sound of the channels past the bound is not
/// heard. The sound battery never reaches this bound — its own reader
/// refuses more than eight channels — so this test says what the
/// library does.
#[test]
fn a_meter_holds_state_for_its_own_channels() {
    let mut heard = LoudnessMeter::new(48_000.0, MAX_CHANNELS);
    let mut channels: Vec<Vec<f32>> = vec![vec![0.0_f32; 48_000]; MAX_CHANNELS];
    if let Some(last) = channels.last_mut() {
        last.fill(0.5);
    }
    heard.process(&channels);
    let mut silent = LoudnessMeter::new(48_000.0, MAX_CHANNELS + 1);
    let mut channels: Vec<Vec<f32>> = vec![vec![0.0_f32; 48_000]; MAX_CHANNELS + 1];
    if let Some(last) = channels.last_mut() {
        last.fill(0.5);
    }
    silent.process(&channels);
    assert!(
        heard.momentary_lufs() > silent.momentary_lufs(),
        "the last channel of the bound is heard: {} and not {}",
        heard.momentary_lufs(),
        silent.momentary_lufs()
    );
}
