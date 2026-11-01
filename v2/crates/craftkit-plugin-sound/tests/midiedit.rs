//! The **editing** of the notes of a MIDI clip: putting them on the
//! grid, moving their pitch and changing how hard they are struck.

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
fn directory(name: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-midiedit-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
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
    std::fs::write(path, write_smf(&file).expect("the file is written")).expect("the file is on the disk");
}

/// A note of that pitch, that many ticks long.
fn note(pitch: u8, start: i64, length: i64) -> Note {
    Note { pitch, velocity: 100, release_velocity: 64, channel: 0, start, length }
}

/// A session with a track, a MIDI clip of those notes, and the clip's
/// id.
fn a_clip(session: &mut Session, name: &str, notes: Vec<Note>) -> (u64, u64) {
    let directory = directory(name);
    let path = format!("{directory}/the-notes.mid");
    a_file(&path, notes);
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    (track, clip)
}

/// The notes of the clip, as `(pitch, start, velocity)`.
fn notes_of(session: &Session, clip: u64) -> Vec<(u8, i64, u8)> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["midi"]["notes"]
        .as_array()
        .expect("the notes are there")
        .iter()
        .map(|note| {
            (
                note["pitch"].as_u64().unwrap_or(0) as u8,
                note["start"].as_i64().unwrap_or(0),
                note["velocity"].as_u64().unwrap_or(0) as u8,
            )
        })
        .collect()
}

/// The notes are put on the session's **grid**, and the length of the
/// clip is worked out again from them.
#[test]
fn the_notes_are_quantised_onto_the_grid() {
    let mut session = session();
    // A note a hundred ticks late: the grid is a quarter note, so the
    // line is at zero and at nine hundred and sixty.
    let (_, clip) = a_clip(&mut session, "quantize", vec![note(60, 100, 960)]);
    session.execute("sound.grid.set", &json!({ "value": "1/4" })).expect("the grid is set");
    session.execute("sound.clip.midi.quantize", &json!({ "clip": clip })).expect("the notes are quantised");
    let notes = notes_of(&session, clip);
    assert_eq!(notes[0].1, 0, "the note is on the line");
    let node = session.document().node(clip).expect("the clip is there");
    // The clip keeps its **room**: the note moved onto the line, and
    // the room the clip had is the room it has. A quarter note is
    // half a second, and the note was a hundred ticks late, so the
    // room was a little more than that.
    assert_eq!(node.data["length"], json!(26_500), "the clip keeps its room");
}

/// A transpose moves the **pitch** of the notes.
#[test]
fn the_notes_are_transposed() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session, "transpose", vec![note(60, 0, 960), note(64, 960, 960)]);
    session
        .execute("sound.clip.midi.transpose", &json!({ "clip": clip, "semitones": 12 }))
        .expect("the notes are transposed");
    let notes = notes_of(&session, clip);
    assert_eq!(notes[0].0, 72, "an octave up");
    assert_eq!(notes[1].0, 76);
}

/// A transpose that would take a note **out of the range** is refused,
/// and the notes are not changed.
#[test]
fn a_transpose_out_of_the_range_is_refused() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session, "transpose-range", vec![note(120, 0, 960)]);
    let refused = session.execute("sound.clip.midi.transpose", &json!({ "clip": clip, "semitones": 12 }));
    assert!(refused.is_err(), "the note of a hundred and twenty would leave the range");
    assert_eq!(notes_of(&session, clip)[0].0, 120, "and the note did not move");
}

/// The **velocity** of the notes is changed, and a velocity outside
/// the range is refused.
#[test]
fn the_velocity_of_the_notes_is_changed() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session, "velocity", vec![note(60, 0, 960)]);
    session
        .execute("sound.clip.midi.velocity", &json!({ "clip": clip, "set": 40 }))
        .expect("the velocity is set");
    assert_eq!(notes_of(&session, clip)[0].2, 40);
    session
        .execute("sound.clip.midi.velocity", &json!({ "clip": clip, "delta": 10 }))
        .expect("the velocity is changed");
    assert_eq!(notes_of(&session, clip)[0].2, 50);
    let refused = session.execute("sound.clip.midi.velocity", &json!({ "clip": clip, "set": 200 }));
    assert!(refused.is_err(), "a note cannot be struck at two hundred");
    assert_eq!(notes_of(&session, clip)[0].2, 50, "and the note was not changed");
}

/// A clip that holds **no notes** cannot be quantised, transposed or
/// given a velocity: there is nothing there to change.
#[test]
fn a_clip_without_notes_is_refused() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 480 }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    for command in [
        json!({ "clip": clip }),
        json!({ "clip": clip, "semitones": 12 }),
        json!({ "clip": clip, "set": 40 }),
    ] {
        let which = if command.get("semitones").is_some() {
            "sound.clip.midi.transpose"
        } else if command.get("set").is_some() {
            "sound.clip.midi.velocity"
        } else {
            "sound.clip.midi.quantize"
        };
        let refused = session.execute(which, &command);
        assert!(refused.is_err(), "{which} has nothing to change");
    }
}
