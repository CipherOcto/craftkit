//! **Trimming** a clip.
//!
//! A trim moves one of the two edges of a clip, and the tree refuses a
//! trim that would cross a bound and says which bound.

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

/// A **bin**, an **item** in it, a **track** in the item, and the
/// **clips** on the track.
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

/// The three numbers of a clip.
fn clock_of(session: &Session, clip: u64) -> (u64, u64, u64) {
    let node = session.document().node(clip).expect("the clip is there");
    let start = node.data["start"].as_u64().expect("the clip has a start");
    let duration = node.data["duration"].as_u64().expect("the clip has a duration");
    let source_in = node.data["source_in"].as_u64().expect("the clip has a source");
    (start, duration, source_in)
}

/// The **in** edge is the head: the clip gets shorter, the media moves
/// on, and the clip moves with it.
#[test]
fn the_in_edge_is_trimmed() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "in", "delta": 5 }))
        .expect("the clip is trimmed");
    assert_eq!(clock_of(&session, clip), (15, 15, 35), "the three clocks moved");
}

/// The **out** edge is the tail: the clip gets longer and stays where
/// it is.
#[test]
fn the_out_edge_is_trimmed() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 0);
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 5 }))
        .expect("the clip is trimmed");
    assert_eq!(clock_of(&session, clip), (10, 25, 0), "the tail grew and the rest stayed");
}

/// A clip is **at least one frame** long.
#[test]
fn a_clip_that_would_be_too_short_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 0);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": -20 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("at least one frame"), "the error says why: {error}");
    assert_eq!(clock_of(&session, clip), (10, 20, 0), "and the clip is as it was");
}

/// A clip may not run **past the end of the media**.
#[test]
fn a_clip_that_would_run_past_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 90);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 5 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("the media is 100 frames long"), "the error says why: {error}");
    assert_eq!(clock_of(&session, clip), (10, 20, 90), "and the clip is as it was");
}

/// A clip may not begin **before the media**.
#[test]
fn a_clip_that_would_begin_before_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 5);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "in", "delta": -10 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("before the media begins"), "the error says why: {error}");
    assert_eq!(clock_of(&session, clip), (10, 20, 5), "and the clip is as it was");
}

/// A clip may not **overlap** the clip before it.
#[test]
fn a_clip_that_would_overlap_the_one_before_it_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, item, 0, 20, 0);
    let clip = a_clip(&mut session, track, item, 30, 10, 30);
    // The head is pulled back to the left, and it would land on the
    // clip that is before it.
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "in", "delta": -15 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("the clip before it ends at frame 20"), "the error says why: {error}");
    assert_eq!(clock_of(&session, clip), (30, 10, 30), "and the clip is as it was");
}

/// A clip may not **overlap** the clip after it.
#[test]
fn a_clip_that_would_overlap_the_one_after_it_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 20, 0);
    a_clip(&mut session, track, item, 40, 10, 0);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 30 }))
        .expect_err("the trim is refused");
    assert!(
        error.to_string().contains("the clip after it starts at frame 40"),
        "the error says why: {error}"
    );
    assert_eq!(clock_of(&session, clip), (0, 20, 0), "and the clip is as it was");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_item, track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": track, "edge": "in", "delta": 5 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("not a clip"), "the error says why: {error}");
}

/// An edge that is **not an edge** is refused.
#[test]
fn an_edge_that_is_not_an_edge_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 20, 0);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "middle", "delta": 5 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("an edge is `in` or `out`"), "the error says why: {error}");
}

/// A delta that is **not a whole number** is refused.
#[test]
fn a_delta_that_is_not_a_whole_number_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 20, 0);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 1.5 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("a whole number"), "the error says why: {error}");
}

/// A trim of **zero frames** is refused: nothing would change.
#[test]
fn a_trim_of_zero_frames_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 20, 0);
    let error = session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 0 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("not a trim"), "the error says why: {error}");
}

/// A node that is **not there** is refused.
#[test]
fn a_node_that_is_not_there_is_refused() {
    let mut session = session();
    let error = session
        .execute("film.clip.trim", &json!({ "clip": 1_000_000, "edge": "out", "delta": 5 }))
        .expect_err("the trim is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
