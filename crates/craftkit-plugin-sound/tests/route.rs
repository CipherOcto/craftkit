//! The **route**: where a track's sound goes.
//!
//! A track that feeds a bus is heard on that bus and not in the main
//! mix; the tree's strip goes to its output and to nowhere else.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-route-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source.
const QUIET: f32 = 0.25;

/// A session with a **track** that plays a quiet source.
fn a_track(session: &mut Session, path: &str) -> u64 {
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
    track
}

/// A **bus**, with a name.
fn a_bus(session: &mut Session, name: &str) -> u64 {
    session.execute("sound.bus.add", &json!({ "name": name })).expect("the bus is added");
    session.document().node_by_name(name).expect("the bus is there").id
}

/// A **reader** of a bus, with a fader six decibels down.
fn a_reader(session: &mut Session, bus: u64) -> u64 {
    session.execute("sound.track.add", &json!({ "name": "return" })).expect("the reader is added");
    let reader = session.document().node_by_name("return").expect("the reader is there").id;
    session
        .execute("sound.track.input", &json!({ "track": reader, "bus": bus }))
        .expect("the reader reads the bus");
    session
        .execute("sound.lane.add", &json!({ "track": reader, "param": "volume", "points": [[0, -6.0]] }))
        .expect("the fader is there");
    reader
}

/// The first sample of what was rendered.
fn rendered(path: &str) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples[0]
}

/// A track that **feeds a bus** is heard on that bus and not in the
/// main mix: the sound is heard through the reader, at the reader's
/// own level.
#[test]
fn a_routed_track_is_heard_on_its_bus() {
    let directory = directory("routed");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let track = a_track(&mut session, &source_path);
    let bus = a_bus(&mut session, "fx");
    session
        .execute("sound.route.add", &json!({ "track": track, "bus": bus }))
        .expect("the track feeds the bus");
    a_reader(&mut session, bus);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    // The track is on the bus and the bus is read: the sound is heard
    // once, at half. Were the track heard in the main mix as well, the
    // sum would be three quarters.
    let expected = QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the track is on its bus: {sample}");
}

/// A track that feeds **nothing** is heard in the main mix, whole.
#[test]
fn an_unrouted_track_is_heard_in_the_main_mix() {
    let directory = directory("unrouted");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    a_track(&mut session, &source_path);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    assert!((sample - QUIET).abs() < 0.01, "the track is heard in the main mix: {sample}");
}

/// A bus that **feeds another bus** is heard there and not by itself:
/// the sound that goes into it goes on.
#[test]
fn a_bus_that_feeds_a_bus_is_heard_there() {
    let directory = directory("chained");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let track = a_track(&mut session, &source_path);
    let first = a_bus(&mut session, "first");
    let second = a_bus(&mut session, "second");
    // The track sends to the first bus, and the first bus feeds the
    // second.
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": first, "level_db": 0.0 }))
        .expect("the send is added");
    session
        .execute("sound.route.add", &json!({ "track": first, "bus": second }))
        .expect("the bus feeds the bus");
    a_reader(&mut session, second);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    // The track is heard in the main mix and the reader reads the
    // second bus, which holds the first bus's sound: the copy is heard
    // at half. Were the first bus not heard where it feeds, the copy
    // would be nothing and the sum would be a quarter.
    let expected = QUIET + QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the bus is heard where it feeds: {sample}");
}
