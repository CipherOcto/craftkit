//! **Rolling** a cut.
//!
//! A roll moves the cut between two clips: the clip before it gets
//! longer and the clip after it gets shorter, and the media of both
//! follows the cut.

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

/// A roll moves the **cut**: the clip before it gets longer and the
/// clip after it gets shorter and moves along.
#[test]
fn a_roll_moves_the_cut() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 5 }))
        .expect("the cut is rolled");
    assert_eq!(clock_of(&session, left), (0, 25, 0), "the clip before the cut got longer");
    assert_eq!(clock_of(&session, right), (25, 15, 5), "the clip after it got shorter and moved");
}

/// The media follows the cut: the clip after the cut shows later
/// media, and the media of the clip before it is where it was.
#[test]
fn the_media_follows_the_cut() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 10);
    let right = a_clip(&mut session, track, item, 20, 20, 30);
    session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 4 }))
        .expect("the cut is rolled");
    assert_eq!(clock_of(&session, left).2, 10, "the media of the first clip is where it was");
    assert_eq!(clock_of(&session, right).2, 34, "and the second clip shows later media");
}

/// A roll the other way moves the cut back.
#[test]
fn a_roll_may_go_backwards() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 10);
    session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": -5 }))
        .expect("the cut is rolled");
    assert_eq!(clock_of(&session, left), (0, 15, 0), "the clip before the cut got shorter");
    assert_eq!(clock_of(&session, right), (15, 25, 5), "and the clip after it got longer");
}

/// A roll the other way that would take the clip after the cut **before
/// the media begins** is refused, and says how far before it would be.
#[test]
fn a_roll_before_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    let error = session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": -5 }))
        .expect_err("the roll is refused");
    assert!(error.to_string().contains("before the media begins"), "the error says so: {error}");
    assert_eq!(clock_of(&session, right), (20, 20, 0), "and the clips are unchanged");
}

/// A roll that would make a clip **shorter than a frame** is refused.
#[test]
fn a_roll_past_the_end_of_a_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    let error = session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": -25 }))
        .expect_err("the roll is refused");
    assert!(error.to_string().contains("at least one frame"), "the error says so: {error}");
    assert_eq!(clock_of(&session, left), (0, 20, 0), "and the clips are unchanged");
}

/// A roll that would run the first clip **past the end of its media**
/// is refused, and the refusal says how long the media is.
#[test]
fn a_roll_past_the_end_of_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    let error = session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 90 }))
        .expect_err("the roll is refused");
    assert!(error.to_string().contains("100 frames long"), "the error says so: {error}");
    assert_eq!(clock_of(&session, left), (0, 20, 0), "and the clips are unchanged");
}

/// Two clips that are **not next to each other** have no cut to roll.
#[test]
fn a_roll_of_clips_that_are_not_next_to_each_other_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 30, 20, 0);
    let error = session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 5 }))
        .expect_err("the roll is refused");
    assert!(error.to_string().contains("ends at frame"), "the error says so: {error}");
}

/// Two clips on **different tracks** have no cut between them.
#[test]
fn a_roll_of_clips_on_different_tracks_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let other = outcome.data["id"].as_u64().expect("the track has an id");
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, other, item, 20, 20, 0);
    let error = session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 5 }))
        .expect_err("the roll is refused");
    assert!(error.to_string().contains("not on the track"), "the error says so: {error}");
}

/// A **locked** track cannot be edited, and a roll is an edit.
#[test]
fn a_roll_on_a_locked_track_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let error = session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 5 }))
        .expect_err("the roll is refused");
    assert!(error.to_string().contains("locked"), "the error says so: {error}");
}

/// The **link** is honoured: the linked cut rolls with the cut that was
/// named, so that the picture and the sound stay in step.
#[test]
fn a_linked_cut_rolls_too() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the track is added");
    let other = outcome.data["id"].as_u64().expect("the track has an id");
    let sound_left = a_clip(&mut session, other, item, 0, 20, 0);
    let sound_right = a_clip(&mut session, other, item, 20, 20, 0);
    session
        .execute("film.clip.link", &json!({ "clip": left, "with": sound_left }))
        .expect("the first pair is linked");
    session
        .execute("film.clip.link", &json!({ "clip": right, "with": sound_right }))
        .expect("the second pair is linked");
    session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 5 }))
        .expect("the cut is rolled");
    assert_eq!(clock_of(&session, left), (0, 25, 0), "the picture rolled");
    assert_eq!(clock_of(&session, sound_left), (0, 25, 0), "and the sound rolled with it");
    assert_eq!(clock_of(&session, sound_right), (25, 15, 5), "and the sound of the second clip too");
}

/// A **transition** over the cut follows the cut.
#[test]
fn a_transition_over_the_cut_follows_it() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    let outcome = session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": left, "to": right, "duration": 4, "align": "center" }),
        )
        .expect("the transition is added");
    let transition = outcome.data["id"].as_u64().expect("the transition has an id");
    let start_of = |session: &Session| {
        session.document().node(transition).expect("the transition is there").data["start"]
            .as_u64()
            .expect("the transition has a start")
    };
    let before = start_of(&session);
    session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 6 }))
        .expect("the cut is rolled");
    assert_eq!(start_of(&session), before + 6, "the transition moved with the cut");
}

/// A roll of **zero frames** is not a roll.
#[test]
fn a_roll_of_nothing_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    let error = session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 0 }))
        .expect_err("the roll is refused");
    assert!(error.to_string().contains("zero"), "the error says so: {error}");
}

/// A node that is **not a clip** is refused.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    let error = session
        .execute("film.clip.roll", &json!({ "left": track, "right": right, "delta": 5 }))
        .expect_err("a track is not a clip");
    assert!(error.to_string().contains("not a clip"), "the error says so: {error}");
    let _ = (item, left);
}

/// The cut is **saved** with the clips.
#[test]
fn the_roll_is_saved() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let left = a_clip(&mut session, track, item, 0, 20, 0);
    let right = a_clip(&mut session, track, item, 20, 20, 0);
    session
        .execute("film.clip.roll", &json!({ "left": left, "right": right, "delta": 5 }))
        .expect("the cut is rolled");
    let json = session.document().to_json();
    let back = craftkit_sdk::Document::from_json(&json).expect("the document is read back");
    let node = back.node(right).expect("the clip is there");
    assert_eq!(node.data["start"].as_u64(), Some(25), "the clip is where the roll put it");
    assert_eq!(node.data["duration"].as_u64(), Some(15), "and it is that long");
}
