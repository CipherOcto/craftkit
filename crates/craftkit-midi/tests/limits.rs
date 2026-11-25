//! The **limits of the midi crate**: the numbers it will not go past.
//!
//! The crate reads a file of notes and writes one, and the numbers it
//! keeps are about how much of a file it reads and how hard a note may
//! be struck. A limit that no test names is a limit nothing proves
//! holds.

use craftkit_midi::{
    Error, MAX_EVENTS, MAX_TRACKS, Note, Sequence, Smf, SmfTrack, VelocityOp, change_velocity, read_smf,
    write_smf,
};

/// A note of a given pitch and velocity.
fn a_note(velocity: u8) -> Note {
    Note { pitch: 60, velocity, release_velocity: 64, channel: 0, start: 0, length: 480 }
}

/// A **file** of the header and the track, written by hand.
///
/// The header of a Standard MIDI File is four bytes of name, four
/// bytes of length, and the format, the number of tracks and the
/// division; a track is four bytes of name, four bytes of length and
/// the events.
fn a_file_of_tracks(tracks: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&(tracks as u16).to_be_bytes());
    out.extend_from_slice(&96u16.to_be_bytes());
    for _ in 0..tracks {
        out.extend_from_slice(b"MTrk");
        out.extend_from_slice(&0u32.to_be_bytes());
    }
    out
}

/// A **file** of the header and a track of that many note-on events.
///
/// The first event says its status and the rest use it: the format
/// allows a running status, and a file of five million events is a
/// file of ten megabytes.
fn a_file_of_events(events: usize) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&1u16.to_be_bytes());
    out.extend_from_slice(&96u16.to_be_bytes());
    out.extend_from_slice(b"MTrk");
    let length = 4 + (events - 1) * 3;
    out.extend_from_slice(&(length as u32).to_be_bytes());
    // The first event: the status and the note and the velocity.
    out.extend_from_slice(&[0x00, 0x90, 0x3c, 0x64]);
    // And the rest of them: the delta, the note and the velocity.
    for _ in 1..events {
        out.extend_from_slice(&[0x00, 0x3c, 0x64]);
    }
    out
}

/// A file with **more tracks** than `MAX_TRACKS` is refused.
#[test]
fn a_file_with_too_many_tracks_is_refused() {
    let bytes = a_file_of_tracks(MAX_TRACKS + 1);
    let error = read_smf(&bytes).expect_err("that file has too many tracks");
    match error {
        Error::TooManyTracks { limit } => assert_eq!(limit, MAX_TRACKS, "the limit is the tree's"),
        other => panic!("the tracks are refused: {other:?}"),
    }
}

/// A file with **more events** than `MAX_EVENTS` is refused.
#[test]
fn a_file_with_too_many_events_is_refused() {
    let bytes = a_file_of_events(MAX_EVENTS + 1);
    let error = read_smf(&bytes).expect_err("that file has too many events");
    match error {
        Error::TooManyEvents { limit } => assert_eq!(limit, MAX_EVENTS, "the limit is the tree's"),
        other => panic!("the events are refused: {other:?}"),
    }
}

/// A note struck **harder** than `MAX_VELOCITY` is refused.
///
/// A velocity of a hundred and fifty is not a note the caller knows
/// about, and the notes are not changed.
#[test]
fn a_velocity_over_the_limit_is_refused() {
    let mut notes = vec![a_note(100)];
    let too_hard = craftkit_midi::MAX_VELOCITY as u8 + 23;
    let error =
        change_velocity(&mut notes, &VelocityOp::Set(too_hard)).expect_err("that velocity is too hard");
    assert_eq!(error.velocity, i32::from(too_hard), "the velocity is named: {error:?}");
    assert_eq!(notes[0].velocity, 100, "and the note is not changed");
}

/// A note struck **softer** than the tree's own minimum is refused.
#[test]
fn a_velocity_under_the_minimum_is_refused() {
    let mut notes = vec![a_note(100)];
    let error = change_velocity(&mut notes, &VelocityOp::Set(0)).expect_err("that velocity is not a strike");
    assert_eq!(error.velocity, 0, "the velocity is named: {error:?}");
    assert_eq!(notes[0].velocity, 100, "and the note is not changed");
}

/// A file that is **within** the limits is read, and its tracks are
/// there.
#[test]
fn a_file_within_the_limits_is_read() {
    let file = Smf {
        format: 1,
        tracks: vec![SmfTrack { name: String::new(), sequence: Sequence::default() }],
        ..Smf::default()
    };
    let bytes = write_smf(&file).expect("the file is written");
    let back = read_smf(&bytes).expect("the file is read");
    assert_eq!(back.tracks.len(), 1, "the track is there: {back:?}");
}
