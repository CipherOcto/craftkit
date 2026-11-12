//! The **nested clip** in the **OTIO** of a sequence.
//!
//! A nested clip is written as a `Stack` of the tracks it holds, and
//! the tracks are written in full — OTIO has no references.

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

/// The **OTIO** of a sequence, as the command writes it.
fn otio_of(session: &mut Session, sequence: u64) -> craftkit_sdk::serde_json::Value {
    let outcome =
        session.execute("film.export.otio", &json!({ "sequence": sequence })).expect("the OTIO is written");
    let text = outcome.data["otio"].as_str().expect("the OTIO is a text");
    craftkit_sdk::serde_json::from_str(text).expect("the OTIO is a JSON text")
}

/// The **children** of the first track of the timeline.
fn children_of(timeline: &craftkit_sdk::serde_json::Value) -> &Vec<craftkit_sdk::serde_json::Value> {
    let tracks = timeline["tracks"]["children"].as_array().expect("the tracks are a list");
    tracks[0]["children"].as_array().expect("the children are a list")
}

/// A nested clip is written as a **stack** of the tracks it holds.
#[test]
fn a_nested_clip_is_written_as_a_stack() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, sequence, 0, 10);
    let second = a_clip(&mut session, track, sequence, 10, 10);
    session
        .execute("film.clip.nest", &json!({ "clips": [first, second], "name": "the nest" }))
        .expect("the clips are nested");
    let timeline = otio_of(&mut session, sequence);
    let children = children_of(&timeline);
    assert_eq!(children.len(), 1, "the track holds the nested clip: {timeline}");
    let stack = &children[0];
    assert_eq!(stack["OTIO_SCHEMA"], json!("Stack.1"), "and it is a stack: {stack}");
    let tracks = stack["children"].as_array().expect("the stack holds tracks");
    assert_eq!(tracks.len(), 1, "the one track the clips were on");
    assert_eq!(tracks[0]["OTIO_SCHEMA"], json!("Track.1"), "and it is a track");
    let clips = tracks[0]["children"].as_array().expect("the track holds clips");
    assert_eq!(clips.len(), 2, "and the clips that were nested are in it");
    assert_eq!(clips[0]["OTIO_SCHEMA"], json!("Clip.2"), "and they are clips");
}

/// The stack says **where** the nested clip begins and how long it
/// is.
#[test]
fn the_stack_says_what_the_nested_clip_lasts() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, sequence, 0, 10);
    let second = a_clip(&mut session, track, sequence, 10, 10);
    session
        .execute("film.clip.nest", &json!({ "clips": [first, second], "name": "the nest" }))
        .expect("the clips are nested");
    let timeline = otio_of(&mut session, sequence);
    let stack = &children_of(&timeline)[0];
    assert_eq!(
        stack["source_range"]["duration"]["value"],
        json!(20.0),
        "the stack lasts as long as the clips"
    );
    assert_eq!(stack["source_range"]["start_time"]["value"], json!(0.0), "and it begins where they do");
}

/// A sequence with **no nested clip** is written as it was: the
/// clips are clips and not stacks.
#[test]
fn a_clip_that_is_not_nested_is_still_a_clip() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, sequence, 0, 10);
    let timeline = otio_of(&mut session, sequence);
    let children = children_of(&timeline);
    assert_eq!(children.len(), 1, "the track holds the clip");
    assert_eq!(children[0]["OTIO_SCHEMA"], json!("Clip.2"), "and it is a clip: {timeline}");
    assert!(children[0].get("source_range").is_some(), "and it says where it begins");
}
