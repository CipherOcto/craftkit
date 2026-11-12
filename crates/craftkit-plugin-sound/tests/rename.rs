//! The **name** of a track and of a clip.
//!
//! The name is what a person calls the thing, and it is not the
//! thing's identity: the id is the identity and the name is a label.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-rename-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of one frame.
fn a_sound(path: &str) {
    let bytes = craftkit_wav::write(1, 48_000, &[0.5]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track called `drums` and a clip on it.
fn a_track(session: &mut Session, directory: &str) -> (u64, u64) {
    let path = format!("{directory}/sound.wav");
    a_sound(&path);
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
    let clip =
        session.document().walk().find(|node| node.kind == "sound.clip").expect("the clip is there").id;
    (track, clip)
}

/// The **name** of a node.
fn name_of(session: &Session, node: u64) -> Option<String> {
    session.document().node(node).map(|node| node.name.clone())
}

/// A track is **renamed**.
#[test]
fn a_track_is_renamed() {
    let mut session = session();
    let directory = directory("track");
    let (track, _) = a_track(&mut session, &directory);
    session
        .execute("sound.track.rename", &json!({ "track": track, "name": "snare" }))
        .expect("the track is renamed");
    assert_eq!(name_of(&session, track), Some("snare".to_string()));
}

/// A clip is **renamed**.
#[test]
fn a_clip_is_renamed() {
    let mut session = session();
    let directory = directory("clip");
    let (_, clip) = a_track(&mut session, &directory);
    session
        .execute("sound.clip.rename", &json!({ "clip": clip, "name": "kick" }))
        .expect("the clip is renamed");
    assert_eq!(name_of(&session, clip), Some("kick".to_string()));
}

/// The name is **trimmed**, and a name of spaces is **no name**.
#[test]
fn an_empty_name_is_refused() {
    let mut session = session();
    let directory = directory("empty");
    let (track, _) = a_track(&mut session, &directory);
    let error = session
        .execute("sound.track.rename", &json!({ "track": track, "name": "   " }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("not an empty name"), "the error says why: {error}");
    assert_eq!(name_of(&session, track), Some("drums".to_string()), "and the name is as it was");
}

/// A name that is **too long** is refused: the tree takes the first
/// two hundred and fifty-six characters and throws the rest away, and
/// a name that was cut in half is not the name that was asked for.
#[test]
fn a_long_name_is_refused() {
    let mut session = session();
    let directory = directory("long");
    let (track, _) = a_track(&mut session, &directory);
    let long = "x".repeat(257);
    let error = session
        .execute("sound.track.rename", &json!({ "track": track, "name": long }))
        .expect_err("the name is refused");
    assert!(error.to_string().contains("at most 256"), "the error says why: {error}");
    // The name at the bound is fine.
    let exact = "x".repeat(256);
    session
        .execute("sound.track.rename", &json!({ "track": track, "name": exact }))
        .expect("the name is taken");
    assert_eq!(name_of(&session, track).map(|name| name.len()), Some(256));
}

/// Two tracks may have the **same name**: the name is a label and the
/// id is the identity.
#[test]
fn two_tracks_may_have_the_same_name() {
    let mut session = session();
    let directory = directory("same");
    let (track, _) = a_track(&mut session, &directory);
    session.execute("sound.track.add", &json!({ "name": "bass" })).expect("the track is added");
    let bass = session.document().node_by_name("bass").expect("the track is there").id;
    session
        .execute("sound.track.rename", &json!({ "track": bass, "name": "drums" }))
        .expect("the track is renamed");
    assert_eq!(name_of(&session, bass), Some("drums".to_string()));
    assert_eq!(name_of(&session, track), Some("drums".to_string()));
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let (track, _) = a_track(&mut session, &directory);
    let error = session
        .execute("sound.track.rename", &json!({ "track": track + 1_000_000, "name": "x" }))
        .expect_err("the rename is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
