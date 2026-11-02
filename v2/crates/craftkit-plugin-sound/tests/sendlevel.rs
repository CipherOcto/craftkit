//! The **settings** of a send: its level, its pan, and whether it is
//! silent.
//!
//! The tree's send has a command for its level and another for the
//! rest. This one has a command for the send's own settings: the send
//! is a thing, and the things about it are the send's.

use craftkit_sdk::serde_json::{Value as Json, json};
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
    let path = std::env::temp_dir().join(format!("craftkit-v2-sendlevel-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of that level: one frame, so that the mix is one frame
/// and the send can be told apart.
fn a_sound(path: &str, level: f32) {
    let bytes = craftkit_wav::write(1, 48_000, &[level]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track, a sound on it and a bus the track sends to.
///
/// The **bus** is a node of its own, and the send is a node under the
/// track that points at it.
fn a_track_with_a_bus(session: &mut Session, directory: &str) -> (u64, u64) {
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
    session.execute("sound.bus.add", &json!({ "name": "reverb" })).expect("the bus is added");
    let bus = session.document().node_by_name("reverb").expect("the bus is there").id;
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0 }))
        .expect("the send is added");
    let send = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.send")
        .map(|node| node.id)
        .expect("the send is there");
    (track, send)
}

/// The **settings** of a send, read back.
fn settings_of(session: &Session, send: u64) -> Json {
    let node = session.document().node(send).expect("the send is there");
    node.data.clone()
}

/// A send's **level** is changed.
#[test]
fn the_level_of_a_send_is_changed() {
    let mut session = session();
    let directory = directory("level");
    let (_, send) = a_track_with_a_bus(&mut session, &directory);
    session.execute("sound.send.set", &json!({ "send": send, "level_db": -6.0 })).expect("the level is set");
    assert_eq!(settings_of(&session, send)["level_db"], json!(-6.0));
}

/// A level outside the tree's range is **refused** and not pulled to
/// the edge.
#[test]
fn a_level_outside_the_bound_is_refused() {
    let mut session = session();
    let directory = directory("bound");
    let (_, send) = a_track_with_a_bus(&mut session, &directory);
    let error = session
        .execute("sound.send.set", &json!({ "send": send, "level_db": 40.0 }))
        .expect_err("the level is refused");
    assert!(error.to_string().contains("minus one hundred and forty-four"), "the error says why: {error}");
    assert_eq!(settings_of(&session, send)["level_db"], json!(0.0), "and the level is as it was");
}

/// A **pan** outside minus one and one is refused.
#[test]
fn a_pan_outside_the_bound_is_refused() {
    let mut session = session();
    let directory = directory("pan");
    let (_, send) = a_track_with_a_bus(&mut session, &directory);
    let error = session
        .execute("sound.send.set", &json!({ "send": send, "pan": 2.0 }))
        .expect_err("the pan is refused");
    assert!(error.to_string().contains("between minus one and one"), "the error says why: {error}");
}

/// A send may be made **silent**, and it may be taken **before the
/// fader**.
#[test]
fn a_send_may_be_silent_and_may_be_pre_fader() {
    let mut session = session();
    let directory = directory("mute");
    let (_, send) = a_track_with_a_bus(&mut session, &directory);
    session
        .execute("sound.send.set", &json!({ "send": send, "mute": true, "pre_fader": true }))
        .expect("the send is changed");
    let settings = settings_of(&session, send);
    assert_eq!(settings["mute"], json!(true));
    assert_eq!(settings["pre_fader"], json!(true));
}

/// A node that is **not a send** is refused.
#[test]
fn a_node_that_is_not_a_send_is_refused() {
    let mut session = session();
    let directory = directory("not-a-send");
    let (track, _) = a_track_with_a_bus(&mut session, &directory);
    let error = session
        .execute("sound.send.set", &json!({ "send": track, "level_db": -6.0 }))
        .expect_err("the send is refused");
    assert!(error.to_string().contains("not a send"), "the error says why: {error}");
}
