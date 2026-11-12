//! The **speed** of a clip.
//!
//! A clip may play its media faster or slower than it is. The speed
//! joins the two clocks: the media a clip plays is its length
//! multiplied by its speed.

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

/// The **speed** of a clip.
fn speed_of(session: &Session, clip: u64) -> f64 {
    let node = session.document().node(clip).expect("the clip is there");
    node.data["speed"].as_f64().expect("the clip has a speed")
}

/// A clip that plays twice as fast is **half as long**, and it plays
/// the same media.
#[test]
fn a_faster_clip_is_shorter() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 2.0 })).expect("the speed is set");
    assert_eq!(clock_of(&session, clip), (0, 25, 0), "the clip is half as long");
    assert_eq!(speed_of(&session, clip), 2.0, "the speed is kept");
}

/// A clip that plays **half as fast** is twice as long, and it plays
/// the same media.
#[test]
fn a_slower_clip_is_longer() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 0.5 })).expect("the speed is set");
    assert_eq!(clock_of(&session, clip), (0, 100, 0), "the clip is twice as long");
}

/// The **media** a clip plays is the same material whatever the speed.
#[test]
fn the_media_is_the_same_material() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 4.0 })).expect("the speed is set");
    let (_, duration, source_in) = clock_of(&session, clip);
    assert_eq!(duration, 13, "the clip is a quarter as long");
    assert_eq!(source_in, 0, "and it begins at the beginning of the media");
}

/// A speed that is **not** a speed is refused.
#[test]
fn a_speed_that_is_not_a_speed_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    let outcome = session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 0.0 }));
    assert!(outcome.is_err(), "the speed of nothing is refused");
    let outcome = session.execute("film.clip.speed", &json!({ "clip": clip, "speed": -2.0 }));
    assert!(outcome.is_err(), "the negative speed is refused");
}

/// A **locked** track refuses a speed.
#[test]
fn a_locked_track_refuses_a_speed() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let outcome = session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 2.0 }));
    assert!(outcome.is_err(), "the locked track refuses the speed");
}

/// A clip that would run into the clip **after** it is refused.
#[test]
fn a_clip_that_would_overlap_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 200);
    let first = a_clip(&mut session, track, item, 0, 50);
    a_clip(&mut session, track, item, 50, 50);
    let outcome = session.execute("film.clip.speed", &json!({ "clip": first, "speed": 0.5 }));
    assert!(outcome.is_err(), "the clip that would run into its neighbour is refused");
}

/// The **trim** of a fast clip moves the media by the speed's worth.
#[test]
fn the_trim_moves_the_media_by_the_speed() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 2.0 })).expect("the speed is set");
    session
        .execute("film.clip.trim", &json!({ "clip": clip, "edge": "in", "delta": 5 }))
        .expect("the clip is trimmed");
    let (start, duration, source_in) = clock_of(&session, clip);
    assert_eq!(start, 5, "the clip moves on the timeline");
    assert_eq!(duration, 20, "and it is five frames shorter");
    assert_eq!(source_in, 10, "and it begins ten frames later in the media");
}

/// The **split** of a fast clip cuts the media at the speed's worth.
#[test]
fn the_split_cuts_the_media_at_the_speed() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 0, 50);
    session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 2.0 })).expect("the speed is set");
    session.execute("film.clip.split", &json!({ "clip": clip, "at": 10 })).expect("the clip is split");
    let node = session.document().node(track).expect("the track is there");
    let clips: Vec<u64> =
        node.children.iter().filter(|child| child.kind == "film.clip").map(|child| child.id).collect();
    assert_eq!(clips.len(), 2, "the clip is in two");
    let right = clips
        .iter()
        .find(|clip| {
            let node = session.document().node(**clip).expect("the clip is there");
            node.data["start"].as_u64() == Some(10)
        })
        .copied();
    let right = right.expect("the second half is there");
    assert_eq!(clock_of(&session, right).2, 20, "the second half begins twenty frames into the media");
}

/// The media a clip plays may not run **past the end** of the media.
///
/// The speed keeps the media the same, but the **rounding** of the
/// new length may make the clip play a frame more than it did: fifty
/// frames of media at three times the speed is seventeen frames long,
/// and seventeen frames at three times the speed is fifty-one frames
/// of media. The tree does not check this; here it is refused.
#[test]
fn the_media_may_not_run_past_its_end() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 50);
    let clip = a_clip(&mut session, track, item, 0, 50);
    let outcome = session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 3.0 }));
    assert!(outcome.is_err(), "the clip that would run past the media is refused");
}
