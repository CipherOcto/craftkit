//! The **velocity** of the notes of a MIDI clip: how hard they are
//! struck, and the ways the battery changes that.
//!
//! The tree's sound editor changes a note's velocity four ways: it
//! sets it, adds to it, scales it and limits it. The **limit** is the
//! one that moves a velocity into the range on purpose — a limit says
//! *no softer than this and no harder than that* — and its own bounds
//! are still refused when they are outside the range.
//!
//! The tree's **humanise** is not here: it moves each note by a random
//! amount, and a note's place that changes each time it is asked for
//! is not a fact.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-velocity-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A note of that pitch, that many ticks long, struck that hard.
fn note(pitch: u8, start: i64, length: i64, velocity: u8) -> Note {
    Note { pitch, velocity, release_velocity: 64, channel: 0, start, length }
}

/// A session with a track, a MIDI clip of those notes, and the clip's
/// id.
///
/// The track is named after the test, so that a session with two clips
/// in it — one made before the other — still says which is which: the
/// clip is the one under the track that was just made.
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

/// The velocities of the clip's notes, in the order they are in.
fn velocities_of(session: &Session, clip: u64) -> Vec<u8> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["midi"]["notes"]
        .as_array()
        .expect("the notes are there")
        .iter()
        .map(|note| note["velocity"].as_u64().expect("the velocity is there") as u8)
        .collect()
}

/// The velocity of the notes is **scaled**, and a factor that would
/// take a note out of the range is refused and no note is changed.
#[test]
fn the_velocity_of_the_notes_is_scaled() {
    let mut session = session();
    let clip = a_clip(&mut session, "scale", vec![note(60, 0, 960, 40), note(62, 960, 960, 60)]);
    session
        .execute("sound.clip.midi.velocity", &json!({ "clip": clip, "scale": 2.0 }))
        .expect("the notes are scaled");
    assert_eq!(velocities_of(&session, clip), vec![80, 120]);
    // A scale of ten would take the note of a hundred and twenty out of
    // the range, and the tree pushes it onto the end of it.
    let clip = a_clip(&mut session, "scale-refused", vec![note(60, 0, 960, 100)]);
    let refused = session.execute("sound.clip.midi.velocity", &json!({ "clip": clip, "scale": 2.0 }));
    assert!(refused.is_err(), "a note cannot be struck at two hundred");
    assert_eq!(velocities_of(&session, clip), vec![100], "and the note was not changed");
}

/// The **limit** moves a note onto the edge of the limit, and the
/// bounds of the limit are refused when they are outside the range.
#[test]
fn the_velocity_of_the_notes_is_limited() {
    let mut session = session();
    let clip = a_clip(&mut session, "limit", vec![note(60, 0, 960, 20), note(62, 960, 960, 120)]);
    session
        .execute("sound.clip.midi.velocity", &json!({ "clip": clip, "min": 40, "max": 100 }))
        .expect("the notes are limited");
    assert_eq!(velocities_of(&session, clip), vec![40, 100]);
    // A limit of zero is not a limit of anything: a velocity of zero is
    // the way a note is let go.
    let clip = a_clip(&mut session, "limit-refused", vec![note(60, 0, 960, 20)]);
    let refused = session.execute("sound.clip.midi.velocity", &json!({ "clip": clip, "min": 0, "max": 100 }));
    assert!(refused.is_err(), "a limit of zero is not a limit");
    assert_eq!(velocities_of(&session, clip), vec![20], "and the note was not changed");
}

/// A limit with **one** bound is refused: a limit says no softer than
/// this **and** no harder than that.
#[test]
fn a_limit_with_one_bound_is_refused() {
    let mut session = session();
    let clip = a_clip(&mut session, "one-bound", vec![note(60, 0, 960, 20)]);
    let refused = session.execute("sound.clip.midi.velocity", &json!({ "clip": clip, "min": 40 }));
    assert!(refused.is_err(), "a limit has two bounds");
}

/// The velocity is changed **one way at a time**: a caller that meant
/// one of them and got the other would not know.
#[test]
fn two_ways_at_once_are_refused() {
    let mut session = session();
    let clip = a_clip(&mut session, "two-ways", vec![note(60, 0, 960, 20)]);
    let refused =
        session.execute("sound.clip.midi.velocity", &json!({ "clip": clip, "set": 40, "delta": 10 }));
    assert!(refused.is_err(), "the velocity is changed one way at a time");
    assert_eq!(velocities_of(&session, clip), vec![20], "and the note was not changed");
}
