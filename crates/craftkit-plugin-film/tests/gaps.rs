//! **Closing** a gap.

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
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": start, "duration": duration, "source_in": 0 }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The **start** of a clip, as the document holds it.
fn start_of(session: &Session, clip: u64) -> u64 {
    session
        .document()
        .node(clip)
        .expect("the clip is there")
        .data
        .get("start")
        .and_then(|start| start.as_u64())
        .unwrap_or(0)
}

/// A gap on a track is **closed**: the clips that were after it move up.
#[test]
fn a_gap_on_a_track_is_closed() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, item, 0, 10);
    let after = a_clip(&mut session, track, item, 30, 10);
    let outcome = session
        .execute("film.timeline.closegap", &json!({ "track": track, "time": 20 }))
        .expect("the gap is closed");
    let gap = &outcome.data["gap"];
    assert_eq!(gap["start"].as_u64(), Some(10), "the gap starts where the clip before it ends");
    assert_eq!(gap["end"].as_u64(), Some(30), "and ends where the clip after it starts");
    assert_eq!(gap["length"].as_u64(), Some(20), "and it is that long");
    assert_eq!(start_of(&session, after), 10, "and the clip after it moved up");
}

/// A place that is **covered** by a clip is not a gap.
#[test]
fn a_place_that_is_covered_is_not_a_gap() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, item, 0, 10);
    let error = session
        .execute("film.timeline.closegap", &json!({ "track": track, "time": 5 }))
        .expect_err("the gap is refused");
    let message = error.to_string();
    assert!(message.contains("there is no gap at frame 5"), "it says why: {message}");
}

/// A track with **no clip after** the place has no gap there.
#[test]
fn a_track_with_no_clip_after_the_place_has_no_gap() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, item, 0, 10);
    let error = session
        .execute("film.timeline.closegap", &json!({ "track": track, "time": 20 }))
        .expect_err("the gap is refused");
    let message = error.to_string();
    assert!(message.contains("no clip after it"), "it says why: {message}");
}

/// A **locked** track may not have its gap closed.
#[test]
fn a_locked_track_may_not_have_its_gap_closed() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, item, 0, 10);
    let after = a_clip(&mut session, track, item, 30, 10);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let error = session
        .execute("film.timeline.closegap", &json!({ "track": track, "time": 20 }))
        .expect_err("the gap is refused");
    let message = error.to_string();
    assert!(message.contains("is locked"), "it says why: {message}");
    assert_eq!(start_of(&session, after), 30, "and nothing moved");
}

/// A place that is **not a whole number** is refused.
#[test]
fn a_place_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, item, 0, 10);
    let error = session
        .execute("film.timeline.closegap", &json!({ "track": track, "time": 20.5 }))
        .expect_err("the gap is refused");
    let message = error.to_string();
    assert!(message.contains("whole number"), "it says why: {message}");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.timeline.closegap", &json!({ "track": item, "time": 0 }))
        .expect_err("the gap is refused");
    let message = error.to_string();
    assert!(message.contains("not a track"), "it says why: {message}");
    let error = session
        .execute("film.timeline.closegap", &json!({ "track": 9999, "time": 0 }))
        .expect_err("the gap is refused");
    let message = error.to_string();
    assert!(message.contains("there is no node"), "and there is nothing: {message}");
}
