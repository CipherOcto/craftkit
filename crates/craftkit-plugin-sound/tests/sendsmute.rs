//! The **sends** of a track are muted at once.
//!
//! The tree's rules: every send of the track is muted, and a muted
//! send is a copy of nothing — the track is still heard on its own and
//! the bus hears nothing.

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
    let path =
        craftkit_scratch::dir(&format!("craftkit-v2-sendsmute-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **stereo** source of that level on both sides.
fn a_source(path: &str) {
    let bytes = craftkit_wav::write(2, 48_000, &[0.25, 0.25, 0.25, 0.25]);
    std::fs::write(path, bytes).expect("the source is written");
}

/// A session with a **track** and two **buses**, and their ids.
fn a_routed(session: &mut Session, path: &str) -> (u64, u64, u64) {
    session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
        .expect("the clip is added");
    session.execute("sound.bus.add", &json!({ "name": "fx" })).expect("the bus is added");
    let fx = session.document().node_by_name("fx").expect("the bus is there").id;
    session.execute("sound.bus.add", &json!({ "name": "reverb" })).expect("the bus is added");
    let reverb = session.document().node_by_name("reverb").expect("the bus is there").id;
    (track, fx, reverb)
}

/// The **sends** of a track, and whether each of them is silent.
fn sends_of(session: &Session, track: u64) -> Vec<(u64, bool)> {
    let node = session.document().node(track).expect("the track is there");
    node.children
        .iter()
        .filter(|child| child.kind == "sound.send")
        .map(|child| (child.id, child.data["mute"].as_bool().expect("the send has a mute")))
        .collect()
}

/// Every send of the track is **muted** at once.
#[test]
fn every_send_of_the_track_is_muted_at_once() {
    let mut session = session();
    let directory = directory("muted");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let (track, fx, reverb) = a_routed(&mut session, &source_path);
    for bus in [fx, reverb] {
        session.execute("sound.send.add", &json!({ "track": track, "bus": bus })).expect("the send is added");
    }
    let outcome = session
        .execute("sound.track.sends.mute", &json!({ "track": track, "muted": true }))
        .expect("the sends are muted");
    assert_eq!(outcome.data["sends"], json!(2), "both sends are muted: {outcome:?}");
    let sends = sends_of(&session, track);
    assert_eq!(sends.len(), 2, "the track has two sends");
    assert!(sends.iter().all(|(_, muted)| *muted), "and both of them are silent: {sends:?}");
}

/// The sends are **heard** again.
#[test]
fn the_sends_are_heard_again() {
    let mut session = session();
    let directory = directory("heard");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let (track, fx, _) = a_routed(&mut session, &source_path);
    session.execute("sound.send.add", &json!({ "track": track, "bus": fx })).expect("the send is added");
    session
        .execute("sound.track.sends.mute", &json!({ "track": track, "muted": true }))
        .expect("the sends are muted");
    session
        .execute("sound.track.sends.mute", &json!({ "track": track, "muted": false }))
        .expect("the sends are heard");
    let sends = sends_of(&session, track);
    assert!(sends.iter().all(|(_, muted)| !*muted), "the send is heard again: {sends:?}");
}

/// A send that is **already** as it is asked to be is left alone.
#[test]
fn a_send_that_is_already_as_it_is_asked_to_be_is_left_alone() {
    let mut session = session();
    let directory = directory("same");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let (track, fx, _) = a_routed(&mut session, &source_path);
    session.execute("sound.send.add", &json!({ "track": track, "bus": fx })).expect("the send is added");
    let outcome = session
        .execute("sound.track.sends.mute", &json!({ "track": track, "muted": false }))
        .expect("the sends are heard");
    assert!(!outcome.changed, "nothing moved: {outcome:?}");
}

/// A track with **no sends** is left as it is.
#[test]
fn a_track_with_no_sends_is_left_as_it_is() {
    let mut session = session();
    let directory = directory("none");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let (track, _, _) = a_routed(&mut session, &source_path);
    let outcome = session
        .execute("sound.track.sends.mute", &json!({ "track": track, "muted": true }))
        .expect("the sends are muted");
    assert!(!outcome.changed, "there are no sends to mute: {outcome:?}");
    assert_eq!(outcome.data["sends"], json!(0), "the track has no sends: {outcome:?}");
}

/// The send of **another track** is not touched.
#[test]
fn the_send_of_another_track_is_not_touched() {
    let mut session = session();
    let directory = directory("other");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let (track, fx, _) = a_routed(&mut session, &source_path);
    session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the track is added");
    let other = session.document().node_by_name("bass").expect("the track is there").id;
    session.execute("sound.send.add", &json!({ "track": other, "bus": fx })).expect("the send is added");
    session
        .execute("sound.track.sends.mute", &json!({ "track": track, "muted": true }))
        .expect("the sends are muted");
    let sends = sends_of(&session, other);
    assert!(sends.iter().all(|(_, muted)| !*muted), "the send of the other track is heard: {sends:?}");
}

/// A **muted** send is a copy of nothing: the track is still heard and
/// the bus hears nothing.
#[test]
fn a_muted_send_is_a_copy_of_nothing() {
    let directory = directory("render");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    a_source(&source_path);
    let mut session = session();
    let (track, fx, _) = a_routed(&mut session, &source_path);
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": fx, "level_db": 0.0 }))
        .expect("the send is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let written = std::fs::read(&output_path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    assert!(
        (sound.samples[0] - 0.5).abs() < 1e-3,
        "the track and the copy are both heard: {}",
        sound.samples[0]
    );
    session
        .execute("sound.track.sends.mute", &json!({ "track": track, "muted": true }))
        .expect("the sends are muted");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let written = std::fs::read(&output_path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    assert!((sound.samples[0] - 0.25).abs() < 1e-3, "the track is still heard: {}", sound.samples[0]);
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let source_path = format!("{directory}/source.wav");
    a_source(&source_path);
    let (track, _, _) = a_routed(&mut session, &source_path);
    let outcome = session.execute("sound.track.sends.mute", &json!({ "track": 9999, "muted": true }));
    assert!(outcome.is_err(), "there is no such track: {outcome:?}");
    let outcome = session.execute("sound.track.sends.mute", &json!({ "track": track, "muted": true }));
    assert!(outcome.is_ok(), "the track is there: {outcome:?}");
}
