//! The **edit decision list** read back: a list of cuts made into a
//! sequence.
//!
//! The events of the list are clips on the tracks of a sequence, and
//! the reels of the list are items.

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
    let path = std::env::temp_dir().join(format!("craftkit-v2-edl-{name}-{}", std::process::id()));
    std::fs::write(&path, text).expect("the file is written");
    path.to_string_lossy().to_string()
}

/// A **list** with two cuts on the video track.
fn a_list() -> String {
    let mut text = String::new();
    text.push_str("TITLE: THE FILM\n");
    text.push_str("FCM: NON-DROP FRAME\n\n");
    text.push_str("001  BEACH    V     C        00:00:00:00 00:00:05:00 00:00:00:00 00:00:05:00\n");
    text.push_str("002  PIER     V     C        00:00:10:00 00:00:15:00 00:00:05:00 00:00:10:00\n");
    text
}

/// The **tracks** of the sequence that was made.
fn the_tracks(session: &Session, sequence: u64) -> Vec<std::sync::Arc<craftkit_sdk::Node>> {
    let node = session.document().node(sequence).expect("the sequence is there");
    node.children.iter().filter(|child| child.kind == "film.track").cloned().collect()
}

/// A list is read into a **sequence**: the events are clips on the
/// video track.
#[test]
fn a_list_is_read_into_a_sequence() {
    let mut session = session();
    let path = a_file("sequence", &a_list());
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    assert_eq!(outcome.data["clips"], json!(2), "there are two clips: {outcome:?}");
    assert_eq!(outcome.data["title"], json!("THE FILM"), "and the title is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let tracks = the_tracks(&session, sequence);
    assert_eq!(tracks.len(), 1, "there is one track");
}

/// The **start** of a clip is the record in, its **length** is the
/// record out less the record in, and its **source** is the source in.
#[test]
fn the_clocks_of_a_clip_come_from_the_event() {
    let mut session = session();
    let path = a_file("clocks", &a_list());
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let tracks = the_tracks(&session, sequence);
    let clips: Vec<_> = tracks[0].children.iter().filter(|child| child.kind == "film.clip").collect();
    assert_eq!(clips.len(), 2, "there are two clips");
    assert_eq!(clips[0].data["start"], json!(0), "the first begins at the start");
    assert_eq!(clips[0].data["duration"], json!(125), "and lasts five seconds");
    assert_eq!(clips[0].data["source_in"], json!(0), "and plays from the beginning of its reel");
    assert_eq!(clips[1].data["start"], json!(125), "the second begins where the first ends");
    assert_eq!(clips[1].data["source_in"], json!(250), "and plays from ten seconds in");
}

/// The **media** of a reel is an item of the bin, and two events of
/// one reel are one item.
#[test]
fn a_reel_is_an_item() {
    let mut session = session();
    let mut text = a_list();
    text.push_str("003  BEACH    V     C        00:00:20:00 00:00:25:00 00:00:10:00 00:00:15:00\n");
    let path = a_file("reel", &text);
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    let bin = outcome.data["bin"].as_u64().expect("the bin has an id");
    let node = session.document().node(bin).expect("the bin is there");
    let reels: Vec<&str> = node
        .children
        .iter()
        .filter(|child| child.kind == "film.item")
        .map(|child| child.name.as_str())
        .collect();
    assert_eq!(reels, vec!["BEACH", "PIER"], "there is one item for every reel");
}

/// An **audio channel** makes an audio track, and `AA` is **two**
/// audio channels — the tree's own rule.
#[test]
fn an_audio_channel_makes_an_audio_track() {
    let mut session = session();
    let mut text = a_list();
    text.push_str("003  BEACH    AA    C        00:00:00:00 00:00:05:00 00:00:00:00 00:00:05:00\n");
    let path = a_file("audio", &text);
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let tracks = the_tracks(&session, sequence);
    let audio: Vec<&str> = tracks
        .iter()
        .filter(|track| track.data["kind"] == json!("audio"))
        .map(|track| track.name.as_str())
        .collect();
    assert_eq!(audio, vec!["the audio 1", "the audio 2"], "the two channels are two tracks");
}

/// A **dissolve** is not a cut: it is a **transition** over the cut
/// before it, and it is read as one.
#[test]
fn a_dissolve_is_a_transition_over_the_cut() {
    let mut session = session();
    let mut text = a_list();
    text.push_str("003  BEACH    V     D    025 00:00:05:00 00:00:10:00 00:00:10:00 00:00:15:00\n");
    let path = a_file("dissolve", &text);
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert!(left_out.is_empty(), "the dissolve is not left out: {outcome:?}");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let tracks = the_tracks(&session, sequence);
    let video = tracks.iter().find(|track| track.data["kind"] == json!("video")).expect("there is a video");
    let transitions: Vec<Json> = video
        .children
        .iter()
        .filter(|child| child.kind == "film.transition")
        .map(|child| child.data.clone())
        .collect();
    assert_eq!(transitions.len(), 1, "there is one transition: {transitions:?}");
    assert_eq!(transitions[0]["duration"], json!(25), "it is twenty-five frames long: {transitions:?}");
    assert_eq!(transitions[0]["start"], json!(250), "and it starts at the cut: {transitions:?}");
    assert_eq!(transitions[0]["align"], json!("start"), "and it starts at the cut: {transitions:?}");
    assert_eq!(transitions[0]["effect"], json!("cross_dissolve"), "and it is a dissolve: {transitions:?}");
}

/// An edit that is **not a cut and not a dissolve** — a wipe, a key —
/// is left out, and said.
#[test]
fn an_edit_that_is_neither_a_cut_nor_a_dissolve_is_left_out() {
    let mut session = session();
    let mut text = a_list();
    text.push_str("003  BEACH    V     W    025 00:00:05:00 00:00:10:00 00:00:10:00 00:00:15:00\n");
    let path = a_file("wipe", &text);
    let outcome = session.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    let left_out = outcome.data["left_out"].as_array().expect("the left out are a list");
    assert_eq!(left_out.len(), 1, "the edit is said: {outcome:?}");
    assert!(left_out[0].as_str().unwrap_or_default().contains("not a cut"), "and it is named");
}

/// A file that is **not a list** is refused, and so is a file that
/// cannot be read.
#[test]
fn a_file_that_is_not_a_list_is_refused() {
    let mut session = session();
    let path = a_file("notlist", "this is not a list of cuts\n");
    let error =
        session.execute("film.import.edl", &json!({ "path": path })).expect_err("the file is refused");
    assert!(error.to_string().contains("no event"), "the refusal says why: {error}");
    let error = session
        .execute("film.import.edl", &json!({ "path": "/there/is/no/such/file.edl" }))
        .expect_err("the file is refused");
    assert!(error.to_string().contains("cannot be read"), "the refusal says why: {error}");
    let error = session.execute("film.import.edl", &json!({ "path": "  " })).expect_err("a file has a path");
    assert!(error.to_string().contains("has a path"), "the refusal says why: {error}");
}

/// A list that this tree wrote is read back into a sequence: the
/// **round trip**.
#[test]
fn a_list_this_tree_wrote_is_read_back() {
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
    // The **text** of the list, and the file it is in.
    let exported =
        session.execute("film.export.edl", &json!({ "sequence": sequence })).expect("the list is written");
    let text = exported.data["edl"].as_str().expect("the text is there");
    let path = a_file("roundtrip", text);
    // The list is **read back**.
    let mut other = crate::session();
    let outcome = other.execute("film.import.edl", &json!({ "path": path })).expect("the list is read");
    let sequence = outcome.data["sequence"].as_u64().expect("the sequence has an id");
    let tracks = the_tracks(&other, sequence);
    let clip = tracks[0].children.iter().find(|child| child.kind == "film.clip").expect("a clip");
    assert_eq!(clip.data["start"], json!(0), "the clip begins at the start: {clip:?}");
    assert_eq!(clip.data["duration"], json!(125), "and lasts as long as before");
    assert_eq!(clip.data["source_in"], json!(50), "and plays from the same place in the media");
}
