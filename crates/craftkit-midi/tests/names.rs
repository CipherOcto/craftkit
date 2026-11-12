//! The **names** of the notes: what a pitch is called.
//!
//! The tree's MIDI crate carries the names — the note names, the drum
//! names and the program names — so that a note can be called "C3".

use craftkit_midi::names::{gm_drum_name, gm_program_name, note_name, note_or_drum_name};

/// The middle C is **C3**, and the octave below the first is **C-2**.
#[test]
fn a_note_is_named_by_its_pitch_and_its_octave() {
    assert_eq!(note_name(60), "C3");
    assert_eq!(note_name(0), "C-2");
    assert_eq!(note_name(127), "G8");
    assert_eq!(note_name(61), "C#3");
    assert_eq!(note_name(69), "A3");
    assert_eq!(note_name(81), "A4");
}

/// The **drums** are on the pitches from thirty-five to eighty-one,
/// and a pitch outside them has no drum name.
#[test]
fn a_drum_is_named_by_its_pitch() {
    assert_eq!(gm_drum_name(36), Some("Bass Drum 1"));
    assert_eq!(gm_drum_name(81), Some("Open Triangle"));
    assert_eq!(gm_drum_name(35), Some("Acoustic Bass Drum"));
    assert_eq!(gm_drum_name(34), None);
    assert_eq!(gm_drum_name(82), None);
}

/// The **programs** are a hundred and twenty-eight, and a program
/// outside them is "Unknown" and not a made-up name.
#[test]
fn a_program_is_named_by_its_number() {
    assert_eq!(gm_program_name(0), "Acoustic Grand Piano");
    assert_eq!(gm_program_name(127), "Gunshot");
    assert_eq!(gm_program_name(200), "Unknown");
}

/// A note on the **drum channel** — the tenth — is a drum, and a note
/// on any other channel is named by its pitch.
#[test]
fn a_note_on_the_drum_channel_is_a_drum() {
    assert_eq!(note_or_drum_name(36, 9), "Bass Drum 1");
    assert_eq!(note_or_drum_name(36, 0), "C1");
    // A pitch outside the drum kit has no drum name even on the drum
    // channel, and is named by its pitch.
    assert_eq!(note_or_drum_name(34, 9), "A#0");
}
