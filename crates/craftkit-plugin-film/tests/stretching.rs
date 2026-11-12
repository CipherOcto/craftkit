//! **Stretching** a clip.
//!
//! A stretch changes the speed of a clip by dragging one of its edges,
//! and the clip plays the same material.

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

/// The three numbers of a clip and the speed it plays at.
fn of(session: &Session, clip: u64) -> (u64, u64, u64, f64) {
    let node = session.document().node(clip).expect("the clip is there");
    let start = node.data["start"].as_u64().expect("the clip has a start");
    let duration = node.data["duration"].as_u64().expect("the clip has a duration");
    let source_in = node.data["source_in"].as_u64().expect("the clip has a source");
    let speed = node.data.get("speed").and_then(|speed| speed.as_f64()).unwrap_or(1.0);
    (start, duration, source_in, speed)
}

/// The **out** edge is stretched: the clip gets longer and plays its
/// media more slowly, and it plays the **same material**.
#[test]
fn the_out_edge_is_stretched() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "out", "delta": 20 }))
        .expect("the clip is stretched");
    let (start, duration, source_in, speed) = of(&session, clip);
    assert_eq!(start, 0, "the clip stayed where it was");
    assert_eq!(duration, 40, "and it got longer");
    assert_eq!(source_in, 10, "and it plays the same material");
    assert!((speed - 0.5).abs() < 1e-9, "and it plays it half as fast: {speed}");
}

/// The **in** edge is stretched: the clip gets shorter and plays its
/// media faster, and it starts later and ends where it did.
#[test]
fn the_in_edge_is_stretched() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "in", "delta": 10 }))
        .expect("the clip is stretched");
    let (start, duration, source_in, speed) = of(&session, clip);
    assert_eq!(start, 10, "the clip starts later");
    assert_eq!(duration, 10, "and it got shorter");
    assert_eq!(source_in, 10, "and it plays the same material");
    assert!((speed - 2.0).abs() < 1e-9, "and it plays it twice as fast: {speed}");
}

/// The **material** is the same: the media the clip plays is its length
/// multiplied by its speed, and the stretch does not change it.
#[test]
fn the_material_is_the_same() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    let (_, _, _, before) = of(&session, clip);
    session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "out", "delta": 10 }))
        .expect("the clip is stretched");
    let (_, duration, _, speed) = of(&session, clip);
    let after = duration as f64 * speed;
    assert!((after - 20.0 * before).abs() < 1e-9, "the material is the same: {after}");
}

/// A stretch that would make a clip **shorter than a frame** is
/// refused.
#[test]
fn a_stretch_past_the_end_of_a_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "in", "delta": 25 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("at least one frame"), "the error says so: {error}");
    assert_eq!(of(&session, clip).1, 20, "and the clip is unchanged");
}

/// A stretch of the out edge that would run the clip **into the clip
/// after it** is refused.
#[test]
fn a_stretch_into_the_next_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    let next = a_clip(&mut session, track, item, 20, 20, 30);
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "out", "delta": 10 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("the clip after it starts at frame 20"), "the error says so: {error}");
    assert_eq!(of(&session, clip).1, 20, "and the clip is unchanged");
    assert_eq!(of(&session, next).0, 20, "and the clip after it is unchanged");
}

/// A stretch of the in edge that would begin the clip **before the clip
/// before it ends** is refused.
#[test]
fn a_stretch_before_the_previous_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let before = a_clip(&mut session, track, item, 0, 20, 10);
    let clip = a_clip(&mut session, track, item, 20, 20, 30);
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "in", "delta": -10 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("the clip before it ends at frame 20"), "the error says so: {error}");
    assert_eq!(of(&session, clip).0, 20, "and the clip is unchanged");
    assert_eq!(of(&session, before).1, 20, "and the clip before it is unchanged");
}

/// A stretch that would start the clip **before the timeline** is
/// refused.
#[test]
fn a_stretch_before_the_timeline_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 5, 20, 10);
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "in", "delta": -10 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("the timeline begins at frame 0"), "the error says so: {error}");
}

/// A clip that **holds a frame** may not be stretched: the material it
/// plays is shorter than the clip.
#[test]
fn a_held_clip_cannot_be_stretched() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 15 })).expect("the frame is held");
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "out", "delta": 10 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("holds a frame"), "the error says so: {error}");
}

/// A clip on a **locked** track may not be stretched.
#[test]
fn a_stretch_on_a_locked_track_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "out", "delta": 10 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("is locked"), "the error says so: {error}");
}

/// The **link** is honoured: the linked clips stretch together, so that
/// the picture and the sound stay in step.
#[test]
fn a_linked_clip_stretches_too() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let sound_track = outcome.data["id"].as_u64().expect("the track has an id");
    let picture = a_clip(&mut session, track, item, 0, 20, 10);
    let sound = a_clip(&mut session, sound_track, item, 0, 20, 10);
    session
        .execute("film.clip.link", &json!({ "clip": picture, "with": sound }))
        .expect("the clips are linked");
    session
        .execute("film.clip.stretch", &json!({ "clip": picture, "edge": "out", "delta": 20 }))
        .expect("the clip is stretched");
    assert_eq!(of(&session, sound).1, 40, "the linked clip stretched too");
}

/// A linked clip that **cannot** take the stretch refuses the whole
/// stretch: the tree scales the group down and says nothing.
#[test]
fn a_linked_clip_that_cannot_stretch_refuses_it() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let sound_track = outcome.data["id"].as_u64().expect("the track has an id");
    let picture = a_clip(&mut session, track, item, 0, 20, 10);
    let sound = a_clip(&mut session, sound_track, item, 0, 20, 10);
    session
        .execute("film.clip.link", &json!({ "clip": picture, "with": sound }))
        .expect("the clips are linked");
    // The sound has a clip after it, so it cannot be stretched as far
    // as the picture can.
    let _after = a_clip(&mut session, sound_track, item, 20, 20, 30);
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": picture, "edge": "out", "delta": 20 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("the clip after it starts at frame 20"), "the error says so: {error}");
    assert_eq!(of(&session, picture).1, 20, "and the clips are unchanged");
    assert_eq!(of(&session, sound).1, 20, "and the clips are unchanged");
}

/// A stretch of **nothing** is refused.
#[test]
fn a_stretch_of_nothing_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "out", "delta": 0 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("zero frames"), "the error says so: {error}");
}

/// An edge that is **not an edge** is refused.
#[test]
fn an_edge_that_is_not_an_edge_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "middle", "delta": 5 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("is not an edge"), "the error says so: {error}");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (_item, track) = a_timeline(&mut session, 200);
    let error = session
        .execute("film.clip.stretch", &json!({ "clip": track, "edge": "out", "delta": 5 }))
        .expect_err("the stretch is refused");
    assert!(error.to_string().contains("not a clip"), "the error says so: {error}");
}

/// The stretch is **saved**: it is in the document after it is done.
#[test]
fn the_stretch_is_saved() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 20, 10);
    session
        .execute("film.clip.stretch", &json!({ "clip": clip, "edge": "out", "delta": 20 }))
        .expect("the clip is stretched");
    let value = session.document().to_json();
    let saved = craftkit_sdk::Document::from_json(&value).expect("the document is read back");
    let node = saved.node(clip).expect("the clip is in the document");
    assert_eq!(node.data["duration"].as_u64(), Some(40), "and it is how long it was stretched");
    assert!(
        (node.data["speed"].as_f64().expect("the speed is there") - 0.5).abs() < 1e-9,
        "and it plays at the speed it was stretched to"
    );
}
