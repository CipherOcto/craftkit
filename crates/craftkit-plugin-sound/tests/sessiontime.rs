//! The **time of the session**: inserting silence into it, and cutting
//! time out of it.
//!
//! Both are changes to the whole of the session's timeline: every
//! track moves, every lane moves and every marker moves.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_sound::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the sound battery.
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

/// A session with a track, a clip, a lane and a marker.
///
/// The clip runs from zero to a thousand, the lane has a breakpoint at
/// eight hundred and the marker is at nine hundred.
fn a_session(session: &mut Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 1_000 }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    session
        .execute("sound.lane.add", &json!({ "track": track, "param": "volume", "points": [[800, -6.0]] }))
        .expect("the lane is added");
    session
        .execute("sound.marker.add", &json!({ "name": "the marker", "start": 900, "end": 950 }))
        .expect("the marker is added");
    clip
}

/// The clip's start.
fn clip_start(session: &Session, clip: u64) -> i64 {
    session.document().node(clip).expect("the clip is there").data["start"]
        .as_i64()
        .expect("the start is there")
}

/// The lane's breakpoints, as `(at, value)`.
fn lane_points(session: &Session) -> Vec<(i64, f64)> {
    let node = session.document().walk().find(|node| node.kind == "sound.lane").expect("the lane is there");
    node.data["points"]
        .as_array()
        .expect("the points are there")
        .iter()
        .map(|point| {
            (point[0].as_i64().expect("the place is there"), point[1].as_f64().expect("the value is there"))
        })
        .collect()
}

/// The marker's start and end.
fn marker_span(session: &Session) -> (i64, i64) {
    let node =
        session.document().walk().find(|node| node.kind == "sound.marker").expect("the marker is there");
    (
        node.data["start"].as_i64().expect("the start is there"),
        node.data["end"].as_i64().expect("the end is there"),
    )
}

/// The silence goes in at a place and everything after it moves later:
/// the clip, the lane's breakpoint and the marker.
#[test]
fn the_silence_goes_in_and_everything_after_it_moves() {
    let mut session = session();
    let clip = a_session(&mut session);
    session.execute("sound.time.insert", &json!({ "at": 500, "length": 200 })).expect("the time is inserted");
    // The clip ran from zero to a thousand, so it starts before the
    // place and does not move.
    assert_eq!(clip_start(&session, clip), 0, "the clip before the place stays");
    // The lane's breakpoint at eight hundred is after the place, so it
    // moves later by the length.
    assert_eq!(lane_points(&session), vec![(1_000, -6.0)], "the breakpoint moves later");
    // And the marker at nine hundred moves later, and it keeps its
    // span.
    assert_eq!(marker_span(&session), (1_100, 1_150), "the marker moves later");
}

/// A clip that starts **after** the place moves later as well.
#[test]
fn a_clip_after_the_place_moves_later() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 600, "length": 400 }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    session.execute("sound.time.insert", &json!({ "at": 500, "length": 200 })).expect("the time is inserted");
    assert_eq!(clip_start(&session, clip), 800, "the clip moves later");
}

/// The time is cut out of a range and everything after it moves
/// earlier: the clip, the lane's breakpoint and the marker.
#[test]
fn the_time_is_cut_out_and_everything_after_it_moves() {
    let mut session = session();
    a_session(&mut session);
    session.execute("sound.time.cut", &json!({ "start": 400, "end": 600 })).expect("the time is cut");
    // The lane's breakpoint at eight hundred is after the range, so it
    // moves earlier by the length of the range.
    assert_eq!(lane_points(&session), vec![(600, -6.0)], "the breakpoint moves earlier");
    // And the marker at nine hundred moves earlier, and it keeps its
    // span.
    assert_eq!(marker_span(&session), (700, 750), "the marker moves earlier");
}

/// A clip that is **inside** the range is thrown away.
#[test]
fn a_clip_inside_the_range_is_thrown_away() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 500, "length": 100 }))
        .expect("the clip is added");
    session.execute("sound.time.cut", &json!({ "start": 400, "end": 700 })).expect("the time is cut");
    let clips = session.document().walk().filter(|node| node.kind == "sound.clip").count();
    assert_eq!(clips, 0, "the clip is gone");
}

/// A **locked** clip does not move: a lock says the clip stays where
/// it is.
#[test]
fn a_locked_clip_does_not_move() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 600, "length": 400 }))
        .expect("the clip is added");
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    session.execute("sound.clip.lock.time", &json!({ "clip": clip })).expect("the clip is locked");
    session.execute("sound.time.insert", &json!({ "at": 500, "length": 200 })).expect("the time is inserted");
    assert_eq!(clip_start(&session, clip), 600, "the locked clip stays where it is");
}

/// A range that is **empty** is refused: there is no time to cut.
#[test]
fn an_empty_range_is_refused() {
    let mut session = session();
    a_session(&mut session);
    let refused = session.execute("sound.time.cut", &json!({ "start": 500, "end": 500 }));
    assert!(refused.is_err(), "the range is empty");
    let refused = session.execute("sound.time.insert", &json!({ "at": 500, "length": 0 }));
    assert!(refused.is_err(), "and there is no silence in nothing");
}
