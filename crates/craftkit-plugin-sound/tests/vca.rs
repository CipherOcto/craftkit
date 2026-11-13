//! The **VCA** of a track.
//!
//! A VCA master is a track that carries no sound of its own: its
//! fader offsets the faders of the tracks under it, and its mute
//! mutes them. It is the tree's own second fader over a group.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-vca-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source: quiet enough that the mix stays
/// inside the file.
const QUIET: f32 = 0.4;

/// A track with a clip, and a **VCA master**, and the ids of both.
fn a_track_under_a_master(session: &mut Session) -> (u64, u64) {
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session
        .execute("sound.track.add", &json!({ "name": "master", "track_kind": "vca" }))
        .expect("the master is added");
    let master = session.document().node_by_name("master").expect("the master is there").id;
    (track, master)
}

/// The **payload** of a track, as the document holds it.
fn payload_of(session: &Session, track: u64) -> craftkit_sdk::serde_json::Value {
    let node = session.document().node(track).expect("the track is there");
    node.data.clone()
}

/// The first sample of what was rendered.
fn rendered(path: &str) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples[0]
}

/// A track may be put **under a VCA master**.
#[test]
fn a_track_may_be_put_under_a_vca() {
    let mut session = session();
    let (track, master) = a_track_under_a_master(&mut session);
    session
        .execute("sound.track.vca", &json!({ "track": track, "vca": master }))
        .expect("the track is under the master");
    assert_eq!(payload_of(&session, track)["vca"].as_u64(), Some(master), "the track is under the master");
}

/// The VCA may be taken **away** again.
#[test]
fn the_vca_may_be_taken_away() {
    let mut session = session();
    let (track, master) = a_track_under_a_master(&mut session);
    session
        .execute("sound.track.vca", &json!({ "track": track, "vca": master }))
        .expect("the track is under the master");
    session
        .execute("sound.track.vca.delete", &json!({ "track": track }))
        .expect("the track is under no master");
    assert_eq!(payload_of(&session, track)["vca"].as_u64(), None, "the track is under no master");
}

/// A track that is **not a VCA master** may not be a master.
#[test]
fn a_track_that_is_not_a_vca_master_may_not_be_a_master() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "drums" })).expect("the track is added");
    let track = session.document().node_by_name("drums").expect("the track is there").id;
    session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the other track is added");
    let other = session.document().node_by_name("bass").expect("the other track is there").id;
    let error = session
        .execute("sound.track.vca", &json!({ "track": track, "vca": other }))
        .expect_err("that track is not a VCA master");
    assert!(
        error.to_string().contains("not a VCA master"),
        "the error says the track is not a VCA master: {error}"
    );
}

/// A track may **not be its own** VCA master.
#[test]
fn a_track_may_not_be_its_own_vca_master() {
    let mut session = session();
    session
        .execute("sound.track.add", &json!({ "name": "master", "track_kind": "vca" }))
        .expect("the master is added");
    let master = session.document().node_by_name("master").expect("the master is there").id;
    let error = session
        .execute("sound.track.vca", &json!({ "track": master, "vca": master }))
        .expect_err("a track cannot be its own master");
    assert!(
        error.to_string().contains("its own VCA master"),
        "the error says the track cannot be its own master: {error}"
    );
}

/// A master that is **not there** is refused.
#[test]
fn a_master_that_is_not_there_is_refused() {
    let mut session = session();
    let (track, _) = a_track_under_a_master(&mut session);
    let error = session
        .execute("sound.track.vca", &json!({ "track": track, "vca": track + 1000 }))
        .expect_err("there is no such master");
    assert!(error.to_string().contains("no track"), "the error says there is no track: {error}");
}

/// The **master's fader** offsets the member's level: the tree's mixer
/// adds the master's level to the member's, and the two are the same
/// thing in gains.
#[test]
fn the_masters_fader_offsets_the_members_level() {
    let directory = directory("fader");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    std::fs::write(&source_path, craftkit_wav::write(1, 48_000, &[QUIET, QUIET]))
        .expect("the source is written");
    let mut session = session();
    session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source =
        session.document().walk().find(|node| node.kind == "sound.source").expect("the source is there").id;
    let (track, master) = a_track_under_a_master(&mut session);
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
        .expect("the clip is added");
    session
        .execute("sound.track.vca", &json!({ "track": track, "vca": master }))
        .expect("the track is under the master");
    // The **master's fader**: six decibels down.
    session
        .execute("sound.track.fader", &json!({ "track": master, "db": -6.0 }))
        .expect("the master is at half");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    let expected = QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the master's fader offsets the member: {sample}");
}

/// A **muted master** mutes its members.
#[test]
fn a_muted_master_mutes_its_members() {
    let directory = directory("mute");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    std::fs::write(&source_path, craftkit_wav::write(1, 48_000, &[QUIET, QUIET]))
        .expect("the source is written");
    let mut session = session();
    session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source =
        session.document().walk().find(|node| node.kind == "sound.source").expect("the source is there").id;
    let (track, master) = a_track_under_a_master(&mut session);
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
        .expect("the clip is added");
    session
        .execute("sound.track.vca", &json!({ "track": track, "vca": master }))
        .expect("the track is under the master");
    session
        .execute("sound.track.mute", &json!({ "track": master, "muted": true }))
        .expect("the master is muted");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    assert!(sample.abs() < 0.01, "a muted master mutes its members: {sample}");
}

/// A **VCA master** carries no sound of its own: it is not in the
/// mix, and the tree's mixer leaves it out.
#[test]
fn a_vca_master_is_not_in_the_mix() {
    let directory = directory("master");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    std::fs::write(&source_path, craftkit_wav::write(1, 48_000, &[QUIET, QUIET]))
        .expect("the source is written");
    let mut session = session();
    session.execute("sound.session.set", &json!({ "sample_rate": 48_000 })).expect("the session is set");
    session.execute("sound.source.add", &json!({ "path": source_path })).expect("the source is added");
    let source =
        session.document().walk().find(|node| node.kind == "sound.source").expect("the source is there").id;
    let (track, master) = a_track_under_a_master(&mut session);
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 2, "source": source }))
        .expect("the clip is added");
    // The master holds a clip of its own, and the tree's VCA master
    // holds no sound: the clip is not heard.
    session
        .execute("sound.clip.add", &json!({ "track": master, "start": 0, "length": 2, "source": source }))
        .expect("the clip is added");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    assert!((sample - QUIET).abs() < 0.01, "only the member is heard: {sample}");
}

/// The VCA is the track's **own**: it is not the group that mixes
/// together, and the tree's group shares the mute and the solo and
/// not the VCA.
#[test]
fn the_vca_is_not_the_mixing_group() {
    let mut session = session();
    let (track, master) = a_track_under_a_master(&mut session);
    session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the other track is added");
    let other = session.document().node_by_name("bass").expect("the other track is there").id;
    session
        .execute("sound.track.group", &json!({ "name": "the pair", "tracks": [track, other] }))
        .expect("the tracks are in a group");
    session
        .execute("sound.track.vca", &json!({ "track": track, "vca": master }))
        .expect("the track is under the master");
    assert_eq!(payload_of(&session, track)["vca"].as_u64(), Some(master), "the track is under the master");
    assert_eq!(payload_of(&session, other)["vca"].as_u64(), None, "and the other one is under no master");
}
