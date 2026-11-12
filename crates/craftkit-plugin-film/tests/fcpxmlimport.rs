//! The **FCPXML** read back: a document made into a sequence.
//!
//! The clips of the spine are clips on the first track of a sequence,
//! and the media of the file are items.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-fcpxml-{name}-{}", std::process::id()));
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **document** with a format, an asset and a spine of one clip.
fn a_document() -> String {
    r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE fcpxml>
<fcpxml version="1.9">
  <resources>
    <format id="r1" name="FFVideoFormat1080p25" frameDuration="1/25s" width="1920" height="1080"/>
    <asset id="r2" name="the beach" start="0s" duration="500/25s">
      <media-rep kind="original-media" src="file:///media/beach.mov"/>
    </asset>
  </resources>
  <library>
    <event name="the film">
      <project name="the sequence">
        <sequence format="r1" duration="125/25s" tcStart="0s" tcFormat="NDF">
          <spine>
            <clip ref="r2" name="the beach" offset="0s" start="50/25s" duration="125/25s"/>
          </spine>
        </sequence>
      </project>
    </event>
  </library>
</fcpxml>
"#
    .to_string()
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

/// A document is read into a **sequence**: the clip is on the spine,
/// and the media of the file is an item.
#[test]
fn a_document_is_read_into_a_sequence() {
    let mut session = session();
    let path = a_file("sequence", &a_document());
    let outcome = session
        .execute("film.import.fcpxml", &json!({ "path": path, "name": "the film" }))
        .expect("the document is read");
    assert_eq!(outcome.data["tracks"], json!(1), "there is one track: {outcome:?}");
    assert_eq!(outcome.data["clips"], json!(1), "and one clip: {outcome:?}");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let track = the_track(&session, sequence);
    assert_eq!(track.data["kind"], json!("video"), "the spine is a video track");
    let clip = the_clip(&track);
    assert_eq!(clip.data["start"], json!(0), "the clip begins at the start");
    assert_eq!(clip.data["duration"], json!(125), "and lasts as long as the text says");
    assert_eq!(clip.data["source_in"], json!(50), "and plays from fifty frames in");
}

/// The **rate** of the sequence is the rate of its format.
#[test]
fn the_rate_is_the_formats_own() {
    let mut session = session();
    let path = a_file("rate", &a_document());
    let outcome =
        session.execute("film.import.fcpxml", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let rate = session
        .execute("film.sequence.rate.get", &json!({ "sequence": sequence }))
        .expect("the rate is read");
    assert_eq!(rate.data["rate"], json!({ "num": 25, "den": 1 }), "the rate is the format's: {rate:?}");
}

/// The **media** of a clip is an item of the bin, with the file of the
/// asset.
#[test]
fn the_media_is_an_item() {
    let mut session = session();
    let path = a_file("media", &a_document());
    let outcome =
        session.execute("film.import.fcpxml", &json!({ "path": path })).expect("the document is read");
    let bin = outcome.data["bin"].as_u64().expect("the bin has an id");
    let node = session.document().node(bin).expect("the bin is there");
    let item = node
        .children
        .iter()
        .find(|child| child.kind == "film.item" && child.name == "the beach")
        .expect("there is an item");
    assert_eq!(item.data["path"], json!("/media/beach.mov"), "the item knows its file");
    assert_eq!(item.data["duration"], json!(500), "and how long the media is");
}

/// A **connected clip** is on a **lane**: the video lanes count up and
/// the audio lanes count down.
#[test]
fn a_connected_clip_is_on_its_lane() {
    let mut session = session();
    let text = a_document().replace(
        "            <clip ref=\"r2\" name=\"the beach\" offset=\"0s\" start=\"50/25s\" duration=\"125/25s\"/>",
        "            <clip ref=\"r2\" name=\"the beach\" offset=\"0s\" start=\"50/25s\" duration=\"125/25s\">\n              <clip ref=\"r2\" lane=\"1\" name=\"the voice\" offset=\"25/25s\" start=\"0s\" duration=\"100/25s\"/>\n              <clip ref=\"r2\" lane=\"-1\" name=\"the music\" offset=\"0s\" start=\"0s\" duration=\"125/25s\"/>\n            </clip>",
    );
    assert!(text.contains("lane=\"1\""), "the lane is in the file");
    let path = a_file("lane", &text);
    let outcome =
        session.execute("film.import.fcpxml", &json!({ "path": path })).expect("the document is read");
    assert_eq!(outcome.data["tracks"], json!(3), "the spine and the two lanes: {outcome:?}");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let node = session.document().node(sequence).expect("the sequence is there");
    let lanes: Vec<(&str, &str)> = node
        .children
        .iter()
        .filter(|child| child.kind == "film.track")
        .map(|track| {
            let clip = track.children.iter().find(|child| child.kind == "film.clip").expect("a clip");
            (track.data["kind"].as_str().unwrap_or_default(), clip.name.as_str())
        })
        .collect();
    assert!(lanes.contains(&("video", "the voice")), "the video lane is there: {lanes:?}");
    assert!(lanes.contains(&("audio", "the music")), "and the audio lane: {lanes:?}");
    // The clip of the video lane sits **over** the clip of the spine.
    let voice = node
        .children
        .iter()
        .filter(|child| child.kind == "film.track")
        .find_map(|track| {
            track.children.iter().find(|child| child.kind == "film.clip" && child.name == "the voice")
        })
        .expect("the clip is there");
    assert_eq!(voice.data["start"], json!(25), "the clip is over the spine: {voice:?}");
}

/// A file that is **not FCPXML** is refused, and so is a file that
/// cannot be read.
#[test]
fn a_file_that_is_not_fcpxml_is_refused() {
    let mut session = session();
    let path = a_file("notxml", "<fcpxml><sequence>");
    let error =
        session.execute("film.import.fcpxml", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("not FCPXML"), "the refusal says why: {error}");
    let path = a_file("otherroot", "<other><sequence/></other>");
    let error =
        session.execute("film.import.fcpxml", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("not FCPXML"), "the refusal says why: {error}");
    let path = a_file("nosequence", "<fcpxml><library><event/></library></fcpxml>");
    let error =
        session.execute("film.import.fcpxml", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("no sequence"), "the refusal says why: {error}");
    let error = session
        .execute("film.import.fcpxml", &json!({ "path": "/there/is/no/such/file.fcpxml" }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("cannot be read"), "the refusal says why: {error}");
    let error =
        session.execute("film.import.fcpxml", &json!({ "path": "  " })).expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}

/// A document that this tree wrote is read back into a sequence: the
/// **round trip**.
#[test]
fn a_document_this_tree_wrote_is_read_back() {
    let mut session = session();
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
    // The **text** of the document, and the file it is in.
    let exported = session
        .execute("film.export.fcpxml", &json!({ "sequence": sequence, "width": 1920, "height": 1080 }))
        .expect("the document is written");
    let text = exported.data["xml"].as_str().expect("the text is there");
    let path = a_file("roundtrip", text);
    // The document is **read back**.
    let mut other = crate::session();
    let outcome =
        other.execute("film.import.fcpxml", &json!({ "path": path })).expect("the document is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let track = the_track(&other, sequence);
    let clip = the_clip(&track);
    assert_eq!(clip.data["start"], json!(0), "the clip begins at the start: {clip:?}");
    assert_eq!(clip.data["duration"], json!(125), "and lasts as long as before");
    assert_eq!(clip.data["source_in"], json!(50), "and plays from the same place in the media");
}
