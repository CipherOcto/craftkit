//! The **MIDI** of a clip: a clip that holds notes.
//!
//! The tree's sound editor has a MIDI clip: a clip whose content is a
//! sequence of notes and controllers, read from and written to a
//! Standard MIDI File. Here the content is a field in the clip's
//! payload, the notes are in the payload too, and the ticks are the
//! kit's own ticks.

use std::sync::atomic::{AtomicUsize, Ordering};

use craftkit_midi::{Note, Sequence, Smf, SmfTrack, write_smf};
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the sound battery.
fn ecosystem() -> Ecosystem {
    Ecosystem::from_inventory().expect("the ecosystem is built")
}

/// A session over the battery's own contributions.
fn session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A directory of this test's own, so that the files do not mix.
///
/// The name is **unique per call**: the tests of one binary run in
/// parallel threads, and two tests that shared a directory would race —
/// one of them removing the folder the other is writing into.
fn directory(name: &str) -> String {
    static COUNTER: AtomicUsize = AtomicUsize::new(0);
    let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-midi-{name}-{unique}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A MIDI file with a note of that pitch, that many ticks long.
fn a_file(path: &str, notes: Vec<Note>) {
    let file = Smf {
        format: 1,
        tracks: vec![SmfTrack {
            name: "the notes".to_string(),
            sequence: Sequence { notes, ctrls: Vec::new() },
        }],
        ..Smf::default()
    };
    let bytes = write_smf(&file).expect("the file is written");
    std::fs::write(path, bytes).expect("the file is on the disk");
}

/// A note of that pitch, that many ticks long.
fn note(pitch: u8, start: i64, length: i64) -> Note {
    Note { pitch, velocity: 100, release_velocity: 64, channel: 0, start, length }
}

/// A session with a track of that name.
fn a_track(session: &mut Session, name: &str) -> u64 {
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    session.document().node_by_name(name).expect("the track is there").id
}

/// The clip of the document.
fn the_clip(session: &Session) -> &craftkit_sdk::Node {
    session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there")
}

/// A MIDI file is imported as a clip that holds **notes**.
#[test]
fn a_midi_file_is_imported_as_a_clip() {
    let mut session = session();
    let directory = directory("import");
    let path = format!("{directory}/the-notes.mid");
    a_file(&path, vec![note(60, 0, 960), note(64, 960, 960)]);
    let track = a_track(&mut session, "piano");
    let imported = session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    assert_eq!(imported.data["notes"], 2, "the two notes are in the clip");
    let clip = the_clip(&session);
    assert_eq!(clip.data["content"], json!("midi"), "the clip's content is MIDI");
    let notes = clip.data["midi"]["notes"].as_array().expect("the notes are there");
    assert_eq!(notes.len(), 2);
    assert_eq!(notes[0]["pitch"], json!(60));
    assert_eq!(notes[0]["start"], json!(0));
    assert_eq!(notes[0]["length"], json!(960));
    assert_eq!(notes[1]["pitch"], json!(64));
    assert_eq!(notes[1]["start"], json!(960));
}

/// The **length** of the clip is **derived**: it is as long as the
/// notes need, at the tempo the document says.
#[test]
fn the_length_of_the_clip_is_derived_from_the_notes() {
    let mut session = session();
    let directory = directory("length");
    let path = format!("{directory}/the-notes.mid");
    // Two quarter notes: nine hundred and sixty ticks each.
    a_file(&path, vec![note(60, 0, 960), note(64, 960, 960)]);
    let track = a_track(&mut session, "piano");
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    let clip = the_clip(&session);
    // A hundred and twenty beats a minute is half a second a quarter
    // note, and the session is forty-eight thousand samples a second.
    assert_eq!(clip.data["length"], json!(48_000), "two quarter notes are a second");
}

/// The notes of a clip are **read** back.
#[test]
fn the_notes_of_a_clip_are_read() {
    let mut session = session();
    let directory = directory("read");
    let path = format!("{directory}/the-notes.mid");
    a_file(&path, vec![note(60, 0, 960)]);
    let track = a_track(&mut session, "piano");
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    let clip = the_clip(&session);
    let read =
        session.execute("sound.clip.midi.read", &json!({ "clip": clip.id })).expect("the notes are read");
    assert_eq!(read.data["midi"], json!(true));
    assert_eq!(read.data["notes"].as_array().map(Vec::len), Some(1));
    assert_eq!(read.data["end_tick"], json!(960), "the sequence ends at the tick of its last note");
}

/// The MIDI of a clip is **exported** to a file, and the file holds the
/// same notes.
#[test]
fn the_midi_of_a_clip_is_exported() {
    let mut session = session();
    let directory = directory("export");
    let path = format!("{directory}/the-notes.mid");
    a_file(&path, vec![note(60, 0, 960), note(67, 960, 480)]);
    let track = a_track(&mut session, "piano");
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    let clip = the_clip(&session);
    let out = format!("{directory}/out.mid");
    let exported = session
        .execute("sound.clip.midi.export", &json!({ "clip": clip.id, "path": out }))
        .expect("the MIDI is exported");
    assert_eq!(exported.data["notes"], 2, "the two notes are written");
    let bytes = std::fs::read(&out).expect("the file is on the disk");
    let back = craftkit_midi::read_smf(&bytes).expect("the file is read");
    let notes: Vec<(u8, i64, i64)> = back
        .tracks
        .iter()
        .flat_map(|track| track.sequence.notes.iter())
        .map(|note| (note.pitch, note.start, note.length))
        .collect();
    assert_eq!(notes, [(60, 0, 960), (67, 960, 480)], "the same notes are in the file");
}

/// A file that is **not** a MIDI file is refused.
#[test]
fn a_file_that_is_not_a_midi_file_is_refused() {
    let mut session = session();
    let directory = directory("not-midi");
    let path = format!("{directory}/sound.wav");
    std::fs::write(&path, b"RIFF____WAVEfmt ").expect("the file is on the disk");
    let track = a_track(&mut session, "piano");
    let refused =
        session.execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }));
    assert!(refused.is_err(), "a sound is not a MIDI file");
    assert!(session.document().walk().all(|node| node.kind != "sound.clip"), "and no clip was made");
}

/// A file with **no notes** is refused: a clip of no length is not a
/// clip.
#[test]
fn a_file_with_no_notes_is_refused() {
    let mut session = session();
    let directory = directory("no-notes");
    let path = format!("{directory}/empty.mid");
    a_file(&path, Vec::new());
    let track = a_track(&mut session, "piano");
    let refused =
        session.execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }));
    assert!(refused.is_err(), "there are no notes to import");
}

/// A clip that is **not** a MIDI clip cannot be exported: a file of
/// notes is not a file of nothing.
#[test]
fn a_clip_that_is_not_a_midi_clip_is_refused() {
    let mut session = session();
    let directory = directory("not-midi-clip");
    let track = a_track(&mut session, "drums");
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 480 }))
        .expect("the clip is added");
    let clip = the_clip(&session);
    let refused = session.execute(
        "sound.clip.midi.export",
        &json!({ "clip": clip.id, "path": format!("{directory}/out.mid") }),
    );
    assert!(refused.is_err(), "the clip holds no notes");
}

/// A MIDI clip is **not sound**: it has no source, and the render
/// leaves it alone. The silence is honest — what the clip holds is
/// notes, and nothing makes them a sound.
#[test]
fn a_midi_clip_is_not_heard() {
    let mut session = session();
    let directory = directory("silent");
    let path = format!("{directory}/the-notes.mid");
    a_file(&path, vec![note(60, 0, 960)]);
    let track = a_track(&mut session, "piano");
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    let out = format!("{directory}/out.wav");
    session.execute("sound.render", &json!({ "path": out })).expect("the sound is rendered");
    let bytes = std::fs::read(&out).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read");
    assert!(
        sound.samples.iter().all(|sample| *sample == 0.0),
        "the clip is silent: it holds notes and not sound"
    );
}
