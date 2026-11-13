//! The **transitions** and the **markers** of an OTIO file.
//!
//! The file is an OTIO document, and the sequence it names has the
//! transitions and the markers the text says.

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
    let path = craftkit_scratch::file(&format!("craftkit-v2-otiomarks-{name}-{}", std::process::id()))
        .expect("the scratch is made");
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **document** with two clips, a **transition** between them and a
/// **marker** of the stack.
fn the_document() -> String {
    let mut text = String::new();
    text.push_str(
        r#"{
  "OTIO_SCHEMA": "Timeline.1",
  "name": "the film",
  "global_start_time": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 0.0 },
  "metadata": {},
  "tracks": {
    "OTIO_SCHEMA": "Stack.1",
    "name": "the tracks",
    "children": [
      {
        "OTIO_SCHEMA": "Track.1",
        "name": "the video",
        "kind": "Video",
        "children": [
          {
            "OTIO_SCHEMA": "Clip.2",
            "name": "the first",
            "source_range": {
              "OTIO_SCHEMA": "TimeRange.1",
              "start_time": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 0.0 },
              "duration": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 100.0 }
            },
            "media_references": {
              "DEFAULT_MEDIA": {
                "OTIO_SCHEMA": "ExternalReference.1",
                "target_url": "file:///media/beach.mov"
              }
            }
          },
          {
            "OTIO_SCHEMA": "Transition.1",
            "name": "cross_dissolve",
            "transition_type": "SMPTE_Dissolve",
            "in_offset": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 25.0 },
            "out_offset": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 25.0 },
            "metadata": {}
          },
          {
            "OTIO_SCHEMA": "Clip.2",
            "name": "the second",
            "source_range": {
              "OTIO_SCHEMA": "TimeRange.1",
              "start_time": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 0.0 },
              "duration": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 100.0 }
            },
            "media_references": {
              "DEFAULT_MEDIA": {
                "OTIO_SCHEMA": "ExternalReference.1",
                "target_url": "file:///media/hills.mov"
              }
            }
          }
        ],
        "markers": [
          {
            "OTIO_SCHEMA": "Marker.2",
            "name": "the mark",
            "comment": "the note",
            "color": "YELLOW",
            "marked_range": {
              "OTIO_SCHEMA": "TimeRange.1",
              "start_time": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 50.0 },
              "duration": { "OTIO_SCHEMA": "RationalTime.1", "rate": 25.0, "value": 25.0 }
            },
            "metadata": {}
          }
        ],
        "metadata": {}
      }
    ],
    "markers": [],
    "metadata": {}
  }
}
"#,
    );
    text
}

/// The **nodes** of a kind under a node.
fn children_of(session: &Session, node: u64, kind: &str) -> Vec<Json> {
    let node = session.document().node(node).expect("the node is there");
    node.children.iter().filter(|child| child.kind == kind).map(|child| child.data.clone()).collect()
}

/// A **transition** joins the clip before it and the clip after it, and
/// where it sits on the cut says how it is aligned.
#[test]
fn a_transition_is_read() {
    let mut session = session();
    let path = a_file("transition", &the_document());
    let outcome =
        session.execute("film.import.otio", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let node = session.document().node(sequence).expect("the sequence is there");
    let track = node.children.iter().find(|child| child.kind == "film.track").expect("there is a track");
    let transitions = children_of(&session, track.id, "film.transition");
    assert_eq!(transitions.len(), 1, "there is one transition: {transitions:?}");
    // The cut is at a hundred, and the transition reaches twenty-five
    // frames on either side of it.
    assert_eq!(transitions[0]["start"], json!(75), "it starts where it sits: {transitions:?}");
    assert_eq!(transitions[0]["duration"], json!(50), "and it is as long as it is: {transitions:?}");
    assert_eq!(transitions[0]["align"], json!("center"), "and it is centred on the cut: {transitions:?}");
    assert_eq!(transitions[0]["effect"], json!("cross_dissolve"), "and it is a dissolve: {transitions:?}");
    let clips: Vec<u64> = node
        .children
        .iter()
        .find(|child| child.kind == "film.track")
        .map(|track| {
            track.children.iter().filter(|child| child.kind == "film.clip").map(|child| child.id).collect()
        })
        .unwrap_or_default();
    assert_eq!(transitions[0]["from"], json!(clips[0]), "it comes from the first clip: {transitions:?}");
    assert_eq!(transitions[0]["to"], json!(clips[1]), "and comes into the second: {transitions:?}");
}

/// The **markers** of a track are markers of the sequence, and their
/// colours are the tree's own.
#[test]
fn the_markers_of_a_track_are_markers_of_the_sequence() {
    let mut session = session();
    let path = a_file("markers", &the_document());
    let outcome =
        session.execute("film.import.otio", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let markers = children_of(&session, sequence, "film.marker");
    assert_eq!(markers.len(), 1, "there is one marker: {markers:?}");
    assert_eq!(markers[0]["start"], json!(50), "it starts where it says: {markers:?}");
    assert_eq!(markers[0]["duration"], json!(25), "and it is as long as it says: {markers:?}");
    assert_eq!(markers[0]["name"], json!("the mark"), "and it is called what it says: {markers:?}");
    assert_eq!(markers[0]["comment"], json!("the note"), "and it says what it says: {markers:?}");
    // The tree's own colours: yellow is yellow.
    assert_eq!(markers[0]["color"], json!("Yellow"), "and it is the colour it names: {markers:?}");
}

/// A sequence that is **written** and read again is the same
/// sequence: the **round trip**.
#[test]
fn a_sequence_that_is_written_is_read_back() {
    let mut session = session();
    // The **timeline**: a bin, an item, a track and two clips.
    let outcome = session.execute("film.bin.add", &json!({ "name": "the bin" })).expect("the bin is added");
    let bin = outcome.data["id"].as_u64().expect("the bin has an id");
    let outcome = session
        .execute("film.item.add", &json!({ "name": "the sequence", "bin": bin, "item": 1, "duration": 600 }))
        .expect("the item is added");
    let sequence = outcome.data["id"].as_u64().expect("the item has an id");
    let outcome = session
        .execute("film.track.add", &json!({ "name": "the track", "sequence": sequence, "kind": "video" }))
        .expect("the track is added");
    let track = outcome.data["id"].as_u64().expect("the track has an id");
    let mut clips = Vec::new();
    for (start, duration) in [(0u64, 100u64), (100, 100)] {
        let outcome = session
            .execute(
                "film.clip.add",
                &json!({
                    "track": track,
                    "item": sequence,
                    "start": start,
                    "duration": duration,
                    "source_in": 0,
                }),
            )
            .expect("the clip is added");
        clips.push(outcome.data["id"].as_u64().expect("the clip has an id"));
    }
    session
        .execute(
            "film.transition.add",
            &json!({ "track": track, "from": clips[0], "to": clips[1], "duration": 50, "align": "center" }),
        )
        .expect("the transition is added");
    session
        .execute("film.marker.add", &json!({ "sequence": sequence, "start": 50, "name": "the mark" }))
        .expect("the marker is added");
    // The **text**, and the file it is written to.
    let outcome =
        session.execute("film.export.otio", &json!({ "sequence": sequence })).expect("the text is written");
    let text = outcome.data["otio"].as_str().expect("the text is a text").to_string();
    let path = a_file("roundtrip", &text);
    // The **read**.
    let ecosystem = ecosystem();
    let mut other = Session::with_formats(
        ecosystem.commands,
        ecosystem.kinds,
        ecosystem.formats,
        ecosystem.migrations,
        ecosystem.evaluators,
    );
    let outcome = other.execute("film.import.otio", &json!({ "path": path })).expect("the document is read");
    let read = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let markers = children_of(&other, read, "film.marker");
    assert_eq!(markers.len(), 1, "the marker is read: {markers:?}");
    assert_eq!(markers[0]["start"], json!(50), "and it is where it was: {markers:?}");
    assert_eq!(markers[0]["name"], json!("the mark"), "and it is called what it was: {markers:?}");
    let node = other.document().node(read).expect("the sequence is there");
    let track = node.children.iter().find(|child| child.kind == "film.track").expect("there is a track");
    let transitions = children_of(&other, track.id, "film.transition");
    assert_eq!(transitions.len(), 1, "the transition is read: {transitions:?}");
    assert_eq!(transitions[0]["duration"], json!(50), "and it is as long as it was: {transitions:?}");
    assert_eq!(transitions[0]["align"], json!("center"), "and it is aligned as it was: {transitions:?}");
}
