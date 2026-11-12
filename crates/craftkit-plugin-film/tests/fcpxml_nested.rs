//! The **nested clip** in the **FCPXML** of a sequence.
//!
//! A nested clip is a media of the tracks it holds, and the clip that
//! stands for it points at that media.

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

/// The **FCPXML** of a sequence, as the command writes it.
fn fcpxml_of(session: &mut Session, sequence: u64) -> (String, Vec<String>) {
    let outcome = session
        .execute("film.export.fcpxml", &json!({ "sequence": sequence, "width": 1920, "height": 1080 }))
        .expect("the FCPXML is written");
    let text = outcome.data["xml"].as_str().expect("the FCPXML is a text").to_string();
    let left_out = outcome.data["left_out"]
        .as_array()
        .expect("the left out is a list")
        .iter()
        .filter_map(|entry| entry.as_str().map(|entry| entry.to_string()))
        .collect();
    (text, left_out)
}

/// A nested clip is written as a **media** that holds the tracks it
/// holds, and the clip points at that media.
#[test]
fn a_nested_clip_is_written_as_a_media() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    let first = a_clip(&mut session, track, sequence, 0, 10);
    let second = a_clip(&mut session, track, sequence, 10, 10);
    session
        .execute("film.clip.nest", &json!({ "clips": [first, second], "name": "the nest" }))
        .expect("the clips are nested");
    let (xml, _left_out) = fcpxml_of(&mut session, sequence);
    assert!(xml.contains("<media id="), "the nested clip is a media: {xml}");
    assert!(xml.contains("<sequence format="), "and the media holds a sequence");
    assert!(xml.contains("the nest"), "and it is named after the nested clip");
    // The **spine** of the media holds the clips that were nested.
    let media = xml.split("<media id=").nth(1).expect("there is a media");
    let media = media.split("</media>").next().expect("the media ends");
    assert!(media.contains("<clip ref="), "and the media holds the clips: {media}");
    assert!(media.contains("duration=\"20/25s\""), "and it lasts as long as they do: {media}");
}

/// A track of a nested clip that is **not the spine** of its media is
/// left out, and the answer says so.
#[test]
fn a_track_that_is_not_the_spine_is_left_out() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    let outcome = session
        .execute(
            "film.track.add",
            &json!({ "name": "the other track", "sequence": sequence, "kind": "audio" }),
        )
        .expect("the track is added");
    let other = outcome.data["id"].as_u64().expect("the track has an id");
    let first = a_clip(&mut session, track, sequence, 0, 10);
    let sound = a_clip(&mut session, other, sequence, 0, 10);
    session
        .execute("film.clip.nest", &json!({ "clips": [first, sound], "name": "the nest" }))
        .expect("the clips are nested");
    let (_xml, left_out) = fcpxml_of(&mut session, sequence);
    assert!(
        left_out.iter().any(|note| note.contains("is not the spine of its media")),
        "the track that is not the spine is left out and said so: {left_out:?}"
    );
}

/// A clip that is **not nested** is still an asset, and the media of
/// a nested clip does not change that.
#[test]
fn a_clip_that_is_not_nested_is_still_an_asset() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, 100);
    a_clip(&mut session, track, sequence, 0, 10);
    let (xml, _left_out) = fcpxml_of(&mut session, sequence);
    assert!(xml.contains("<asset id="), "the clip is an asset: {xml}");
    assert!(!xml.contains("<media id="), "and there is no media: {xml}");
}
