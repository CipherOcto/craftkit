//! The **frame hold** of a clip.
//!
//! A clip may hold a frame: it plays its media up to a point in the
//! media and then holds that frame to the end of the clip. The hold is
//! a **source** frame — a frame of the media — and not a frame of the
//! timeline.

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
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": start,
                "duration": duration,
                "source_in": 0,
            }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The **three numbers** of a clip: where it is, how long it is and
/// where in the media it begins.
fn clock_of(session: &Session, clip: u64) -> (u64, u64, u64) {
    let node = session.document().node(clip).expect("the clip is there");
    let start = node.data["start"].as_u64().expect("the clip has a start");
    let duration = node.data["duration"].as_u64().expect("the clip has a duration");
    let source_in = node.data["source_in"].as_u64().expect("the clip has a source");
    (start, duration, source_in)
}

/// The **frame** a clip holds, if it holds one.
fn hold_of(session: &Session, clip: u64) -> Option<u64> {
    let node = session.document().node(clip).expect("the clip is there");
    node.data.get("frame_hold").and_then(|hold| hold.as_u64())
}

/// A clip may **hold** a frame of the media it plays.
#[test]
fn a_clip_may_hold_a_frame() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 20 })).expect("the hold is set");
    assert_eq!(hold_of(&session, clip), Some(20), "the clip holds the frame at 20 of the media");
    assert_eq!(clock_of(&session, clip), (0, 50, 0), "and the clip has not moved");
}

/// The hold is a **source** frame: it is a frame of the media and not a
/// frame of the timeline.
#[test]
fn the_hold_is_a_frame_of_the_media() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 30, 50);
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 40 })).expect("the hold is set");
    assert_eq!(hold_of(&session, clip), Some(40), "the frame is the frame of the media");
}

/// A hold **outside** the media the clip plays is refused.
#[test]
fn a_hold_outside_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    let outcome = session.execute("film.clip.hold", &json!({ "clip": clip, "at": 200 }));
    assert!(outcome.is_err(), "the frame 200 is not in the media");
    assert_eq!(hold_of(&session, clip), None, "and the clip holds nothing");
}

/// A hold **past** the media the clip plays is refused: the clip plays
/// fifty frames of the media and holds one of them.
#[test]
fn a_hold_past_the_material_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    let outcome = session.execute("film.clip.hold", &json!({ "clip": clip, "at": 60 }));
    assert!(outcome.is_err(), "the frame 60 is past the material of the clip");
}

/// A **locked** track refuses a hold.
#[test]
fn a_locked_track_refuses_a_hold() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let outcome = session.execute("film.clip.hold", &json!({ "clip": clip, "at": 20 }));
    assert!(outcome.is_err(), "the locked track refuses the hold");
}

/// The hold may be **taken away**, and a clip that holds nothing cannot
/// have its hold taken twice.
#[test]
fn the_hold_may_be_taken_away() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 20 })).expect("the hold is set");
    session.execute("film.clip.hold.delete", &json!({ "clip": clip })).expect("the hold is taken away");
    assert_eq!(hold_of(&session, clip), None, "the clip holds nothing");
    let outcome = session.execute("film.clip.hold.delete", &json!({ "clip": clip }));
    assert!(outcome.is_err(), "a clip that holds nothing has no hold to take away");
}

/// The tree's rule: a held clip is a clip whose material is **shorter**
/// than the clip, and the trim honours it.
#[test]
fn a_held_clip_has_shorter_material_than_the_clip() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 10 })).expect("the hold is set");
    assert_eq!(clock_of(&session, clip).1, 50, "the clip is still fifty frames long");
    assert_eq!(hold_of(&session, clip), Some(10), "but it holds the frame at 10");
}

/// A held clip has no **head handle**: the media before its in point is
/// not there to be taken, so a trim of the in edge moves the clip and
/// leaves the media where it is.
#[test]
fn the_in_edge_of_a_held_clip_moves_the_clip_not_the_media() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 50);
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 20 })).expect("the hold is set");
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "in", "delta": 10 }))
        .expect("the clip is trimmed");
    assert_eq!(clock_of(&session, clip), (20, 40, 0), "the clip moved and the media did not");
}

/// The **out edge** of a held clip changes the clip and not the media
/// either, because the media ends at the hold.
#[test]
fn the_out_edge_of_a_held_clip_changes_only_the_clip() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.hold", &json!({ "clip": clip, "at": 10 })).expect("the hold is set");
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "out", "delta": 10 }))
        .expect("the clip is trimmed");
    assert_eq!(clock_of(&session, clip), (0, 60, 0), "the clip is longer and the media is the same");
}
