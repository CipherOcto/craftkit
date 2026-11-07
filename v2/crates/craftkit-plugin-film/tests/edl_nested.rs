//! The **nested clip** in the **EDL** of a sequence.
//!
//! A nested clip has no tape and no file: its reel is `AX`, the tree's
//! own rule, and its name is written in a comment.

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

/// The **EDL** of a sequence, as the command writes it.
fn edl_of(session: &mut Session, sequence: u64) -> String {
    let outcome = session
        .execute("film.export.edl", &json!({ "sequence": sequence, "title": "the list" }))
        .expect("the list is written");
    outcome.data["edl"].as_str().expect("the list is a text").to_string()
}

/// A nested clip is written with the reel **`AX`**, the tree's rule:
/// it has no tape and no file.
#[test]
fn a_nested_clip_has_the_reel_ax() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, sequence, 0, 10);
    let second = a_clip(&mut session, track, sequence, 10, 10);
    session
        .execute("film.clip.nest", &json!({ "clips": [first, second], "name": "the nest" }))
        .expect("the clips are nested");
    let edl = edl_of(&mut session, sequence);
    let event = edl.lines().find(|line| line.starts_with("001")).expect("there is an event");
    assert!(event.contains("AX"), "the reel of the nested clip is AX: {edl}");
    assert!(event.contains("V"), "and it is on the video channel");
}

/// The **name** of a nested clip is written in a comment, because the
/// reel `AX` says nothing of it.
#[test]
fn the_name_of_a_nested_clip_is_in_a_comment() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, sequence, 0, 10);
    let second = a_clip(&mut session, track, sequence, 10, 10);
    session
        .execute("film.clip.nest", &json!({ "clips": [first, second], "name": "the nest" }))
        .expect("the clips are nested");
    let edl = edl_of(&mut session, sequence);
    assert!(edl.contains("* FROM CLIP NAME: the nest"), "the name of the nested clip is in a comment: {edl}");
}

/// A clip that is **not nested** is still an event with the reel of
/// its item.
#[test]
fn a_clip_that_is_not_nested_has_the_reel_of_its_item() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, sequence, 0, 10);
    let edl = edl_of(&mut session, sequence);
    let event = edl.lines().find(|line| line.starts_with("001")).expect("there is an event");
    assert!(event.contains("THE_MEDI"), "the reel is the name of the item, in eight characters: {edl}");
    assert!(!edl.contains("AX"), "and there is no AX: {edl}");
}
