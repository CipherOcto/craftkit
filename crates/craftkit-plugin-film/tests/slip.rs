//! **Slipping** a clip.
//!
//! A slip moves a clip's place in its media and leaves its place on the
//! timeline alone, and a slip that would run past the end of the media
//! is refused and says how long the media is.

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

/// The three numbers of a clip.
fn clock_of(session: &Session, clip: u64) -> (u64, u64, u64) {
    let node = session.document().node(clip).expect("the clip is there");
    let start = node.data["start"].as_u64().expect("the clip has a start");
    let duration = node.data["duration"].as_u64().expect("the clip has a duration");
    let source_in = node.data["source_in"].as_u64().expect("the clip has a source");
    (start, duration, source_in)
}

/// A slip moves the clip's place in the **media** and nothing else: the
/// clip stays where it is on the timeline and how long it is.
#[test]
fn a_slip_moves_the_media_and_not_the_timeline() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    session.execute("film.clip.slip", &json!({ "clip": clip, "delta": 5 })).expect("the clip is slipped");
    let (start, duration, source_in) = clock_of(&session, clip);
    assert_eq!((start, duration, source_in), (10, 20, 35), "the media moved and the timeline did not");
}

/// A slip the other way moves the media back.
#[test]
fn a_slip_may_go_backwards() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    session.execute("film.clip.slip", &json!({ "clip": clip, "delta": -10 })).expect("the clip is slipped");
    let (_, _, source_in) = clock_of(&session, clip);
    assert_eq!(source_in, 20, "the media moved back");
}

/// A slip that would run **past the end of the media** is refused, and
/// the refusal says how long the media is: the tree clamps it and says
/// nothing.
#[test]
fn a_slip_past_the_end_of_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    let error = session
        .execute("film.clip.slip", &json!({ "clip": clip, "delta": 60 }))
        .expect_err("the slip is refused");
    assert!(error.to_string().contains("100 frames long"), "the error says so: {error}");
    let (_, _, source_in) = clock_of(&session, clip);
    assert_eq!(source_in, 30, "and the clip is unchanged");
}

/// A slip that would begin **before the media** is refused.
#[test]
fn a_slip_before_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    let error = session
        .execute("film.clip.slip", &json!({ "clip": clip, "delta": -40 }))
        .expect_err("the slip is refused");
    assert!(error.to_string().contains("before the media"), "the error says so: {error}");
    let (_, _, source_in) = clock_of(&session, clip);
    assert_eq!(source_in, 30, "and the clip is unchanged");
}

/// A slip of **zero frames** is not a slip.
#[test]
fn a_slip_of_nothing_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    let error = session
        .execute("film.clip.slip", &json!({ "clip": clip, "delta": 0 }))
        .expect_err("the slip is refused");
    assert!(error.to_string().contains("zero"), "the error says so: {error}");
}

/// A slip counts the media through the **speed**: a clip that plays
/// twice as fast moves twice as far in the media, so that the picture
/// it shows moves the same ten frames. The tree's slip does not count
/// the speed through it.
#[test]
fn a_slip_counts_the_speed() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 2.0 })).expect("the clip is sped up");
    let (_, _, source_in) = clock_of(&session, clip);
    assert_eq!(source_in, 30, "the media the clip plays is the same");
    session.execute("film.clip.slip", &json!({ "clip": clip, "delta": 10 })).expect("the clip is slipped");
    let (_, _, source_in) = clock_of(&session, clip);
    assert_eq!(source_in, 50, "twenty media frames moved, for ten frames of picture");
}

/// The **link** is honoured: the linked clips slip together, so that the
/// picture and the sound stay in step.
#[test]
fn linked_clips_slip_together() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let other_track = outcome.data["id"].as_u64().expect("the track has an id");
    let other = a_clip(&mut session, other_track, item, 10, 20, 30);
    session.execute("film.clip.link", &json!({ "clip": clip, "with": other })).expect("the clips are linked");
    session.execute("film.clip.slip", &json!({ "clip": clip, "delta": 8 })).expect("the clips are slipped");
    assert_eq!(clock_of(&session, clip).2, 38, "the picture slipped");
    assert_eq!(clock_of(&session, other).2, 38, "and the sound slipped with it");
}

/// A linked clip that **cannot** take the slip refuses it: the tree
/// scales the slip of the group down and says nothing.
#[test]
fn a_linked_clip_that_cannot_take_the_slip_refuses_it() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    let bin = session.document().walk().find(|node| node.kind == "film.bin").expect("the bin is there").id;
    let outcome = session
        .execute(
            "film.item.add",
            &json!({ "name": "the short media", "bin": bin, "item": 2, "duration": 40 }),
        )
        .expect("the other item is added");
    let short_item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let other_track = outcome.data["id"].as_u64().expect("the track has an id");
    let other = a_clip(&mut session, other_track, short_item, 10, 20, 30);
    session.execute("film.clip.link", &json!({ "clip": clip, "with": other })).expect("the clips are linked");
    let error = session
        .execute("film.clip.slip", &json!({ "clip": clip, "delta": 15 }))
        .expect_err("the slip is refused");
    assert!(error.to_string().contains("40 frames long"), "the error says so: {error}");
    assert_eq!(clock_of(&session, clip).2, 30, "and the picture is unchanged");
    assert_eq!(clock_of(&session, other).2, 30, "and the sound is unchanged");
}

/// A **locked** track cannot be edited, and a slip is an edit.
#[test]
fn a_slip_on_a_locked_track_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let error = session
        .execute("film.clip.slip", &json!({ "clip": clip, "delta": 5 }))
        .expect_err("the slip is refused");
    assert!(error.to_string().contains("locked"), "the error says so: {error}");
}

/// A clip that is **not a clip** is refused.
#[test]
fn a_clip_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let error = session
        .execute("film.clip.slip", &json!({ "clip": track, "delta": 5 }))
        .expect_err("a track is not a clip");
    assert!(error.to_string().contains("not a clip"), "the error says so: {error}");
    let _ = item;
}

/// A **held** clip may be slipped: it plays its material up to the
/// frame it holds, so the slip may not begin after that frame.
#[test]
fn a_held_clip_may_be_slipped() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 40 })).expect("the clip holds a frame");
    session.execute("film.clip.slip", &json!({ "clip": clip, "delta": 5 })).expect("the clip is slipped");
    assert_eq!(clock_of(&session, clip).2, 35, "the media moved and the hold stayed");
    let error = session
        .execute("film.clip.slip", &json!({ "clip": clip, "delta": 20 }))
        .expect_err("the slip is refused");
    assert!(error.to_string().contains("the frame it holds"), "the error says so: {error}");
}

/// The place in the media is **saved** with the clip.
#[test]
fn the_place_in_the_media_is_saved() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20, 30);
    session.execute("film.clip.slip", &json!({ "clip": clip, "delta": 5 })).expect("the clip is slipped");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(clip).expect("the clip is there");
    assert_eq!(node.data["source_in"].as_u64(), Some(35), "the clip still shows that media");
}
