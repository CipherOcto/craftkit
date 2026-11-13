//! The **reader**: a track that reads a bus.
//!
//! A bus is a destination: sound collects there and is not heard there.
//! Something must read it and take it into the mix — the tree's aux is
//! a strip that reads a bus, and the bus is silent on its own.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-reader-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source: quiet enough that the track, the
/// bus and the reader stay inside the file.
const QUIET: f32 = 0.25;

/// A session with a **track** that plays a quiet source, and a **bus**
/// the track sends to.
fn a_track_and_a_bus(session: &mut Session, path: &str) -> (u64, u64) {
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
    let bus = session.document().node_by_name("fx").expect("the bus is there").id;
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": 0.0 }))
        .expect("the send is added");
    (track, bus)
}

/// A **reader**: a track that reads the bus, with a fader six decibels
/// down.
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

/// A bus that is **read** is heard through the reader and not by
/// itself: the track is heard, and the reader's copy is heard at the
/// reader's own level.
#[test]
fn a_bus_that_is_read_is_heard_through_the_reader() {
    let directory = directory("read");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, bus) = a_track_and_a_bus(&mut session, &source_path);
    a_reader(&mut session, bus);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    // The track is heard whole and the reader's copy is at half: the
    // bus is not heard by itself, or the sum would be three quarters.
    let expected = QUIET + QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the bus is read, not heard: {sample}");
}

/// A reader holds no **clips**: it takes a bus into the mix, and a
/// clip on it would be a clip on a place that is not a track.
#[test]
fn a_reader_holds_no_clips() {
    let directory = directory("clip");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, bus) = a_track_and_a_bus(&mut session, &source_path);
    let reader = a_reader(&mut session, bus);
    let source = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there");
    let refused = session
        .execute("sound.clip.add", &json!({ "track": reader, "start": 0, "length": 2, "source": source }));
    let message = match refused {
        Ok(_) => panic!("a clip on a reader is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("reads a bus"), "the refusal says why: {message}");
}

/// A reader has no **sends**: it carries the bus's sound and does not
/// copy itself anywhere.
#[test]
fn a_reader_has_no_sends() {
    let directory = directory("send");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, bus) = a_track_and_a_bus(&mut session, &source_path);
    let reader = a_reader(&mut session, bus);
    session.execute("sound.bus.add", &json!({ "name": "other" })).expect("the bus is added");
    let other = session.document().node_by_name("other").expect("the bus is there").id;
    let refused = session.execute("sound.send.add", &json!({ "track": reader, "bus": other }));
    let message = match refused {
        Ok(_) => panic!("a send from a reader is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("no sends"), "the refusal says why: {message}");
}

/// A reader may be **routed**: it may feed a bus and be heard there
/// and not in the main mix.
#[test]
fn a_reader_may_be_routed() {
    let directory = directory("route");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, bus) = a_track_and_a_bus(&mut session, &source_path);
    let reader = a_reader(&mut session, bus);
    session.execute("sound.bus.add", &json!({ "name": "other" })).expect("the bus is added");
    let other = session.document().node_by_name("other").expect("the bus is there").id;
    session
        .execute("sound.route.add", &json!({ "track": reader, "bus": other }))
        .expect("the reader is routed");
    let routes = session.document().relations_from(reader, Some("route"));
    assert_eq!(routes.len(), 1, "the reader feeds the bus");
    assert_eq!(routes[0].to, other, "and it is the bus that was named");
}

/// A track reads **one** bus: the same bus twice is refused, and so is
/// a second bus.
#[test]
fn a_track_reads_one_bus() {
    let directory = directory("one");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, bus) = a_track_and_a_bus(&mut session, &source_path);
    let reader = a_reader(&mut session, bus);
    let twice = session.execute("sound.track.input", &json!({ "track": reader, "bus": bus }));
    assert!(twice.is_err(), "the same bus twice is refused");
    session.execute("sound.bus.add", &json!({ "name": "other" })).expect("the bus is added");
    let other = session.document().node_by_name("other").expect("the bus is there").id;
    let second = session.execute("sound.track.input", &json!({ "track": reader, "bus": other }));
    let message = match second {
        Ok(_) => panic!("a second bus is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("already reads"), "the refusal says why: {message}");
}
