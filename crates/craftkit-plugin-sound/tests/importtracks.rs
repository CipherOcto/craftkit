//! The **tracks** of a MIDI file: a track of the session for every
//! track of the file.
//!
//! The tree's import makes a track for each part of the file — a
//! flute and a drum kit are two tracks — and here the file's parts
//! become tracks too.

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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-importtracks-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A note of that pitch, that many ticks long.
fn note(pitch: u8, start: i64, length: i64) -> Note {
    Note { pitch, velocity: 100, release_velocity: 64, channel: 0, start, length }
}

/// A file with **two** tracks: a flute and a drum kit, and a third
/// track that holds nothing at all.
fn a_file(name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.mid");
    let file = Smf {
        format: 1,
        tracks: vec![
            SmfTrack {
                name: "the flute".to_string(),
                sequence: Sequence { notes: vec![note(72, 0, 960), note(74, 960, 960)], ctrls: Vec::new() },
            },
            SmfTrack {
                name: "the drums".to_string(),
                sequence: Sequence { notes: vec![note(36, 0, 240)], ctrls: Vec::new() },
            },
            SmfTrack { name: "the empty one".to_string(), sequence: Sequence::default() },
        ],
        tempos: vec![(0, 100.0)],
        ..Smf::default()
    };
    std::fs::write(&path, write_smf(&file).expect("the file is written")).expect("the file is on the disk");
    path
}

/// The **names** of the tracks the document has.
fn track_names(session: &Session) -> Vec<String> {
    session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.track")
        .map(|node| node.name.clone())
        .collect()
}

/// A track of the session for every track of the file that holds
/// something: the file's empty track is not a track.
#[test]
fn every_track_of_the_file_becomes_a_track() {
    let mut session = session();
    let path = a_file("thetracks");
    let outcome = session
        .execute("sound.track.midi.import", &json!({ "path": path, "start": 0 }))
        .expect("the file is imported");
    assert_eq!(outcome.data["count"], 2);
    assert_eq!(track_names(&session), vec!["the flute".to_string(), "the drums".to_string()]);
}

/// Each track of the file holds **its own** notes: the clip of the
/// flute does not hold the drums.
#[test]
fn each_track_holds_its_own_notes() {
    let mut session = session();
    let path = a_file("theownnotes");
    session
        .execute("sound.track.midi.import", &json!({ "path": path, "start": 0 }))
        .expect("the file is imported");
    let mut clips: Vec<(String, Vec<u8>)> = Vec::new();
    for node in session.document().walk() {
        if node.kind != "sound.clip" {
            continue;
        }
        let pitches = node.data["midi"]["notes"]
            .as_array()
            .expect("the notes are there")
            .iter()
            .map(|note| note["pitch"].as_u64().expect("the pitch is there") as u8)
            .collect();
        clips.push((node.name.clone(), pitches));
    }
    clips.sort();
    assert_eq!(clips.len(), 2);
    assert_eq!(clips[0].0, "the drums");
    assert_eq!(clips[0].1, vec![36]);
    assert_eq!(clips[1].0, "the flute");
    assert_eq!(clips[1].1, vec![72, 74]);
}

/// The file's **music** may come in with it, as it does with the clip
/// import.
#[test]
fn the_music_of_the_file_comes_in_with_it() {
    let mut session = session();
    let path = a_file("themap");
    let outcome = session
        .execute("sound.track.midi.import", &json!({ "path": path, "start": 0 }))
        .expect("the file is imported");
    assert_eq!(outcome.data["tempos"], 1);
    let tempo =
        session.document().walk().find(|node| node.kind == "sound.tempo").expect("the tempo is there");
    assert_eq!(tempo.data["tick"], 0);
    assert!((tempo.data["bpm"].as_f64().expect("the tempo is there") - 100.0).abs() < 0.001);
}

/// A file that holds **no notes** is refused: there is nothing to
/// import.
#[test]
fn a_file_with_no_notes_is_refused() {
    let mut session = session();
    let directory = directory("nothing");
    let path = format!("{directory}/nothing.mid");
    let file = Smf { format: 1, tracks: Vec::new(), ..Smf::default() };
    std::fs::write(&path, write_smf(&file).expect("the file is written")).expect("the file is on the disk");
    let error = session
        .execute("sound.track.midi.import", &json!({ "path": path, "start": 0 }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("no notes"), "the error says why: {error}");
    assert!(track_names(&session).is_empty(), "and no track was made");
}
