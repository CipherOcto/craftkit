//! The **editing of the notes** of a MIDI clip: adding them, changing
//! them, taking them away and moving them.
//!
//! The tree's MIDI editor has a pencil: a note is added, edited, taken
//! away and moved, and the tree clamps what it is given. Here a note
//! outside the range a MIDI note has is **refused**.

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

/// A session with a track and a MIDI clip of no notes on it.
fn a_clip(session: &mut Session) -> (u64, u64) {
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session.execute("sound.clip.midi.add", &json!({ "track": track, "start": 0 })).expect("the clip is made");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    (track, clip)
}

/// The notes of the clip, as `(pitch, start, length, velocity)`.
fn notes_of(session: &Session, clip: u64) -> Vec<(u8, i64, i64, u8)> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["midi"]["notes"]
        .as_array()
        .expect("the notes are there")
        .iter()
        .map(|note| {
            (
                note["pitch"].as_u64().unwrap_or(0) as u8,
                note["start"].as_i64().unwrap_or(0),
                note["length"].as_i64().unwrap_or(0),
                note["velocity"].as_u64().unwrap_or(0) as u8,
            )
        })
        .collect()
}

/// A MIDI clip is made with **no notes** in it, and it is a bar long
/// at the document's tempo.
#[test]
fn a_midi_clip_is_made_with_no_notes() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    let node = session.document().node(clip).expect("the clip is there");
    assert_eq!(node.data["content"], json!("midi"), "the clip holds notes");
    assert!(notes_of(&session, clip).is_empty(), "and there are none of them yet");
    // A hundred and twenty beats a minute is half a second a quarter
    // note, and a bar is four of them.
    assert_eq!(node.data["length"], json!(96_000), "the clip is a bar long");
}

/// A **note** is added to the clip, and the clip grows to hold it.
#[test]
fn a_note_is_added() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute(
            "sound.clip.midi.note.add",
            &json!({ "clip": clip, "pitch": 60, "start": 0, "length": 960, "velocity": 90 }),
        )
        .expect("the note is added");
    assert_eq!(notes_of(&session, clip), [(60, 0, 960, 90)], "the note is there");
    let node = session.document().node(clip).expect("the clip is there");
    assert_eq!(node.data["length"], json!(96_000), "and the clip keeps its room");
}

/// A note is **edited**: its pitch, its place, its length and its
/// velocity.
#[test]
fn a_note_is_edited() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 60, "start": 0 }))
        .expect("the note is added");
    session
        .execute(
            "sound.clip.midi.note.edit",
            &json!({ "clip": clip, "index": 0, "pitch": 72, "length": 480, "velocity": 40 }),
        )
        .expect("the note is edited");
    assert_eq!(notes_of(&session, clip), [(72, 0, 480, 40)], "the note is changed");
}

/// A note is **taken away**.
#[test]
fn a_note_is_removed() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 60, "start": 0 }))
        .expect("the first note is added");
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 64, "start": 960 }))
        .expect("the second note is added");
    let removed = session
        .execute("sound.clip.midi.note.remove", &json!({ "clip": clip, "indices": [0] }))
        .expect("the note is removed");
    assert_eq!(removed.data["removed"], 1, "one note went");
    assert_eq!(notes_of(&session, clip), [(64, 960, 480, 100)], "and the other is left");
}

/// Notes are **moved**: later, and higher.
#[test]
fn notes_are_moved() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 60, "start": 0 }))
        .expect("the note is added");
    session
        .execute(
            "sound.clip.midi.notes.move",
            &json!({ "clip": clip, "indices": [0], "ticks": 960, "semitones": 12 }),
        )
        .expect("the note is moved");
    assert_eq!(notes_of(&session, clip), [(72, 960, 480, 100)], "the note has moved");
}

/// A pitch, a velocity or a channel outside the range a MIDI note has
/// is **refused**, and the note is not made.
#[test]
fn a_note_outside_the_range_is_refused() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    for params in [
        json!({ "clip": clip, "pitch": 200, "start": 0 }),
        json!({ "clip": clip, "pitch": 60, "start": 0, "velocity": 0 }),
        json!({ "clip": clip, "pitch": 60, "start": 0, "velocity": 300 }),
        json!({ "clip": clip, "pitch": 60, "start": 0, "channel": 20 }),
        json!({ "clip": clip, "pitch": 60, "start": 0, "length": 0 }),
        json!({ "clip": clip, "pitch": 60, "start": -1 }),
    ] {
        let refused = session.execute("sound.clip.midi.note.add", &params);
        assert!(refused.is_err(), "{params} is refused");
    }
    assert!(notes_of(&session, clip).is_empty(), "and no note was made");
}

/// A note that would leave the range when it is **moved** is refused,
/// and no note moves.
#[test]
fn a_move_out_of_the_range_is_refused() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 120, "start": 0 }))
        .expect("the note is added");
    let refused = session
        .execute("sound.clip.midi.notes.move", &json!({ "clip": clip, "indices": [0], "semitones": 12 }));
    assert!(refused.is_err(), "the note would leave the range");
    assert_eq!(notes_of(&session, clip), [(120, 0, 480, 100)], "and the note did not move");
}

/// An **index** that names no note is refused: a note that is not
/// there is not edited and not removed.
#[test]
fn an_index_that_names_no_note_is_refused() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    let refused =
        session.execute("sound.clip.midi.note.edit", &json!({ "clip": clip, "index": 3, "pitch": 60 }));
    assert!(refused.is_err(), "there is no third note");
    let refused = session.execute("sound.clip.midi.note.remove", &json!({ "clip": clip, "indices": [0] }));
    assert!(refused.is_err(), "and there is no note to take away");
}

/// A clip that holds **no notes** cannot be given one: it is not a
/// MIDI clip at all.
#[test]
fn a_clip_that_is_not_a_midi_clip_is_refused() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 4_800 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    for (command, params) in [
        ("sound.clip.midi.note.add", json!({ "clip": clip, "pitch": 60, "start": 0 })),
        ("sound.clip.midi.note.edit", json!({ "clip": clip, "index": 0 })),
        ("sound.clip.midi.note.remove", json!({ "clip": clip, "indices": [0] })),
        ("sound.clip.midi.notes.move", json!({ "clip": clip, "indices": [0] })),
    ] {
        let refused = session.execute(command, &params);
        assert!(refused.is_err(), "{command} has no notes to work on");
    }
}
