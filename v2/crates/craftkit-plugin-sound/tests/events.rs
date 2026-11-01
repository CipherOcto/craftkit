//! The **events** of the session: the markers and the tempo map.

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

/// The **tempos** of the document, as `(tick, bpm)`.
fn tempos_of(session: &Session) -> Vec<(i64, f64)> {
    let mut tempos: Vec<(i64, f64)> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.tempo")
        .filter_map(|node| {
            let tick = node.data.get("tick").and_then(|tick| tick.as_i64())?;
            let bpm = node.data.get("bpm").and_then(|bpm| bpm.as_f64())?;
            Some((tick, bpm))
        })
        .collect();
    tempos.sort_by_key(|(tick, _)| *tick);
    tempos
}

/// A marker is **moved** and **renamed**.
#[test]
fn a_marker_is_changed() {
    let mut session = new_session();
    session
        .execute("sound.marker.add", &json!({ "name": "chorus", "start": 4_800 }))
        .expect("the marker is added");
    let marker =
        session.document().walk().find(|node| node.kind == "sound.marker").expect("the marker is there").id;
    session
        .execute("sound.marker.set", &json!({ "marker": marker, "name": "bridge", "start": 9_600 }))
        .expect("the marker is changed");
    let node = session.document().node(marker).expect("the marker is there");
    assert_eq!(node.name, "bridge", "the marker is renamed");
    assert_eq!(node.data["start"], json!(9_600), "and moved");
}

/// The **tempos** are scaled: every tempo is multiplied by the factor
/// and the times do not move.
#[test]
fn the_tempos_are_scaled() {
    let mut session = new_session();
    session.execute("sound.tempo.set", &json!({ "bpm": 120.0, "tick": 0 })).expect("the tempo is set");
    session.execute("sound.tempo.set", &json!({ "bpm": 90.0, "tick": 960 })).expect("the other tempo is set");
    let scaled =
        session.execute("sound.tempo.scale", &json!({ "factor": 2.0 })).expect("the tempos are scaled");
    assert_eq!(scaled.data["tempos"], 2, "both tempos were scaled");
    let tempos = tempos_of(&session);
    assert_eq!(tempos, [(0, 240.0), (960, 180.0)], "the tempos are twice as fast");
}

/// A factor that would take a tempo **out of the range** is refused,
/// and no tempo is scaled.
#[test]
fn a_factor_out_of_the_range_is_refused() {
    let mut session = new_session();
    session.execute("sound.tempo.set", &json!({ "bpm": 600.0, "tick": 0 })).expect("the tempo is set");
    let refused = session.execute("sound.tempo.scale", &json!({ "factor": 10.0 }));
    assert!(refused.is_err(), "the tempo would be six thousand beats a minute");
    assert_eq!(tempos_of(&session), [(0, 600.0)], "and no tempo was scaled");
}

/// A **range** of the tempo map is made constant: the tempo at the
/// start is the one given, the changes inside go, and the tempo at
/// the end comes back.
#[test]
fn a_range_of_the_tempo_map_is_made_constant() {
    let mut session = new_session();
    session.execute("sound.tempo.set", &json!({ "bpm": 100.0, "tick": 0 })).expect("the tempo is set");
    session
        .execute("sound.tempo.set", &json!({ "bpm": 140.0, "tick": 960 }))
        .expect("the other tempo is set");
    session
        .execute("sound.tempo.constant", &json!({ "bpm": 90.0, "start": 480, "end": 1920 }))
        .expect("the range is made constant");
    let tempos = tempos_of(&session);
    assert!(tempos.contains(&(480, 90.0)), "the tempo at the start of the range is the new one: {tempos:?}");
    assert!(
        !tempos.iter().any(|(tick, _)| *tick > 480 && *tick < 1920),
        "and the changes inside it are gone: {tempos:?}"
    );
    assert!(
        tempos.iter().any(|(tick, bpm)| *tick >= 1920 && (*bpm - 140.0).abs() < 0.001),
        "and the tempo at the end of it comes back: {tempos:?}"
    );
}
