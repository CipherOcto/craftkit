//! The **nested clip** of a sequence.
//!
//! A clip that holds other clips: a little timeline of its own,
//! standing on a track of the sequence around it. The tree's compound
//! clip, and the tree's taking of it apart.

use craftkit_sdk::serde_json::json;
use craftkit_sdk::{Ecosystem, Session};

/// The battery's **anchor**: the one name that keeps its commands
/// linked into the program.
#[allow(dead_code)]
static ANCHOR: craftkit_sdk::PluginAnchor = craftkit_plugin_film::CRAFTKIT_PLUGIN_ANCHOR;

/// The ecosystem of the film battery.
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

/// A **bin**, an **item** in it, and a **track** in the item.
fn a_timeline(session: &mut Session, media: u64) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the media", "bin": bin, "item": 1, "duration": media }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **clip** on a track.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": start,
                "duration": duration,
                "source_in": 0,
            }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The **start** of a clip.
fn start_of(session: &Session, clip: u64) -> u64 {
    session.document().node(clip).expect("the clip is there").data["start"].as_u64().unwrap_or(0)
}

/// The clips that are **on a track**.
fn clips_on(session: &Session, track: u64) -> Vec<u64> {
    session
        .document()
        .node(track)
        .map(|node| {
            node.children.iter().filter(|child| child.kind == "film.clip").map(|child| child.id).collect()
        })
        .unwrap_or_default()
}

/// The clips are **nested**: the clip that is made holds them, and it
/// stands where the earliest of them stood.
#[test]
fn the_clips_are_nested() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, item, 10, 20);
    let second = a_clip(&mut session, track, item, 30, 10);
    let outcome = session
        .execute("film.clip.nest", &json!({ "clips": [first, second], "name": "the nest" }))
        .expect("the clips are nested");
    let compound = outcome.data["id"].as_u64().expect("the nested clip has an id");
    assert_eq!(outcome.data["start"], json!(10), "the nested clip starts where the first clip did");
    assert_eq!(outcome.data["duration"], json!(30), "and it lasts as long as both of them");
    assert_eq!(outcome.data["members"], json!([first, second]), "and it holds the two clips");
    // The clips are **inside** it, and their starts are the starts
    // they have inside it.
    let node = session.document().node(compound).expect("the nested clip is there");
    let tracks: Vec<u64> =
        node.children.iter().filter(|child| child.kind == "film.track").map(|c| c.id).collect();
    assert_eq!(tracks.len(), 1, "the nested clip holds one track");
    let nested = session.document().node(tracks[0]).expect("the nested track is there");
    let members: Vec<u64> =
        nested.children.iter().filter(|child| child.kind == "film.clip").map(|c| c.id).collect();
    assert_eq!(members, vec![first, second], "and the track holds the two clips");
    assert_eq!(start_of(&session, first), 0, "the first clip starts at the beginning of the nest");
    assert_eq!(start_of(&session, second), 20, "and the second one where it was");
}

/// The clips of **two tracks** are nested: the nested clip holds a
/// track for each of them.
#[test]
fn the_clips_of_two_tracks_are_nested() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": item, "kind": "audio" }))
        .expect("the second track is added");
    let other = outcome.data["id"].as_u64().expect("the track has an id");
    let picture = a_clip(&mut session, track, item, 10, 20);
    let sound = a_clip(&mut session, other, item, 10, 20);
    let outcome = session
        .execute("film.clip.nest", &json!({ "clips": [picture, sound], "name": "the nest" }))
        .expect("the clips are nested");
    let compound = outcome.data["id"].as_u64().expect("the nested clip has an id");
    let tracks = outcome.data["tracks"].as_array().expect("the tracks are listed");
    assert_eq!(tracks.len(), 2, "the nested clip holds a track for each of them");
    let node = session.document().node(compound).expect("the nested clip is there");
    let held: Vec<u64> =
        node.children.iter().filter(|child| child.kind == "film.track").map(|c| c.id).collect();
    assert_eq!(held, tracks.as_slice().to_vec(), "and they are the tracks it holds");
}

/// A clip that holds **nothing** is refused.
#[test]
fn a_clip_that_holds_nothing_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20);
    let error = session
        .execute("film.clip.nest", &json!({ "clips": [], "name": "the nest" }))
        .expect_err("the nesting is refused");
    assert!(error.to_string().contains("holds nothing"), "the refusal says why: {error}");
    // The clip is where it was.
    assert_eq!(clips_on(&session, track), vec![clip], "the clip is still on the track");
}

/// The clips of **two sequences** are refused: there is nowhere to put
/// the clip that would hold them.
#[test]
fn the_clips_of_two_sequences_are_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let (other_item, other_track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, item, 10, 20);
    let second = a_clip(&mut session, other_track, other_item, 10, 20);
    let error = session
        .execute("film.clip.nest", &json!({ "clips": [first, second], "name": "the nest" }))
        .expect_err("the nesting is refused");
    assert!(error.to_string().contains("sequence"), "the refusal says why: {error}");
}

/// A **locked** track refuses the nesting.
#[test]
fn a_locked_track_refuses_the_nesting() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20);
    session
        .execute("film.track.lock", &json!({ "track": track, "locked": true }))
        .expect("the track is locked");
    let error = session
        .execute("film.clip.nest", &json!({ "clips": [clip], "name": "the nest" }))
        .expect_err("the nesting is refused");
    assert!(error.to_string().contains("is locked"), "the refusal says why: {error}");
}

/// The nested clip is **taken apart**: the clips go back to the tracks
/// they came from, at the places they had.
#[test]
fn the_nested_clip_is_taken_apart() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, item, 10, 20);
    let second = a_clip(&mut session, track, item, 30, 10);
    let outcome = session
        .execute("film.clip.nest", &json!({ "clips": [first, second], "name": "the nest" }))
        .expect("the clips are nested");
    let compound = outcome.data["id"].as_u64().expect("the nested clip has an id");
    session
        .execute("film.clip.ungroup", &json!({ "clip": compound }))
        .expect("the nested clip is taken apart");
    let clips = clips_on(&session, track);
    assert_eq!(clips.len(), 2, "the two clips are on the track again");
    assert!(clips.contains(&first) && clips.contains(&second), "and they are the two of them");
    assert_eq!(start_of(&session, first), 10, "the first clip is where it was");
    assert_eq!(start_of(&session, second), 30, "and the second one too");
    assert!(session.document().node(compound).is_none(), "and the nested clip is gone");
}

/// A clip that holds **no clips** is not taken apart.
#[test]
fn a_clip_that_holds_no_clips_is_refused() {
    let mut session = session();
    let (item, track) = a_timeline(&mut session, 100);
    let clip = a_clip(&mut session, track, item, 10, 20);
    let error = session
        .execute("film.clip.ungroup", &json!({ "clip": clip }))
        .expect_err("the taking apart is refused");
    assert!(error.to_string().contains("holds no clips"), "the refusal says why: {error}");
}
