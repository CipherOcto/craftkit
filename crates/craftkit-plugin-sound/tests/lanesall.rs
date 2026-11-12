//! The **automation of a whole track**: every lane of it at once.
//!
//! The tree's editor has a "Thin All" and a "Write to Current": the
//! first thins every lane of every track, and the second takes the
//! value each lane holds at the playhead and writes it over a range.
//!
//! Here they are **one track's** lanes, because a track is a node and
//! a node is something a command can be given.

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

/// A session with a track and two lanes on it: a volume and a pan.
fn a_track(session: &mut Session) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute(
            "sound.lane.add",
            &json!({ "track": track, "param": "volume", "points": [[0, -6.0], [480, -6.0], [960, -6.0]] }),
        )
        .expect("the volume lane is added");
    session
        .execute(
            "sound.lane.add",
            &json!({ "track": track, "param": "pan", "points": [[0, 0.0], [480, 0.0], [960, 0.0]] }),
        )
        .expect("the pan lane is added");
    track
}

/// The breakpoints of every lane of the track, as `(param, at, value)`.
fn points_of(session: &Session, track: u64) -> Vec<(String, i64, f64)> {
    let mut points: Vec<(String, i64, f64)> = Vec::new();
    for node in session.document().walk() {
        if node.kind != "sound.lane" {
            continue;
        }
        let Some(parent) =
            session.document().walk().find(|parent| parent.children.iter().any(|child| child.id == node.id))
        else {
            continue;
        };
        if parent.id != track {
            continue;
        }
        let param = node.data["param"].as_str().unwrap_or("").to_string();
        for point in node.data["points"].as_array().expect("the points are there") {
            points.push((param.clone(), point[0].as_i64().unwrap_or(0), point[1].as_f64().unwrap_or(0.0)));
        }
    }
    points.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    points
}

/// Every lane of the track is **thinned** at once: the breakpoints the
/// curve does not need are taken away from all of them.
#[test]
fn every_lane_of_the_track_is_thinned() {
    let mut session = new_session();
    let track = a_track(&mut session);
    // The three points of each lane are in a straight line, so a
    // tolerance of a tenth of a decibel takes the middle one away.
    let thinned =
        session.execute("sound.lane.thin.all", &json!({ "track": track })).expect("the lanes are thinned");
    assert_eq!(thinned.data["removed"], 2, "one breakpoint off each lane: {thinned:?}");
    let points = points_of(&session, track);
    assert_eq!(points.len(), 4, "two points on each lane: {points:?}");
}

/// The **write to current** takes the **static** value of each
/// parameter — the fader, the pan, the mute — and writes it over a
/// range, so that the curve is flat over it.
#[test]
fn the_static_value_is_written_over_a_range() {
    let mut session = new_session();
    let track = a_track(&mut session);
    // The fader is moved down, so that the value written is not the
    // one the lane happens to hold.
    session
        .execute("sound.track.fader", &json!({ "track": track, "db": -12.0 }))
        .expect("the fader is moved");
    // A breakpoint in the middle of the range, so that the curve is
    // not already flat. The volume moves in decibels and the pan
    // between minus one and one, so the value is the one each lane
    // may have.
    for node in session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.lane")
        .map(|node| node.id)
        .collect::<Vec<_>>()
    {
        let param = session
            .document()
            .node(node)
            .map(|node| node.data["param"].as_str().unwrap_or("").to_string())
            .unwrap_or_default();
        let value = if param == "pan" { 0.5 } else { -24.0 };
        session
            .execute("sound.lane.point.set", &json!({ "lane": node, "at": 240, "value": value }))
            .expect("the breakpoint is set");
    }
    let written = session
        .execute("sound.lane.write.all", &json!({ "track": track, "start": 120, "end": 600 }))
        .expect("the lanes are written");
    assert_eq!(written.data["lanes"], 2, "both lanes are written: {written:?}");
    let points = points_of(&session, track);
    // The volume lane: the fader is minus twelve decibels, and that is
    // what is written at both ends of the range.
    let volume: Vec<(i64, f64)> =
        points.iter().filter(|(param, _, _)| param == "volume").map(|(_, at, value)| (*at, *value)).collect();
    assert!(volume.contains(&(120, -12.0)), "the value at the start of the range: {volume:?}");
    assert!(volume.contains(&(600, -12.0)), "and at the end of it: {volume:?}");
    // And the breakpoint that was in the middle is gone.
    assert!(!volume.iter().any(|(at, _)| *at == 240), "the middle one is gone: {volume:?}");
    // The pan lane: the track's own pan is the middle.
    let pan: Vec<(i64, f64)> =
        points.iter().filter(|(param, _, _)| param == "pan").map(|(_, at, value)| (*at, *value)).collect();
    assert!(pan.contains(&(120, 0.0)), "the middle of the pan: {pan:?}");
}

/// The lanes of a track that has **no** lanes are written and thinned
/// without a word: there is nothing to do, and that is not a failure.
#[test]
fn a_track_with_no_lanes_is_nothing_to_do() {
    let mut session = new_session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    let thinned =
        session.execute("sound.lane.thin.all", &json!({ "track": track })).expect("the lanes are thinned");
    assert_eq!(thinned.data["removed"], 0, "nothing was taken away: {thinned:?}");
    let written = session
        .execute("sound.lane.write.all", &json!({ "track": track, "start": 0, "end": 100 }))
        .expect("the lanes are written");
    assert_eq!(written.data["lanes"], 0, "and nothing was written: {written:?}");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = new_session();
    let track = a_track(&mut session);
    let refused = session.execute("sound.lane.thin.all", &json!({ "track": track + 1_000 }));
    assert!(refused.is_err(), "there is no such node");
}
