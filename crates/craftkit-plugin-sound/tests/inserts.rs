//! The **chain** of inserts: the plugins on a track, and their order.
//!
//! The tree's insert lives in a **slot** on the track and the tree's
//! move swaps two slots. Here the insert is a **node** under the track
//! and the move swaps two nodes — and an insert that is taken away is
//! taken away, and not left in an empty slot.

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

/// A directory of this test's own, so that the files do not mix.
fn directory(name: &str) -> String {
    let path = craftkit_scratch::dir(&format!("craftkit-v2-inserts-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of that level: one frame, so that the mix is one frame
/// and the insert can be told apart.
fn a_sound(path: &str, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &[level]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track called `drums` and a sound on it.
fn a_track(session: &mut Session, directory: &str) -> u64 {
    let path = format!("{directory}/sound.wav");
    a_sound(&path, 0.5);
    session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 1, "source": source }))
        .expect("the clip is added");
    track
}

/// The rendered sound, read back from the file.
fn rendered(path: &str) -> craftkit_wav::Sound {
    let bytes = std::fs::read(path).expect("the sound is on the disk");
    craftkit_wav::read(&bytes, &craftkit_wav::Limits::default()).expect("the sound is read")
}

/// The **chain** of a track: the names of its inserts, in the order
/// they are run in.
fn chain_of(session: &Session, track: u64) -> Vec<String> {
    let node = session.document().node(track).expect("the track is there");
    node.children
        .iter()
        .filter(|child| child.kind == "sound.insert")
        .map(|child| child.name.clone())
        .collect()
}

/// An insert is **taken away**, and the sound is the sound without it.
#[test]
fn an_insert_is_taken_away() {
    let mut session = session();
    let directory = directory("removed");
    let track = a_track(&mut session, &directory);
    let insert = session
        .execute("sound.insert.add", &json!({ "track": track, "plugin": "gain" }))
        .expect("the insert is added")
        .data["id"]
        .as_u64()
        .expect("the insert has an id");
    session
        .execute("sound.insert.set", &json!({ "insert": insert, "parameters": { "gain_db": -6.0 } }))
        .expect("the gain is down");
    let quiet_path = format!("{directory}/quiet.wav");
    session.execute("sound.render", &json!({ "path": quiet_path })).expect("the sound is rendered");
    session.execute("sound.insert.remove", &json!({ "insert": insert })).expect("the insert is taken away");
    let loud_path = format!("{directory}/loud.wav");
    session.execute("sound.render", &json!({ "path": loud_path })).expect("the sound is rendered");

    let quiet = rendered(&quiet_path);
    let loud = rendered(&loud_path);
    assert!((quiet.samples[0] - 0.25).abs() < 1e-3, "the insert was heard: {}", quiet.samples[0]);
    assert!((loud.samples[0] - 0.5).abs() < 1e-3, "and now it is not: {}", loud.samples[0]);
    assert_eq!(chain_of(&session, track), Vec::<String>::new(), "the chain is empty");
}

/// Two inserts **swap their places** in the chain.
#[test]
fn two_inserts_swap_their_places() {
    let mut session = session();
    let directory = directory("swapped");
    let track = a_track(&mut session, &directory);
    let first = session
        .execute("sound.insert.add", &json!({ "track": track, "plugin": "gain" }))
        .expect("the insert is added")
        .data["id"]
        .as_u64()
        .expect("the insert has an id");
    let second = session
        .execute("sound.insert.add", &json!({ "track": track, "plugin": "lowpass" }))
        .expect("the insert is added")
        .data["id"]
        .as_u64()
        .expect("the insert has an id");
    assert_eq!(chain_of(&session, track), vec!["gain".to_string(), "lowpass".to_string()]);
    session
        .execute("sound.insert.move", &json!({ "insert": first, "other": second }))
        .expect("the inserts are moved");
    assert_eq!(chain_of(&session, track), vec!["lowpass".to_string(), "gain".to_string()]);
}

/// The insert that is **not there** is refused: a node that is not an
/// insert is not an insert.
#[test]
fn a_node_that_is_not_an_insert_is_refused() {
    let mut session = session();
    let directory = directory("not-an-insert");
    let track = a_track(&mut session, &directory);
    let error = session
        .execute("sound.insert.remove", &json!({ "insert": track }))
        .expect_err("the insert is refused");
    assert!(error.to_string().contains("not an insert"), "the error says why: {error}");
}

/// Two inserts of **different** things are refused: a chain is the
/// chain of one thing.
#[test]
fn two_inserts_of_different_things_are_refused() {
    let mut session = session();
    let directory = directory("different");
    let track = a_track(&mut session, &directory);
    session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the track is added");
    let other = session.document().node_by_name("bass").expect("the track is there").id;
    let first = session
        .execute("sound.insert.add", &json!({ "track": track, "plugin": "gain" }))
        .expect("the insert is added")
        .data["id"]
        .as_u64()
        .expect("the insert has an id");
    let second = session
        .execute("sound.insert.add", &json!({ "track": other, "plugin": "gain" }))
        .expect("the insert is added")
        .data["id"]
        .as_u64()
        .expect("the insert has an id");
    let error = session
        .execute("sound.insert.move", &json!({ "insert": first, "other": second }))
        .expect_err("the move is refused");
    assert!(error.to_string().contains("not on the same thing"), "the error says why: {error}");
}
