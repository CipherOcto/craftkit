//! The **trim** of a MIDI clip: the notes move with the audio they are
//! in.
//!
//! The tree's trim of a MIDI clip **rebases** its notes: the notes
//! before the trim are dropped and the notes after it are counted from
//! the clip's new head.

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

/// A session with a track and a MIDI clip of two notes: one at the
/// head and one a quarter note later.
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

/// The notes of the clip, as `(pitch, start)`.
fn notes_of(session: &Session, clip: u64) -> Vec<(u8, i64)> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["midi"]["notes"]
        .as_array()
        .expect("the notes are there")
        .iter()
        .map(|note| {
            (
                note["pitch"].as_u64().expect("the pitch is there") as u8,
                note["start"].as_i64().expect("the start is there"),
            )
        })
        .collect()
}

/// The trim takes the notes **off the head**: the notes before it are
/// gone, and the notes after it are counted from the clip's new head.
#[test]
fn the_trim_takes_the_notes_off_the_head() {
    let mut session = session();
    let clip = a_clip(&mut session);
    // The clip is trimmed a quarter note in: twenty-four thousand
    // samples, which is nine hundred and sixty ticks.
    session
        .execute("sound.clip.trim", &json!({ "clip": clip, "from": 24_000, "to": 48_000 }))
        .expect("the clip is trimmed");
    let notes = notes_of(&session, clip);
    // The note at the head is gone: it was before the trim.
    assert_eq!(notes.len(), 1, "one note is left: {notes:?}");
    // And the note that was a quarter note into the clip is at the
    // head of the trimmed clip.
    assert_eq!(notes[0], (64, 0), "the second note: {notes:?}");
}

/// The trim of the **tail** leaves the notes alone: the clip is
/// shorter and the notes it has are the ones it had.
#[test]
fn the_trim_of_the_tail_leaves_the_notes_alone() {
    let mut session = session();
    let clip = a_clip(&mut session);
    session
        .execute("sound.clip.trim", &json!({ "clip": clip, "from": 0, "to": 24_000 }))
        .expect("the clip is trimmed");
    let notes = notes_of(&session, clip);
    assert_eq!(notes.len(), 2, "both notes are there: {notes:?}");
    assert_eq!(notes[0], (60, 0));
    assert_eq!(notes[1], (64, 960));
}
