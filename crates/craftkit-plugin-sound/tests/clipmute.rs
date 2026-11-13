//! A **muted clip** is not played and is still there.
//!
//! The tree's renderer skips a muted clip and the tree's editor keeps
//! it, so that a clip that is quiet is a clip that may be loud again.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-clipmute-{name}")).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// The **amplitude** of the source.
const QUIET: f32 = 0.25;

/// How long the source is, in samples.
const LONG: usize = 1000;

/// A session with a **track** that plays a quiet source, and the clip.
fn a_clip(session: &mut Session, path: &str) -> (u64, u64) {
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
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": LONG, "source": source }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    (track, clip)
}

/// The first sample of what was rendered, and zero when the render was
/// empty: a document whose only clip is muted has nothing to render.
fn rendered(path: &str) -> f32 {
    let written = std::fs::read(path).expect("the sound is on the disk");
    let sound = craftkit_wav::read(&written, &craftkit_wav::Limits::default()).expect("the sound is read");
    sound.samples.first().copied().unwrap_or(0.0)
}

/// A **muted clip** is not heard, and it is still there.
#[test]
fn a_muted_clip_is_not_heard_and_is_still_there() {
    let directory = directory("muted");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, clip) = a_clip(&mut session, &source_path);
    session.execute("sound.clip.mute", &json!({ "clip": clip })).expect("the clip is muted");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    assert!(sample.abs() < 0.01, "the muted clip is not heard: {sample}");
    // The clip is still there: the document still holds it.
    let still = session.document().node(clip).expect("the clip is still there");
    assert_eq!(still.kind, "sound.clip", "the clip is still there");
}

/// A clip that is **unmuted** is heard again.
#[test]
fn a_clip_that_is_unmuted_is_heard_again() {
    let directory = directory("unmuted");
    let source_path = format!("{directory}/source.wav");
    let output_path = format!("{directory}/out.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (_track, clip) = a_clip(&mut session, &source_path);
    session.execute("sound.clip.mute", &json!({ "clip": clip, "muted": true })).expect("the clip is muted");
    session
        .execute("sound.clip.mute", &json!({ "clip": clip, "muted": false }))
        .expect("the clip is not muted");
    session.execute("sound.render", &json!({ "path": output_path })).expect("the sound is rendered");
    let sample = rendered(&output_path);
    assert!((sample - QUIET).abs() < 0.01, "the clip is heard again: {sample}");
}

/// A node that is **not a clip** is refused by name.
#[test]
fn a_node_that_is_not_a_clip_is_refused() {
    let directory = directory("not-a-clip");
    let source_path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &vec![QUIET; LONG]);
    std::fs::write(&source_path, bytes).expect("the source is written");
    let mut session = session();
    let (track, _clip) = a_clip(&mut session, &source_path);
    let refused = session.execute("sound.clip.mute", &json!({ "clip": track }));
    let message = match refused {
        Ok(_) => panic!("a track is not a clip"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("not a clip"), "the refusal says why: {message}");
}
