//! The **MIDI file**: the notes and controllers that go into one, and
//! the ones that come out of one.
//!
//! A MIDI file is a header and a list of tracks, and each track is a
//! run of events on a timeline of ticks. The reader is patient with
//! what the format allows and refuses what it does not; the writer
//! writes a format one file at nine hundred and sixty ticks a quarter
//! note.

use craftkit_midi::{CtrlEvent, CtrlKind, Error, Note, PPQ, Sequence, Smf, SmfTrack, read_smf, write_smf};

/// A note of a given pitch, starting at a tick.
fn note(pitch: u8, start: i64, length: i64) -> Note {
    Note { pitch, velocity: 100, release_velocity: 64, channel: 0, start, length }
}

/// A file with one track of the notes, and nothing else.
fn a_file(notes: Vec<Note>) -> Smf {
    Smf {
        format: 1,
        tracks: vec![SmfTrack { name: String::new(), sequence: Sequence { notes, ctrls: Vec::new() } }],
        ..Smf::default()
    }
}

/// The notes of a file, as `(pitch, start, length)`.
fn notes_of(smf: &Smf) -> Vec<(u8, i64, i64)> {
    let mut notes: Vec<(u8, i64, i64)> = smf
        .tracks
        .iter()
        .flat_map(|track| track.sequence.notes.iter())
        .map(|note| (note.pitch, note.start, note.length))
        .collect();
    notes.sort_unstable();
    notes
}

/// A file that is written is a file that is read back with the same
/// notes in it.
#[test]
fn a_file_round_trips() {
    let file = a_file(vec![note(60, 0, 480), note(64, 480, 480), note(67, 960, 960)]);
    let back = read_smf(&write_smf(&file).expect("the file is written")).expect("the file is read");
    assert_eq!(notes_of(&back), [(60, 0, 480), (64, 480, 480), (67, 960, 960)]);
}

/// A file with **no** notes is a file with no tracks, and it reads
/// back as no tracks.
#[test]
fn an_empty_file_round_trips() {
    let back = read_smf(&write_smf(&Smf::default()).expect("the file is written")).expect("the file is read");
    assert!(back.tracks.is_empty(), "there is nothing in the file: {back:?}");
    let one = Smf { tracks: vec![SmfTrack::default()], ..Smf::default() };
    let back = read_smf(&write_smf(&one).expect("the file is written")).expect("the file is read");
    assert_eq!(back.tracks.len(), 1, "the empty track is still a track");
}

/// The **ticks** are counted at nine hundred and sixty a quarter note,
/// whatever the file said.
#[test]
fn the_ticks_are_the_model_ticks() {
    let file = a_file(vec![note(60, PPQ as i64, PPQ as i64)]);
    let back = read_smf(&write_smf(&file).expect("the file is written")).expect("the file is read");
    assert_eq!(notes_of(&back), [(60, 960, 960)], "a quarter note is nine hundred and sixty ticks");
}

/// A **controller** survives the file: a control change, a program
/// change, a pressure and a pitch bend.
#[test]
fn the_controllers_survive_the_file() {
    let file = Smf {
        format: 1,
        tracks: vec![SmfTrack {
            name: "drums".to_string(),
            sequence: Sequence {
                notes: Vec::new(),
                ctrls: vec![
                    CtrlEvent { tick: 0, channel: 0, kind: CtrlKind::Control(7), value: 100 },
                    CtrlEvent { tick: 10, channel: 0, kind: CtrlKind::Program, value: 42 },
                    CtrlEvent { tick: 20, channel: 9, kind: CtrlKind::ChannelPressure, value: 64 },
                    CtrlEvent { tick: 30, channel: 0, kind: CtrlKind::PolyPressure(60), value: 30 },
                    CtrlEvent { tick: 40, channel: 0, kind: CtrlKind::PitchBend, value: -8192 },
                ],
            },
        }],
        ..Smf::default()
    };
    let back = read_smf(&write_smf(&file).expect("the file is written")).expect("the file is read");
    let track = back.tracks.first().expect("the track is there");
    assert_eq!(track.name, "drums", "the name of the track is in the file");
    let ctrls = &track.sequence.ctrls;
    assert_eq!(ctrls.len(), 5, "every controller is there: {ctrls:?}");
    assert_eq!(ctrls[0].kind, CtrlKind::Control(7));
    assert_eq!(ctrls[0].value, 100);
    assert_eq!(ctrls[1].kind, CtrlKind::Program);
    assert_eq!(ctrls[2].kind, CtrlKind::ChannelPressure);
    assert_eq!(ctrls[3].kind, CtrlKind::PolyPressure(60));
    assert_eq!(ctrls[4].kind, CtrlKind::PitchBend);
    assert_eq!(ctrls[4].value, -8192, "the bend keeps its sign");
}

/// The **tempos** and the **meters** of the file are the file's own,
/// and they are read back.
#[test]
fn the_tempos_and_the_meters_survive_the_file() {
    let file = Smf { tempos: vec![(0, 120.0), (960, 90.0)], meters: vec![(0, 3, 4)], ..Smf::default() };
    let back = read_smf(&write_smf(&file).expect("the file is written")).expect("the file is read");
    // A tempo is written in microseconds a quarter note, so it comes
    // back as the nearest tempo that is.
    assert_eq!(back.tempos.len(), 2, "the tempos are there: {:?}", back.tempos);
    assert_eq!(back.tempos[0], (0, 120.0));
    assert!((back.tempos[1].1 - 90.0).abs() < 0.001, "the second tempo is ninety: {:?}", back.tempos);
    assert_eq!(back.meters, [(0, 3, 4)], "the meter is there");
    assert!(back.tracks.is_empty(), "the conductor track is not a track of its own");
}

/// A file that is **not** a MIDI file is refused, and it is refused by
/// name.
#[test]
fn a_file_that_is_not_a_midi_file_is_refused() {
    let refused = read_smf(b"RIFF....WAVEfmt ");
    assert_eq!(refused.err(), Some(Error::NotSmf), "it is not a MIDI file");
}

/// A file that ends in the middle of something is refused, and the
/// refusal says where.
#[test]
fn a_truncated_file_is_refused() {
    let file = write_smf(&a_file(vec![note(60, 0, 480)])).expect("the file is written");
    let cut = &file[..file.len() - 4];
    let refused = read_smf(cut);
    match refused {
        Err(Error::Truncated { .. }) | Err(Error::ChunkTooLong { .. }) => {}
        other => panic!("the file ends in the middle: {other:?}"),
    }
}

/// A **number** longer than the four bytes the format allows is
/// refused: it is not a number the format has.
#[test]
fn a_number_that_is_too_long_is_refused() {
    // A track chunk with a note-on whose delta is five bytes long.
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"MThd");
    file.extend_from_slice(&6u32.to_be_bytes());
    file.extend_from_slice(&0u16.to_be_bytes());
    file.extend_from_slice(&1u16.to_be_bytes());
    file.extend_from_slice(&960u16.to_be_bytes());
    file.extend_from_slice(b"MTrk");
    file.extend_from_slice(&6u32.to_be_bytes());
    file.extend_from_slice(&[0x80, 0x80, 0x80, 0x80, 0x80]);
    file.extend_from_slice(&[0x90, 60, 100]);
    let refused = read_smf(&file);
    match refused {
        Err(Error::NumberTooLong { .. }) => {}
        other => panic!("the number is too long: {other:?}"),
    }
}

/// A chunk that says it is longer than the file is refused: the file
/// does not hold it.
#[test]
fn a_chunk_longer_than_the_file_is_refused() {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"MThd");
    file.extend_from_slice(&6u32.to_be_bytes());
    file.extend_from_slice(&0u16.to_be_bytes());
    file.extend_from_slice(&1u16.to_be_bytes());
    file.extend_from_slice(&960u16.to_be_bytes());
    file.extend_from_slice(b"MTrk");
    file.extend_from_slice(&1_000u32.to_be_bytes());
    file.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
    match read_smf(&file) {
        Err(Error::ChunkTooLong { .. }) => {}
        other => panic!("the chunk is longer than the file: {other:?}"),
    }
}

/// Two notes of the same pitch that are **on top of each other** do
/// not swap their ends: the first is trimmed to where the second
/// starts, and of two that start together only the longer is written.
#[test]
fn notes_that_overlap_do_not_swap_ends() {
    let file = a_file(vec![
        note(60, 0, 1000),
        note(60, 100, 200),
        note(60, 1000, 100),
        note(60, 2000, 500),
        note(60, 2000, 900),
    ]);
    let back = read_smf(&write_smf(&file).expect("the file is written")).expect("the file is read");
    assert_eq!(notes_of(&back), [(60, 0, 100), (60, 100, 200), (60, 1000, 100), (60, 2000, 900)]);
}

/// A note that was never let go **ends** at the end of the track, and
/// a note-off with no note-on is **ignored**.
#[test]
fn a_note_that_is_never_let_go_ends_at_the_end_of_the_track() {
    let mut file: Vec<u8> = Vec::new();
    file.extend_from_slice(b"MThd");
    file.extend_from_slice(&6u32.to_be_bytes());
    file.extend_from_slice(&0u16.to_be_bytes());
    file.extend_from_slice(&1u16.to_be_bytes());
    file.extend_from_slice(&960u16.to_be_bytes());
    let mut track: Vec<u8> = Vec::new();
    // A note-on at tick zero and a note-off with no note-on.
    track.extend_from_slice(&[0x00, 0x90, 60, 100]);
    track.extend_from_slice(&[0x60, 0x80, 62, 64]);
    // The end of the track at tick 960.
    track.extend_from_slice(&[0x60, 0xFF, 0x2F, 0x00]);
    file.extend_from_slice(b"MTrk");
    file.extend_from_slice(&u32::try_from(track.len()).unwrap_or(0).to_be_bytes());
    file.extend_from_slice(&track);
    let back = read_smf(&file).expect("the file is read");
    let notes = &back.tracks.first().expect("the track is there").sequence.notes;
    assert_eq!(notes.len(), 1, "the note-off with no note-on is ignored: {notes:?}");
    // The note ends where the track does: the end of the track is at
    // tick 192, and the note-off that had no note-on did not move it.
    assert_eq!((notes[0].pitch, notes[0].start, notes[0].length), (60, 0, 192));
}

/// A **tempo** that is not a tempo cannot be written, and the writer
/// says so.
#[test]
fn a_tempo_that_is_not_a_tempo_is_refused() {
    let file = Smf { tempos: vec![(0, 0.0)], ..Smf::default() };
    match write_smf(&file) {
        Err(Error::CannotWrite { .. }) => {}
        other => panic!("the tempo is not a tempo: {other:?}"),
    }
}

/// A meter whose **denominator** is not a power of two cannot be
/// written either: the format has no way of saying it.
#[test]
fn a_meter_that_is_not_a_meter_is_refused() {
    let file = Smf { meters: vec![(0, 3, 5)], ..Smf::default() };
    match write_smf(&file) {
        Err(Error::CannotWrite { .. }) => {}
        other => panic!("the denominator is not a power of two: {other:?}"),
    }
}

/// The **sequence** takes the duplicates out: two notes of the same
/// pitch and start are one note, and the longer one is kept.
#[test]
fn the_sequence_takes_the_duplicates_out() {
    let mut sequence = Sequence {
        notes: vec![note(60, 0, 100), note(60, 0, 400)],
        ctrls: vec![
            CtrlEvent { tick: 0, channel: 0, kind: CtrlKind::Control(7), value: 10 },
            CtrlEvent { tick: 0, channel: 0, kind: CtrlKind::Control(7), value: 20 },
        ],
    };
    assert_eq!(sequence.remove_duplicates(), 2, "one note and one controller went");
    assert_eq!(sequence.notes.len(), 1);
    assert_eq!(sequence.notes[0].length, 400, "the longer note is the one that is kept");
    assert_eq!(sequence.ctrls.len(), 1);
    assert_eq!(sequence.ctrls[0].value, 20, "the last controller is the one that is kept");
}

/// The **end** of a sequence is the end of the last note and the tick
/// of the last controller, whichever is later.
#[test]
fn the_end_of_a_sequence_is_its_last_thing() {
    let mut sequence = Sequence { notes: vec![note(60, 0, 480)], ctrls: Vec::new() };
    assert_eq!(sequence.end_tick(), 480);
    sequence.ctrls.push(CtrlEvent { tick: 960, channel: 0, kind: CtrlKind::Program, value: 1 });
    assert_eq!(sequence.end_tick(), 960, "the controller is later");
    assert_eq!(Sequence::default().end_tick(), 0, "an empty sequence ends nowhere");
}
