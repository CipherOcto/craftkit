//! The **FCP7** read back: an `xmeml` document made into a sequence.
//!
//! The clip items of a track are clips on the tracks of a sequence,
//! and the files of the document are items.

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

/// A file of this test's own, so that two tests do not fight over one
/// file.
fn a_file(name: &str, text: &str) -> String {
    let path = std::env::temp_dir().join(format!("craftkit-v2-fcp7-{name}-{}", std::process::id()));
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **document** with a sequence, a video track and an audio track.
fn a_document() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<xmeml version="4">
  <sequence>
    <name>the film</name>
    <duration>125</duration>
    <rate><timebase>25</timebase><ntsc>FALSE</ntsc></rate>
    <media>
      <video>
        <track>
          <clipitem>
            <name>the beach</name>
            <duration>125</duration>
            <start>0</start>
            <end>125</end>
            <in>50</in>
            <out>175</out>
            <file id="r1">
              <name>the beach.mov</name>
              <pathurl>file:///media/beach.mov</pathurl>
              <duration>500</duration>
              <rate><timebase>25</timebase></rate>
            </file>
          </clipitem>
        </track>
      </video>
      <audio>
        <track>
          <clipitem>
            <name>the music</name>
            <duration>125</duration>
            <start>0</start>
            <end>125</end>
            <in>0</in>
            <out>125</out>
            <file id="r2">
              <name>the music.wav</name>
              <pathurl>file:///media/music.wav</pathurl>
              <duration>500</duration>
            </file>
          </clipitem>
        </track>
      </audio>
    </media>
  </sequence>
</xmeml>
"#
    .to_string()
}

/// The **tracks** of the sequence that was made.
fn the_tracks(session: &Session, sequence: u64) -> Vec<std::sync::Arc<craftkit_sdk::Node>> {
    let node = session.document().node(sequence).expect("the sequence is there");
    node.children.iter().filter(|child| child.kind == "film.track").cloned().collect()
}

/// A document is read into a **sequence**: the clip items are clips on
/// the tracks of the sequence.
#[test]
fn a_document_is_read_into_a_sequence() {
    let mut session = session();
    let path = a_file("sequence", &a_document());
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    assert_eq!(outcome.data["tracks"], json!(2), "the video and the audio: {outcome:?}");
    assert_eq!(outcome.data["clips"], json!(2), "and a clip each: {outcome:?}");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let tracks = the_tracks(&session, sequence);
    let kinds: Vec<String> =
        tracks.iter().map(|track| track.data["kind"].as_str().unwrap_or_default().to_string()).collect();
    assert_eq!(kinds, vec!["video".to_string(), "audio".to_string()], "the tracks are the two");
}

/// The **clocks** of a clip come from its clip item: the `start`, the
/// `in` and the `out`.
#[test]
fn the_clocks_of_a_clip_come_from_the_clip_item() {
    let mut session = session();
    let path = a_file("clocks", &a_document());
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let tracks = the_tracks(&session, sequence);
    let clip = tracks[0].children.iter().find(|child| child.kind == "film.clip").expect("there is a clip");
    assert_eq!(clip.data["start"], json!(0), "the clip begins at the start");
    assert_eq!(clip.data["duration"], json!(125), "and lasts from the in to the out");
    assert_eq!(clip.data["source_in"], json!(50), "and plays from the in");
}

/// The **rate** of the sequence is the rate of the document, and a
/// rate that is **NTSC** is the tree's own rational one.
#[test]
fn the_rate_is_the_documents_own() {
    let mut session = session();
    let path = a_file("rate", &a_document());
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let rate = session
        .execute("film.sequence.rate.get", &json!({ "sequence": sequence }))
        .expect("the rate is read");
    assert_eq!(rate.data["rate"], json!({ "num": 25, "den": 1 }), "the rate is the document's: {rate:?}");
    // An **NTSC** rate is the fraction behind it.
    let text = a_document()
        .replace("<timebase>25</timebase><ntsc>FALSE</ntsc>", "<timebase>30</timebase><ntsc>TRUE</ntsc>");
    let path = a_file("ntsc", &text);
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let rate = session
        .execute("film.sequence.rate.get", &json!({ "sequence": sequence }))
        .expect("the rate is read");
    assert_eq!(rate.data["rate"], json!({ "num": 30000, "den": 1001 }), "and it is the tree's: {rate:?}");
}

/// The **media** of a clip item is an item of the bin, with the file
/// of the document.
#[test]
fn the_media_is_an_item() {
    let mut session = session();
    let path = a_file("media", &a_document());
    let outcome =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect("the document is read");
    let bin = outcome.data["bin"].as_u64().expect("the bin has an id");
    let node = session.document().node(bin).expect("the bin is there");
    let item = node
        .children
        .iter()
        .find(|child| child.kind == "film.item" && child.name == "the beach.mov")
        .expect("there is an item");
    assert_eq!(item.data["path"], json!("/media/beach.mov"), "the item knows its file");
    assert_eq!(item.data["duration"], json!(500), "and how long the media is");
}

/// A file that is **not FCP7 XML** is refused, and so is a file that
/// cannot be read.
#[test]
fn a_file_that_is_not_fcp7_is_refused() {
    let mut session = session();
    let path = a_file("notxml", "<xmeml><sequence>");
    let error =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("not FCP7"), "the refusal says why: {error}");
    let path = a_file("otherroot", "<other><sequence/></other>");
    let error =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("not FCP7"), "the refusal says why: {error}");
    let path = a_file("nosequence", "<xmeml><bin><name>the bin</name></bin></xmeml>");
    let error =
        session.execute("film.import.fcp7", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("no sequence"), "the refusal says why: {error}");
    let error = session
        .execute("film.import.fcp7", &json!({ "path": "/there/is/no/such/file.xml" }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("cannot be read"), "the refusal says why: {error}");
    let error = session.execute("film.import.fcp7", &json!({ "path": "  " })).expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}
