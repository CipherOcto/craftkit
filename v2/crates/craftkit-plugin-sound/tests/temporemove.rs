//! The **taking away** of a tempo.
//!
//! The tempo map is a list of events, and a map that can only grow is
//! not a map a person can work with.

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

/// A tempo is **taken away**.
#[test]
fn a_tempo_is_taken_away() {
    let mut session = session();
    session.execute("sound.tempo.set", &json!({ "bpm": 120.0, "tick": 0 })).expect("the tempo is set");
    session.execute("sound.tempo.set", &json!({ "bpm": 90.0, "tick": 960 })).expect("the tempo is set");
    assert_eq!(tempos_of(&session), vec![(0, 120.0), (960, 90.0)]);
    session.execute("sound.tempo.remove", &json!({ "tick": 960 })).expect("the tempo is taken away");
    assert_eq!(tempos_of(&session), vec![(0, 120.0)], "and the map is shorter");
}

/// The tempo at the **first tick** is the document's tempo, and it
/// stays.
#[test]
fn the_first_tempo_stays() {
    let mut session = session();
    session.execute("sound.tempo.set", &json!({ "bpm": 120.0, "tick": 0 })).expect("the tempo is set");
    let error =
        session.execute("sound.tempo.remove", &json!({ "tick": 0 })).expect_err("the tempo is refused");
    assert!(error.to_string().contains("document's tempo"), "the error says why: {error}");
    assert_eq!(tempos_of(&session), vec![(0, 120.0)], "and the tempo is there");
}

/// A tick with **no tempo** is refused: a tempo that was never there
/// is not a tempo that was taken away.
#[test]
fn a_tick_with_no_tempo_is_refused() {
    let mut session = session();
    session.execute("sound.tempo.set", &json!({ "bpm": 120.0, "tick": 0 })).expect("the tempo is set");
    let error =
        session.execute("sound.tempo.remove", &json!({ "tick": 960 })).expect_err("the removal is refused");
    assert!(error.to_string().contains("no tempo at tick"), "the error says why: {error}");
}

/// A **meter** is taken away as well: the meter map is a map, and a
/// map that can only grow is not a map.
#[test]
fn a_meter_is_taken_away() {
    let mut session = session();
    session
        .execute("sound.meter.set", &json!({ "numerator": 4, "denominator": 4, "tick": 0 }))
        .expect("the meter is set");
    session
        .execute("sound.meter.set", &json!({ "numerator": 3, "denominator": 4, "tick": 1920 }))
        .expect("the meter is set");
    let meters: Vec<i64> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.meter")
        .filter_map(|node| node.data.get("tick").and_then(|tick| tick.as_i64()))
        .collect();
    assert_eq!(meters.len(), 2, "there are two meters");
    session.execute("sound.meter.remove", &json!({ "tick": 1920 })).expect("the meter is taken away");
    let meters: Vec<i64> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.meter")
        .filter_map(|node| node.data.get("tick").and_then(|tick| tick.as_i64()))
        .collect();
    assert_eq!(meters, vec![0], "and the map is shorter");
}

/// The **first** meter stays: it is the document's meter.
#[test]
fn the_first_meter_stays() {
    let mut session = session();
    session
        .execute("sound.meter.set", &json!({ "numerator": 4, "denominator": 4, "tick": 0 }))
        .expect("the meter is set");
    let error =
        session.execute("sound.meter.remove", &json!({ "tick": 0 })).expect_err("the meter is refused");
    assert!(error.to_string().contains("document's meter"), "the error says why: {error}");
}

/// A **key** is taken away as well.
#[test]
fn a_key_is_taken_away() {
    let mut session = session();
    session.execute("sound.key.set", &json!({ "key": "C major", "tick": 0 })).expect("the key is set");
    session.execute("sound.key.set", &json!({ "key": "G major", "tick": 1920 })).expect("the key is set");
    let keys: Vec<i64> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.key")
        .filter_map(|node| node.data.get("tick").and_then(|tick| tick.as_i64()))
        .collect();
    assert_eq!(keys.len(), 2, "there are two keys");
    session.execute("sound.key.remove", &json!({ "tick": 1920 })).expect("the key is taken away");
    let keys: Vec<i64> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.key")
        .filter_map(|node| node.data.get("tick").and_then(|tick| tick.as_i64()))
        .collect();
    assert_eq!(keys, vec![0], "and the map is shorter");
}
