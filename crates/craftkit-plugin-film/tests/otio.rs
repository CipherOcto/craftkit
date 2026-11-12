//! The **OTIO** of a sequence.

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

/// The **timeline** of a sequence.
fn timeline_of(session: &mut Session, sequence: u64) -> Json {
    let outcome =
        session.execute("film.export.otio", &json!({ "sequence": sequence })).expect("the text is written");
    outcome.data["timeline"].clone()
}

/// The **tracks** of a timeline.
fn tracks_of(timeline: &Json) -> &Vec<Json> {
    timeline["tracks"]["children"].as_array().expect("the tracks are a list")
}

/// **The text is a timeline of the format.**
#[test]
fn the_text_is_a_timeline_of_the_format() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let timeline = timeline_of(&mut session, sequence);
    assert_eq!(timeline["OTIO_SCHEMA"], json!("Timeline.1"));
    assert_eq!(timeline["name"], json!("the media"));
    assert_eq!(timeline["tracks"]["OTIO_SCHEMA"], json!("Stack.1"));
    assert_eq!(timeline["global_start_time"]["OTIO_SCHEMA"], json!("RationalTime.1"));
    assert_eq!(timeline["global_start_time"]["rate"], json!(25.0));
}

/// **The tracks are the tracks of the sequence, and each says which
/// kind it is.**
#[test]
fn the_tracks_say_which_kind_they_are() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the sound", "sequence": sequence, "kind": "audio" }))
        .expect("the track is added");
    let sound = outcome.data["id"].as_u64().expect("the track has an id");
    a_clip(&mut session, sound, sequence, 0, 125, 0);
    let timeline = timeline_of(&mut session, sequence);
    let tracks = tracks_of(&timeline);
    assert_eq!(tracks.len(), 2);
    assert_eq!(tracks[0]["kind"], json!("Video"), "the video track is first");
    assert_eq!(tracks[1]["kind"], json!("Audio"));
}

/// **A clip says where it begins and how long it is, and it points at
/// the media it plays.**
#[test]
fn a_clip_says_where_it_begins_and_how_long_it_is() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 10);
    let timeline = timeline_of(&mut session, sequence);
    let tracks = tracks_of(&timeline);
    let children = tracks[0]["children"].as_array().expect("the children are a list");
    let clip = &children[0];
    assert_eq!(clip["OTIO_SCHEMA"], json!("Clip.2"));
    assert_eq!(clip["source_range"]["start_time"]["value"], json!(10.0));
    assert_eq!(clip["source_range"]["duration"]["value"], json!(125.0));
    let reference = &clip["media_references"]["DEFAULT_MEDIA"];
    assert_eq!(reference["OTIO_SCHEMA"], json!("MissingReference.1"));
}

/// **An item with a path is a reference to that path.**
#[test]
fn an_item_with_a_path_is_a_reference_to_it() {
    let mut session = session();
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute(
            "film.item.add",
            &json!({ "name": "the media", "bin": bin, "item": 1, "duration": 600, "path": "/the/clip.mov" }),
        )
        .expect("the item is added");
    let media = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": media, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    a_clip(&mut session, track, media, 0, 125, 0);
    let timeline = timeline_of(&mut session, media);
    let tracks = tracks_of(&timeline);
    let children = tracks[0]["children"].as_array().expect("the children are a list");
    let reference = &children[0]["media_references"]["DEFAULT_MEDIA"];
    assert_eq!(reference["OTIO_SCHEMA"], json!("ExternalReference.1"));
    assert_eq!(reference["target_url"], json!("file:///the/clip.mov"));
}

/// **The hole between two clips is a gap.**
#[test]
fn the_hole_between_two_clips_is_a_gap() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 25, 0);
    a_clip(&mut session, track, sequence, 50, 25, 0);
    let timeline = timeline_of(&mut session, sequence);
    let tracks = tracks_of(&timeline);
    let children = tracks[0]["children"].as_array().expect("the children are a list");
    assert_eq!(children.len(), 3);
    assert_eq!(children[1]["OTIO_SCHEMA"], json!("Gap.1"));
    assert_eq!(children[1]["source_range"]["duration"]["value"], json!(25.0));
}

/// **A transition is a transition, and it says its offsets.**
#[test]
fn a_transition_says_its_offsets() {
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
    let timeline = timeline_of(&mut session, sequence);
    let tracks = tracks_of(&timeline);
    let children = tracks[0]["children"].as_array().expect("the children are a list");
    let transition = children
        .iter()
        .find(|child| child["OTIO_SCHEMA"] == json!("Transition.1"))
        .expect("the transition is there");
    assert_eq!(transition["transition_type"], json!("SMPTE_Dissolve"));
    assert_eq!(transition["in_offset"]["value"], json!(6.0));
    assert_eq!(transition["out_offset"]["value"], json!(6.0));
}

/// **The markers of the sequence are on the stack.**
#[test]
fn the_markers_are_on_the_stack() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    session
        .execute("film.marker.add", &json!({ "sequence": sequence, "start": 10, "name": "the mark" }))
        .expect("the marker is added");
    let timeline = timeline_of(&mut session, sequence);
    let markers = timeline["tracks"]["markers"].as_array().expect("the markers are a list");
    assert_eq!(markers.len(), 1);
    assert_eq!(markers[0]["OTIO_SCHEMA"], json!("Marker.2"));
    assert_eq!(markers[0]["name"], json!("the mark"));
    assert_eq!(markers[0]["marked_range"]["start_time"]["value"], json!(10.0));
}

/// **A clip that is retimed carries a time warp.**
#[test]
fn a_clip_that_is_retimed_carries_a_time_warp() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    let clip = a_clip(&mut session, track, sequence, 0, 25, 0);
    session.execute("film.clip.speed", &json!({ "clip": clip, "speed": 2.0 })).expect("the speed is set");
    let timeline = timeline_of(&mut session, sequence);
    let tracks = tracks_of(&timeline);
    let children = tracks[0]["children"].as_array().expect("the children are a list");
    let effects = children[0]["effects"].as_array().expect("the effects are a list");
    assert_eq!(effects[0]["OTIO_SCHEMA"], json!("LinearTimeWarp.1"));
    assert_eq!(effects[0]["time_scalar"], json!(2.0));
}

/// **The text is a text, and it is the text of the timeline.**
#[test]
fn the_text_is_the_text_of_the_timeline() {
    let mut session = session();
    let (sequence, track) = a_timeline(&mut session, "the media");
    a_clip(&mut session, track, sequence, 0, 125, 0);
    let outcome =
        session.execute("film.export.otio", &json!({ "sequence": sequence })).expect("the text is written");
    let text = outcome.data["otio"].as_str().expect("the text is a text").to_string();
    assert!(text.starts_with('{'), "{text}");
    assert!(text.contains("\"OTIO_SCHEMA\": \"Timeline.1\""), "{text}");
}
