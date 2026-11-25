//! The **cuts of the sound battery**: the places where it stops early
//! and says so.
//!
//! A reader that stops at a bound must say that it stopped: an import
//! that cuts the name of a track and says nothing has given the user a
//! track with a name the file never gave it. Every cut of this battery
//! is named here, and the bound it stops at is named in **code**, so that
//! the proof and the bound cannot drift apart.

use craftkit_midi::{Note, Sequence, Smf, SmfTrack, write_smf};
use craftkit_plugin_sound::MAX_MIDI_NAME_CHARS;
use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the battery.
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

/// A **directory** of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-sound-names-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A note of that pitch, that many ticks long.
fn note(pitch: u8, start: i64, length: i64) -> Note {
    Note { pitch, velocity: 100, release_velocity: 64, channel: 0, start, length }
}

/// A file with **one** track, and the name the file gives it.
fn a_file(name: &str, track_name: &str) -> String {
    let directory = directory(name);
    let path = format!("{directory}/{name}.mid");
    let file = Smf {
        format: 1,
        tracks: vec![SmfTrack {
            name: track_name.to_string(),
            sequence: Sequence { notes: vec![note(72, 0, 960)], ctrls: Vec::new() },
        }],
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

/// The **name of a track** of a file over `MAX_MIDI_NAME_CHARS` is cut,
/// and the import says so.
///
/// A file may name a track with a very long string, and the name is cut
/// at the bound: the track the document holds is not the track the file
/// named, and the answer says that it is not.
#[test]
fn the_name_of_a_track_is_cut_and_the_import_says_so() {
    let mut session = session();
    let long = "n".repeat(MAX_MIDI_NAME_CHARS + 10);
    let path = a_file("thecutname", &long);
    let outcome = session
        .execute("sound.track.midi.import", &json!({ "path": path, "start": 0 }))
        .expect("the file is imported");
    let cut = outcome.data["names_cut"].as_u64().expect("the answer says how many were cut");
    assert_eq!(cut, 1, "the name is cut at the bound: {outcome:?}");
    assert_eq!(outcome.data["truncated"], json!(true), "the answer says it stopped: {outcome:?}");
    let names = track_names(&session);
    assert_eq!(names.len(), 1, "the track is there: {names:?}");
    assert_eq!(names[0].chars().count(), MAX_MIDI_NAME_CHARS, "the name is cut at the bound: {names:?}");
}

/// The **name of a track** of exactly `MAX_MIDI_NAME_CHARS` is not cut,
/// and the import says so.
#[test]
fn the_name_of_a_track_at_the_bound_is_not_cut() {
    let mut session = session();
    let name = "n".repeat(MAX_MIDI_NAME_CHARS);
    let path = a_file("thewholeName", &name);
    let outcome = session
        .execute("sound.track.midi.import", &json!({ "path": path, "start": 0 }))
        .expect("the file is imported");
    let cut = outcome.data["names_cut"].as_u64().expect("the answer says how many were cut");
    assert_eq!(cut, 0, "nothing was cut: {outcome:?}");
    assert_eq!(outcome.data["truncated"], json!(false), "the answer says it did not stop: {outcome:?}");
    let names = track_names(&session);
    assert_eq!(names.len(), 1, "the track is there: {names:?}");
    assert_eq!(names[0], name, "the name is the name the file gave: {names:?}");
}
