//! The **OTIO** read back: a timeline made into a sequence.
//!
//! The clips of a timeline are clips on the tracks of a sequence, and
//! the media of the file are items.

use craftkit_sdk::serde_json::{Value as Json, json};
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

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-otio-{name}-{}", std::process::id()));
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **time**: the rate and the frames.
fn time(rate: f64, frames: f64) -> Json {
    json!({ "OTIO_SCHEMA": "RationalTime.1", "rate": rate, "value": frames })
}

/// A **range**: where it begins and how long it is.
fn range(rate: f64, start: f64, duration: f64) -> Json {
    json!({
        "OTIO_SCHEMA": "TimeRange.1",
        "start_time": time(rate, start),
        "duration": time(rate, duration),
    })
}

/// A **clip** of the timeline, with its media.
fn a_clip(name: &str, start: f64, duration: f64) -> Json {
    json!({
        "OTIO_SCHEMA": "Clip.2",
        "name": name,
        "source_range": range(25.0, start, duration),
        "effects": [],
        "markers": [],
        "enabled": true,
        "media_references": {
            "DEFAULT_MEDIA": {
                "OTIO_SCHEMA": "ExternalReference.1",
                "name": name,
                "target_url": format!("file:///media/{name}.mov"),
                "available_range": range(25.0, 0.0, 500.0),
                "available_image_bounds": null,
                "metadata": {}
            }
        },
        "active_media_reference_key": "DEFAULT_MEDIA",
        "metadata": {}
    })
}

/// A **gap**: a hole in the timeline.
fn a_gap(duration: f64) -> Json {
    json!({
        "OTIO_SCHEMA": "Gap.1",
        "name": "",
        "source_range": range(25.0, 0.0, duration),
        "effects": [],
        "markers": [],
        "enabled": true,
        "metadata": {}
    })
}

/// A **transition**: a dissolve between two clips.
fn a_transition() -> Json {
    json!({
        "OTIO_SCHEMA": "Transition.1",
        "name": "cross dissolve",
        "transition_type": "SMPTE_Dissolve",
        "in_offset": time(25.0, 0.0),
        "out_offset": time(25.0, 0.0),
        "metadata": {}
    })
}

/// A **timeline** with one track and the children it is given.
fn a_timeline_with(children: Vec<Json>, kind: &str) -> String {
    json!({
        "OTIO_SCHEMA": "Timeline.1",
        "name": "the film",
        "global_start_time": time(25.0, 0.0),
        "tracks": {
            "OTIO_SCHEMA": "Stack.1",
            "name": "the film",
            "children": [
                {
                    "OTIO_SCHEMA": "Track.1",
                    "name": "the track",
                    "kind": kind,
                    "children": children
                }
            ]
        }
    })
    .to_string()
}

/// A **timeline** with one track and one clip.
fn a_timeline() -> String {
    a_timeline_with(vec![a_clip("the beach", 50.0, 125.0)], "Video")
}

/// The **track** of the sequence that was made.
fn the_track(session: &Session, sequence: u64) -> std::sync::Arc<craftkit_sdk::Node> {
    let node = session.document().node(sequence).expect("the sequence is there");
    node.children.iter().find(|child| child.kind == "film.track").cloned().expect("there is a track")
}

/// The **clip** of the track.
fn the_clip(track: &craftkit_sdk::Node) -> std::sync::Arc<craftkit_sdk::Node> {
    track.children.iter().find(|child| child.kind == "film.clip").cloned().expect("there is a clip")
}

/// A timeline is read into a **sequence**: the clip is on a track, and
/// the media of the file is an item.
#[test]
fn a_timeline_is_read_into_a_sequence() {
    let mut session = session();
    let path = a_file("sequence", &a_timeline());
    let outcome = session
        .execute("film.import.otio", &json!({ "path": path, "name": "the film" }))
        .expect("the timeline is read");
    assert_eq!(outcome.data["tracks"], json!(1), "there is one track: {outcome:?}");
    assert_eq!(outcome.data["clips"], json!(1), "and one clip: {outcome:?}");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let track = the_track(&session, sequence);
    assert_eq!(track.data["kind"], json!("video"), "and it is a video track");
    let clip = the_clip(&track);
    assert_eq!(clip.data["start"], json!(0), "the clip begins at the start");
    assert_eq!(clip.data["duration"], json!(125), "and lasts as long as the text says");
    assert_eq!(clip.data["source_in"], json!(50), "and plays from fifty frames in");
}

/// The **rate** of the sequence is the rate of the timeline.
#[test]
fn the_rate_is_the_timelines_own() {
    let mut session = session();
    let path = a_file("rate", &a_timeline());
    let outcome =
        session.execute("film.import.otio", &json!({ "path": path })).expect("the timeline is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let rate = session
        .execute("film.sequence.rate.get", &json!({ "sequence": sequence }))
        .expect("the rate is read");
    assert_eq!(rate.data["rate"], json!({ "num": 25, "den": 1 }), "the rate is the timeline's: {rate:?}");
}

/// A **gap** is a hole: the clips after it move along.
#[test]
fn a_gap_is_a_hole() {
    let mut session = session();
    let text = a_timeline_with(vec![a_gap(25.0), a_clip("the beach", 50.0, 125.0)], "Video");
    let path = a_file("gap", &text);
    let outcome =
        session.execute("film.import.otio", &json!({ "path": path })).expect("the timeline is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let track = the_track(&session, sequence);
    let clip = the_clip(&track);
    assert_eq!(clip.data["start"], json!(25), "the clip begins after the gap: {clip:?}");
}

/// A track whose kind is `Audio` is an **audio** track, and every
/// other track is a video one.
#[test]
fn an_audio_track_is_an_audio_track() {
    let mut session = session();
    let text = a_timeline_with(vec![a_clip("the beach", 50.0, 125.0)], "Audio");
    let path = a_file("audio", &text);
    let outcome =
        session.execute("film.import.otio", &json!({ "path": path })).expect("the timeline is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let track = the_track(&session, sequence);
    assert_eq!(track.data["kind"], json!("audio"), "the track is an audio one");
}

/// A file that is **not OTIO** is refused, and so is a file that
/// cannot be read.
#[test]
fn a_file_that_is_not_otio_is_refused() {
    let mut session = session();
    let path = a_file("nototio", "this is not a timeline");
    let error =
        session.execute("film.import.otio", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("not OTIO"), "the refusal says why: {error}");
    let path = a_file("notimeline", &json!({ "OTIO_SCHEMA": "Marker.2", "name": "no" }).to_string());
    let error =
        session.execute("film.import.otio", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("no timeline"), "the refusal says why: {error}");
    let error = session
        .execute("film.import.otio", &json!({ "path": "/there/is/no/such/file.otio" }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("cannot be read"), "the refusal says why: {error}");
    let error = session.execute("film.import.otio", &json!({ "path": "  " })).expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}

/// A **transition** is left out, and said.
#[test]
fn a_transition_is_left_out_and_said() {
    let mut session = session();
    let text = a_timeline_with(vec![a_transition(), a_clip("the beach", 50.0, 125.0)], "Video");
    let path = a_file("transition", &text);
    let outcome =
        session.execute("film.import.otio", &json!({ "path": path })).expect("the timeline is read");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert_eq!(left_out.len(), 1, "the transition is said: {outcome:?}");
    assert!(left_out[0].as_str().unwrap_or_default().contains("transition"), "and it is named");
}

/// The **media** of a clip is an item of the bin, with the file of the
/// reference.
#[test]
fn the_media_is_an_item() {
    let mut session = session();
    let path = a_file("media", &a_timeline());
    let outcome =
        session.execute("film.import.otio", &json!({ "path": path })).expect("the timeline is read");
    let bin = outcome.data["bin"].as_u64().expect("the bin has an id");
    let node = session.document().node(bin).expect("the bin is there");
    let item = node
        .children
        .iter()
        .find(|child| child.kind == "film.item" && child.name == "the beach")
        .expect("there is an item");
    assert_eq!(item.name, "the beach", "the item is named after the clip");
    assert_eq!(item.data["path"], json!("/media/the beach.mov"), "and it knows its file");
    assert_eq!(item.data["duration"], json!(500), "and how long the media is");
}

/// A timeline that this tree wrote is read back into a sequence: the
/// **round trip**.
#[test]
fn a_timeline_this_tree_wrote_is_read_back() {
    let mut session = session();
    // A **bin**, an **item**, a **track** and a **clip**.
    let outcome = session.execute("film.bin.add", &json!({ "name": "the media" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute(
            "film.item.add",
            &json!({ "name": "the beach", "bin": bin, "item": 1, "path": "/media/beach.mov", "duration": 500 }),
        )
        .expect("the item is added");
    let item = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the film", "bin": bin, "item": 2 }))
        .expect("the sequence is added");
    let sequence = outcome.data["id"].as_u64().expect("the sequence has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    session
        .execute(
            "film.clip.add",
            &json!({ "track": track, "item": item, "start": 0, "duration": 125, "source_in": 50 }),
        )
        .expect("the clip is added");
    // The **text** of the timeline, and the file it is in.
    let exported = session
        .execute("film.export.otio", &json!({ "sequence": sequence }))
        .expect("the timeline is written");
    let text = exported.data["otio"].as_str().expect("the text is there");
    let path = a_file("roundtrip", text);
    // The timeline is **read back**.
    let mut other = crate::session();
    let outcome = other.execute("film.import.otio", &json!({ "path": path })).expect("the timeline is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let track = the_track(&other, sequence);
    let clip = the_clip(&track);
    assert_eq!(clip.data["start"], json!(0), "the clip begins at the start: {clip:?}");
    assert_eq!(clip.data["duration"], json!(125), "and lasts as long as before");
    assert_eq!(clip.data["source_in"], json!(50), "and plays from the same place in the media");
}
