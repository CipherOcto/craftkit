//! The **playhead** of a sequence.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
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

/// A **sequence**: an item with a track and a clip in it.
fn a_sequence(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let sequence = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "V1", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    session
        .execute("film.clip.add", &json!({ "track": track, "item": sequence, "start": 0, "duration": 50 }))
        .expect("the clip is added");
    (bin, sequence)
}

/// The **frame** the playhead is at.
fn frame(session: &mut Session, sequence: u64) -> u64 {
    let outcome =
        session.execute("film.playhead.get", &json!({ "sequence": sequence })).expect("the playhead is read");
    outcome.data["frame"].as_u64().expect("the playhead has a frame")
}

/// The playhead may be **set** and **read**.
#[test]
fn the_playhead_may_be_set_and_read() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    assert_eq!(frame(&mut session, sequence), 0, "the playhead begins at the start");
    session
        .execute("film.playhead.set", &json!({ "sequence": sequence, "frame": 12 }))
        .expect("the playhead is set");
    assert_eq!(frame(&mut session, sequence), 12, "and it is where it was put");
    session
        .execute("film.playhead.set", &json!({ "sequence": sequence, "frame": -3 }))
        .expect("the playhead is set again");
    assert!(
        session.execute("film.playhead.set", &json!({ "sequence": sequence, "frame": 1.5 })).is_err(),
        "and a frame that is not a whole number is refused"
    );
}

/// The playhead may go to the **start** and to the **end**.
#[test]
fn the_playhead_may_go_to_the_start_and_to_the_end() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    session
        .execute("film.playhead.set", &json!({ "sequence": sequence, "frame": 20 }))
        .expect("the playhead is set");
    session
        .execute("film.playhead.end", &json!({ "sequence": sequence }))
        .expect("the playhead goes to the end");
    assert_eq!(frame(&mut session, sequence), 50, "the end is the end of the last clip");
    session
        .execute("film.playhead.start", &json!({ "sequence": sequence }))
        .expect("the playhead goes to the start");
    assert_eq!(frame(&mut session, sequence), 0, "and the start is the first frame");
}

/// The playhead may be **stepped**.
#[test]
fn the_playhead_may_be_stepped() {
    let mut session = session();
    let (_bin, sequence) = a_sequence(&mut session);
    session
        .execute("film.playhead.set", &json!({ "sequence": sequence, "frame": 10 }))
        .expect("the playhead is set");
    session
        .execute("film.playhead.step", &json!({ "sequence": sequence, "frames": 5 }))
        .expect("the playhead is stepped");
    assert_eq!(frame(&mut session, sequence), 15, "five frames on");
    session
        .execute("film.playhead.step", &json!({ "sequence": sequence, "frames": -20 }))
        .expect("the playhead is stepped back");
    assert_eq!(frame(&mut session, sequence), 0, "and a step past the start stops at the start");
}

/// A sequence that is **not** a sequence is refused.
#[test]
fn a_sequence_that_is_not_a_sequence_is_refused() {
    let mut session = session();
    let (bin, _sequence) = a_sequence(&mut session);
    let outcome = session.execute("film.playhead.get", &json!({ "sequence": bin }));
    assert!(outcome.is_err(), "a bin is not a sequence");
    let error = session
        .execute("film.playhead.get", &json!({ "sequence": 9999 }))
        .expect_err("there is no such sequence");
    assert!(error.to_string().contains("9999"), "and the error says which");
}
