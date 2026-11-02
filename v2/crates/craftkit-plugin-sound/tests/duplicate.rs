//! The **duplicate** of a track: a copy of the track, next to it.
//!
//! The tree's editor duplicates a track: the copy has the same clips,
//! the same automation, the same inserts and the same sends, and it
//! stands where the old one stands.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-duplicate-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **sound** of one frame.
fn a_sound(path: &str) {
    let bytes = craftkit_wav::write(1, 48_000, &[0.5]);
    std::fs::write(path, bytes).expect("the sound is written");
}

/// A session with a track, a sound on it, a lane and a bus.
fn a_track(session: &mut Session, directory: &str) -> u64 {
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
    session
        .execute("sound.lane.add", &json!({ "track": track, "param": "volume", "points": [[0, 1.0]] }))
        .expect("the lane is added");
    session.execute("sound.bus.add", &json!({ "name": "reverb" })).expect("the bus is added");
    let bus = session.document().node_by_name("reverb").expect("the bus is there").id;
    session
        .execute("sound.send.add", &json!({ "track": track, "bus": bus, "level_db": -3.0 }))
        .expect("the send is added");
    track
}

/// The **children** of a node of a kind.
fn children_of(session: &Session, node: u64, kind: &str) -> Vec<String> {
    let node = session.document().node(node).expect("the node is there");
    node.children.iter().filter(|child| child.kind == kind).map(|child| child.name.clone()).collect()
}

/// The **clips** of a track: they are under its takes.
fn clips_of(session: &Session, track: u64) -> Vec<String> {
    let node = session.document().node(track).expect("the track is there");
    let mut names = Vec::new();
    for playlist in node.children.iter().filter(|child| child.kind == "sound.playlist") {
        for clip in playlist.children.iter().filter(|child| child.kind == "sound.clip") {
            names.push(clip.name.clone());
        }
    }
    names
}

/// The duplicate of a track has the **clips**, the **automation**, the
/// **inserts** and the **sends** of the old one.
#[test]
fn a_duplicate_carries_the_track() {
    let mut session = session();
    let directory = directory("carries");
    let track = a_track(&mut session, &directory);
    let outcome = session
        .execute("sound.track.duplicate", &json!({ "track": track }))
        .expect("the track is duplicated");
    let duplicate = outcome.data["duplicate"].as_u64().expect("the copy is there");

    assert_eq!(outcome.data["clips"], json!(1), "the clip is copied");
    assert_eq!(outcome.data["lanes"], json!(1), "the lane is copied");
    assert_eq!(outcome.data["sends"], json!(1), "the send is copied");
    assert_eq!(clips_of(&session, duplicate).len(), 1, "the copy has the clip");
    assert_eq!(children_of(&session, duplicate, "sound.lane").len(), 1, "and the lane");
    assert_eq!(children_of(&session, duplicate, "sound.send").len(), 1, "and the send");
}

/// The copy is a **new** track: its clips are new clips, and they
/// play the same source.
#[test]
fn the_copy_is_a_new_track() {
    let mut session = session();
    let directory = directory("new");
    let track = a_track(&mut session, &directory);
    let outcome = session
        .execute("sound.track.duplicate", &json!({ "track": track }))
        .expect("the track is duplicated");
    let duplicate = outcome.data["duplicate"].as_u64().expect("the copy is there");
    assert_ne!(duplicate, track, "it is another track");

    let clips: Vec<u64> =
        session.document().walk().filter(|node| node.kind == "sound.clip").map(|node| node.id).collect();
    assert_eq!(clips.len(), 2, "the clip is copied, and the copy is its own clip");
    let sources: Vec<u64> = clips
        .iter()
        .filter_map(|clip| session.document().node(*clip))
        .filter_map(|clip| clip.data.get("source").and_then(|source| source.as_u64()))
        .collect();
    assert_eq!(sources.len(), 2, "both clips play a source");
    assert_eq!(sources[0], sources[1], "and it is the same source: a sound is a file");
}

/// The copy stands **where the old one stands**.
#[test]
fn the_copy_stands_where_the_old_one_stands() {
    let mut session = session();
    let directory = directory("place");
    let track = a_track(&mut session, &directory);
    session.execute("sound.track.duplicate", &json!({ "track": track })).expect("the track is duplicated");
    let names: Vec<String> = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.track")
        .map(|node| node.name.clone())
        .collect();
    assert_eq!(names, vec!["drums".to_string(), "drums.dup".to_string()]);
}

/// The copy may leave the **playlists** out.
#[test]
fn the_copy_may_leave_the_playlists_out() {
    let mut session = session();
    let directory = directory("no-playlists");
    let track = a_track(&mut session, &directory);
    let outcome = session
        .execute("sound.track.duplicate", &json!({ "track": track, "playlists": false }))
        .expect("the track is duplicated");
    let duplicate = outcome.data["duplicate"].as_u64().expect("the copy is there");
    assert_eq!(outcome.data["clips"], json!(0));
    assert_eq!(clips_of(&session, duplicate).len(), 0, "and there is no clip");
    assert_eq!(children_of(&session, duplicate, "sound.lane").len(), 1, "the lane is still there");
}

/// A **master** is refused: it holds the fader of the mix and not a
/// sound of its own.
#[test]
fn a_master_is_refused() {
    let mut session = session();
    let directory = directory("master");
    a_track(&mut session, &directory);
    session
        .execute("sound.track.add", &json!({ "name": "main", "track_kind": "master" }))
        .expect("the track is added");
    let master = session.document().node_by_name("main").expect("the track is there").id;
    let error = session
        .execute("sound.track.duplicate", &json!({ "track": master }))
        .expect_err("the duplicate is refused");
    assert!(error.to_string().contains("master"), "the error says why: {error}");
}

/// A node that is **not a track** is refused.
#[test]
fn a_node_that_is_not_a_track_is_refused() {
    let mut session = session();
    let directory = directory("not-a-track");
    let track = a_track(&mut session, &directory);
    let error = session
        .execute("sound.track.duplicate", &json!({ "track": track + 1_000_000 }))
        .expect_err("the duplicate is refused");
    assert!(error.to_string().contains("there is no node"), "the error says why: {error}");
}
