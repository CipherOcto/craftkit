//! The **separation** of a MIDI clip: the cut divides the notes, and
//! each piece keeps the notes that are its own.
//!
//! The tree's editor cuts a take and the notes go with their half: the
//! notes before the cut stay in the first piece and the notes after it
//! move to the second, moved back by the cut so that the second
//! piece's notes are counted from its own head.

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
fn a_clip(session: &mut Session) -> (u64, u64) {
    session.execute("sound.track.add", &json!({ "name": "piano" })).expect("the track is added");
    let track = session.document().node_by_name("piano").expect("the track is there").id;
    session
        .execute("sound.clip.midi.add", &json!({ "track": track, "start": 0, "length": 48_000 }))
        .expect("the clip is made");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    // A note at the head and one a quarter note later. The document's
    // tempo is a hundred and twenty, so a quarter note is half a
    // second: twenty-four thousand samples.
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 60, "start": 0, "length": 960 }))
        .expect("the first note is added");
    session
        .execute(
            "sound.clip.midi.note.add",
            &json!({ "clip": clip, "pitch": 64, "start": 960, "length": 960 }),
        )
        .expect("the second note is added");
    (track, clip)
}

/// The notes of every clip, as `(clip, pitch, start)`.
fn notes_of(session: &Session) -> Vec<(u64, u8, i64)> {
    let mut notes: Vec<(u64, u8, i64)> = Vec::new();
    for node in session.document().walk() {
        if node.kind != "sound.clip" {
            continue;
        }
        let Some(list) = node.data["midi"]["notes"].as_array() else { continue };
        for note in list {
            notes.push((
                node.id,
                note["pitch"].as_u64().expect("the pitch is there") as u8,
                note["start"].as_i64().expect("the start is there"),
            ));
        }
    }
    notes.sort();
    notes
}

/// The cut divides the **notes**: the note before the cut stays in the
/// first piece and the note after it moves to the second, counted from
/// the second piece's own head.
#[test]
fn the_cut_divides_the_notes() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    // The document's tempo is a hundred and twenty, so a quarter note
    // is half a second: twenty-four thousand samples, and nine hundred
    // and sixty ticks. The cut is a quarter note in.
    let separated = session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 24_000 }))
        .expect("the clip is separated");
    let second = separated.data["second"].as_u64().expect("the second piece is there");
    let notes = notes_of(&session);
    // The first note is the head of the first piece.
    assert_eq!(notes[0], (clip, 60, 0), "the first note: {notes:?}");
    // And the second note is the head of the second piece: it was a
    // quarter note into the clip, and the second piece starts a
    // quarter note in, so it is at the head of it.
    assert_eq!(notes[1], (second, 64, 0), "the second note: {notes:?}");
}

/// The second piece's notes are counted from **its own head**: a note
/// that was a quarter note into the clip and a quarter note after the
/// cut is at the head of the second piece.
#[test]
fn the_second_piece_counts_its_notes_from_its_own_head() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    // A third note, a quarter note after the second one.
    session
        .execute(
            "sound.clip.midi.note.add",
            &json!({ "clip": clip, "pitch": 67, "start": 1920, "length": 960 }),
        )
        .expect("the third note is added");
    let separated = session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 24_000 }))
        .expect("the clip is separated");
    let second = separated.data["second"].as_u64().expect("the second piece is there");
    let notes = notes_of(&session);
    // The third note was a quarter note after the cut, so it is a
    // quarter note into the second piece.
    assert_eq!(notes[2], (second, 67, 960), "the third note: {notes:?}");
}

/// A MIDI clip that is separated holds its notes **once**: a note is
/// not in both pieces.
#[test]
fn a_note_is_not_in_both_pieces() {
    let mut session = session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute("sound.clip.separate", &json!({ "clip": clip, "at": 12_000 }))
        .expect("the clip is separated");
    let notes = notes_of(&session);
    assert_eq!(notes.len(), 2, "there are two notes and not four: {notes:?}");
}
