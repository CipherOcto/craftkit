//! **Replacing** the media a clip plays.

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

/// A **bin**, and a **media item** in it, and their ids.
fn an_item(session: &mut Session, name: &str, item: u64, duration: u64) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": name, "bin": bin, "item": item, "duration": duration }))
        .expect("the item is added");
    let media = outcome.data["id"].as_u64().expect("the item has an id");
    (bin, media)
}

/// A **sequence** with a **track**, and a **clip** on it.
fn a_clip(session: &mut Session, media: u64, duration: u64) -> (u64, u64) {
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": media, "item": 9 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": media, "start": 0, "duration": duration, "source_in": 0 }),
        )
        .expect("the clip is added");
    let clip = outcome.data["id"].as_u64().expect("the clip has an id");
    (track, clip)
}

/// The media a clip plays, as the document's edge says it.
fn media_of(session: &Session, clip: u64) -> Option<u64> {
    session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "clip" && relation.from == clip)
        .map(|relation| relation.to)
}

/// The **in point** of a clip, as the document holds it.
fn source_in_of(session: &Session, clip: u64) -> u64 {
    session
        .document()
        .node(clip)
        .expect("the clip is there")
        .data
        .get("source_in")
        .and_then(|source_in| source_in.as_u64())
        .unwrap_or(0)
}

/// A clip may play a **different media**.
#[test]
fn a_clip_may_play_a_different_media() {
    let mut session = session();
    let (_bin, first) = an_item(&mut session, "the first", 1, 100);
    let (_bin, second) = an_item(&mut session, "the second", 2, 100);
    let (_track, clip) = a_clip(&mut session, first, 10);
    let outcome = session
        .execute("film.clip.replace", &json!({ "clip": clip, "item": second }))
        .expect("the media is replaced");
    assert_eq!(outcome.data["item"].as_u64(), Some(second), "the new media is said");
    assert_eq!(outcome.data["old_item"].as_u64(), Some(first), "and the old one");
    assert_eq!(media_of(&session, clip), Some(second), "and the clip plays it now");
}

/// The clip plays the new media from its **beginning**.
#[test]
fn the_clip_plays_the_new_media_from_its_beginning() {
    let mut session = session();
    let (_bin, first) = an_item(&mut session, "the first", 1, 100);
    let (_bin, second) = an_item(&mut session, "the second", 2, 100);
    let (_track, clip) = a_clip(&mut session, first, 10);
    session
        .execute("film.clip.replace", &json!({ "clip": clip, "item": second }))
        .expect("the media is replaced");
    assert_eq!(source_in_of(&session, clip), 0, "the in point is the beginning");
}

/// A clip that would run **past the end** of the new media is refused.
#[test]
fn a_clip_that_would_run_past_the_end_is_refused() {
    let mut session = session();
    let (_bin, first) = an_item(&mut session, "the first", 1, 100);
    let (_bin, second) = an_item(&mut session, "the second", 2, 40);
    let (_track, clip) = a_clip(&mut session, first, 60);
    let error = session
        .execute("film.clip.replace", &json!({ "clip": clip, "item": second }))
        .expect_err("the replace is refused");
    let message = error.to_string();
    assert!(message.contains("would run to frame 60"), "it says where it would run to: {message}");
    assert!(message.contains("40 frames long"), "and how long the media is: {message}");
    assert_eq!(media_of(&session, clip), Some(first), "and the clip still plays the old media");
}

/// A media with **no frames** is refused.
#[test]
fn a_media_with_no_frames_is_refused() {
    let mut session = session();
    let (_bin, first) = an_item(&mut session, "the first", 1, 100);
    let (_bin, second) = an_item(&mut session, "the second", 2, 0);
    let (_track, clip) = a_clip(&mut session, first, 10);
    let error = session
        .execute("film.clip.replace", &json!({ "clip": clip, "item": second }))
        .expect_err("the replace is refused");
    let message = error.to_string();
    assert!(message.contains("no frames to play"), "it says why: {message}");
}

/// A **locked** track may not have its media replaced.
#[test]
fn a_locked_track_may_not_have_its_media_replaced() {
    let mut session = session();
    let (_bin, first) = an_item(&mut session, "the first", 1, 100);
    let (_bin, second) = an_item(&mut session, "the second", 2, 100);
    let (track, clip) = a_clip(&mut session, first, 10);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let error = session
        .execute("film.clip.replace", &json!({ "clip": clip, "item": second }))
        .expect_err("the replace is refused");
    let message = error.to_string();
    assert!(message.contains("is locked"), "it says why: {message}");
    assert_eq!(media_of(&session, clip), Some(first), "and the clip still plays the old media");
}

/// A node that is **not a clip** is refused, and so is a node that is
/// not an **item**.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_bin, first) = an_item(&mut session, "the first", 1, 100);
    let (bin, second) = an_item(&mut session, "the second", 2, 100);
    let (_track, clip) = a_clip(&mut session, first, 10);
    let error = session
        .execute("film.clip.replace", &json!({ "clip": bin, "item": second }))
        .expect_err("the replace is refused");
    let message = error.to_string();
    assert!(message.contains("not a clip"), "it says why: {message}");
    let error = session
        .execute("film.clip.replace", &json!({ "clip": clip, "item": bin }))
        .expect_err("the replace is refused");
    let message = error.to_string();
    assert!(message.contains("not an item"), "and the media must be an item: {message}");
    let error = session
        .execute("film.clip.replace", &json!({ "clip": 9999, "item": second }))
        .expect_err("the replace is refused");
    let message = error.to_string();
    assert!(message.contains("there is no node"), "and there is nothing: {message}");
}
