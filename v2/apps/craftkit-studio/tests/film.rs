//! The film battery: the document model of a **video editor**.
//!
//! A project is three things at once: a tree of bins, a flat registry of
//! items, and a timeline of tracks and clips. A clip has **two clocks**:
//! where it starts on the timeline, and where in the media it begins.

use craftkit_sdk::Ecosystem;
use craftkit_sdk::serde_json::json;

/// The ecosystem the tests drive.
fn ecosystem() -> Ecosystem {
    craftkit_studio::ecosystem().expect("the ecosystem is built")
}

/// A project with a bin, an item, a sequence, a track and a clip.
fn a_project() -> (craftkit_sdk::Session, u64) {
    let mut session = ecosystem().session();
    session.execute("film.bin.add", &json!({ "name": "media" })).expect("the bin is added");
    let bin = 1;
    session
        .execute(
            "film.item.add",
            &json!({ "name": "take one", "bin": bin, "item": 100, "path": "media/take-1.mov", "duration": 250 }),
        )
        .expect("the item is added");
    // A sequence is an item too, and it holds the tracks.
    session
        .execute("film.item.add", &json!({ "name": "main sequence", "bin": bin, "item": 200, "duration": 0 }))
        .expect("the sequence is added");
    let sequence = 3;
    session
        .execute("film.track.add", &json!({ "name": "V1", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let track = 4;
    session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": 2, "start": 48, "duration": 100, "source_in": 12 }),
        )
        .expect("the clip is added");
    (session, track)
}

/// The battery is linked in when the feature is on.
#[cfg(feature = "film")]
#[test]
fn the_film_battery_is_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("film.bin.add").is_some(), "the battery is in");
    assert!(ecosystem.commands.get("film.clip.add").is_some());
    assert!(ecosystem.kinds.get("film.bin").is_some());
    assert!(ecosystem.kinds.get("film.clip").is_some());
    assert!(ecosystem.evaluators.get("film.timeline").is_some());
}

/// And it is not there when the feature is off.
#[cfg(not(feature = "film"))]
#[test]
fn the_film_battery_is_not_linked_in() {
    let ecosystem = ecosystem();
    assert!(ecosystem.commands.get("film.bin.add").is_none(), "the battery is out");
}

/// A bin holds items and other bins.
#[cfg(feature = "film")]
#[test]
fn a_bin_holds_items_and_other_bins() {
    let mut session = ecosystem().session();
    session.execute("film.bin.add", &json!({ "name": "media" })).expect("the bin is added");
    session.execute("film.bin.add", &json!({ "name": "audio", "parent": 1 })).expect("the bin is added");
    let bin = session.document().node(1).expect("the bin is there");
    assert_eq!(bin.children.len(), 1, "the second bin is in the first");
    assert_eq!(bin.children[0].kind, "film.bin");
}

/// A clip has the **two clocks**.
#[cfg(feature = "film")]
#[test]
fn a_clip_has_the_two_clocks() {
    let (session, _) = a_project();
    let clip = session.document().node(5).expect("the clip is there");
    assert_eq!(clip.kind, "film.clip");
    assert_eq!(clip.data["start"], json!(48), "where it starts on the timeline");
    assert_eq!(clip.data["duration"], json!(100), "how long it is");
    assert_eq!(clip.data["source_in"], json!(12), "where in the media it begins");
}

/// A clip says which item it plays, with an edge.
#[cfg(feature = "film")]
#[test]
fn a_clip_says_which_item_it_plays() {
    let (session, _) = a_project();
    let relation = session
        .document()
        .relations
        .iter()
        .find(|relation| relation.kind == "clip")
        .expect("the edge is there");
    assert_eq!(relation.from, 5, "the clip is one end");
    assert_eq!(relation.to, 2, "the item is the other");
}

/// The order of a timeline is **not** the order of the tree.
#[cfg(feature = "film")]
#[test]
fn the_order_of_a_timeline_is_not_the_order_of_the_tree() {
    let (mut session, track) = a_project();
    // A second clip, made *before* the first one on the timeline.
    session
        .execute("film.clip.add", &json!({ "track": track, "item": 2, "start": 0, "duration": 24 }))
        .expect("the clip is added");
    let outcome = session.execute("film.timeline", &json!({})).expect("the timeline is read");
    let clips = outcome.data["tracks"][0]["clips"].as_array().expect("there are clips");
    assert_eq!(clips.len(), 2);
    // The tree has them in the order they were made: the clip at 48 came
    // first. The timeline has them in the order they start.
    assert_eq!(clips[0]["start"], json!(0), "the timeline is in order");
    assert_eq!(clips[1]["start"], json!(48));
}

/// The length of a track is **derived**: it is the end of the last clip.
#[cfg(feature = "film")]
#[test]
fn the_length_of_a_track_is_derived() {
    let (session, _) = a_project();
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "film.timeline").collect();
    assert_eq!(timeline.len(), 1, "one track: {plan:?}");
    assert_eq!(timeline[0].value["end"], json!(148), "48 + 100");
}

/// And it changes when a clip moves.
#[cfg(feature = "film")]
#[test]
fn the_length_changes_when_a_clip_moves() {
    let (mut session, track) = a_project();
    session
        .execute("film.clip.add", &json!({ "track": track, "item": 2, "start": 200, "duration": 24 }))
        .expect("the clip is added");
    let plan = session.evaluate();
    let timeline: Vec<_> = plan.items.iter().filter(|item| item.kind == "film.timeline").collect();
    assert_eq!(timeline[0].value["end"], json!(224), "the longest clip wins");
}

/// A clip that is zero frames long is refused.
#[cfg(feature = "film")]
#[test]
fn a_clip_that_is_zero_frames_long_is_refused() {
    let (mut session, track) = a_project();
    let error = session
        .execute("film.clip.add", &json!({ "track": track, "item": 2, "start": 0, "duration": 0 }))
        .expect_err("the clip has no length");
    assert!(error.to_string().contains("zero frames"), "{error}");
}

/// A clip on a track that is not there is refused.
#[cfg(feature = "film")]
#[test]
fn a_clip_on_a_track_that_is_not_there_is_refused() {
    let (mut session, _) = a_project();
    let error = session
        .execute("film.clip.add", &json!({ "track": 999, "item": 2, "start": 0, "duration": 10 }))
        .expect_err("the track is not there");
    assert!(error.to_string().contains("no node"), "{error}");
}

/// The item's own id is kept, because the ids of the nodes are the core's.
#[cfg(feature = "film")]
#[test]
fn the_items_own_id_is_kept() {
    let (session, _) = a_project();
    let item = session.document().node(2).expect("the item is there");
    assert_eq!(item.data["item"], json!(100), "the editor's own id is in the payload");
}
