//! The **snap**: a clip that is put against its neighbour.
//!
//! The tree's editor has an *Edit ▸ Snap to* menu: **Next** puts a
//! clip's end against the start of the clip that follows it, and
//! **Previous** puts a clip's start against the end of the clip that
//! comes before it.

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

/// A **track** with three clips on it, and their ids.
fn three_clips(session: &mut Session) -> (u64, Vec<u64>) {
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    let mut clips = Vec::new();
    for start in [0, 300, 1000] {
        session
            .execute("sound.clip.add", &json!({ "track": track, "start": start, "length": 100 }))
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

/// The clip's **end** meets the start of the clip that follows it.
#[test]
fn the_clip_meets_the_next_one() {
    let mut session = session();
    let (track, clips) = three_clips(&mut session);
    session
        .execute("sound.clip.snap", &json!({ "clips": [clips[1]], "direction": "next" }))
        .expect("the clip is snapped");
    assert_eq!(starts_of(&session, track), vec![0, 900, 1000], "the clip meets the next one");
}

/// The clip's **start** meets the end of the clip that comes before it.
#[test]
fn the_clip_meets_the_previous_one() {
    let mut session = session();
    let (track, clips) = three_clips(&mut session);
    session
        .execute("sound.clip.snap", &json!({ "clips": [clips[1]], "direction": "previous" }))
        .expect("the clip is snapped");
    assert_eq!(starts_of(&session, track), vec![0, 100, 1000], "the clip meets the previous one");
}

/// The clips that are **named** are the destinations too.
///
/// The tree's editor found that out the hard way: ignoring the
/// selected neighbours made *Snap to Next* pile every selected clip
/// onto the same later clip.
#[test]
fn the_named_clips_are_the_destinations_too() {
    let mut first = session();
    let (track, clips) = three_clips(&mut first);
    first
        .execute("sound.clip.snap", &json!({ "clips": clips[0..2], "direction": "next" }))
        .expect("the clips are snapped");
    assert_eq!(starts_of(&first, track), vec![800, 900, 1000], "the clips are snapped to each other");
    let mut other = session();
    let (track, clips) = three_clips(&mut other);
    other
        .execute("sound.clip.snap", &json!({ "clips": clips[0..2], "direction": "previous" }))
        .expect("the clips are snapped");
    assert_eq!(starts_of(&other, track), vec![0, 100, 1000], "the clips are snapped to each other");
}

/// A clip with **no neighbour** is not moved, and a direction that is
/// not one of the two is refused by name.
#[test]
fn a_snap_that_cannot_be_done_is_refused() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 100 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    let made = session
        .execute("sound.clip.snap", &json!({ "clips": [clip], "direction": "next" }))
        .expect("the clip is snapped");
    assert_eq!(made.data.get("moved").and_then(|moved| moved.as_u64()), Some(0), "a clip alone is not moved");
    assert_eq!(starts_of(&session, track), vec![0], "and it stays where it is");
    let refused = session.execute("sound.clip.snap", &json!({ "clips": [clip], "direction": "sideways" }));
    let message = match refused {
        Ok(_) => panic!("a direction that is not one of the two is refused"),
        Err(error) => error.to_string(),
    };
    assert!(message.contains("not one of the two"), "the refusal says why: {message}");
}

/// The neighbours are the clips of the clip's **own list**.
///
/// A track may have several playlists — the takes — and a clip in
/// another take is not on this one's timeline.
#[test]
fn the_neighbours_are_the_clips_of_the_same_take() {
    let mut session = session();
    session.execute("sound.track.add", &json!({ "name": "take" })).expect("the track is added");
    let track = session.document().node_by_name("take").expect("the track is there").id;
    session
        .execute("sound.clip.add", &json!({ "track": track, "start": 0, "length": 100 }))
        .expect("the clip is added");
    let clip = session
        .document()
        .walk()
        .find(|node| node.kind == "sound.clip")
        .map(|node| node.id)
        .expect("the clip is there");
    // A **second take** on the same track, with a clip in it.
    session
        .execute("sound.playlist.add", &json!({ "track": track, "name": "alt" }))
        .expect("the take is added");
    session
        .execute("sound.clip.add", &json!({ "track": track, "playlist": "alt", "start": 300, "length": 100 }))
        .expect("the clip is added");
    let made = session
        .execute("sound.clip.snap", &json!({ "clips": [clip], "direction": "next" }))
        .expect("the clip is snapped");
    assert_eq!(
        made.data.get("moved").and_then(|moved| moved.as_u64()),
        Some(0),
        "the other take is not a neighbour"
    );
}
