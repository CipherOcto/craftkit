//! The **controllers** of a MIDI clip, and the **duplicate notes**.
//!
//! A MIDI clip holds notes and controllers; the notes had a pencil
//! (§223) and the controllers had none.

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
fn new_session() -> Session {
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

/// The controllers of the clip, as `(kind, tick, value)`.
fn ctrls_of(session: &Session, clip: u64) -> Vec<(String, i64, i64)> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["midi"]["ctrls"]
        .as_array()
        .expect("the controllers are there")
        .iter()
        .map(|ctrl| {
            (
                ctrl["kind"].as_str().unwrap_or("").to_string(),
                ctrl["tick"].as_i64().unwrap_or(0),
                ctrl["value"].as_i64().unwrap_or(0),
            )
        })
        .collect()
}

/// A **controller** is added to a clip: a control change, a pitch
/// bend, a pressure or a program.
#[test]
fn a_controller_is_added() {
    let mut session = new_session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute(
            "sound.clip.midi.ctrl.add",
            &json!({ "clip": clip, "kind": "control", "number": 7, "at": 0, "value": 100 }),
        )
        .expect("the controller is added");
    session
        .execute(
            "sound.clip.midi.ctrl.add",
            &json!({ "clip": clip, "kind": "pitch_bend", "at": 480, "value": -8192 }),
        )
        .expect("the bend is added");
    let ctrls = ctrls_of(&session, clip);
    assert_eq!(ctrls, [("control".to_string(), 0, 100), ("pitch_bend".to_string(), 480, -8192)]);
}

/// A controller is **edited**, and an index that names none is
/// refused.
#[test]
fn a_controller_is_edited() {
    let mut session = new_session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute(
            "sound.clip.midi.ctrl.add",
            &json!({ "clip": clip, "kind": "control", "number": 7, "at": 0, "value": 100 }),
        )
        .expect("the controller is added");
    session
        .execute("sound.clip.midi.ctrl.edit", &json!({ "clip": clip, "index": 0, "value": 40, "at": 960 }))
        .expect("the controller is edited");
    assert_eq!(ctrls_of(&session, clip), [("control".to_string(), 960, 40)]);
    let refused =
        session.execute("sound.clip.midi.ctrl.edit", &json!({ "clip": clip, "index": 3, "value": 40 }));
    assert!(refused.is_err(), "there is no fourth controller");
}

/// A controller is **taken off** a clip.
#[test]
fn a_controller_is_removed() {
    let mut session = new_session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute(
            "sound.clip.midi.ctrl.add",
            &json!({ "clip": clip, "kind": "control", "number": 7, "at": 0, "value": 100 }),
        )
        .expect("the controller is added");
    session
        .execute(
            "sound.clip.midi.ctrl.add",
            &json!({ "clip": clip, "kind": "program", "at": 480, "value": 12 }),
        )
        .expect("the program is added");
    let removed = session
        .execute("sound.clip.midi.ctrl.remove", &json!({ "clip": clip, "indices": [0] }))
        .expect("the controller is removed");
    assert_eq!(removed.data["removed"], 1, "one controller went");
    assert_eq!(ctrls_of(&session, clip), [("program".to_string(), 480, 12)]);
}

/// A kind that is **not** a controller is refused, and so is a number
/// for a kind that has none, and a kind that has one without it.
#[test]
fn a_controller_outside_its_range_is_refused() {
    let mut session = new_session();
    let (_, clip) = a_clip(&mut session);
    for params in [
        json!({ "clip": clip, "kind": "brightness", "at": 0, "value": 1 }),
        json!({ "clip": clip, "kind": "control", "at": 0, "value": 1 }),
        json!({ "clip": clip, "kind": "program", "number": 7, "at": 0, "value": 1 }),
        json!({ "clip": clip, "kind": "control", "number": 7, "at": 0, "value": 300 }),
        json!({ "clip": clip, "kind": "control", "number": 7, "at": 0, "value": 1, "channel": 20 }),
        json!({ "clip": clip, "kind": "pitch_bend", "at": 0, "value": 9000 }),
        json!({ "clip": clip, "kind": "control", "number": 7, "at": -1, "value": 1 }),
    ] {
        let refused = session.execute("sound.clip.midi.ctrl.add", &params);
        assert!(refused.is_err(), "{params} is refused");
    }
    assert!(ctrls_of(&session, clip).is_empty(), "and no controller was made");
}

/// The **duplicate notes** are taken out: two notes of the same
/// channel, pitch and start are one note, and the longer one is kept.
#[test]
fn the_duplicate_notes_are_taken_out() {
    let mut session = new_session();
    let (_, clip) = a_clip(&mut session);
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 60, "start": 0, "length": 100 }))
        .expect("the first note is added");
    session
        .execute("sound.clip.midi.note.add", &json!({ "clip": clip, "pitch": 60, "start": 0, "length": 400 }))
        .expect("the second note is added");
    let tidied = session
        .execute("sound.clip.midi.notes.dedupe", &json!({ "clip": clip }))
        .expect("the notes are tidied");
    assert_eq!(tidied.data["removed"], 1, "one note went");
    let node = session.document().node(clip).expect("the clip is there");
    let notes = node.data["midi"]["notes"].as_array().expect("the notes are there");
    assert_eq!(notes.len(), 1, "and one is left");
    assert_eq!(notes[0]["length"], json!(400), "the longer one is the one that is kept");
}
