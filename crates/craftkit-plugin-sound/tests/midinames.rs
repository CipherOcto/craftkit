//! The **names** of the notes of a MIDI clip: what a pitch is called.
//!
//! The tree's MIDI crate carries a table of names — a note is "C3", a
//! drum is "Bass Drum 1" and a program is "Acoustic Grand Piano" —
//! and this battery gives them back with the notes they belong to.

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

/// A session with a track and a MIDI clip of two notes.
fn a_clip(session: &mut Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.midi.add", &json!({ "track": track, "start": 0, "length": 48_000 }))
        .expect("the clip is made");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 60, "start": 0, "length": 960 }))
        .expect("the first note is added");
    session
        .execute(
            "sound.clip.midi.note.add",
            &json!({ "clip": clip, "pitch": 64, "start": 960, "length": 960 }),
        )
        .expect("the second note is added");
    clip
}

/// The names of the clip's notes, in the order they are in.
fn names_of(session: &mut Session, clip: u64) -> Vec<String> {
    let read =
        session.execute("sound.clip.midi.names", &json!({ "clip": clip })).expect("the names are read");
    read.data["names"]
        .as_array()
        .expect("the names are there")
        .iter()
        .map(|note| note["name"].as_str().expect("the name is there").to_string())
        .collect()
}

/// The notes are named by their pitch: the middle C is "C3".
#[test]
fn the_notes_are_named_by_their_pitch() {
    let mut session = session();
    let clip = a_clip(&mut session);
    assert_eq!(names_of(&mut session, clip), vec!["C3".to_string(), "E3".to_string()]);
}

/// A note on the **drum channel** — the tenth — is a drum and is named
/// as one.
#[test]
fn a_note_on_the_drum_channel_is_named_as_a_drum() {
    let mut session = session();
    let clip = a_clip(&mut session);
    session
        .execute(
            "sound.clip.midi.note.add",
            &json!({ "clip": clip, "pitch": 36, "start": 1920, "length": 960, "channel": 9 }),
        )
        .expect("the drum is added");
    let names = names_of(&mut session, clip);
    assert!(names.contains(&"Bass Drum 1".to_string()), "the drum is named: {names:?}");
}

/// A clip that holds **no notes** says so, and it is not a failure.
#[test]
fn a_clip_with_no_notes_says_so() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 48_000 }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    let read =
        session.execute("sound.clip.midi.names", &json!({ "clip": clip })).expect("the names are read");
    assert_eq!(read.data["midi"], false, "the clip holds no notes");
}
