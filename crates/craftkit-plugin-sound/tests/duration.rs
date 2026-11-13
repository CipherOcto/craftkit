//! The **length** of the notes of a MIDI clip: how long they last.
//!
//! The tree's sound editor changes a note's length four ways: it sets
//! it, adds to it, scales it and makes it **legato** — every note
//! ends a gap of ticks before the next note starts, and the last note
//! keeps its length.
//!
//! A note is always at least one tick long: a note of no length is a
//! note that cannot be heard, and that is a **shape** and not a bound.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-duration-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A note of that pitch, that many ticks long.
fn note(pitch: u8, start: i64, length: i64) -> Note {
    Note { pitch, velocity: 100, release_velocity: 64, channel: 0, start, length }
}

/// A session with a track, a MIDI clip of those notes, and the clip's
/// id.
///
/// The track and the file are named after the test, so that a session
/// with two clips in it says which is which.
fn a_clip(session: &mut Session, name: &str, notes: Vec<Note>) -> u64 {
    let directory = directory(name);
    let path = format!("{directory}/{name}.mid");
    let file = Smf {
        format: 1,
        tracks: vec![SmfTrack {
            name: "the notes".to_string(),
            sequence: Sequence { notes, ctrls: Vec::new() },
        }],
        ..Smf::default()
    };
    std::fs::write(&path, write_smf(&file).expect("the file is written")).expect("the file is on the disk");
    session.execute("sound.track.add", &json!({ "name": name })).expect("the track is added");
    let track = session.document().node_by_name(name).expect("the track is there").id;
    session
        .execute("sound.clip.midi.import", &json!({ "track": track, "start": 0, "path": path }))
        .expect("the file is imported");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip" && node.name == name)
        .expect("the clip is there")
        .id
}

/// The lengths of the clip's notes, in the order they are in.
fn lengths_of(session: &Session, clip: u64) -> Vec<i64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["midi"]["notes"]
        .as_array()
        .expect("the notes are there")
        .iter()
        .map(|note| note["length"].as_i64().expect("the length is there"))
        .collect()
}

/// The length of the notes is **set**, **changed** and **scaled**, and
/// a note is never shorter than one tick.
#[test]
fn the_length_of_the_notes_is_changed() {
    let mut session = session();
    let clip = a_clip(&mut session, "set", vec![note(60, 0, 960), note(62, 960, 960)]);
    session
        .execute("sound.clip.midi.duration", &json!({ "clip": clip, "set": 240 }))
        .expect("the notes are set");
    assert_eq!(lengths_of(&session, clip), vec![240, 240]);
    session
        .execute("sound.clip.midi.duration", &json!({ "clip": clip, "delta": 240 }))
        .expect("the notes are changed");
    assert_eq!(lengths_of(&session, clip), vec![480, 480]);
    session
        .execute("sound.clip.midi.duration", &json!({ "clip": clip, "scale": 2.0 }))
        .expect("the notes are scaled");
    assert_eq!(lengths_of(&session, clip), vec![960, 960]);
}

/// A note of **no length** is a note that cannot be heard, and it is
/// one tick long: that is a shape and not a bound.
#[test]
fn a_note_is_never_shorter_than_a_tick() {
    let mut session = session();
    let clip = a_clip(&mut session, "floor", vec![note(60, 0, 960)]);
    session
        .execute("sound.clip.midi.duration", &json!({ "clip": clip, "set": 0 }))
        .expect("the notes are set");
    assert_eq!(lengths_of(&session, clip), vec![1]);
}

/// The **legato**: every note ends a gap of ticks before the next note
/// starts, and the last note keeps its length.
#[test]
fn the_notes_are_made_legato() {
    let mut session = session();
    let clip = a_clip(&mut session, "legato", vec![note(60, 0, 100), note(62, 960, 100)]);
    session
        .execute("sound.clip.midi.duration", &json!({ "clip": clip, "gap": 40 }))
        .expect("the notes are legato");
    let lengths = lengths_of(&session, clip);
    assert_eq!(lengths[0], 920, "the first note ends forty ticks before the next");
    assert_eq!(lengths[1], 100, "and the last note keeps its length");
}

/// The length is changed **one way at a time**: a caller that meant
/// one of them and got the other would not know.
#[test]
fn two_ways_at_once_are_refused() {
    let mut session = session();
    let clip = a_clip(&mut session, "two-ways", vec![note(60, 0, 960)]);
    let refused =
        session.execute("sound.clip.midi.duration", &json!({ "clip": clip, "set": 240, "delta": 40 }));
    assert!(refused.is_err(), "the length is changed one way at a time");
    assert_eq!(lengths_of(&session, clip), vec![960], "and the note was not changed");
}

/// The **room** of the clip grows when the notes need more of it, and
/// it does not shrink when they need less: the room is the room.
#[test]
fn the_room_of_the_clip_grows_and_does_not_shrink() {
    let mut session = session();
    let clip = a_clip(&mut session, "room", vec![note(60, 0, 960), note(62, 960, 960)]);
    let before = session.document().node(clip).expect("the clip is there").data["length"]
        .as_i64()
        .expect("the length is there");
    session
        .execute("sound.clip.midi.duration", &json!({ "clip": clip, "set": 1920 }))
        .expect("the notes are set");
    let after = session.document().node(clip).expect("the clip is there").data["length"]
        .as_i64()
        .expect("the length is there");
    assert!(after > before, "the notes need more room than they had");
    session
        .execute("sound.clip.midi.duration", &json!({ "clip": clip, "set": 240 }))
        .expect("the notes are set");
    let smaller = session.document().node(clip).expect("the clip is there").data["length"]
        .as_i64()
        .expect("the length is there");
    assert_eq!(smaller, after, "and the room does not shrink");
}
