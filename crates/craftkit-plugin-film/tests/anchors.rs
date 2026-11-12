//! The **synchronisation** of a clip by its anchor.

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

/// The **anchor** of a clip: the frame of the media it begins at.
fn anchor_of(session: &Session, clip: u64) -> u64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["source_in"].as_u64().expect("the clip has an anchor")
}

/// The **start** of a clip on the timeline.
fn start_of(session: &Session, clip: u64) -> u64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["start"].as_u64().expect("the clip has a start")
}

/// A clip is put **in step** with another one by their **in points**.
#[test]
fn a_clip_is_put_in_step_by_its_in_points() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, item, 0, 20, 40);
    let second = a_clip(&mut session, track, item, 30, 20, 10);
    session
        .execute("film.clip.synchronise", &json!({ "clip": second, "reference": first }))
        .expect("the clip is in step");
    assert_eq!(
        anchor_of(&session, second),
        40,
        "the clip begins at the frame of the media the reference begins at"
    );
    assert_eq!(start_of(&session, second), 30, "and the clip is where it was on the timeline");
}

/// A clip may be put in step by its **out points**.
#[test]
fn a_clip_may_be_put_in_step_by_its_out_points() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, item, 0, 20, 40);
    let second = a_clip(&mut session, track, item, 30, 10, 10);
    session
        .execute("film.clip.synchronise", &json!({ "clip": second, "reference": first, "method": "out" }))
        .expect("the clip is in step");
    // The reference ends at frame 60 of the media, and so does the clip.
    assert_eq!(
        anchor_of(&session, second),
        50,
        "the clip ends at the frame of the media the reference ends at"
    );
}

/// A clip that is **longer** than the reference cannot be in step by
/// its out points.
#[test]
fn a_clip_longer_than_the_reference_cannot_be_in_step_by_its_out_points() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, item, 0, 10, 5);
    let second = a_clip(&mut session, track, item, 30, 20, 10);
    let value = session
        .execute("film.clip.synchronise", &json!({ "clip": second, "reference": first, "method": "out" }))
        .expect_err("the synchronisation is refused");
    assert!(
        value.to_string().contains("longer than the reference"),
        "the refusal says why, and it says {}",
        value
    );
}

/// A clip that is **already in step** is refused: there is nothing to
/// do.
#[test]
fn a_clip_that_is_already_in_step_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, item, 0, 20, 40);
    let second = a_clip(&mut session, track, item, 30, 20, 40);
    let value = session
        .execute("film.clip.synchronise", &json!({ "clip": second, "reference": first }))
        .expect_err("the synchronisation is refused");
    assert!(value.to_string().contains("already in step"), "the refusal says why, and it says {}", value);
}

/// A method that is **not a method** is refused.
#[test]
fn a_method_that_is_not_a_method_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, item, 0, 20, 40);
    let second = a_clip(&mut session, track, item, 30, 20, 10);
    let value = session
        .execute(
            "film.clip.synchronise",
            &json!({ "clip": second, "reference": first, "method": "by the weather" }),
        )
        .expect_err("the synchronisation is refused");
    assert!(value.to_string().contains("not a method"), "the refusal says why, and it says {}", value);
}

/// A clip cannot be in step with **itself**.
#[test]
fn a_clip_cannot_be_in_step_with_itself() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 20, 40);
    let value = session
        .execute("film.clip.synchronise", &json!({ "clip": clip, "reference": clip }))
        .expect_err("the synchronisation is refused");
    assert!(value.to_string().contains("itself"), "the refusal says why, and it says {}", value);
}

/// A clip that would play **past the end of the media** is refused.
#[test]
fn a_clip_that_would_play_past_the_end_of_the_media_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 50);
    let first = a_clip(&mut session, track, item, 0, 20, 40);
    let second = a_clip(&mut session, track, item, 30, 20, 10);
    let value = session
        .execute("film.clip.synchronise", &json!({ "clip": second, "reference": first }))
        .expect_err("the synchronisation is refused");
    assert!(value.to_string().contains("the media runs from"), "the refusal says why, and it says {}", value);
}
