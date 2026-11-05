//! The **FCPXML** of a sequence.

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

/// A **bin**, an **item** in it, a **track** in the item.
fn a_timeline(session: &mut Session, name: &str) -> (u64, u64) {
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": name, "bin": bin, "item": 1, "duration": 600 }))
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": item, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    (item, track)
}

/// A **clip** on a track.
fn a_clip(session: &mut Session, track: u64, item: u64, start: u64, duration: u64, source_in: u64) -> u64 {
    let outcome = session
        .execute(
            "film.clip.add",
            &json!({
                "track": track,
                "item": item,
                "start": start,
                "duration": duration,
                "source_in": source_in,
            }),
        )
        .expect("the clip is added");
    outcome.data["id"].as_u64().expect("the clip has an id")
}

/// The **text** of the FCPXML of a sequence.
fn xml_of(session: &mut Session, sequence: u64) -> String {
    let outcome = session
        .execute("film.export.fcpxml", &json!({ "sequence": sequence, "width": 1920, "height": 1080 }))
        .expect("the text is written");
    outcome.data["xml"].as_str().expect("the text is a text").to_string()
}

/// What was **left out**.
fn left_out_of(session: &mut Session, sequence: u64) -> Vec<String> {
    let outcome = session
        .execute("film.export.fcpxml", &json!({ "sequence": sequence, "width": 1920, "height": 1080 }))
        .expect("the text is written");
    outcome.data["left_out"]
        .as_array()
        .expect("the list is a list")
        .iter()
        .map(|entry| entry.as_str().unwrap_or_default().to_string())
        .collect()
}

/// **The text is a document of the format.**
#[test]
fn the_text_is_a_document_of_the_format() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let text = xml_of(&mut session, sequence);
    assert!(text.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE fcpxml>\n"), "{text}");
    assert!(text.contains("<fcpxml version=\"1.9\">"), "{text}");
    assert!(text.contains("</fcpxml>"), "{text}");
}

/// **The format is the rate of the sequence and the size the caller
/// says.**
#[test]
fn the_format_is_the_rate_and_the_size() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "25" }))
        .expect("the rate is set");
    a_clip(&mut session, track, sequence, 0, 25, 0);
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("frameDuration=\"1/25s\""), "{text}");
    assert!(text.contains("width=\"1920\""), "{text}");
    assert!(text.contains("height=\"1080\""), "{text}");
}

/// **The spine is the first video track, and the clip is on it.**
#[test]
fn the_spine_is_the_first_video_track() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("<spine>"), "{text}");
    assert!(
        text.contains(
            "<clip ref=\"r2\" name=\"clip at 0\" offset=\"0s\" start=\"0s\" duration=\"125/25s\"/>"
        ),
        "{text}"
    );
}

/// **The times are rational and counted in frames.**
#[test]
fn the_times_are_counted_in_frames() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    session
        .execute("film.sequence.rate.set", &json!({ "sequence": sequence, "rate": "30000/1001" }))
        .expect("the rate is set");
    a_clip(&mut session, track, sequence, 0, 30, 0);
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("duration=\"30030/30000s\""), "{text}");
}

/// **The hole between two clips is a gap.**
#[test]
fn the_hole_between_two_clips_is_a_gap() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 25, 0);
    a_clip(&mut session, track, sequence, 50, 25, 0);
    let text = xml_of(&mut session, sequence);
    assert!(
        text.contains("<gap name=\"Gap\" offset=\"25/25s\" start=\"25/25s\" duration=\"25/25s\"/>"),
        "{text}"
    );
}

/// **A clip on another track is a connected clip, on a lane.**
#[test]
fn a_clip_on_another_track_is_a_connected_clip() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the second", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let second = outcome.data["id"].as_u64().expect("the track has an id");
    a_clip(&mut session, second, sequence, 0, 50, 0);
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("lane=\"1\""), "the second video track is on lane one: {text}");
}

/// **An audio track is on a lane below zero.**
#[test]
fn an_audio_track_is_on_a_lane_below_zero() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": sequence, "kind": "audio" }))
        .expect("the track is added");
    let sound = outcome.data["id"].as_u64().expect("the track has an id");
    a_clip(&mut session, sound, sequence, 0, 50, 0);
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("lane=\"-1\""), "the audio track is on lane minus one: {text}");
}

/// **A clip that is retimed says where its source goes.**
#[test]
fn a_clip_that_is_retimed_says_where_its_source_goes() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let clip = a_clip(&mut session, track, sequence, 0, 25, 0);
    session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 2.0 })).expect("the speed is set");
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("<timeMap>"), "{text}");
    assert!(text.contains("value=\"0s\""), "{text}");
    assert!(text.contains("value=\"26/25s\""), "the source is twice as long as the clip: {text}");
}

/// **A transition of the spine is written, and one of another track is
/// left out and said.**
#[test]
fn a_transition_of_the_spine_is_written() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let first = a_clip(&mut session, track, sequence, 0, 25, 0);
    let second = a_clip(&mut session, track, sequence, 25, 25, 0);
    session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": first, "to": second, "duration": 12 }),
        )
        .expect("the transition is added");
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("<transition name=\"cross_dissolve\""), "{text}");
    assert!(text.contains("duration=\"12/25s\""), "{text}");
    assert!(text.contains("<filter-video ref=\"r3\""), "{text}");
}

/// **A transition on a track that is not the spine is left out, and the
/// answer says so.**
#[test]
fn a_transition_of_another_track_is_left_out_and_said() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the second", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let second = outcome.data["id"].as_u64().expect("the track has an id");
    let first = a_clip(&mut session, second, sequence, 0, 25, 0);
    let next = a_clip(&mut session, second, sequence, 25, 25, 0);
    session
        .execute(
            "film.transition.add",
            &json!({ "track": second, "from": first, "to": next, "duration": 12 }),
        )
        .expect("the transition is added");
    let left_out = left_out_of(&mut session, sequence);
    assert_eq!(left_out.len(), 1, "the transition is left out: {left_out:?}");
    assert!(left_out[0].contains("left out"), "{left_out:?}");
}

/// **A marker is written inside the element it sits on.**
#[test]
fn a_marker_is_written_inside_its_element() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    session
        .execute("film.marker.add", &json!({ "sequence": sequence, "start": 10, "name": "the mark" }))
        .expect("the marker is added");
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("<marker start=\"10/25s\" duration=\"0s\" name=\"the mark\"/>"), "{text}");
}

/// **A clip that could not be made is not on the timeline.**
#[test]
fn a_clip_with_no_item_is_not_on_the_timeline() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let outcome = session.execute(
        "film.clip.add",
        &json!({ "track": track, "item": 9999, "start": 0, "duration": 25, "source_in": 0 }),
    );
    assert!(outcome.is_err(), "a clip with no item is refused");
    let text = xml_of(&mut session, sequence);
    assert!(text.contains("<spine>"), "and the spine is there: {text}");
    assert!(!text.contains("<clip "), "and it is empty: {text}");
}

/// **A size that is not a size is refused.**
#[test]
fn a_size_that_is_not_a_size_is_refused() {
    let mut session = session();
    let (sequence, _) = a_timeline(&mut session, "the media");
    for (width, height) in [(0, 1080), (1920, 0), (-1, 1080)] {
        let outcome = session.execute(
            "film.export.fcpxml",
            &json!({ "sequence": sequence, "width": width, "height": height }),
        );
        assert!(outcome.is_err(), "a size of {width} by {height} is refused");
    }
}
