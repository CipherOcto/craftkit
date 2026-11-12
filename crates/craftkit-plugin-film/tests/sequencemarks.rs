//! The **marks** of a sequence: the tree's own program in and out.

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

/// A **bin**, an **item** in it, and a **track** in the item: a
/// sequence.
fn a_sequence(session: &mut Session) -> u64 {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    item
}

/// The **marks** a sequence carries.
fn marks_of(session: &Session, sequence: u64) -> (Option<u64>, Option<u64>) {
    let node = session.document().node(sequence).expect("the sequence is there");
    let mark_in = node.data.get("sequence_mark_in").and_then(|frame| frame.as_u64());
    let mark_out = node.data.get("sequence_mark_out").and_then(|frame| frame.as_u64());
    (mark_in, mark_out)
}

/// The **mark in** and the **mark out** are set, and they are the
/// tree's own.
#[test]
fn the_marks_are_set() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 10 }))
        .expect("the mark in is set");
    session
        .execute("film.sequence.mark.out", &json!({ "sequence": sequence, "frame": 40 }))
        .expect("the mark out is set");
    assert_eq!(marks_of(&session, sequence), (Some(10), Some(40)), "the marks are the tree's");
}

/// **Setting the mark in after the mark out** takes the mark out away:
/// the tree's own rule.
#[test]
fn setting_the_mark_in_after_the_mark_out_takes_it_away() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 10 }))
        .expect("the mark in is set");
    session
        .execute("film.sequence.mark.out", &json!({ "sequence": sequence, "frame": 40 }))
        .expect("the mark out is set");
    let outcome = session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 50 }))
        .expect("the mark in is set");
    assert_eq!(outcome.data["mark_out"], json!(null), "the mark out is gone");
    assert_eq!(marks_of(&session, sequence), (Some(50), None), "and the sequence says so");
}

/// **Setting the mark out before the mark in** takes the mark in away:
/// the tree's own rule.
#[test]
fn setting_the_mark_out_before_the_mark_in_takes_it_away() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 10 }))
        .expect("the mark in is set");
    session
        .execute("film.sequence.mark.out", &json!({ "sequence": sequence, "frame": 40 }))
        .expect("the mark out is set");
    let outcome = session
        .execute("film.sequence.mark.out", &json!({ "sequence": sequence, "frame": 5 }))
        .expect("the mark out is set");
    assert_eq!(outcome.data["mark_in"], json!(null), "the mark in is gone");
    assert_eq!(marks_of(&session, sequence), (None, Some(5)), "and the sequence says so");
}

/// **A mark that does not cross the other** leaves it alone: the
/// tree's own rule.
#[test]
fn a_mark_that_does_not_cross_the_other_leaves_it_alone() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 10 }))
        .expect("the mark in is set");
    session
        .execute("film.sequence.mark.out", &json!({ "sequence": sequence, "frame": 40 }))
        .expect("the mark out is set");
    session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 20 }))
        .expect("the mark in is set");
    assert_eq!(marks_of(&session, sequence), (Some(20), Some(40)), "both marks are still there");
}

/// **The marks are taken away**: the in, the out, or both.
#[test]
fn the_marks_are_taken_away() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 10 }))
        .expect("the mark in is set");
    session
        .execute("film.sequence.mark.out", &json!({ "sequence": sequence, "frame": 40 }))
        .expect("the mark out is set");
    session
        .execute("film.sequence.mark.clear", &json!({ "sequence": sequence, "which": "in" }))
        .expect("the mark in is taken away");
    assert_eq!(marks_of(&session, sequence), (None, Some(40)), "the mark out is still there");
    session
        .execute("film.sequence.mark.clear", &json!({ "sequence": sequence, "which": "both" }))
        .expect("both marks are taken away");
    assert_eq!(marks_of(&session, sequence), (None, None), "and there is nothing left");
}

/// **A mark that is not a whole number** is refused, and so is a frame
/// before the beginning of the timeline: the tree's own rule.
#[test]
fn a_frame_that_is_not_a_frame_is_refused() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let error = session
        .execute("film.sequence.mark.in", &json!({ "sequence": sequence, "frame": 1.5 }))
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("whole number"), "the error says why: {error}");
    let error = session
        .execute("film.sequence.mark.out", &json!({ "sequence": sequence, "frame": -1.0 }))
        .expect_err("the mark is refused");
    assert!(error.to_string().contains("frame 0"), "the error says why: {error}");
}

/// **A mark that is not one of the tree's own** is refused: the tree's
/// own three commands are one command here.
#[test]
fn a_mark_that_is_not_one_of_the_tree_s_own_is_refused() {
    let mut session = session();
    let sequence = a_sequence(&mut session);
    let error = session
        .execute("film.sequence.mark.clear", &json!({ "sequence": sequence, "which": "middle" }))
        .expect_err("the marks are refused");
    assert!(error.to_string().contains("`in`, `out` or `both`"), "the error says why: {error}");
}
