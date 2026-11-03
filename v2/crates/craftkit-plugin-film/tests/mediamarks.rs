//! The **marks** of a media.
//!
//! A media may carry a mark in and a mark out, and the material a clip
//! plays is between them: a clip may not begin before the mark in and
//! may not run past the mark out.

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

/// A **bin**, an **item** in it, a **track** in the item.
fn a_timeline(session: &mut Session, media: u64) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": media }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **clip** on a track, and its id.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64, source_in: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": start,
                "duration": duration,
                "source_in": source_in,
            }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The marks of a media.
fn marks_of(session: &Session, item: u64) -> (Option<u64>, Option<u64>) {
    let node = session.document().node(item).expect("the item is there");
    let mark_in = node.data.get("mark_in").and_then(|mark| mark.as_u64());
    let mark_out = node.data.get("mark_out").and_then(|mark| mark.as_u64());
    (mark_in, mark_out)
}

/// The marks are **put on** a media, and they are the media's own.
#[test]
fn the_marks_are_put_on_a_media() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let outcome = session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 10, "mark_out": 90 }))
        .expect("the marks are set");
    assert_eq!(outcome.data["mark_in"].as_u64(), Some(10), "the mark in is said");
    assert_eq!(outcome.data["mark_out"].as_u64(), Some(90), "and the mark out");
    assert_eq!(outcome.data["length"].as_u64(), Some(100), "and how long the media is");
    let (mark_in, mark_out) = marks_of(&session, item);
    assert_eq!(mark_in, Some(10), "the media carries the mark in");
    assert_eq!(mark_out, Some(90), "and the mark out");
}

/// Only what is **named** is changed.
#[test]
fn only_what_is_named_is_changed() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 10, "mark_out": 90 }))
        .expect("the marks are set");
    session.execute("film.item.mark", &json!({ "item": item, "mark_in": 20 })).expect("the mark in is set");
    let (mark_in, mark_out) = marks_of(&session, item);
    assert_eq!(mark_in, Some(20), "the mark in is the one that was named");
    assert_eq!(mark_out, Some(90), "and the mark out is as it was");
}

/// A mark **outside** the media is refused.
#[test]
fn a_mark_outside_the_media_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 200 }))
        .expect_err("the mark is refused");
    let message = error.to_string();
    assert!(message.contains("the mark in is at frame 200"), "it says where the mark is: {message}");
    assert!(message.contains("100 frames long"), "and how long the media is: {message}");
    let (mark_in, _) = marks_of(&session, item);
    assert_eq!(mark_in, None, "and nothing was written");
}

/// A mark in **after** the mark out is refused.
#[test]
fn a_mark_in_after_the_mark_out_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 80, "mark_out": 40 }))
        .expect_err("the marks are refused");
    let message = error.to_string();
    assert!(message.contains("the first is after the second"), "it says why: {message}");
    let (mark_in, mark_out) = marks_of(&session, item);
    assert_eq!(mark_in, None, "and nothing was written");
    assert_eq!(mark_out, None, "at all");
}

/// A frame that is **not a frame** is refused.
#[test]
fn a_frame_that_is_not_a_frame_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 10.5 }))
        .expect_err("the mark is refused");
    let message = error.to_string();
    assert!(message.contains("a whole number"), "it says why: {message}");
    let error = session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": -1 }))
        .expect_err("the mark is refused");
    let message = error.to_string();
    assert!(message.contains("a whole number"), "it says why: {message}");
}

/// A node that is **not a media** is refused.
#[test]
fn a_node_that_is_not_a_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.item.mark", &json!({ "item": track, "mark_in": 10 }))
        .expect_err("the mark is refused");
    let message = error.to_string();
    assert!(message.contains("and not an item"), "it says what it is: {message}");
    let error = session
        .execute("film.item.mark", &json!({ "item": 9999, "mark_in": 10 }))
        .expect_err("the mark is refused");
    let message = error.to_string();
    assert!(message.contains("there is no node"), "it says there is nothing: {message}");
    let (_, mark_out) = marks_of(&session, item);
    assert_eq!(mark_out, None, "and nothing was written");
}

/// A clip may not run **past the mark out**.
#[test]
fn a_clip_may_not_run_past_the_mark_out() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 10, "mark_out": 90 }))
        .expect("the marks are set");
    let clip = a_clip(&mut session, track, item, 0, 80, 10);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 1 }))
        .expect_err("the trim is refused");
    let message = error.to_string();
    assert!(message.contains("would run to frame 91"), "it says where it would run to: {message}");
    assert!(message.contains("its marks are 10 and 90"), "and what the media allows: {message}");
}

/// A clip may not begin **before the mark in**.
#[test]
fn a_clip_may_not_begin_before_the_mark_in() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 10, "mark_out": 90 }))
        .expect("the marks are set");
    let clip = a_clip(&mut session, track, item, 0, 80, 10);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "in", "delta": -5 }))
        .expect_err("the trim is refused");
    let message = error.to_string();
    assert!(message.contains("the media begins at frame 10"), "it says where the media begins: {message}");
}

/// A **slip** stays between the marks.
#[test]
fn a_slip_stays_between_the_marks() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 10, "mark_out": 90 }))
        .expect("the marks are set");
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    let error = session
        .execute("film.clip.slip", &json!({ "clip": clip, "delta": 80 }))
        .expect_err("the slip is refused");
    let message = error.to_string();
    assert!(message.contains("would run to frame 110"), "it says where it would run to: {message}");
}

/// A media with **no marks** is all of itself.
#[test]
fn a_media_with_no_marks_is_all_of_itself() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 100, 0);
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 1 }))
        .expect_err("the trim is refused");
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 1 }))
        .expect_err("the trim is refused");
    let message = error.to_string();
    assert!(message.contains("the media is 100 frames long"), "it says how long the media is: {message}");
}

/// The marks are **saved** with the document.
#[test]
fn the_marks_are_saved() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    session
        .execute("film.item.mark", &json!({ "item": item, "mark_in": 10, "mark_out": 90 }))
        .expect("the marks are set");
    let value = session.document().to_json();
    let text = value.to_string();
    assert!(text.contains("mark_in"), "the mark in is in the document");
    assert!(text.contains("mark_out"), "and the mark out");
}
