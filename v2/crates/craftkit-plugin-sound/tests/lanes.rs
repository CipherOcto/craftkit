//! The **editing of a lane**: the breakpoints of a curve, and the
//! curve they make.

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
fn new_session() -> Session {
    let ecosystem = ecosystem();
    Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    )
}

/// A session with a track and a lane of that parameter on it.
fn a_lane(session: &mut Session, param: &str) -> (u64, u64) {
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session.execute("sound.lane.add", &json!({ "track": track, "param": param })).expect("the lane is added");
    let lane =
        session.document().walk().find(|node| node.kind == "sound.lane").expect("the lane is there").id;
    (track, lane)
}

/// The breakpoints of the lane, as `(at, value)`.
fn points_of(session: &Session, lane: u64) -> Vec<(i64, f64)> {
    let node = session.document().node(lane).expect("the lane is there");
    node.data["points"]
        .as_array()
        .expect("the points are there")
        .iter()
        .map(|point| (point[0].as_i64().unwrap_or(0), point[1].as_f64().unwrap_or(0.0)))
        .collect()
}

/// A **breakpoint** is set on a lane, and the points are in order of
/// their time.
#[test]
fn a_breakpoint_is_set() {
    let mut session = new_session();
    let (_, lane) = a_lane(&mut session, "volume");
    session
        .execute("sound.lane.point.set", &json!({ "lane": lane, "at": 480, "value": -6.0 }))
        .expect("the breakpoint is set");
    session
        .execute("sound.lane.point.set", &json!({ "lane": lane, "at": 240, "value": -3.0 }))
        .expect("the other breakpoint is set");
    assert_eq!(points_of(&session, lane), [(240, -3.0), (480, -6.0)], "the points are in order");
}

/// A breakpoint that is **already there** takes the new value.
#[test]
fn a_breakpoint_that_is_there_takes_the_new_value() {
    let mut session = new_session();
    let (_, lane) = a_lane(&mut session, "volume");
    session
        .execute("sound.lane.point.set", &json!({ "lane": lane, "at": 480, "value": -6.0 }))
        .expect("the breakpoint is set");
    session
        .execute("sound.lane.point.set", &json!({ "lane": lane, "at": 480, "value": -12.0 }))
        .expect("the breakpoint is set again");
    assert_eq!(points_of(&session, lane), [(480, -12.0)], "the value is the new one");
}

/// A breakpoint is **taken off** a lane, and a time that has none is
/// refused.
#[test]
fn a_breakpoint_is_taken_off() {
    let mut session = new_session();
    let (_, lane) = a_lane(&mut session, "volume");
    session
        .execute("sound.lane.point.set", &json!({ "lane": lane, "at": 480, "value": -6.0 }))
        .expect("the breakpoint is set");
    session
        .execute("sound.lane.point.remove", &json!({ "lane": lane, "at": 480 }))
        .expect("the breakpoint is taken off");
    assert!(points_of(&session, lane).is_empty(), "the lane is bare");
    let refused = session.execute("sound.lane.point.remove", &json!({ "lane": lane, "at": 480 }));
    assert!(refused.is_err(), "there is no breakpoint there to take away");
}

/// A lane is **cleared**: all of it, or a range of it.
#[test]
fn a_lane_is_cleared() {
    let mut session = new_session();
    let (_, lane) = a_lane(&mut session, "volume");
    for at in [0, 240, 480, 960] {
        session
            .execute("sound.lane.point.set", &json!({ "lane": lane, "at": at, "value": -6.0 }))
            .expect("the breakpoint is set");
    }
    let cleared = session
        .execute("sound.lane.clear", &json!({ "lane": lane, "start": 200, "end": 500 }))
        .expect("the lane is cleared");
    assert_eq!(cleared.data["removed"], 2, "the two in the range went");
    assert_eq!(points_of(&session, lane), [(0, -6.0), (960, -6.0)], "and the rest stayed");
    let cleared = session.execute("sound.lane.clear", &json!({ "lane": lane })).expect("the lane is cleared");
    assert_eq!(cleared.data["removed"], 2, "and the rest went too");
    assert!(points_of(&session, lane).is_empty(), "the lane is bare");
}

/// A value is **written** over a range of a lane: the lane is flat
/// over the range, and the curve outside it is kept.
#[test]
fn a_value_is_written_over_a_range() {
    let mut session = new_session();
    let (_, lane) = a_lane(&mut session, "volume");
    session
        .execute("sound.lane.point.set", &json!({ "lane": lane, "at": 0, "value": 0.0 }))
        .expect("the first breakpoint is set");
    session
        .execute("sound.lane.point.set", &json!({ "lane": lane, "at": 1000, "value": 0.0 }))
        .expect("the last breakpoint is set");
    session
        .execute("sound.lane.write", &json!({ "lane": lane, "start": 200, "end": 600, "value": -20.0 }))
        .expect("the value is written");
    let points = points_of(&session, lane);
    assert!(
        points.contains(&(200, -20.0)) && points.contains(&(600, -20.0)),
        "the range is flat: {points:?}"
    );
    assert!(points.contains(&(0, 0.0)), "and the curve before it is kept: {points:?}");
    assert!(points.contains(&(1000, 0.0)), "and the curve after it: {points:?}");
}

/// A lane is **thinned**: the breakpoints the curve does not need go,
/// and the ones it needs stay.
#[test]
fn a_lane_is_thinned() {
    let mut session = new_session();
    let (_, lane) = a_lane(&mut session, "volume");
    // A straight line: every breakpoint between the ends is one the
    // curve does not need.
    for at in [0, 100, 200, 300, 400] {
        session
            .execute("sound.lane.point.set", &json!({ "lane": lane, "at": at, "value": -1.0 }))
            .expect("the breakpoint is set");
    }
    let thinned = session
        .execute("sound.lane.thin", &json!({ "lane": lane, "tolerance": 0.5 }))
        .expect("the lane is thinned");
    assert_eq!(thinned.data["removed"], 3, "the three in the middle went");
    assert_eq!(points_of(&session, lane), [(0, -1.0), (400, -1.0)], "the ends stayed");
}

/// The **pan** is between minus one and one and the **mute** is
/// between zero and one, and a value outside is refused.
#[test]
fn a_value_outside_the_range_is_refused() {
    let mut session = new_session();
    let (_, pan) = a_lane(&mut session, "pan");
    let refused = session.execute("sound.lane.point.set", &json!({ "lane": pan, "at": 0, "value": 2.0 }));
    assert!(refused.is_err(), "a pan of two is not a pan");
    session
        .execute("sound.lane.point.set", &json!({ "lane": pan, "at": 0, "value": -1.0 }))
        .expect("a pan of minus one is a pan");
    let mut other = new_session();
    let (_, mute) = a_lane(&mut other, "mute");
    let refused = session.execute("sound.lane.point.set", &json!({ "lane": mute, "at": 0, "value": 2.0 }));
    assert!(refused.is_err(), "a mute of two is not a mute");
}

/// A node that is **not a lane** is refused.
#[test]
fn a_node_that_is_not_a_lane_is_refused() {
    let mut session = new_session();
    let (track, _) = a_lane(&mut session, "volume");
    let refused = session.execute("sound.lane.point.set", &json!({ "lane": track, "at": 0, "value": -6.0 }));
    assert!(refused.is_err(), "the track is not a lane");
}
