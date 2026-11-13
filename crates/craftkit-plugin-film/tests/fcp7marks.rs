//! The **transitions** and the **markers** of an FCP7 document.
//!
//! The file is an `xmeml` document, and the sequence it names has the
//! transitions and the markers the document says.

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
    let path = craftkit_scratch::file(&format!("craftkit-v2-fcp7marks-{name}-{}", std::process::id()))
        .expect("the scratch is made");
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **document** with two clips, a **transition** between them, a
/// **marker** of the sequence and a marker of the media.
fn the_document() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<xmeml version="4">
  <sequence>
    <name>the film</name>
    <duration>250</duration>
    <rate><timebase>25</timebase><ntsc>FALSE</ntsc></rate>
    <marker>
      <in>110</in>
      <out>135</out>
      <name>the mark</name>
      <comment>the note</comment>
      <color>Mango</color>
    </marker>
    <media>
      <video>
        <track>
          <clipitem>
            <name>the first</name>
            <duration>100</duration>
            <start>0</start>
            <end>100</end>
            <in>0</in>
            <out>100</out>
            <file id="r1">
              <name>the beach.mov</name>
              <pathurl>file:///media/beach.mov</pathurl>
              <duration>500</duration>
            </file>
          </clipitem>
          <transitionitem>
            <name>the dissolve</name>
            <start>75</start>
            <end>125</end>
            <alignment>center</alignment>
            <effect><name>Cross Dissolve</name><effectid>Cross Dissolve</effectid></effect>
          </transitionitem>
          <clipitem>
            <name>the second</name>
            <duration>100</duration>
            <start>100</start>
            <end>200</end>
            <in>0</in>
            <out>100</out>
            <file id="r2">
              <name>the hills.mov</name>
              <pathurl>file:///media/hills.mov</pathurl>
              <duration>500</duration>
            </file>
          </clipitem>
        </track>
      </video>
      <video>
        <track>
          <clipitem>
            <name>the media</name>
            <duration>100</duration>
            <start>0</start>
            <end>100</end>
            <in>0</in>
            <out>100</out>
            <file id="r3">
              <name>the master.mov</name>
              <pathurl>file:///media/master.mov</pathurl>
              <duration>500</duration>
            </file>
            <marker><in>10</in><out>20</out><name>the media mark</name></marker>
          </clipitem>
        </track>
      </video>
    </media>
  </sequence>
</xmeml>
"#
    .to_string()
}

/// A **document** of this test's own.
fn a_document(name: &str) -> String {
    a_file(name, &the_document())
}

/// The **nodes** of a kind under a node.
fn children_of(session: &Session, node: u64, kind: &str) -> Vec<Json> {
    let node = session.document().node(node).expect("the node is there");
    node.children.iter().filter(|child| child.kind == kind).map(|child| child.data.clone()).collect()
}

/// The **markers** of the sequence are markers of it.
#[test]
fn the_markers_of_the_sequence_are_markers_of_it() {
    let mut session = session();
    let path = a_document("markers");
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let markers = children_of(&session, sequence, "film.marker");
    assert_eq!(markers.len(), 1, "there is one marker: {markers:?}");
    assert_eq!(markers[0]["start"], json!(110), "it starts where it says: {markers:?}");
    assert_eq!(markers[0]["duration"], json!(25), "and it is as long as it says: {markers:?}");
    assert_eq!(markers[0]["name"], json!("the mark"), "and it is called what it says: {markers:?}");
    assert_eq!(markers[0]["comment"], json!("the note"), "and it says what it says: {markers:?}");
    assert_eq!(markers[0]["color"], json!("Mango"), "and it is the colour it names: {markers:?}");
}

/// The **markers of a clip** are markers of that clip, and this tree
/// has no marker of a clip: they are left out, and said.
#[test]
fn the_markers_of_a_clip_are_left_out() {
    let mut session = session();
    let path = a_document("media");
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert_eq!(left_out.len(), 1, "one thing is left out: {left_out:?}");
    assert!(
        left_out[0].as_str().unwrap_or_default().contains("markers of the clip"),
        "and it is the markers: {left_out:?}"
    );
}

/// A **transition** joins the clip that ends in it and the clip that
/// begins in it.
#[test]
fn a_transition_is_read() {
    let mut session = session();
    let path = a_document("transition");
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let node = session.document().node(sequence).expect("the sequence is there");
    let track = node.children.iter().find(|child| child.kind == "film.track").expect("there is a track");
    let transitions = children_of(&session, track.id, "film.transition");
    assert_eq!(transitions.len(), 1, "there is one transition: {transitions:?}");
    assert_eq!(transitions[0]["start"], json!(75), "it starts where the file says: {transitions:?}");
    assert_eq!(transitions[0]["duration"], json!(50), "and it is as long as it says: {transitions:?}");
    assert_eq!(transitions[0]["align"], json!("center"), "and it is centred: {transitions:?}");
    assert_eq!(transitions[0]["effect"], json!("cross_dissolve"), "and it is a dissolve: {transitions:?}");
    // The **clips** it joins: the one that ends in it and the one
    // that begins in it.
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

/// A transition of an **effect this tree does not have** is left out,
/// and the answer says so.
#[test]
fn a_transition_of_an_effect_this_tree_does_not_have_is_left_out() {
    let mut session = session();
    let text = the_document().replace("Cross Dissolve", "Iris Dissolve");
    let path = a_file("effect", &text);
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert!(
        left_out.iter().any(|line| line.as_str().unwrap_or_default().contains("Iris Dissolve")),
        "it is said: {left_out:?}"
    );
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let node = session.document().node(sequence).expect("the sequence is there");
    let track = node.children.iter().find(|child| child.kind == "film.track").expect("there is a track");
    let transitions = children_of(&session, track.id, "film.transition");
    assert!(transitions.is_empty(), "and there is no transition: {transitions:?}");
}
