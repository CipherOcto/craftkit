//! The **loop**: the copies of a clip that follow it.
//!
//! The tree's editor pastes a copy of the clip onto the track and
//! remembers the loop in a field on the clip. This tree says what the
//! tree can only infer: the copy is **named by the clip it follows**,
//! in a relation, so that the loop can be found again without guessing.

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
    let path = craftkit_scratch::dir(&format!("craftkit-v2-loop-{name}")).expect("the directory is made");
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

/// A **track** with one clip on it, and the ids of the two.
fn one_clip(session: &mut Session, directory: &str, start: i64, length: i64) -> (u64, u64) {
    let source = a_source(session, directory);
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute(
            "sound.clip.add",
            &json!({ "track": track, "start": start, "length": length, "source": source }),
        )
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    (track, clip)
}

/// The clips of a track, in the order they start, with their spans.
fn clips_of(session: &Session, track: u64) -> Vec<(i64, i64)> {
    let document = session.document();
    let parent_of = |id: u64| {
        document.walk().find(|node| node.children.iter().any(|child| child.id == id)).map(|node| node.id)
    };
    let mut spans: Vec<(i64, i64, u64)> = Vec::new();
    for node in document.walk().filter(|node| node.kind == "sound.clip") {
        let Some(owner) = parent_of(node.id) else { continue };
        let owner = if owner == track { owner } else { parent_of(owner).unwrap_or(owner) };
        if owner != track {
            continue;
        }
        let start = node.data.get("start").and_then(|start| start.as_i64()).unwrap_or(0);
        let length = node.data.get("length").and_then(|length| length.as_i64()).unwrap_or(0);
        spans.push((start, length, node.id));
    }
    spans.sort();
    spans.into_iter().map(|(start, length, _)| (start, length)).collect()
}

/// The **loop** edges of a clip: the clips it is followed by.
fn loop_edges(session: &Session, clip: u64) -> Vec<u64> {
    session.document().relations_from(clip, Some("loop")).into_iter().map(|relation| relation.to).collect()
}

/// The copies are made, and each one **follows** the one before it.
#[test]
fn the_copies_are_made_and_they_are_linked() {
    let directory = directory("copies");
    let mut session = session();
    let (track, clip) = one_clip(&mut session, &directory, 0, 100);
    let made =
        session.execute("sound.clip.loop", &json!({ "clip": clip, "count": 3 })).expect("the clip is looped");
    assert_eq!(made.data.get("loops").and_then(|loops| loops.as_u64()), Some(2), "two copies are made");
    assert_eq!(
        clips_of(&session, track),
        vec![(0, 100), (100, 100), (200, 100)],
        "the copies follow the clip"
    );
    assert_eq!(loop_edges(&session, clip).len(), 1, "the clip is followed by one copy");
}

/// The last copy is **cut short** to fit the length that was asked for.
#[test]
fn the_last_copy_is_cut_short() {
    let directory = directory("cut");
    let mut session = session();
    let (track, clip) = one_clip(&mut session, &directory, 0, 100);
    session.execute("sound.clip.fade", &json!({ "clip": clip, "fade_out": 90 })).expect("the clip is faded");
    session.execute("sound.clip.loop", &json!({ "clip": clip, "length": 250 })).expect("the clip is looped");
    assert_eq!(
        clips_of(&session, track),
        vec![(0, 100), (100, 100), (200, 50)],
        "the last copy is cut short"
    );
    // The **fade** is shortened to fit: a fade past the end of the clip
    // is a fade that is not heard.
    let last = session
        .document()
        .walk()
        .filter(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .last()
        .expect("the last clip is there");
    let fade = session.document().node(last).expect("the clip is there").data.get("fade_out").cloned();
    let fade = fade.expect("the fade is there");
    assert_eq!(fade.get("len").and_then(|len| len.as_i64()), Some(50), "the fade is cut to fit");
}

/// The loop **fills** to the next clip on the track.
#[test]
fn the_loop_fills_to_the_next_clip() {
    let directory = directory("fill");
    let mut session = session();
    let (track, clip) = one_clip(&mut session, &directory, 0, 100);
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 350, "length": 50 }))
        .expect("the next clip is added");
    session.execute("sound.clip.loop", &json!({ "clip": clip, "fill": true })).expect("the clip is looped");
    assert_eq!(
        clips_of(&session, track),
        vec![(0, 100), (100, 100), (200, 100), (300, 50), (350, 50)],
        "the loop fills up to the next clip, and the next clip is still there"
    );
}

/// The copies are **taken away**, and the edges go with them.
#[test]
fn the_loop_is_taken_away() {
    let directory = directory("unloop");
    let mut session = session();
    let (track, clip) = one_clip(&mut session, &directory, 0, 100);
    session.execute("sound.clip.loop", &json!({ "clip": clip, "count": 3 })).expect("the clip is looped");
    let made =
        session.execute("sound.clip.unloop", &json!({ "clip": clip })).expect("the loop is taken away");
    assert_eq!(made.data.get("copies").and_then(|copies| copies.as_u64()), Some(2), "two copies go");
    assert_eq!(clips_of(&session, track), vec![(0, 100)], "and only the clip is left");
    assert!(loop_edges(&session, clip).is_empty(), "and the edges go with them");
}

/// The copies may **stay**: they are clips in their own right, and
/// nothing says they are a loop.
#[test]
fn the_copies_may_stay() {
    let directory = directory("flatten");
    let mut session = session();
    let (track, clip) = one_clip(&mut session, &directory, 0, 100);
    session.execute("sound.clip.loop", &json!({ "clip": clip, "count": 3 })).expect("the clip is looped");
    let made = session
        .execute("sound.clip.unloop", &json!({ "clip": clip, "mode": "flatten" }))
        .expect("the loop is taken away");
    assert_eq!(
        made.data.get("removed").and_then(|removed| removed.as_bool()),
        Some(false),
        "nothing is removed"
    );
    assert_eq!(clips_of(&session, track), vec![(0, 100), (100, 100), (200, 100)], "the copies stay");
    assert!(loop_edges(&session, clip).is_empty(), "and the edge is gone");
}

/// A clip that is not looped is **refused**, and a loop that is asked
/// for twice is refused too.
#[test]
fn a_loop_that_cannot_be_made_is_refused() {
    let directory = directory("refused");
    let mut session = session();
    let (_track, clip) = one_clip(&mut session, &directory, 0, 100);
    let refused = session.execute("sound.clip.unloop", &json!({ "clip": clip }));
    let message = match refused {
        Ok(_) => panic!("a clip that is not looped is not unlooped"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("not a looped clip"), "the refusal says why: {message}");
    // A loop of more than a thousand times is refused by name.
    let refused = session.execute("sound.clip.loop", &json!({ "clip": clip, "count": 5000 }));
    let message = match refused {
        Ok(_) => panic!("a clip loops at most a thousand times"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("1000 times"), "the refusal says why: {message}");
    // A clip that is already followed by a copy may not be looped
    // again: the chain is a line and not a tree.
    session.execute("sound.clip.loop", &json!({ "clip": clip, "count": 2 })).expect("the clip is looped");
    let refused = session.execute("sound.clip.loop", &json!({ "clip": clip, "count": 2 }));
    assert!(refused.is_err(), "a clip that is already looped is refused");
}
