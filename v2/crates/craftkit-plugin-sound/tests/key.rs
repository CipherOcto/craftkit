//! The **key** of the session: what the music is in.
//!
//! The tree's sound editor has a key change — a place where the key
//! changes — and the ruler draws it. Here the key is a node of its own
//! with two commands: one sets it, one reads it.

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

/// The **keys** the session has, in the order they happen.
fn keys_of(session: &mut Session) -> Vec<(i64, String)> {
    let read = session.execute("sound.key.read", &json!({})).expect("the keys are read");
    read.data["keys"]
        .as_array()
        .expect("the keys are there")
        .iter()
        .map(|key| {
            (
                key["tick"].as_i64().expect("the tick is there"),
                key["name"].as_str().expect("the name is there").to_string(),
            )
        })
        .collect()
}

/// The key is set at a place and read back by its name.
#[test]
fn a_key_is_set_at_a_place_and_read_back() {
    let mut session = session();
    session.execute("sound.key.set", &json!({ "key": "G major", "tick": 1920 })).expect("the key is set");
    assert_eq!(keys_of(&mut session), vec![(1920, "G major".to_string())]);
}

/// The key is kept as the **number** it stands for: the sharps and the
/// mode — the way a MIDI file carries it.
#[test]
fn a_key_is_kept_as_its_sharps_and_its_mode() {
    let mut session = session();
    session.execute("sound.key.set", &json!({ "key": "E flat minor" })).expect("the key is set");
    let node = session.document().walk().find(|node| node.kind == "sound.key").expect("the key is there");
    assert_eq!(node.data["sharps"], -6);
    assert_eq!(node.data["minor"], true);
    assert_eq!(node.data["tick"], 0);
}

/// A second key at the **same place** changes the one that is there,
/// and it does not leave a second key behind.
#[test]
fn a_key_at_the_same_place_changes_the_one_that_is_there() {
    let mut session = session();
    session
        .execute("sound.key.set", &json!({ "key": "C major", "tick": 960 }))
        .expect("the first key is set");
    session
        .execute("sound.key.set", &json!({ "key": "A minor", "tick": 960 }))
        .expect("the second key is set");
    assert_eq!(keys_of(&mut session), vec![(960, "A minor".to_string())]);
}

/// A name that is not a key of the table is **refused**: the tree takes
/// any string at all, and a string nobody can read is not a key.
#[test]
fn a_name_that_is_not_a_key_is_refused() {
    let mut session = session();
    let error =
        session.execute("sound.key.set", &json!({ "key": "H major" })).expect_err("the key is refused");
    assert!(error.to_string().contains("H major"), "the error says why: {error}");
    assert!(keys_of(&mut session).is_empty(), "and no key was set");
}

/// The key **in effect** at a place is the last change at or before it,
/// and a place before the first change has no key.
#[test]
fn the_key_in_effect_at_a_place_is_the_last_change_before_it() {
    let mut session = session();
    session.execute("sound.key.set", &json!({ "key": "C major", "tick": 0 })).expect("the first key is set");
    session
        .execute("sound.key.set", &json!({ "key": "D major", "tick": 3840 }))
        .expect("the second key is set");

    let read = session.execute("sound.key.read", &json!({ "tick": 1920 })).expect("the key is read");
    assert_eq!(read.data["current"]["name"], "C major");
    let read = session.execute("sound.key.read", &json!({ "tick": 9600 })).expect("the key is read");
    assert_eq!(read.data["current"]["name"], "D major");
}

/// A session with **no** key change says so, and it is not a failure.
#[test]
fn a_session_with_no_key_says_so() {
    let mut session = session();
    let read = session.execute("sound.key.read", &json!({})).expect("the keys are read");
    assert_eq!(read.data["keys"].as_array().expect("the keys are there").len(), 0);
    assert!(read.data["current"].is_null(), "and no key is in effect");
}
