//! The **transitions** and the **markers** of an FCPXML file.
//!
//! The file is an FCPXML, and the sequence it names has the
//! transitions and the markers the file says.

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

/// A path of this test's own, so that two tests do not fight over one
/// file.
fn a_path(name: &str) -> String {
    craftkit_scratch::file(&format!("craftkit-v2-fcpxmlmarks-{name}-{}", std::process::id()))
        .expect("the scratch is made")
        .to_string_lossy()
        .to_string()
}

/// **Write** a file of this test's own.
fn a_file(name: &str, text: &str) -> String {
    let path = a_path(name);
    std::fs::write(&path, text.as_bytes()).expect("the file is written");
    path
}

/// A file of **two clips**, a **transition** between them and two
/// **markers** on the second.
fn the_file(name: &str) -> String {
    a_file(
        name,
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE fcpxml>
<fcpxml version="1.9">
  <resources>
    <format id="r1" name="FFVideoFormat1080p" frameDuration="1/25s"/>
    <asset id="r2" name="the media" src="file:///tmp/the.mov" start="0s" duration="250/25s" hasVideo="1" hasAudio="1"/>
  </resources>
  <library>
    <event name="the event">
      <project name="the project">
        <sequence format="r1" duration="250/25s" tcStart="0s" tcFormat="NDF">
          <spine>
            <clip ref="r2" lane="1" name="the first" offset="0s" start="0s" duration="100/25s"/>
            <transition name="cross_dissolve" offset="50/25s" duration="50/25s"><filter-video ref="r3" name="cross_dissolve"/></transition>
            <clip ref="r2" lane="1" name="the second" offset="100/25s" start="0s" duration="150/25s">
              <marker start="10/25s" duration="0s" value="the mark"/>
              <chapter-marker start="20/25s" duration="25/25s" value="the chapter" note="the note"/>
            </clip>
          </spine>
        </sequence>
      </project>
    </event>
  </library>
</fcpxml>
"#,
    )
}

/// The **nodes** of a kind under a node.
fn children_of(session: &Session, node: u64, kind: &str) -> Vec<Json> {
    let node = session.document().node(node).expect("the node is there");
    node.children.iter().filter(|child| child.kind == kind).map(|child| child.data.clone()).collect()
}

/// A **transition** of the spine joins the clip before it and the clip
/// after it, and where it sits on the cut says how it is aligned.
#[test]
fn a_transition_is_read() {
    let mut session = session();
    let path = the_file("transition");
    let outcome = session.execute("film.import.fcpxml", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["clips"], json!(2), "there are two clips: {outcome:?}");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let node = session.document().node(sequence).expect("the sequence is there");
    let track = node.children.iter().find(|child| child.kind == "film.track").expect("there is a track");
    let transitions = children_of(&session, track.id, "film.transition");
    assert_eq!(transitions.len(), 1, "there is one transition: {transitions:?}");
    assert_eq!(transitions[0]["duration"], json!(50), "it is fifty frames long: {transitions:?}");
    assert_eq!(transitions[0]["start"], json!(50), "and it starts at fifty: {transitions:?}");
    // The **cut** is at a hundred, and the transition ends at it: the
    // tree's own way of saying it is aligned to the end of the cut.
    assert_eq!(transitions[0]["align"], json!("end"), "it ends at the cut: {transitions:?}");
    assert_eq!(transitions[0]["effect"], json!("cross_dissolve"), "and it is a dissolve: {transitions:?}");
    // The **clips** the transition joins: the one before it and the
    // one after it.
    let node = session.document().node(track.id).expect("the track is there");
    let clips: Vec<u64> =
        node.children.iter().filter(|child| child.kind == "film.clip").map(|child| child.id).collect();
    assert_eq!(transitions[0]["from"], json!(clips[0]), "it comes from the first clip: {transitions:?}");
    assert_eq!(transitions[0]["to"], json!(clips[1]), "and it comes into the second: {transitions:?}");
}

/// The **markers** of a clip of the spine are markers of the
/// sequence.
#[test]
fn the_markers_of_a_clip_are_markers_of_the_sequence() {
    let mut session = session();
    let path = the_file("markers");
    let outcome = session.execute("film.import.fcpxml", &json!({ "path": path })).expect("the file is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let markers = children_of(&session, sequence, "film.marker");
    assert_eq!(markers.len(), 2, "there are two markers: {markers:?}");
    // The second clip starts at a hundred, and the marker is ten
    // frames into it.
    assert_eq!(markers[0]["start"], json!(110), "the marker is where the clip is: {markers:?}");
    assert_eq!(markers[0]["name"], json!("the mark"), "and it is called what it says: {markers:?}");
    assert_eq!(markers[0]["kind"], json!("Comment"), "a marker is a comment: {markers:?}");
    assert_eq!(markers[0]["color"], json!("Blue"), "and it is blue: {markers:?}");
    assert_eq!(markers[0]["duration"], json!(0), "and it is a point: {markers:?}");
}

/// A **chapter** is a chapter, and it is mango.
#[test]
fn a_chapter_is_a_chapter() {
    let mut session = session();
    let path = the_file("chapter");
    let outcome = session.execute("film.import.fcpxml", &json!({ "path": path })).expect("the file is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let markers = children_of(&session, sequence, "film.marker");
    let chapter = markers.iter().find(|marker| marker["name"] == json!("the chapter")).expect("it is there");
    assert_eq!(chapter["kind"], json!("Chapter"), "it is a chapter: {markers:?}");
    assert_eq!(chapter["color"], json!("Mango"), "and it is mango: {markers:?}");
    assert_eq!(chapter["comment"], json!("the note"), "and it says what it says: {markers:?}");
    assert_eq!(chapter["duration"], json!(25), "and it is as long as it says: {markers:?}");
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
            &json!({ "track": track, "from": clips[0], "to": clips[1], "duration": 50, "align": "end" }),
        )
        .expect("the transition is added");
    session
        .execute("film.marker.add", &json!({ "sequence": sequence, "start": 110, "name": "the mark" }))
        .expect("the marker is added");
    // The **text**, and the file it is written to.
    let outcome = session
        .execute("film.export.fcpxml", &json!({ "sequence": sequence, "width": 1920, "height": 1080 }))
        .expect("the text is written");
    let text = outcome.data["xml"].as_str().expect("the text is a text").to_string();
    let path = a_file("roundtrip", &text);
    // The **read**.
    let mut other = {
        let ecosystem = ecosystem();
        Session::with_formats(
            ecosystem.commands,
            ecosystem.kinds,
            ecosystem.formats,
            ecosystem.migrations,
            ecosystem.evaluators,
        )
    };
    let outcome = other.execute("film.import.fcpxml", &json!({ "path": path })).expect("the file is read");
    assert_eq!(outcome.data["clips"], json!(2), "both clips are read: {outcome:?}");
    let read = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let markers = children_of(&other, read, "film.marker");
    assert_eq!(markers.len(), 1, "the marker is read: {markers:?}");
    assert_eq!(markers[0]["start"], json!(110), "and it is where it was: {markers:?}");
    assert_eq!(markers[0]["name"], json!("the mark"), "and it is called what it was: {markers:?}");
    let node = other.document().node(read).expect("the sequence is there");
    let track = node.children.iter().find(|child| child.kind == "film.track").expect("there is a track");
    let transitions = children_of(&other, track.id, "film.transition");
    assert_eq!(transitions.len(), 1, "the transition is read: {transitions:?}");
    assert_eq!(transitions[0]["duration"], json!(50), "and it is as long as it was: {transitions:?}");
    assert_eq!(transitions[0]["align"], json!("end"), "and it is aligned as it was: {transitions:?}");
}
