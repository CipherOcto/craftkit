//! The **work area** of a sequence.
//!
//! The range of the sequence the editor is working in: an in point
//! and an out point. The tree's work area, and the tree's rules.

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

/// A **bin**, an **item** in it, and a **track** in the item.
fn a_sequence(session: &mut Session) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 100 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// The work area is **set** and read back.
#[test]
fn the_work_area_is_set_and_read_back() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    session
        .execute("film.workarea.set", &json!({ "sequence": item, "start": 10, "end": 40 }))
        .expect("the work area is set");
    let answer =
        session.execute("film.workarea.get", &json!({ "sequence": item })).expect("the work area is read");
    assert_eq!(answer.data["work_area"]["start"], json!(10), "the work area begins where it was set");
    assert_eq!(answer.data["work_area"]["end"], json!(40), "and ends where it was set");
    assert_eq!(answer.data["work_area"]["length"], json!(31), "and it is thirty-one frames long");
}

/// A sequence that says nothing has **no** work area — and that is not
/// the same as a work area of one frame at the start.
#[test]
fn a_sequence_with_no_work_area_has_none() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    let answer =
        session.execute("film.workarea.get", &json!({ "sequence": item })).expect("the work area is read");
    assert!(answer.data["work_area"].is_null(), "there is no work area: {}", answer.data);
    session
        .execute("film.workarea.set", &json!({ "sequence": item, "start": 0, "end": 0 }))
        .expect("the work area is set");
    let answer =
        session.execute("film.workarea.get", &json!({ "sequence": item })).expect("the work area is read");
    assert_eq!(answer.data["work_area"]["length"], json!(1), "and now there is one of a single frame");
}

/// The work area is **taken away**: an empty value takes it away.
#[test]
fn the_work_area_is_taken_away() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    session
        .execute("film.workarea.set", &json!({ "sequence": item, "start": 10, "end": 40 }))
        .expect("the work area is set");
    session
        .execute("film.workarea.clear", &json!({ "sequence": item }))
        .expect("the work area is taken away");
    let answer =
        session.execute("film.workarea.get", &json!({ "sequence": item })).expect("the work area is read");
    assert!(answer.data["work_area"].is_null(), "and there is none any more: {}", answer.data);
}

/// A work area that **ends before it begins** is refused.
#[test]
fn a_work_area_that_ends_before_it_begins_is_refused() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    let error = session
        .execute("film.workarea.set", &json!({ "sequence": item, "start": 40, "end": 10 }))
        .expect_err("the work area is refused");
    assert!(error.to_string().contains("ends at frame 10"), "the refusal says why: {error}");
}

/// A frame that is **not a whole number** is refused, and so is one
/// before the beginning of the timeline.
#[test]
fn a_frame_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    let error = session
        .execute("film.workarea.set", &json!({ "sequence": item, "start": 10.5, "end": 40 }))
        .expect_err("the work area is refused");
    assert!(error.to_string().contains("whole number"), "the refusal says why: {error}");
    let error = session
        .execute("film.workarea.set", &json!({ "sequence": item, "start": -1, "end": 40 }))
        .expect_err("the work area is refused");
    assert!(error.to_string().contains("frame 0"), "the refusal says why: {error}");
}

/// The work area is a fact of **one** sequence, and the other sequence
/// keeps its own.
#[test]
fn the_work_area_is_a_fact_of_one_sequence() {
    let mut session = session();
    let (item, _track) = a_sequence(&mut session);
    let (other, _other_track) = a_sequence(&mut session);
    session
        .execute("film.workarea.set", &json!({ "sequence": item, "start": 10, "end": 40 }))
        .expect("the work area is set");
    let answer =
        session.execute("film.workarea.get", &json!({ "sequence": other })).expect("the work area is read");
    assert!(answer.data["work_area"].is_null(), "the other sequence has none: {}", answer.data);
}
