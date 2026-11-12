//! **Reversing** a clip.
//!
//! A clip may play its media backwards, and the flag is a property of
//! the direction and not of the media — and it changes what the
//! clip's edges consume.

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

/// A **clip** on a track.
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

/// The three numbers of a clip.
fn clock_of(session: &Session, clip: u64) -> (u64, u64, u64) {
    let node = session.document().node(clip).expect("the clip is there");
    let start = node.data["start"].as_u64().expect("the clip has a start");
    let duration = node.data["duration"].as_u64().expect("the clip has a duration");
    let source_in = node.data["source_in"].as_u64().expect("the clip has a source");
    (start, duration, source_in)
}

/// Whether a clip plays **backwards**.
fn is_reversed(session: &Session, clip: u64) -> bool {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["reverse"].as_bool().unwrap_or(false)
}

/// Reverse a clip.
fn reverse(session: &mut Session, clip: u64) {
    session.execute("film.clip.reverse", &json!({ "clip": clip })).expect("the clip is reversed");
}

/// A clip may play its media **backwards**.
#[test]
fn a_clip_may_play_its_media_backwards() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    assert!(!is_reversed(&session, clip), "the clip plays forwards to begin with");
    reverse(&mut session, clip);
    assert!(is_reversed(&session, clip), "and backwards now");
}

/// A clip reversed **twice** plays forwards again.
#[test]
fn a_clip_reversed_twice_plays_forwards_again() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    reverse(&mut session, clip);
    reverse(&mut session, clip);
    assert!(!is_reversed(&session, clip), "the clip plays forwards again");
}

/// A reversed clip's **in edge** does not move the media: the clip
/// plays backwards and so its head is the *end* of its media.
#[test]
fn the_in_edge_of_a_reversed_clip_does_not_move_the_media() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    reverse(&mut session, clip);
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "in", "delta": 5 }))
        .expect("the clip is trimmed");
    assert_eq!(clock_of(&session, clip), (15, 15, 30), "the media stayed where it was");
}

/// A reversed clip's **out edge** takes the media *before* its in
/// point.
#[test]
fn the_out_edge_of_a_reversed_clip_takes_the_media_before_it() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    reverse(&mut session, clip);
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 5 }))
        .expect("the clip is trimmed");
    assert_eq!(clock_of(&session, clip), (10, 25, 25), "the media moved backwards");
}

/// A reversed clip's out edge may not go **before the media begins**.
#[test]
fn the_out_edge_of_a_reversed_clip_stops_at_the_media() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 5);
    reverse(&mut session, clip);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 10 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("before the media begins"), "the refusal says why: {error}");
}

/// The **split** of a reversed clip keeps the content continuous: the
/// piece before the place shows the later media, and the piece after it
/// shows the earlier media.
#[test]
fn the_split_of_a_reversed_clip_keeps_the_content_continuous() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    reverse(&mut session, clip);
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": clip, "at": 15 })).expect("the clip is split");
    let new = outcome.data["new"].as_u64().expect("there is a new clip");
    assert_eq!(clock_of(&session, clip), (10, 5, 45), "the first piece shows the later media");
    assert_eq!(clock_of(&session, new), (15, 15, 30), "and the second shows the earlier media");
}

/// The **split** of a clip that plays forwards moves the media with
/// the cut, as it always did.
#[test]
fn the_split_of_a_clip_that_plays_forwards_moves_the_media() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    let outcome =
        session.execute("film.clip.split", &json!({ "clip": clip, "at": 15 })).expect("the clip is split");
    let new = outcome.data["new"].as_u64().expect("there is a new clip");
    assert_eq!(clock_of(&session, clip), (10, 5, 30), "the first piece keeps the media");
    assert_eq!(clock_of(&session, new), (15, 15, 35), "and the second has the later media");
}

/// A clip that is **not there** is refused.
#[test]
fn a_clip_that_is_not_there_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let _clip = a_clip(&mut session, track, item, 10, 20, 30);
    let error = session
        .execute("film.clip.reverse", &json!({ "clip": 1_000_000 }))
        .expect_err("the reversal is refused");
    assert!(error.to_string().contains("there is no clip"), "the refusal says why: {error}");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (item, _track) = a_timeline(&mut session, 100);
    let error =
        session.execute("film.clip.reverse", &json!({ "clip": item })).expect_err("the reversal is refused");
    assert!(error.to_string().contains("not a clip"), "the refusal says why: {error}");
}
