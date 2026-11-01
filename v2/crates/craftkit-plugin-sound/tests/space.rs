//! The **spacing** of the clips: each one butted against the last.
//!
//! The tree's editor lays the pieces of a take out by hand, and the
//! spacing command does it: the clips are moved so that each one
//! starts where the last one ended.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-space-{name}"));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("the directory is made");
    path.to_string_lossy().to_string()
}

/// A **source** of two samples, and the file it is in.
fn a_source(session: &mut Session, directory: &str) -> u64 {
    let path = format!("{directory}/source.wav");
    let bytes = craftkit_wav::write(1, 48_000, &[0.25, 0.25]);
    std::fs::write(&path, bytes).expect("the source is written");
    session.execute("sound.source.add", &json!({ "path": path })).expect("the source is added");
    session
        .document()
        .walk()
        .find(|node| node.kind == "sound.source")
        .map(|node| node.id)
        .expect("the source is there")
}

/// A **track** with three clips, and their ids.
fn three_clips(session: &mut Session, directory: &str) -> (u64, Vec<u64>) {
    let source = a_source(session, directory);
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let mut clips = Vec::new();
    for start in [0, 500, 2000] {
        session
            .execute(
                "sound.clip.add",
                &json!({ "track": track, "start": start, "length": 100, "source": source }),
            )
            .expect("the clip is added");
        let id = session
            .document()
            .walk()
            .filter(|node| node.kind == "sound.clip")
            .last()
            .map(|node| node.id)
            .expect("the clip is there");
        clips.push(id);
    }
    (track, clips)
}

/// Where the clips of a track **start**.
///
/// A clip is under a playlist and a playlist is under a track, so the
/// track is the parent or the grandparent.
fn starts_of(session: &Session, track: u64) -> Vec<i64> {
    let document = session.document();
    let parent_of = |id: u64| {
        document.walk().find(|node| node.children.iter().any(|child| child.id == id)).map(|node| node.id)
    };
    let mut starts: Vec<(i64, u64)> = Vec::new();
    for node in document.walk().filter(|node| node.kind == "sound.clip") {
        let Some(owner) = parent_of(node.id) else { continue };
        let owner = if owner == track { owner } else { parent_of(owner).unwrap_or(owner) };
        if owner != track {
            continue;
        }
        let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        starts.push((start, node.id));
    }
    starts.sort();
    starts.into_iter().map(|(start, _)| start).collect()
}

/// The clips are **butted** against each other, with a gap between
/// them.
#[test]
fn the_clips_are_butted_against_each_other() {
    let directory = directory("space");
    let mut session = session();
    let (track, clips) = three_clips(&mut session, &directory);
    session.execute("sound.clip.space", &json!({ "clips": clips, "gap": 10 })).expect("the clips are spaced");
    assert_eq!(starts_of(&session, track), vec![0, 110, 220], "the clips are butted together");
}

/// The clips may be spaced **start to start**: their starts are the
/// same distance apart, and their lengths are their own.
#[test]
fn the_clips_may_be_spaced_start_to_start() {
    let directory = directory("starts");
    let mut session = session();
    let (track, clips) = three_clips(&mut session, &directory);
    session
        .execute("sound.clip.space", &json!({ "clips": clips, "gap": 50, "mode": "start-to-start" }))
        .expect("the clips are spaced");
    assert_eq!(starts_of(&session, track), vec![0, 50, 100], "the starts are the same distance apart");
}

/// One clip is not **spaced**, and a gap that is not a gap is refused
/// by name.
#[test]
fn a_spacing_that_cannot_be_done_is_refused() {
    let directory = directory("refused");
    let mut session = session();
    let (_track, clips) = three_clips(&mut session, &directory);
    let refused = session.execute("sound.clip.space", &json!({ "clips": [clips[0]], "gap": 10 }));
    let message = match refused {
        Ok(_) => panic!("one clip is not spaced"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("two or more"), "the refusal says why: {message}");
    let refused = session.execute("sound.clip.space", &json!({ "clips": clips, "gap": -5 }));
    assert!(refused.is_err(), "a negative gap is refused");
}

/// The clips of a track may be put **in front** of the others, or
/// **behind** them.
///
/// The order the clips are drawn in is the order they are in — and the
/// clips of a track are mixed and not laid over one another, so the
/// order is what a person sees and not what a person hears.
#[test]
fn a_clip_may_be_put_in_front_of_the_others() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let mut clips = Vec::new();
    for start in [0, 200, 400] {
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": start, "length": 100 }))
            .expect("the clip is added");
        clips.push(
            session
                .document()
                .walk()
                .filter(|node| node.kind == "sound.clip")
                .last()
                .map(|node| node.id)
                .expect("the clip is there"),
        );
    }
    session
        .execute("sound.clip.order", &json!({ "clip": clips[0], "place": "front" }))
        .expect("the clip is put in front");
    let drawn: Vec<u64> =
        session.document().walk().filter(|node| node.kind == "sound.clip").map(|node| node.id).collect();
    assert_eq!(drawn, vec![clips[1], clips[2], clips[0]], "the clip is drawn last");
    let refused = session.execute("sound.clip.order", &json!({ "clip": clips[0], "place": "middle" }));
    assert!(refused.is_err(), "a clip goes to the front or to the back");
}
