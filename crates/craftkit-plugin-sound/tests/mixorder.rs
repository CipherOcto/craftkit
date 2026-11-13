//! The **order of the mix**: the buses and the readers, in the order
//! they are mixed.
//!
//! A bus is mixed before the readers that read it, and a reader is
//! mixed before the bus it feeds. A reader that reads a bus and feeds
//! another is the tree's chain of auxes, and it is heard at the end of
//! the chain and not at the start.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-mixorder-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source: quiet enough that the track, the
/// bus and the readers stay inside the file.
const QUIET: f32 = 0.25;

/// A session with a **track** that plays a quiet source and sends it
/// to a bus called `fx`.
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

/// A **reader**: a track that reads a bus, with a fader at the level
/// that is given.
fn a_reader(session: &mut Session, name: &str, bus: u64, decibels: f64) -> u64 {
    session.execute("sound.track.add", &json!({ "name": name })).expect("the reader is added");
    let reader = session.document().node_by_name(name).expect("the reader is there").id;
    session
        .execute("sound.track.input", &json!({ "track": reader, "bus": bus }))
        .expect("the reader reads the bus");
    session
        .execute("sound.lane.add", &json!({ "track": reader, "param": "volume", "points": [[0, decibels]] }))
        .expect("the fader is there");
    reader
}

/// The first sample of what was rendered.
fn rendered(path: &str) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples[0]
}

/// A reader that reads a bus and feeds another is heard at the **end**
/// of the chain: the bus is read before it is fed.
#[test]
fn a_reader_that_feeds_a_bus_is_heard_at_the_end_of_the_chain() {
    let directory = directory("chain");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, fx) = a_track_and_a_bus(&mut session, &source_path);
    // The first reader reads `fx`, and feeds `out`.
    let first = a_reader(&mut session, "return", fx, -6.0);
    session.execute("sound.bus.add", &json!({ "name": "out" })).expect("the bus is added");
    let out = session.document().node_by_name("out").expect("the bus is there").id;
    session
        .execute("sound.route.add", &json!({ "track": first, "bus": out }))
        .expect("the reader feeds the bus");
    // The second reader reads `out`, and is heard in the mix.
    a_reader(&mut session, "final", out, 0.0);
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    // The track is heard whole, and the chain is heard at the level
    // the first reader's fader gave it. If the mix were in the wrong
    // order, the last reader would have read the bus before it was
    // fed, and the sum would be the track alone.
    let expected = QUIET + QUIET * 10.0_f32.powf(-6.0 / 20.0);
    assert!((sample - expected).abs() < 0.01, "the chain is heard at its end: {sample}");
}

/// A mix that **loops** — a reader that feeds the bus it reads — is
/// refused by name.
#[test]
fn a_mix_that_loops_is_refused() {
    let directory = directory("loop");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[QUIET, QUIET]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, fx) = a_track_and_a_bus(&mut session, &source_path);
    let reader = a_reader(&mut session, "return", fx, 0.0);
    // The reader feeds the bus it reads: the mix would never settle.
    session
        .execute("sound.route.add", &json!({ "track": reader, "bus": fx }))
        .expect("the reader feeds the bus");
    let refused = session.execute("sound.render", &json!({ "path": format!("{directory}/out.wav") }));
    let message = match refused {
        Ok(_) => panic!("a mix that loops is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("loops"), "the refusal says why: {message}");
}
