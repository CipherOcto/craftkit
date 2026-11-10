//! The playhead goes to the **marks** of a sequence: the tree's own
//! go to the in and go to the out.

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
fn a_sequence(session: &mut Session) -> u64 {
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
    sequence
}

/// The **frame** the playhead is at.
fn frame(session: &mut Session, sequence: u64) -> u64 {
    let outcome =
        session.execute("film.playhead.get", &json!({ "sequence": sequence })).expect("the playhead is read");
    outcome.data["frame"].as_u64().expect("the playhead has a frame")
}

/// The playhead goes to the **mark in** and to the **mark out**: the
/// tree's own rules.
#[test]
fn the_playhead_goes_to_the_marks() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 10 }))
        .expect("the mark in is set");
    session
        .execute("film.sequence.mark.out", &json!({ "sequence": sequence, "frame": 40 }))
        .expect("the mark out is set");
    session
        .execute("film.playhead.goto.mark", &json!({ "sequence": sequence, "which": "in" }))
        .expect("the playhead goes to the mark in");
    assert_eq!(frame(&mut session, sequence), 10, "the playhead is at the mark in");
    session
        .execute("film.playhead.goto.mark", &json!({ "sequence": sequence, "which": "out" }))
        .expect("the playhead goes to the mark out");
    assert_eq!(frame(&mut session, sequence), 40, "the playhead is at the mark out");
}

/// **A sequence with no mark in** sends the playhead to the **start**:
/// the tree's own rule.
#[test]
fn a_sequence_with_no_mark_in_sends_the_playhead_to_the_start() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    session
        .execute("film.playhead.set", &json!({ "sequence": sequence, "frame": 20 }))
        .expect("the playhead is set");
    session
        .execute("film.playhead.goto.mark", &json!({ "sequence": sequence, "which": "in" }))
        .expect("the playhead goes to the mark in");
    assert_eq!(frame(&mut session, sequence), 0, "the playhead is at the start");
}

/// **A sequence with no mark out** sends the playhead to the **end**:
/// the tree's own rule.
#[test]
fn a_sequence_with_no_mark_out_sends_the_playhead_to_the_end() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    session
        .execute("film.playhead.set", &json!({ "sequence": sequence, "frame": 20 }))
        .expect("the playhead is set");
    session
        .execute("film.playhead.goto.mark", &json!({ "sequence": sequence, "which": "out" }))
        .expect("the playhead goes to the mark out");
    assert_eq!(frame(&mut session, sequence), 50, "the playhead is at the end of the sequence");
}

/// **A mark that is not one of the tree's own** is refused.
#[test]
fn a_mark_that_is_not_one_of_the_tree_s_own_is_refused() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let error = session
        .execute("film.playhead.goto.mark", &json!({ "sequence": sequence, "which": "middle" }))
        .expect_err("the playhead does not go");
    assert!(error.to_string().contains("`in` or `out`"), "the error says why: {error}");
}
